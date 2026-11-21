use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 通用通配符网卡匹配（支持精确匹配、前缀 `eth*`、后缀 `*0` 或全部 `*`）
pub fn matches_pattern(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        name.ends_with(suffix)
    } else {
        pattern == name
    }
}

/// 判定是否应当保留此网卡（应用包含白名单、排除黑名单与虚拟过滤）
pub fn should_keep_interface(
    name: &str,
    is_virtual: bool,
    include_patterns: &[String],
    exclude_patterns: &[String],
    ignore_virtual: bool,
) -> bool {
    if ignore_virtual && is_virtual {
        return false;
    }
    for pat in exclude_patterns {
        if matches_pattern(pat, name) {
            return false;
        }
    }
    if !include_patterns.is_empty() {
        return include_patterns
            .iter()
            .any(|pat| matches_pattern(pat, name));
    }
    true
}

/// 周期出账流量状态管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleTrafficManager {
    pub current_cycle_start: DateTime<Utc>,
    pub cycle_outbound_bytes: u64,
    pub last_total_tx: Option<u64>,
}

impl CycleTrafficManager {
    pub fn new(cycle_start_day: u8) -> Self {
        let now = Utc::now();
        let cycle_start = Self::calculate_cycle_start(now, cycle_start_day);
        Self {
            current_cycle_start: cycle_start,
            cycle_outbound_bytes: 0,
            last_total_tx: None,
        }
    }

    /// 根据指定日计算最近一个出账周期的起点
    pub fn calculate_cycle_start(now: DateTime<Utc>, cycle_start_day: u8) -> DateTime<Utc> {
        let day = cycle_start_day.clamp(1, 28) as u32;
        let year = now.year();
        let month = now.month();

        if now.day() >= day {
            chrono::NaiveDate::from_ymd_opt(year, month, day)
                .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap())
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        } else {
            // 上个月该日
            let (prev_year, prev_month) = if month == 1 {
                (year - 1, 12)
            } else {
                (year, month - 1)
            };
            chrono::NaiveDate::from_ymd_opt(prev_year, prev_month, day)
                .unwrap_or_else(|| {
                    chrono::NaiveDate::from_ymd_opt(prev_year, prev_month, 1).unwrap()
                })
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        }
    }

    /// 从文件加载或创建
    pub fn load_or_init(dir: &Path, cycle_start_day: u8) -> Self {
        let file = dir.join("cycle_traffic.json");
        if let Ok(content) = std::fs::read_to_string(&file)
            && let Ok(mgr) = serde_json::from_str::<Self>(&content)
        {
            return mgr;
        }
        Self::new(cycle_start_day)
    }

    pub fn save(&self, dir: &Path) {
        let file = dir.join("cycle_traffic.json");
        let _ = std::fs::create_dir_all(dir);
        if let Ok(content) = serde_json::to_string(self) {
            let _ = std::fs::write(&file, content);
        }
    }

    /// 累计当期出站总流量
    pub fn update(&mut self, current_total_tx: u64, now: DateTime<Utc>, cycle_start_day: u8) {
        let expected_cycle_start = Self::calculate_cycle_start(now, cycle_start_day);

        // 如果周期跨越了，重置本周期出站流量
        if expected_cycle_start > self.current_cycle_start {
            self.current_cycle_start = expected_cycle_start;
            self.cycle_outbound_bytes = 0;
            self.last_total_tx = Some(current_total_tx);
            return;
        }

        if let Some(last_tx) = self.last_total_tx {
            if current_total_tx >= last_tx {
                let delta = current_total_tx - last_tx;
                self.cycle_outbound_bytes = self.cycle_outbound_bytes.saturating_add(delta);
            } else {
                // 网卡计数器重置（如重启），取当前值作为基线
                self.cycle_outbound_bytes =
                    self.cycle_outbound_bytes.saturating_add(current_total_tx);
            }
        }

        self.last_total_tx = Some(current_total_tx);
    }
}
