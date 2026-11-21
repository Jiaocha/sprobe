#[cfg(windows)]
use crate::snapshot::{CpuSection, CpuTotal, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use windows_sys::Win32::Foundation::FILETIME;

unsafe extern "system" {
    fn GetSystemTimes(
        lpIdleTime: *mut FILETIME,
        lpKernelTime: *mut FILETIME,
        lpUserTime: *mut FILETIME,
    ) -> i32;
}

#[derive(Debug, Clone, Copy, Default)]
struct CpuTimes {
    idle_100ns: u64,
    kernel_100ns: u64,
    user_100ns: u64,
}

fn filetime_to_u64(ft: &FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

pub struct WindowsCpuCollector {
    last_times: Option<CpuTimes>,
}

impl Default for WindowsCpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsCpuCollector {
    pub fn new() -> Self {
        Self { last_times: None }
    }

    fn read_system_times() -> Result<CpuTimes, String> {
        let mut idle_ft: FILETIME = unsafe { std::mem::zeroed() };
        let mut kernel_ft: FILETIME = unsafe { std::mem::zeroed() };
        let mut user_ft: FILETIME = unsafe { std::mem::zeroed() };

        let ret = unsafe { GetSystemTimes(&mut idle_ft, &mut kernel_ft, &mut user_ft) };
        if ret == 0 {
            return Err("GetSystemTimes 调用失败".to_string());
        }

        Ok(CpuTimes {
            idle_100ns: filetime_to_u64(&idle_ft),
            kernel_100ns: filetime_to_u64(&kernel_ft),
            user_100ns: filetime_to_u64(&user_ft),
        })
    }

    pub fn collect(&mut self) -> Result<CpuSection, String> {
        let current = Self::read_system_times()?;
        let logical_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        // 将 100ns 转换为秒 (1s = 10,000,000 * 100ns)
        let to_seconds = |t: u64| t as f64 / 10_000_000.0;

        let user_seconds = to_seconds(current.user_100ns);
        let idle_seconds = to_seconds(current.idle_100ns);
        // Windows kernel_time 包含 idle，因此真实系统时间需要扣减 idle
        let real_kernel_100ns = current.kernel_100ns.saturating_sub(current.idle_100ns);
        let system_seconds = to_seconds(real_kernel_100ns);

        let (usage_ratio, idle_ratio, quality) = if let Some(prev) = self.last_times {
            let d_user = current.user_100ns.saturating_sub(prev.user_100ns);
            let d_kernel = current.kernel_100ns.saturating_sub(prev.kernel_100ns);
            let d_idle = current.idle_100ns.saturating_sub(prev.idle_100ns);

            let d_total = d_user.saturating_add(d_kernel);
            if d_total > 0 {
                let d_sys = d_kernel.saturating_sub(d_idle);
                let u_ratio = (d_user + d_sys) as f64 / d_total as f64;
                let i_ratio = d_idle as f64 / d_total as f64;
                (
                    u_ratio.clamp(0.0, 1.0),
                    i_ratio.clamp(0.0, 1.0),
                    Quality::Measured,
                )
            } else {
                (0.0, 1.0, Quality::Estimated)
            }
        } else {
            // 首次采样建立基线，比率为估计值/0
            (0.0, 1.0, Quality::Estimated)
        };

        self.last_times = Some(current);

        let now = Utc::now();
        Ok(CpuSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            logical_count,
            physical_count: logical_count,
            total: CpuTotal {
                usage_ratio,
                idle_ratio,
                iowait_ratio: 0.0,
                steal_ratio: 0.0,
                user_seconds_total: user_seconds,
                nice_seconds_total: 0.0,
                system_seconds_total: system_seconds,
                idle_seconds_total: idle_seconds,
                iowait_seconds_total: 0.0,
                irq_seconds_total: 0.0,
                softirq_seconds_total: 0.0,
                steal_seconds_total: 0.0,
            },
            cores: Vec::new(),
        })
    }
}
