//! Linux /proc/stat CPU 指标采集与解析

use crate::snapshot::{CpuCore, CpuSection, CpuTotal, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use std::collections::HashMap;

/// 单个 CPU 原始计数器（时钟滴答 USER_HZ，通常 100Hz = 0.01 秒/tick）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawCpuStat {
    pub name: String,
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

impl RawCpuStat {
    /// 计算参与总时间统计的时钟滴答总数
    /// 注：根据 Linux 内核实现，guest 已包含在 user 中，guest_nice 已包含在 nice 中
    pub fn total_ticks(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }
}

/// 解析 /proc/stat 文本内容，提取总 CPU 和各 CPU 核
pub fn parse_proc_stat(content: &str) -> Result<(RawCpuStat, Vec<RawCpuStat>), String> {
    let mut total_cpu: Option<RawCpuStat> = None;
    let mut core_cpus = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("cpu") {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let name = match parts.next() {
            Some(n) => n,
            None => continue,
        };

        let parse_field = |parts: &mut std::str::SplitWhitespace| -> u64 {
            parts
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        };

        let stat = RawCpuStat {
            name: name.to_string(),
            user: parse_field(&mut parts),
            nice: parse_field(&mut parts),
            system: parse_field(&mut parts),
            idle: parse_field(&mut parts),
            iowait: parse_field(&mut parts),
            irq: parse_field(&mut parts),
            softirq: parse_field(&mut parts),
            steal: parse_field(&mut parts),
            guest: parse_field(&mut parts),
            guest_nice: parse_field(&mut parts),
        };

        if name == "cpu" {
            total_cpu = Some(stat);
        } else if name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()) {
            core_cpus.push(stat);
        }
    }

    match total_cpu {
        Some(total) => Ok((total, core_cpus)),
        None => Err("未在 /proc/stat 中找到汇总 'cpu' 行".to_string()),
    }
}

/// 计算两组时钟滴答之间的差分比率 (usage, idle, iowait, steal)
pub fn calculate_diff_ratios(prev: &RawCpuStat, curr: &RawCpuStat) -> (f64, f64, f64, f64) {
    let delta_user = curr.user.saturating_sub(prev.user);
    let delta_nice = curr.nice.saturating_sub(prev.nice);
    let delta_system = curr.system.saturating_sub(prev.system);
    let delta_idle = curr.idle.saturating_sub(prev.idle);
    let delta_iowait = curr.iowait.saturating_sub(prev.iowait);
    let delta_irq = curr.irq.saturating_sub(prev.irq);
    let delta_softirq = curr.softirq.saturating_sub(prev.softirq);
    let delta_steal = curr.steal.saturating_sub(prev.steal);

    let delta_total = delta_user
        + delta_nice
        + delta_system
        + delta_idle
        + delta_iowait
        + delta_irq
        + delta_softirq
        + delta_steal;

    if delta_total == 0 {
        return (0.0, 1.0, 0.0, 0.0);
    }

    let non_idle = delta_total.saturating_sub(delta_idle + delta_iowait);
    let usage_ratio = (non_idle as f64 / delta_total as f64).clamp(0.0, 1.0);
    let idle_ratio = (delta_idle as f64 / delta_total as f64).clamp(0.0, 1.0);
    let iowait_ratio = (delta_iowait as f64 / delta_total as f64).clamp(0.0, 1.0);
    let steal_ratio = (delta_steal as f64 / delta_total as f64).clamp(0.0, 1.0);

    (usage_ratio, idle_ratio, iowait_ratio, steal_ratio)
}

/// Linux CPU 采集器
pub struct LinuxCpuCollector {
    prev_total: Option<RawCpuStat>,
    prev_cores: HashMap<String, RawCpuStat>,
    user_hz: f64,
}

impl Default for LinuxCpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxCpuCollector {
    pub fn new() -> Self {
        Self {
            prev_total: None,
            prev_cores: HashMap::new(),
            user_hz: 100.0, // Linux 标准 USER_HZ 默认为 100
        }
    }

    /// 使用文本解析并生成快照分区（便于测试与真实采集复用）
    pub fn collect_from_content(&mut self, proc_stat_content: &str) -> Result<CpuSection, String> {
        let (curr_total, curr_cores) = parse_proc_stat(proc_stat_content)?;
        let now = Utc::now();

        let (usage_ratio, idle_ratio, iowait_ratio, steal_ratio) =
            if let Some(ref prev) = self.prev_total {
                calculate_diff_ratios(prev, &curr_total)
            } else {
                (0.0, 1.0, 0.0, 0.0)
            };

        let hz = self.user_hz;
        let total = CpuTotal {
            usage_ratio,
            idle_ratio,
            iowait_ratio,
            steal_ratio,
            user_seconds_total: curr_total.user as f64 / hz,
            nice_seconds_total: curr_total.nice as f64 / hz,
            system_seconds_total: curr_total.system as f64 / hz,
            idle_seconds_total: curr_total.idle as f64 / hz,
            iowait_seconds_total: curr_total.iowait as f64 / hz,
            irq_seconds_total: curr_total.irq as f64 / hz,
            softirq_seconds_total: curr_total.softirq as f64 / hz,
            steal_seconds_total: curr_total.steal as f64 / hz,
        };

        let mut cores = Vec::with_capacity(curr_cores.len());
        for core in &curr_cores {
            let (c_usage, c_idle, c_iowait, c_steal) =
                if let Some(prev) = self.prev_cores.get(&core.name) {
                    calculate_diff_ratios(prev, core)
                } else {
                    (0.0, 1.0, 0.0, 0.0)
                };

            cores.push(CpuCore {
                cpu: core.name.clone(),
                usage_ratio: c_usage,
                idle_ratio: c_idle,
                iowait_ratio: c_iowait,
                steal_ratio: c_steal,
                user_seconds_total: core.user as f64 / hz,
                nice_seconds_total: core.nice as f64 / hz,
                system_seconds_total: core.system as f64 / hz,
                idle_seconds_total: core.idle as f64 / hz,
                iowait_seconds_total: core.iowait as f64 / hz,
                irq_seconds_total: core.irq as f64 / hz,
                softirq_seconds_total: core.softirq as f64 / hz,
                steal_seconds_total: core.steal as f64 / hz,
                frequency_hertz: None,
            });
        }

        let logical_count = if curr_cores.is_empty() {
            1
        } else {
            curr_cores.len()
        };

        // 更新历史快照
        self.prev_total = Some(curr_total);
        self.prev_cores = curr_cores
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();

        Ok(CpuSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Derived,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            logical_count,
            physical_count: logical_count, // 可进一步结合 /proc/cpuinfo 拓扑，默认对齐
            total,
            cores,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn collect(&mut self) -> Result<CpuSection, String> {
        let content = std::fs::read_to_string("/proc/stat")
            .map_err(|e| format!("读取 /proc/stat 失败: {e}"))?;
        self.collect_from_content(&content)
    }
}
