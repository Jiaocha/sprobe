use crate::config::CollectConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tracing::{info, warn};

pub struct Scheduler {
    config: CollectConfig,
    is_running: Arc<AtomicBool>,
    fast_in_progress: Arc<AtomicBool>,
    fast_skipped_total: Arc<AtomicU64>,
}

impl Scheduler {
    pub fn new(config: CollectConfig) -> Self {
        Self {
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            fast_in_progress: Arc::new(AtomicBool::new(false)),
            fast_skipped_total: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn skipped_count(&self) -> u64 {
        self.fast_skipped_total.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
    }

    /// 执行一轮调度节拍（带防重入保护）
    /// 如果上一轮采集仍未结束，则跳过本次并累加 skipped_total
    pub fn tick_fast<F>(&self, task: F) -> bool
    where
        F: FnOnce(),
    {
        // 尝试获取防重入锁 (CAS: false -> true)
        if self
            .fast_in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            let skipped = self.fast_skipped_total.fetch_add(1, Ordering::SeqCst) + 1;
            warn!("Fast 节奏组发生重入冲突，跳过本轮执行 (累计跳过: {skipped})");
            return false;
        }

        // 获得锁，执行采集任务
        task();

        // 任务完成，释放防重入锁
        self.fast_in_progress.store(false, Ordering::Release);
        true
    }

    /// 启动常驻调度循环骨架（非阻塞在独立线程或运行时中启动）
    pub fn run_loop<F>(&self, mut on_tick: F)
    where
        F: FnMut(),
    {
        self.is_running.store(true, Ordering::SeqCst);
        let interval = Duration::from_secs(self.config.fast_interval_seconds);

        info!(
            "调度器已启动，Fast 周期: {}s",
            self.config.fast_interval_seconds
        );

        while self.is_running.load(Ordering::SeqCst) {
            self.tick_fast(&mut on_tick);
            std::thread::sleep(interval);
        }

        info!("调度器已优雅退出");
    }
}
