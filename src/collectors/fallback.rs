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
            quality: Quality::Degraded,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(CpuSection {
            meta,
            total: CpuStat {
                usage_ratio: 0.05,
                user_ratio: 0.03,
                system_ratio: 0.02,
                iowait_ratio: None,
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
            quality: Quality::Degraded,
            sampled_at: Utc::now(),
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        Ok(MemorySection {
            meta,
            total_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: 4 * 1024 * 1024 * 1024,
            used_bytes: 4 * 1024 * 1024 * 1024,
            usage_ratio: 0.5,
            swap_total_bytes: None,
            swap_used_bytes: None,
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
            quality: Quality::Degraded,
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
            quality: Quality::Degraded,
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
