//! 本地健康评估引擎与状态机（文档 3.6 节与 4.11 节）

use crate::config::HealthConfig;
use crate::snapshot::{
    CpuSection, FilesystemsSection, HealthCheckItem, HealthSection, HealthStatus, MemorySection,
    ProcessesSection,
};
use chrono::{DateTime, Utc};

/// 健康引擎上下文状态
pub struct HealthEngine {
    config: HealthConfig,
    current_status: HealthStatus,
    status_since: DateTime<Utc>,
    recovered_at: Option<DateTime<Utc>>,
    consecutive_ok_cycles: usize,
}

impl HealthEngine {
    pub fn new(config: HealthConfig) -> Self {
        let now = Utc::now();
        Self {
            config,
            current_status: HealthStatus::Ok,
            status_since: now,
            recovered_at: None,
            consecutive_ok_cycles: 0,
        }
    }

    /// 针对当前快照分区执行健康评估
    pub fn evaluate(
        &mut self,
        cpu: &CpuSection,
        memory: &MemorySection,
        filesystems: &FilesystemsSection,
        processes: &ProcessesSection,
        wal_snapshots_count: u64,
        push_auth_failed: bool,
    ) -> HealthSection {
        self.evaluate_with_network(
            cpu,
            memory,
            filesystems,
            processes,
            None,
            wal_snapshots_count,
            push_auth_failed,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_with_network(
        &mut self,
        cpu: &CpuSection,
        memory: &MemorySection,
        filesystems: &FilesystemsSection,
        processes: &ProcessesSection,
        network: Option<&crate::snapshot::NetworkSection>,
        wal_snapshots_count: u64,
        push_auth_failed: bool,
    ) -> HealthSection {
        if !self.config.enabled {
            return HealthSection {
                status: HealthStatus::Ok,
                status_code: 0,
                since: None,
                recovered_at: None,
                checks: Vec::new(),
            };
        }

        let now = Utc::now();
        let mut checks = Vec::new();
        let thresholds = &self.config.thresholds;

        // 1. push_auth_failed: 最近一次推送 401/403 立即锁存 critical
        if push_auth_failed {
            checks.push(HealthCheckItem {
                id: "push_auth_failed".to_string(),
                status: HealthStatus::Critical,
                severity: "critical".to_string(),
                message: "推送认证失败 (401/403)，需检查 Ingest Token".to_string(),
                observed: Some(1.0),
                threshold: Some(0.0),
                since: now,
            });
        }

        // 2. cpu_saturation: cpu_usage_ratio > 0.90 且 iowait < 0.20
        if cpu.total.usage_ratio > thresholds.cpu_usage_ratio
            && cpu.total.iowait_ratio < thresholds.cpu_iowait_ratio
        {
            checks.push(HealthCheckItem {
                id: "cpu_saturation".to_string(),
                status: HealthStatus::Degraded,
                severity: "warning".to_string(),
                message: format!("CPU 使用率饱和 ({:.1}%)", cpu.total.usage_ratio * 100.0),
                observed: Some(cpu.total.usage_ratio),
                threshold: Some(thresholds.cpu_usage_ratio),
                since: now,
            });
        }

        // 3. memory_pressure: > 0.95 critical; > 0.90 warning
        if memory.usage_ratio > thresholds.memory_usage_critical {
            checks.push(HealthCheckItem {
                id: "memory_pressure".to_string(),
                status: HealthStatus::Critical,
                severity: "critical".to_string(),
                message: format!("物理内存严重不足 ({:.1}%)", memory.usage_ratio * 100.0),
                observed: Some(memory.usage_ratio),
                threshold: Some(thresholds.memory_usage_critical),
                since: now,
            });
        } else if memory.usage_ratio > thresholds.memory_usage_warning {
            checks.push(HealthCheckItem {
                id: "memory_pressure".to_string(),
                status: HealthStatus::Degraded,
                severity: "warning".to_string(),
                message: format!("物理内存偏高 ({:.1}%)", memory.usage_ratio * 100.0),
                observed: Some(memory.usage_ratio),
                threshold: Some(thresholds.memory_usage_warning),
                since: now,
            });
        }

        // 4. disk_space: 遍历各挂载卷，> 0.95 critical; > 0.90 warning
        for fs in &filesystems.items {
            if fs.usage_ratio > thresholds.disk_usage_critical {
                checks.push(HealthCheckItem {
                    id: format!("disk_space:{}", fs.mountpoint),
                    status: HealthStatus::Critical,
                    severity: "critical".to_string(),
                    message: format!(
                        "磁盘 [{}] 空间严重不足 ({:.1}%)",
                        fs.mountpoint,
                        fs.usage_ratio * 100.0
                    ),
                    observed: Some(fs.usage_ratio),
                    threshold: Some(thresholds.disk_usage_critical),
                    since: now,
                });
            } else if fs.usage_ratio > thresholds.disk_usage_warning {
                checks.push(HealthCheckItem {
                    id: format!("disk_space:{}", fs.mountpoint),
                    status: HealthStatus::Degraded,
                    severity: "warning".to_string(),
                    message: format!(
                        "磁盘 [{}] 空间将满 ({:.1}%)",
                        fs.mountpoint,
                        fs.usage_ratio * 100.0
                    ),
                    observed: Some(fs.usage_ratio),
                    threshold: Some(thresholds.disk_usage_warning),
                    since: now,
                });
            }
        }

        // 5. process_missing: 监视进程丢失判定
        for w in &processes.watch {
            if !w.present {
                checks.push(HealthCheckItem {
                    id: format!("process_missing:{}", w.name),
                    status: HealthStatus::Critical,
                    severity: "critical".to_string(),
                    message: format!("受监视的关键进程 [{}] 未运行", w.name),
                    observed: Some(0.0),
                    threshold: Some(1.0),
                    since: now,
                });
            }
        }

        // 6. agent_backlog: WAL 积压超限判定
        if wal_snapshots_count > thresholds.agent_backlog_snapshots {
            checks.push(HealthCheckItem {
                id: "agent_backlog".to_string(),
                status: HealthStatus::Degraded,
                severity: "warning".to_string(),
                message: format!("WAL 待发送快照积压过高 ({wal_snapshots_count} 个)"),
                observed: Some(wal_snapshots_count as f64),
                threshold: Some(thresholds.agent_backlog_snapshots as f64),
                since: now,
            });
        }

        // 7. traffic_quota: 月度周期出站流量预算预警
        if let Some(net) = network
            && let Some(ratio) = net.cycle_outbound_ratio
            && ratio >= 0.80
        {
            checks.push(HealthCheckItem {
                id: "traffic_quota".to_string(),
                status: if ratio >= 1.0 {
                    HealthStatus::Critical
                } else {
                    HealthStatus::Degraded
                },
                severity: if ratio >= 1.0 {
                    "critical".to_string()
                } else {
                    "warning".to_string()
                },
                message: format!(
                    "月度出站流量已使用 {:.1}% (超出 80% 预算警戒线)",
                    ratio * 100.0
                ),
                observed: Some(ratio),
                threshold: Some(0.80),
                since: now,
            });
        }

        // 汇总计算当前整体状态
        let has_critical = checks.iter().any(|c| c.status == HealthStatus::Critical);
        let has_degraded = checks.iter().any(|c| c.status == HealthStatus::Degraded);

        let new_status = if has_critical {
            HealthStatus::Critical
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Ok
        };

        // 状态机滞后与恢复跟踪
        if new_status != self.current_status {
            if new_status == HealthStatus::Ok {
                self.consecutive_ok_cycles += 1;
                // 连续 3 次正常方可完全恢复为 ok
                if self.consecutive_ok_cycles >= 3 {
                    self.current_status = HealthStatus::Ok;
                    self.status_since = now;
                    self.recovered_at = Some(now);
                    self.consecutive_ok_cycles = 0;
                }
            } else {
                // 进入异常状态立即转换并记录起始时间
                self.current_status = new_status;
                self.status_since = now;
                self.recovered_at = None;
                self.consecutive_ok_cycles = 0;
            }
        } else if new_status == HealthStatus::Ok {
            self.consecutive_ok_cycles = 0;
        }

        let status_code = match self.current_status {
            HealthStatus::Ok => 0,
            HealthStatus::Degraded => 1,
            HealthStatus::Critical => 2,
            HealthStatus::Unknown => 3,
        };

        HealthSection {
            status: self.current_status,
            status_code,
            since: Some(self.status_since),
            recovered_at: self.recovered_at,
            checks,
        }
    }
}
