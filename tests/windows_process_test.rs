//! Windows 真实进程采集测试

#[cfg(windows)]
#[test]
fn test_windows_process_collector() {
    use sprobe::collectors::windows::process::WindowsProcessCollector;

    let collector = WindowsProcessCollector::new();
    let watch_list = vec!["svchost.exe".to_string(), "explorer.exe".to_string()];
    let proc_section = collector
        .collect(5, &watch_list)
        .expect("Windows 进程采集失败");

    println!("系统总进程数: {}", proc_section.count);
    println!("系统总线程数: {}", proc_section.thread_count);
    assert!(proc_section.count > 10, "系统进程数应大于 10");
    assert!(
        proc_section.thread_count > proc_section.count,
        "线程数应大于进程数"
    );

    println!("Top 5 内存消耗进程:");
    for (i, p) in proc_section.top.iter().enumerate() {
        println!(
            "  [{}] PID: {}, Name: {}, RSS: {:.2} MB",
            i + 1,
            p.pid,
            p.name,
            p.rss_bytes as f64 / 1024.0 / 1024.0
        );
    }
    assert!(!proc_section.top.is_empty());

    println!("Watch 进程状态:");
    for w in &proc_section.watch {
        println!(
            "  进程: {}, 存在: {}, 实例数: {}, PIDs: {:?}",
            w.name, w.present, w.count, w.pids
        );
    }
}
