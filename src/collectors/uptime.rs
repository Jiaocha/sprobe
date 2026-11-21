//! 开机时间 (UptimeSection) 采集

use crate::snapshot::{Quality, SectionMeta, SectionStatus, UptimeSection};
use chrono::{DateTime, Utc};
use std::time::Instant;

pub struct UptimeCollector {
    agent_start_instant: Instant,
}

impl Default for UptimeCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl UptimeCollector {
    pub fn new() -> Self {
        Self {
            agent_start_instant: Instant::now(),
        }
    }

    pub fn collect(&self) -> Result<(UptimeSection, DateTime<Utc>), String> {
        let now = Utc::now();
        let agent_seconds = self.agent_start_instant.elapsed().as_secs_f64();

        #[cfg(windows)]
        let (host_seconds, host_boot_time) = {
            let millis = unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() };
            let secs = millis as f64 / 1000.0;
            let boot = now - chrono::Duration::milliseconds(millis as i64);
            (secs, boot)
        };

        #[cfg(target_os = "linux")]
        let (host_seconds, host_boot_time) = {
            let content = std::fs::read_to_string("/proc/uptime")
                .map_err(|e| format!("读取 /proc/uptime 失败: {e}"))?;
            let secs = content
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            let boot = now - chrono::Duration::milliseconds((secs * 1000.0) as i64);
            (secs, boot)
        };

        #[cfg(not(any(windows, target_os = "linux")))]
        let (host_seconds, host_boot_time) = (agent_seconds, now);

        let section = UptimeSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            host_seconds,
            host_boot_time,
            agent_seconds,
            clock_adjusted: false,
        };

        Ok((section, host_boot_time))
    }
}
