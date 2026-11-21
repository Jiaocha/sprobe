//! Linux /proc/meminfo 内存与交换分区指标采集与解析

use crate::snapshot::{MemorySection, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use std::collections::HashMap;

/// 解析 /proc/meminfo 文本内容，返回键值映射（单位：KiB）
pub fn parse_proc_meminfo(content: &str) -> HashMap<String, u64> {
    let mut map = HashMap::new();
    for line in content.lines() {
        if let Some((key, val_part)) = line.split_once(':') {
            let key = key.trim().to_string();
            let val = val_part
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            map.insert(key, val);
        }
    }
    map
}

/// Linux 内存采集器
#[derive(Debug, Default)]
pub struct LinuxMemoryCollector;

impl LinuxMemoryCollector {
    pub fn new() -> Self {
        Self
    }

    /// 从 /proc/meminfo 文本内容解析并构造 MemorySection
    pub fn collect_from_content(&self, content: &str) -> Result<MemorySection, String> {
        let mem = parse_proc_meminfo(content);

        let total_kib = *mem.get("MemTotal").unwrap_or(&0);
        let free_kib = *mem.get("MemFree").unwrap_or(&0);
        let buffers_kib = *mem.get("Buffers").unwrap_or(&0);
        let cached_kib = *mem.get("Cached").unwrap_or(&0);
        let sreclaimable_kib = *mem.get("SReclaimable").unwrap_or(&0);
        let shmem_kib = *mem.get("Shmem").unwrap_or(&0);
        let dirty_kib = *mem.get("Dirty").unwrap_or(&0);
        let writeback_kib = *mem.get("Writeback").unwrap_or(&0);
        let swap_total_kib = *mem.get("SwapTotal").unwrap_or(&0);
        let swap_free_kib = *mem.get("SwapFree").unwrap_or(&0);

        if total_kib == 0 {
            return Err("/proc/meminfo 中缺少有效的 MemTotal 字段".to_string());
        }

        let (available_bytes, available_source) = if let Some(&avail_kib) = mem.get("MemAvailable")
        {
            (avail_kib * 1024, "kernel_memavailable".to_string())
        } else {
            // Linux 3.14 之前内核回退计算公式：
            // available = (free + buffers + cached + sreclaimable) - shmem
            let sum_kib = free_kib + buffers_kib + cached_kib + sreclaimable_kib;
            let fallback_kib = sum_kib.saturating_sub(shmem_kib);
            (fallback_kib * 1024, "fallback_calculation".to_string())
        };

        let total_bytes = total_kib * 1024;
        let free_bytes = free_kib * 1024;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let usage_ratio = (used_bytes as f64 / total_bytes as f64).clamp(0.0, 1.0);

        let swap_total_bytes = swap_total_kib * 1024;
        let swap_free_bytes = swap_free_kib * 1024;
        let swap_used_bytes = swap_total_bytes.saturating_sub(swap_free_bytes);
        let swap_usage_ratio = if swap_total_bytes > 0 {
            Some((swap_used_bytes as f64 / swap_total_bytes as f64).clamp(0.0, 1.0))
        } else {
            None
        };

        Ok(MemorySection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: Utc::now(),
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            available_source,
            total_bytes,
            available_bytes,
            used_bytes,
            free_bytes,
            cached_bytes: cached_kib * 1024,
            buffers_bytes: buffers_kib * 1024,
            usage_ratio,
            dirty_bytes: dirty_kib * 1024,
            writeback_bytes: writeback_kib * 1024,
            swap_total_bytes,
            swap_used_bytes,
            swap_free_bytes,
            swap_usage_ratio,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn collect(&self) -> Result<MemorySection, String> {
        let content = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("读取 /proc/meminfo 失败: {e}"))?;
        self.collect_from_content(&content)
    }
}
