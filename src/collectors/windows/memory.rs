#[cfg(windows)]
use crate::snapshot::{MemorySection, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use std::mem::size_of;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub struct WindowsMemoryCollector;

impl Default for WindowsMemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsMemoryCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<MemorySection, String> {
        let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;

        let ret = unsafe { GlobalMemoryStatusEx(&mut status) };
        if ret == 0 {
            return Err("GlobalMemoryStatusEx 调用失败".to_string());
        }

        let total_bytes = status.ullTotalPhys;
        let available_bytes = status.ullAvailPhys;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_ratio = if total_bytes > 0 {
            used_bytes as f64 / total_bytes as f64
        } else {
            0.0
        };

        // 页面文件视作 Swap
        let swap_total_bytes = status.ullTotalPageFile.saturating_sub(total_bytes);
        let page_used = status
            .ullTotalPageFile
            .saturating_sub(status.ullAvailPageFile);
        let swap_used_bytes = page_used.saturating_sub(used_bytes);
        let swap_free_bytes = swap_total_bytes.saturating_sub(swap_used_bytes);
        let swap_usage_ratio = if swap_total_bytes > 0 {
            Some(swap_used_bytes as f64 / swap_total_bytes as f64)
        } else {
            None
        };

        let now = Utc::now();
        Ok(MemorySection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            available_source: "windows_globalmemorystatus".to_string(),
            total_bytes,
            available_bytes,
            used_bytes,
            free_bytes: available_bytes,
            cached_bytes: 0,
            buffers_bytes: 0,
            usage_ratio,
            dirty_bytes: 0,
            writeback_bytes: 0,
            swap_total_bytes,
            swap_used_bytes,
            swap_free_bytes,
            swap_usage_ratio,
        })
    }
}
