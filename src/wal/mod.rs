use crate::snapshot::Snapshot;
use crc32fast::Hasher;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

const WAL_MAGIC: [u8; 4] = *b"SPW1";
const WAL_VERSION: u16 = 1;
const DEFAULT_SEGMENT_MAX_BYTES: u64 = 8 * 1024 * 1024; // 8 MiB

#[derive(Debug, Error)]
pub enum WalError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 序列化/反序列化错误: {0}")]
    Json(#[from] serde_json::Error),
    #[error("WAL 记录魔数不匹配: 期望 SPW1")]
    InvalidMagic,
    #[error("WAL 记录 CRC32C 校验失败")]
    CrcMismatch,
    #[error("WAL 分段文件损坏")]
    CorruptedSegment,
}

fn parse_segment_name(name: &str) -> Option<u64> {
    if name.starts_with("segment_") && name.ends_with(".wal") && name.len() >= 12 {
        name[8..name.len() - 4].parse::<u64>().ok()
    } else {
        None
    }
}

pub struct WalManager {
    dir: PathBuf,
    max_bytes: usize,
    fsync_mode: String,
    current_segment_id: u64,
    current_file: Option<File>,
    current_file_bytes: u64,
    acked_sequence: u64,
}

impl WalManager {
    pub fn open<P: AsRef<Path>>(
        dir: P,
        max_bytes: usize,
        fsync_mode: &str,
    ) -> Result<Self, WalError> {
        let dir_buf = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir_buf)?;

        // 读取持久化的 acked_sequence 水位
        let ack_file = dir_buf.join("acked_sequence");
        let acked_sequence = if ack_file.exists() {
            fs::read_to_string(&ack_file)
                .ok()
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0)
        } else {
            0
        };

        let mut mgr = Self {
            dir: dir_buf,
            max_bytes,
            fsync_mode: fsync_mode.to_string(),
            current_segment_id: 1,
            current_file: None,
            current_file_bytes: 0,
            acked_sequence,
        };

        // 查找最新分段
        mgr.init_current_segment()?;
        Ok(mgr)
    }

    fn segment_path(&self, id: u64) -> PathBuf {
        self.dir.join(format!("segment_{id:08}.wal"))
    }

    fn init_current_segment(&mut self) -> Result<(), WalError> {
        let mut max_id = 1;
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = parse_segment_name(&name)
                && id > max_id
            {
                max_id = id;
            }
        }
        self.current_segment_id = max_id;
        self.open_current_segment()?;
        Ok(())
    }

    fn open_current_segment(&mut self) -> Result<(), WalError> {
        let path = self.segment_path(self.current_segment_id);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        self.current_file_bytes = file.metadata()?.len();
        self.current_file = Some(file);
        Ok(())
    }

    pub fn append_snapshot(&mut self, snapshot: &Snapshot) -> Result<(), WalError> {
        // 如果当前段达到上限，滚动至新段
        if self.current_file_bytes >= DEFAULT_SEGMENT_MAX_BYTES {
            self.current_segment_id += 1;
            self.open_current_segment()?;
        }

        let payload = serde_json::to_vec(snapshot)?;
        let len = payload.len() as u32;

        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let crc = hasher.finalize();

        let mut record = Vec::with_capacity(4 + 2 + 4 + 4 + payload.len());
        record.extend_from_slice(&WAL_MAGIC);
        record.extend_from_slice(&WAL_VERSION.to_le_bytes());
        record.extend_from_slice(&len.to_le_bytes());
        record.extend_from_slice(&crc.to_le_bytes());
        record.extend_from_slice(&payload);

        let file = self.current_file.as_mut().ok_or_else(|| {
            WalError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "WAL 文件未打开",
            ))
        })?;

        file.write_all(&record)?;
        self.current_file_bytes += record.len() as u64;

        if self.fsync_mode == "always" {
            file.sync_data()?;
        }

        // 检查目录总配额，如果超限则淘汰最旧段
        self.enforce_quota()?;
        Ok(())
    }

    pub fn sync(&mut self) -> Result<(), WalError> {
        if self.fsync_mode == "never" {
            return Ok(());
        }
        if let Some(ref mut file) = self.current_file {
            file.sync_data()?;
        }
        Ok(())
    }

    pub fn update_ack_watermark(&mut self, sequence: u64) -> Result<(), WalError> {
        if sequence > self.acked_sequence {
            self.acked_sequence = sequence;
            let ack_file = self.dir.join("acked_sequence");
            fs::write(ack_file, sequence.to_string())?;
        }
        Ok(())
    }

    pub fn acked_sequence(&self) -> u64 {
        self.acked_sequence
    }

    pub fn pending_count(&self) -> u64 {
        0
    }

    pub fn recover_snapshots(&self) -> Result<Vec<Snapshot>, WalError> {
        let mut snapshots = Vec::new();

        // 查找所有段文件并按序号升序排序
        let mut segments = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = parse_segment_name(&name) {
                segments.push((id, entry.path()));
            }
        }
        segments.sort_by_key(|(id, _)| *id);

        for (_id, path) in segments {
            let mut file = File::open(&path)?;
            let total_len = file.metadata()?.len();
            let mut offset = 0;

            while offset + 14 <= total_len {
                file.seek(SeekFrom::Start(offset))?;

                let mut header = [0u8; 14];
                if file.read_exact(&mut header).is_err() {
                    break;
                }

                if header[0..4] != WAL_MAGIC {
                    break;
                }

                let _ver = u16::from_le_bytes([header[4], header[5]]);
                let len = u32::from_le_bytes([header[6], header[7], header[8], header[9]]) as usize;
                let expected_crc =
                    u32::from_le_bytes([header[10], header[11], header[12], header[13]]);

                if offset + 14 + len as u64 > total_len {
                    // 尾部截断
                    break;
                }

                let mut payload = vec![0u8; len];
                if file.read_exact(&mut payload).is_err() {
                    break;
                }

                let mut hasher = Hasher::new();
                hasher.update(&payload);
                if hasher.finalize() != expected_crc {
                    // CRC 损坏
                    break;
                }

                if let Ok(snap) = serde_json::from_slice::<Snapshot>(&payload)
                    && snap.sequence > self.acked_sequence
                {
                    snapshots.push(snap);
                }

                offset += 14 + len as u64;
            }
        }

        // 按照序号排序，重复项去重
        snapshots.sort_by_key(|s| s.sequence);
        snapshots.dedup_by_key(|s| s.sequence);

        Ok(snapshots)
    }

    fn enforce_quota(&mut self) -> Result<(), WalError> {
        let mut total_size = 0u64;
        let mut segments = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = parse_segment_name(&name) {
                let len = entry.metadata()?.len();
                total_size += len;
                segments.push((id, entry.path(), len));
            }
        }

        segments.sort_by_key(|(id, _, _)| *id);

        // 如果超出最大字节限制，按最旧段依次淘汰（保留当前活动段）
        while total_size > self.max_bytes as u64 && segments.len() > 1 {
            let (oldest_id, oldest_path, len) = segments.remove(0);
            if oldest_id < self.current_segment_id {
                let _ = fs::remove_file(&oldest_path);
                total_size = total_size.saturating_sub(len);
            }
        }

        Ok(())
    }
}
