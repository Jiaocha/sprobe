use sprobe::config::CollectConfig;
use sprobe::scheduler::Scheduler;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn test_scheduler_reentrancy_protection() {
    let config = CollectConfig {
        fast_interval_seconds: 1,
        ..Default::default()
    };
    let scheduler = Arc::new(Scheduler::new(config));

    // 1. 正常执行 tick
    let executed = scheduler.tick_fast(|| {
        // do something quick
    });
    assert!(executed);
    assert_eq!(scheduler.skipped_count(), 0);

    // 2. 模拟一个正在执行中的长耗时采集任务
    let barrier_start = Arc::new(AtomicBool::new(false));
    let barrier_exit = Arc::new(AtomicBool::new(false));

    let sched_clone = Arc::clone(&scheduler);
    let start_clone = Arc::clone(&barrier_start);
    let exit_clone = Arc::clone(&barrier_exit);

    let handle = thread::spawn(move || {
        sched_clone.tick_fast(|| {
            start_clone.store(true, Ordering::SeqCst);
            // 等待外部信号才退出
            while !exit_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
            }
        });
    });

    // 等待子线程进入任务
    while !barrier_start.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(5));
    }

    // 3. 在上一轮仍在执行时，尝试再次触发 tick_fast -> 必须被跳过！
    let reentered = scheduler.tick_fast(|| {
        panic!("防重入失败！不应该执行此任务");
    });
    assert!(!reentered, "重入冲突时应该返回 false");
    assert_eq!(scheduler.skipped_count(), 1, "跳过计数器应该加 1");

    // 再次尝试触发 -> 依然跳过
    let reentered2 = scheduler.tick_fast(|| {});
    assert!(!reentered2);
    assert_eq!(scheduler.skipped_count(), 2);

    // 4. 通知子线程退出长耗时任务
    barrier_exit.store(true, Ordering::SeqCst);
    handle.join().unwrap();

    // 5. 子线程任务结束后，锁已释放，后续 tick 恢复正常
    let executed_after = scheduler.tick_fast(|| {});
    assert!(executed_after, "锁释放后应恢复执行");
    assert_eq!(scheduler.skipped_count(), 2);
}
