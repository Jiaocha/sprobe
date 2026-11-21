use crate::snapshot::*;
use chrono::Utc;

pub struct FallbackCpuCollector;

impl FallbackCpuCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&mut self) -> Result<CpuSection, String> {
        let meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Estimated,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(CpuSection {
            meta,
            logical_count: 2,
            physical_count: 1,
            total: CpuTotal {
                usage_ratio: 0.05,
                idle_ratio: 0.95,
                iowait_ratio: 0.0,
                steal_ratio: 0.0,
                user_seconds_total: 10.0,
                nice_seconds_total: 0.0,
                system_seconds_total: 5.0,
                idle_seconds_total: 200.0,
                iowait_seconds_total: 0.0,
                irq_seconds_total: 0.0,
                softirq_seconds_total: 0.0,
                steal_seconds_total: 0.0,
            },
            cores: Vec::new(),
        })
    }
}

pub struct FallbackMemoryCollector;

impl FallbackMemoryCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<MemorySection, String> {
        let meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Estimated,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(MemorySection {
            meta,
            available_source: "fallback".to_string(),
            total_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: 4 * 1024 * 1024 * 1024,
            used_bytes: 4 * 1024 * 1024 * 1024,
            free_bytes: 4 * 1024 * 1024 * 1024,
            cached_bytes: 0,
            buffers_bytes: 0,
            usage_ratio: 0.5,
            dirty_bytes: 0,
            writeback_bytes: 0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            swap_free_bytes: 0,
            swap_usage_ratio: Some(0.0),
        })
    }
}

pub struct FallbackFilesystemCollector;

impl FallbackFilesystemCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self) -> Result<FilesystemsSection, String> {
        let meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Estimated,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(FilesystemsSection {
            meta,
            truncated: false,
            items: Vec::new(),
        })
    }
}

pub struct FallbackProcessCollector;

impl FallbackProcessCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(&self, _top_n: usize, _watch: &[String]) -> Result<ProcessesSection, String> {
        let meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Estimated,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(ProcessesSection {
            meta,
            visibility: "full".to_string(),
            count: 0,
            running_count: 0,
            sleeping_count: 0,
            blocked_count: 0,
            zombie_count: 0,
            thread_count: 0,
            truncated: false,
            top: Vec::new(),
            watch: Vec::new(),
        })
    }
}
