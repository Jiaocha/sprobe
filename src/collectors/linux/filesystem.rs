//! Linux /proc/mounts 与文件系统容量/inode 采集与解析

use crate::snapshot::{FilesystemItem, Quality};
#[cfg(target_os = "linux")]
use crate::snapshot::{FilesystemsSection, SectionMeta, SectionStatus};
#[cfg(target_os = "linux")]
use chrono::Utc;
use std::collections::HashSet;

/// 被默认过滤掉的虚拟伪文件系统类型
const IGNORED_FSTYPES: &[&str] = &[
    "proc",
    "sysfs",
    "devpts",
    "cgroup",
    "cgroup2",
    "pstore",
    "bpf",
    "autofs",
    "mqueue",
    "hugetlbfs",
    "debugfs",
    "tracefs",
    "fusectl",
    "configfs",
    "ramfs",
    "binfmt_misc",
    "securityfs",
    "efivarfs",
    "devtmpfs",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMountEntry {
    pub device: String,
    pub mountpoint: String,
    pub fstype: String,
    pub options: Vec<String>,
}

/// 解析 /proc/mounts 文本行
pub fn parse_mounts(content: &str) -> Vec<RawMountEntry> {
    let mut mounts = Vec::new();
    let mut seen_mountpoints = HashSet::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let device = parts[0].to_string();
        let mountpoint = parts[1].to_string();
        let fstype = parts[2].to_string();
        let options = parts[3]
            .split(',')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        // 过滤常见的虚拟伪文件系统
        if IGNORED_FSTYPES.contains(&fstype.as_str()) {
            continue;
        }

        // 避免重复挂载点
        if seen_mountpoints.contains(&mountpoint) {
            continue;
        }
        seen_mountpoints.insert(mountpoint.clone());

        mounts.push(RawMountEntry {
            device,
            mountpoint,
            fstype,
            options,
        });
    }

    mounts
}

/// Linux 文件系统采集器
#[derive(Debug, Default)]
pub struct LinuxFilesystemCollector;

impl LinuxFilesystemCollector {
    pub fn new() -> Self {
        Self
    }

    /// 使用挂载点信息构造文件系统项（配合模拟数据或 statvfs）
    pub fn build_items_from_mounts(&self, mounts: &[RawMountEntry]) -> Vec<FilesystemItem> {
        let mut items = Vec::new();

        for m in mounts {
            let read_only = m.options.iter().any(|opt| opt == "ro");

            items.push(FilesystemItem {
                device: m.device.clone(),
                volume_id: None,
                mountpoint: m.mountpoint.clone(),
                mountpoints: vec![m.mountpoint.clone()],
                fstype: m.fstype.clone(),
                read_only,
                total_bytes: 0,
                used_bytes: 0,
                free_bytes: 0,
                available_bytes: 0,
                reserved_bytes: 0,
                usage_ratio: 0.0,
                inodes_total: 0,
                inodes_used: 0,
                inodes_free: 0,
                inodes_usage_ratio: 0.0,
                stale: false,
                quality: Quality::Measured,
            });
        }

        items.sort_by(|a, b| a.mountpoint.cmp(&b.mountpoint));
        items
    }

    #[cfg(target_os = "linux")]
    pub fn collect(&self) -> Result<FilesystemsSection, String> {
        let content = std::fs::read_to_string("/proc/mounts")
            .map_err(|e| format!("读取 /proc/mounts 失败: {e}"))?;
        let mounts = parse_mounts(&content);
        let mut items = Vec::new();

        for m in mounts {
            let read_only = m.options.iter().any(|opt| opt == "ro");
            let c_path =
                std::ffi::CString::new(m.mountpoint.as_str()).map_err(|e| e.to_string())?;

            let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
            let res = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
            if res != 0 {
                continue;
            }

            let block_size = if stat.f_frsize > 0 {
                stat.f_frsize
            } else {
                stat.f_bsize
            } as u64;
            let total_bytes = stat.f_blocks as u64 * block_size;
            let free_bytes = stat.f_bfree as u64 * block_size;
            let available_bytes = stat.f_bavail as u64 * block_size;
            let used_bytes = total_bytes.saturating_sub(free_bytes);
            let reserved_bytes = free_bytes.saturating_sub(available_bytes);
            let usage_ratio = if total_bytes > 0 {
                (used_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let inodes_total = stat.f_files as u64;
            let inodes_free = stat.f_ffree as u64;
            let inodes_used = inodes_total.saturating_sub(inodes_free);
            let inodes_usage_ratio = if inodes_total > 0 {
                (inodes_used as f64 / inodes_total as f64).clamp(0.0, 1.0)
            } else {
                0.0
            };

            items.push(FilesystemItem {
                device: m.device,
                volume_id: None,
                mountpoint: m.mountpoint.clone(),
                mountpoints: vec![m.mountpoint],
                fstype: m.fstype,
                read_only,
                total_bytes,
                used_bytes,
                free_bytes,
                available_bytes,
                reserved_bytes,
                usage_ratio,
                inodes_total,
                inodes_used,
                inodes_free,
                inodes_usage_ratio,
                stale: false,
                quality: Quality::Measured,
            });
        }

        items.sort_by(|a, b| a.mountpoint.cmp(&b.mountpoint));

        Ok(FilesystemsSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: Utc::now(),
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            truncated: false,
            items,
        })
    }
}
