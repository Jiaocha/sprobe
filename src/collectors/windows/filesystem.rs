//! Windows 文件系统容量与卷信息采集

use crate::snapshot::{FilesystemItem, FilesystemsSection, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows_sys::Win32::Foundation::MAX_PATH;
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};

const DRIVE_UNKNOWN: u32 = 0;
const DRIVE_NO_ROOT_DIR: u32 = 1;
const DRIVE_CDROM: u32 = 5;
const FILE_READ_ONLY_VOLUME: u32 = 0x0008_0000;

#[derive(Debug, Default)]
pub struct WindowsFilesystemCollector;

impl WindowsFilesystemCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<FilesystemsSection, String> {
        let mut buffer = [0u16; 512];
        let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };
        if len == 0 {
            return Err("GetLogicalDriveStringsW 调用失败".to_string());
        }

        let mut items = Vec::new();
        let mut offset = 0;

        while offset < len as usize && buffer[offset] != 0 {
            let mut end = offset;
            while end < len as usize && buffer[end] != 0 {
                end += 1;
            }

            let drive_slice = &buffer[offset..end];
            let drive_str = OsString::from_wide(drive_slice)
                .to_string_lossy()
                .to_string();
            offset = end + 1;

            // 获取驱动器类型
            let drive_type = unsafe { GetDriveTypeW(drive_slice.as_ptr()) };
            if drive_type == DRIVE_CDROM
                || drive_type == DRIVE_UNKNOWN
                || drive_type == DRIVE_NO_ROOT_DIR
            {
                continue;
            }

            // 获取卷与文件系统信息
            let mut volume_name = [0u16; (MAX_PATH + 1) as usize];
            let mut fs_name = [0u16; (MAX_PATH + 1) as usize];
            let mut serial_number = 0u32;
            let mut max_component_len = 0u32;
            let mut flags = 0u32;

            let vol_ok = unsafe {
                GetVolumeInformationW(
                    drive_slice.as_ptr(),
                    volume_name.as_mut_ptr(),
                    volume_name.len() as u32,
                    &mut serial_number,
                    &mut max_component_len,
                    &mut flags,
                    fs_name.as_mut_ptr(),
                    fs_name.len() as u32,
                )
            };

            let fstype = if vol_ok != 0 {
                let fs_len = fs_name
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(fs_name.len());
                OsString::from_wide(&fs_name[..fs_len])
                    .to_string_lossy()
                    .to_string()
            } else {
                "NTFS".to_string()
            };

            let read_only = (flags & FILE_READ_ONLY_VOLUME) != 0;

            // 获取容量空间
            let mut free_available = 0u64;
            let mut total_bytes = 0u64;
            let mut total_free = 0u64;

            let space_ok = unsafe {
                GetDiskFreeSpaceExW(
                    drive_slice.as_ptr(),
                    &mut free_available,
                    &mut total_bytes,
                    &mut total_free,
                )
            };

            if space_ok != 0 && total_bytes > 0 {
                let available_bytes = free_available;
                let free_bytes = total_free;
                let used_bytes = total_bytes.saturating_sub(free_bytes);
                let reserved_bytes = free_bytes.saturating_sub(available_bytes);
                let usage_ratio = (used_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0);

                let mountpoint = drive_str.clone();
                let device = drive_str.trim_end_matches('\\').to_string();

                items.push(FilesystemItem {
                    device,
                    volume_id: if serial_number != 0 {
                        Some(format!("{serial_number:08X}"))
                    } else {
                        None
                    },
                    mountpoint: mountpoint.clone(),
                    mountpoints: vec![mountpoint],
                    fstype,
                    read_only,
                    total_bytes,
                    used_bytes,
                    free_bytes,
                    available_bytes,
                    reserved_bytes,
                    usage_ratio,
                    inodes_total: 0,
                    inodes_used: 0,
                    inodes_free: 0,
                    inodes_usage_ratio: 0.0,
                    stale: false,
                    quality: Quality::Measured,
                });
            }
        }

        // 按 mountpoint 排序以保证稳定输出
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
