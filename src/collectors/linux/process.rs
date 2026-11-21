//! Linux /proc 进程聚合与 Top-N 采集

use crate::snapshot::{
    ProcessTopItem, ProcessWatchItem, ProcessesSection, Quality, SectionMeta, SectionStatus,
};
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProcStat {
    pub pid: u32,
    pub name: String,
    pub state: char,
    pub num_threads: usize,
    pub rss_bytes: u64,
}

/// 解析单条 /proc/[pid]/stat 内容
pub fn parse_proc_pid_stat(content: &str) -> Option<RawProcStat> {
    let open_paren = content.find('(')?;
    let close_paren = content.rfind(')')?;
    if close_paren <= open_paren {
        return None;
    }

    let pid = content[..open_paren].trim().parse::<u32>().ok()?;
    let name = content[open_paren + 1..close_paren].to_string();

    let rest = content[close_paren + 1..].trim();
    let fields: Vec<&str> = rest.split_whitespace().collect();
    if fields.is_empty() {
        return None;
    }

    let state = fields[0].chars().next().unwrap_or('S');
    // fields[0] 是 state (字段 3)
    // 字段 20 是 num_threads，在 fields 中的索引为 20 - 3 = 17
    let num_threads = fields
        .get(17)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    // 字段 24 是 rss (以 pages 计)，在 fields 中的索引为 24 - 3 = 21
    let rss_pages = fields
        .get(21)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let rss_bytes = rss_pages * 4096; // 默认页大小 4KiB

    Some(RawProcStat {
        pid,
        name,
        state,
        num_threads,
        rss_bytes,
    })
}

/// Linux 进程采集器
#[derive(Debug, Default)]
pub struct LinuxProcessCollector;

impl LinuxProcessCollector {
    pub fn new() -> Self {
        Self
    }

    /// 从原始列表聚合生成 ProcessesSection
    pub fn aggregate(
        &self,
        raw_list: Vec<RawProcStat>,
        top_n: usize,
        watch_names: &[String],
    ) -> ProcessesSection {
        let total_count = raw_list.len();
        let mut total_threads = 0usize;
        let mut running = 0usize;
        let mut sleeping = 0usize;
        let mut blocked = 0usize;
        let mut zombie = 0usize;

        for p in &raw_list {
            total_threads += p.num_threads;
            match p.state {
                'R' => running += 1,
                'S' => sleeping += 1,
                'D' => blocked += 1,
                'Z' => zombie += 1,
                _ => sleeping += 1,
            }
        }

        let now = Utc::now();

        // 匹配 watch 进程
        let mut watch = Vec::new();
        for watch_name in watch_names {
            let lower_watch = watch_name.to_lowercase();
            let mut matched_pids = Vec::new();
            for p in &raw_list {
                if p.name.to_lowercase() == lower_watch
                    || p.name.to_lowercase().starts_with(&lower_watch)
                {
                    matched_pids.push(p.pid);
                }
            }
            let count = matched_pids.len();
            watch.push(ProcessWatchItem {
                name: watch_name.clone(),
                present: count > 0,
                count,
                pids: matched_pids,
            });
        }

        // 计算 Top-N（按 RSS 降序）
        let mut sorted = raw_list;
        sorted.sort_by_key(|p| std::cmp::Reverse(p.rss_bytes));
        let top_limit = top_n.min(sorted.len());
        let mut top = Vec::with_capacity(top_limit);

        for p in sorted.into_iter().take(top_limit) {
            top.push(ProcessTopItem {
                pid: p.pid,
                start_time: now,
                name: p.name,
                state: p.state.to_string(),
                cpu_usage_ratio: 0.0,
                rss_bytes: p.rss_bytes,
            });
        }

        ProcessesSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            visibility: "full".to_string(),
            count: total_count,
            running_count: running,
            sleeping_count: sleeping,
            blocked_count: blocked,
            zombie_count: zombie,
            thread_count: total_threads,
            truncated: false,
            top,
            watch,
        }
    }

    #[cfg(target_os = "linux")]
    pub fn collect(
        &self,
        top_n: usize,
        watch_names: &[String],
    ) -> Result<ProcessesSection, String> {
        let mut raw_list = Vec::new();

        let entries =
            std::fs::read_dir("/proc").map_err(|e| format!("读取 /proc 目录失败: {e}"))?;
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str.chars().all(|c| c.is_ascii_digit()) {
                let stat_path = entry.path().join("stat");
                if let Ok(content) = std::fs::read_to_string(&stat_path) {
                    if let Some(proc_stat) = parse_proc_pid_stat(&content) {
                        raw_list.push(proc_stat);
                    }
                }
            }
        }

        Ok(self.aggregate(raw_list, top_n, watch_names))
    }
}
