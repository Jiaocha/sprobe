//! Windows 进程聚合与 Top-N 采集

use crate::snapshot::{
    ProcessTopItem, ProcessWatchItem, ProcessesSection, Quality, SectionMeta, SectionStatus,
};
use chrono::Utc;
use std::ffi::OsString;
use std::mem::size_of;
use std::os::windows::ffi::OsStringExt;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

#[derive(Debug, Default)]
pub struct WindowsProcessCollector;

impl WindowsProcessCollector {
    pub fn new() -> Self {
        Self
    }

    pub fn collect(
        &self,
        top_n: usize,
        watch_names: &[String],
    ) -> Result<ProcessesSection, String> {
        let snapshot_handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if snapshot_handle == INVALID_HANDLE_VALUE {
            return Err("CreateToolhelp32Snapshot 调用失败".to_string());
        }

        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut total_processes = 0usize;
        let mut total_threads = 0usize;
        let mut all_proc_items = Vec::new();

        let ok = unsafe { Process32FirstW(snapshot_handle, &mut entry) };
        if ok != 0 {
            loop {
                total_processes += 1;
                total_threads += entry.cntThreads as usize;

                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe_name = OsString::from_wide(&entry.szExeFile[..name_len])
                    .to_string_lossy()
                    .to_string();

                let pid = entry.th32ProcessID;

                // 尝试获取内存工作集 (RSS)
                let mut rss_bytes = 0u64;
                let h_proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
                if !h_proc.is_null() && h_proc != INVALID_HANDLE_VALUE {
                    let mut pmc: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
                    pmc.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                    let mem_ok = unsafe { K32GetProcessMemoryInfo(h_proc, &mut pmc, pmc.cb) };
                    if mem_ok != 0 {
                        rss_bytes = pmc.WorkingSetSize as u64;
                    }
                    unsafe { CloseHandle(h_proc) };
                }

                all_proc_items.push((pid, exe_name, rss_bytes));

                if unsafe { Process32NextW(snapshot_handle, &mut entry) } == 0 {
                    break;
                }
            }
        }

        unsafe { CloseHandle(snapshot_handle) };

        let now = Utc::now();

        // 匹配 watch 进程列表
        let mut watch = Vec::new();
        for watch_name in watch_names {
            let lower_watch = watch_name.to_lowercase();
            let mut matched_pids = Vec::new();
            for (pid, exe_name, _) in &all_proc_items {
                if exe_name.to_lowercase() == lower_watch
                    || exe_name.to_lowercase().starts_with(&lower_watch)
                {
                    matched_pids.push(*pid);
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
        all_proc_items.sort_by_key(|p| std::cmp::Reverse(p.2));
        let top_limit = top_n.min(all_proc_items.len());
        let mut top = Vec::with_capacity(top_limit);

        for (pid, exe_name, rss_bytes) in all_proc_items.into_iter().take(top_limit) {
            top.push(ProcessTopItem {
                pid,
                start_time: now, // 简化默认当前时间，后续可扩展 ProcessTimes
                name: exe_name,
                state: "running".to_string(),
                cpu_usage_ratio: 0.0,
                rss_bytes,
            });
        }

        let running_count = top.len().max(1);
        let sleeping_count = total_processes.saturating_sub(running_count);

        Ok(ProcessesSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Measured,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            visibility: "full".to_string(),
            count: total_processes,
            running_count,
            sleeping_count,
            blocked_count: 0,
            zombie_count: 0,
            thread_count: total_threads,
            truncated: false,
            top,
            watch,
        })
    }
}
