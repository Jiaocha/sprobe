//! Windows 文件系统真实采集测试

#[cfg(windows)]
#[test]
fn test_windows_filesystem_collector() {
    use sprobe::collectors::windows::filesystem::WindowsFilesystemCollector;

    let collector = WindowsFilesystemCollector::new();
    let fs_section = collector.collect().expect("Windows 文件系统采集失败");

    assert!(!fs_section.items.is_empty(), "必须至少采集到一个驱动器");
    println!("采集到 {} 个文件系统驱动器:", fs_section.items.len());
    for item in &fs_section.items {
        println!(
            "  [{}] mountpoint: {}, fstype: {}, total: {:.2} GB, used: {:.2} GB, usage: {:.1}%",
            item.device,
            item.mountpoint,
            item.fstype,
            item.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
            item.used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
            item.usage_ratio * 100.0
        );
        assert!(item.total_bytes > 0);
        assert!(item.usage_ratio >= 0.0 && item.usage_ratio <= 1.0);
    }
}
