use crate::config::SprobeConfig;
use crate::snapshot::Capabilities;
use std::path::Path;

#[derive(serde::Serialize)]
pub struct WalDiagnostic {
    pub dir: String,
    pub max_bytes: usize,
    pub segment_count: usize,
    pub pending_count: u64,
    pub disk_usage_bytes: u64,
}

#[derive(serde::Serialize)]
pub struct DiagnosePackage {
    pub agent_version: String,
    pub commit: String,
    pub os: String,
    pub arch: String,
    pub generated_at: String,
    pub redacted_config_summary: serde_json::Value,
    pub capabilities: Capabilities,
    pub wal: WalDiagnostic,
}

pub fn generate_diagnose_package(
    config: &SprobeConfig,
    wal_dir: &Path,
) -> Result<DiagnosePackage, Box<dyn std::error::Error>> {
    let now = chrono::Utc::now().to_rfc3339();

    // 统计 WAL 目录信息
    let mut segment_count = 0;
    let mut disk_usage_bytes = 0;
    let mut pending_count = 0;

    if wal_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(wal_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    disk_usage_bytes += meta.len();
                    if let Some(name) = entry.file_name().to_str()
                        && name.starts_with("segment_")
                        && name.ends_with(".wal")
                    {
                        segment_count += 1;
                    }
                }
            }
        }

        // 尝试只读读取 WAL 待处理数量
        if let Ok(wal) =
            crate::wal::WalManager::open(wal_dir, config.storage.wal.max_bytes, "never")
        {
            pending_count = wal.pending_count();
        }
    }

    let wal_diag = WalDiagnostic {
        dir: wal_dir.display().to_string(),
        max_bytes: config.storage.wal.max_bytes,
        segment_count,
        pending_count,
        disk_usage_bytes,
    };

    // 配置摘要（脱敏）
    let config_summary = serde_json::json!({
        "schema_version": config.schema_version,
        "hostname": config.agent.hostname,
        "fast_interval_seconds": config.collect.fast_interval_seconds,
        "push_enabled": config.output.push.enabled,
        "push_endpoint": config.output.push.endpoint,
        "heartbeat_endpoint": config.output.push.heartbeat_endpoint,
        "token_configured": !config.output.push.token_file.is_empty(),
        "prometheus_enabled": config.output.prometheus.enabled,
        "prometheus_listen": config.output.prometheus.listen,
        "health_enabled": config.health.enabled,
    });

    let package = DiagnosePackage {
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        commit: "dev".to_string(),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        generated_at: now,
        redacted_config_summary: config_summary,
        capabilities: Capabilities::default(),
        wal: wal_diag,
    };

    Ok(package)
}
