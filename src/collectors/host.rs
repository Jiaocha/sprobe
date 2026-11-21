//! 主机元信息 (HostMeta) 采集

use crate::snapshot::HostMeta;
use chrono::{DateTime, Utc};

/// 采集当前宿主机元信息
pub fn collect_host_meta(boot_time: DateTime<Utc>) -> HostMeta {
    let hostname = match std::env::var("COMPUTERNAME") {
        Ok(h) => h,
        Err(_) => std::env::var("HOSTNAME").unwrap_or_else(|_| "localhost".to_string()),
    };

    #[cfg(windows)]
    let (os_name, kernel) = { ("Windows".to_string(), "NT".to_string()) };

    #[cfg(target_os = "linux")]
    let (os_name, kernel) = {
        let os_str = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|c| {
                for line in c.lines() {
                    if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                        return Some(val.trim_matches('"').to_string());
                    }
                }
                None
            })
            .unwrap_or_else(|| "Linux".to_string());

        let kernel_str = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        (os_str, kernel_str)
    };

    #[cfg(not(any(windows, target_os = "linux")))]
    let (os_name, kernel) = (std::env::consts::OS.to_string(), "unknown".to_string());

    HostMeta {
        hostname,
        os_name,
        kernel,
        architecture: std::env::consts::ARCH.to_string(),
        virtualization: None,
        cpu_model: None,
        boot_time,
    }
}
