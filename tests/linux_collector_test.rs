//! Linux 采集器与解析器测试

use sprobe::collectors::linux::cpu::{LinuxCpuCollector, parse_proc_stat};
use sprobe::collectors::linux::memory::LinuxMemoryCollector;

const MOCK_PROC_STAT_1: &str = r#"cpu  1000 100 500 8000 200 50 20 10 0 0
cpu0 500 50 250 4000 100 25 10 5 0 0
cpu1 500 50 250 4000 100 25 10 5 0 0
intr 11493054 23 11 0 0
ctxt 1990473
btime 1609459200
processes 12345
procs_running 1
procs_blocked 0
"#;

const MOCK_PROC_STAT_2: &str = r#"cpu  1200 100 600 8500 200 50 20 10 0 0
cpu0 600 50 300 4250 100 25 10 5 0 0
cpu1 600 50 300 4250 100 25 10 5 0 0
intr 11493060 23 11 0 0
ctxt 1990500
btime 1609459200
processes 12348
procs_running 2
procs_blocked 0
"#;

const MOCK_PROC_MEMINFO_WITH_AVAILABLE: &str = r#"MemTotal:       16384000 kB
MemFree:         4096000 kB
MemAvailable:   10240000 kB
Buffers:          512000 kB
Cached:          6144000 kB
SwapCached:            0 kB
Active:          5120000 kB
Inactive:        4096000 kB
SwapTotal:       2048000 kB
SwapFree:        1024000 kB
Dirty:              1024 kB
Writeback:             0 kB
AnonPages:       3072000 kB
Mapped:           512000 kB
Shmem:            256000 kB
SReclaimable:     512000 kB
"#;

const MOCK_PROC_MEMINFO_FALLBACK: &str = r#"MemTotal:       16384000 kB
MemFree:         4096000 kB
Buffers:          512000 kB
Cached:          6144000 kB
SwapTotal:             0 kB
SwapFree:              0 kB
Dirty:                 0 kB
Writeback:             0 kB
Shmem:            256000 kB
SReclaimable:     512000 kB
"#;

#[test]
fn test_linux_proc_stat_parse_and_diff() {
    let (total, cores) = parse_proc_stat(MOCK_PROC_STAT_1).expect("解析 /proc/stat 失败");
    assert_eq!(total.name, "cpu");
    assert_eq!(total.user, 1000);
    assert_eq!(total.idle, 8000);
    assert_eq!(cores.len(), 2);
    assert_eq!(cores[0].name, "cpu0");
    assert_eq!(cores[1].name, "cpu1");

    let mut collector = LinuxCpuCollector::new();

    // 首次采集（基线）
    let first = collector
        .collect_from_content(MOCK_PROC_STAT_1)
        .expect("首次采样失败");
    assert_eq!(first.logical_count, 2);
    // 首次采样无前序数据，使用率为 0
    assert_eq!(first.total.usage_ratio, 0.0);

    // 第二次采样（差分）
    // delta_user = 200, delta_system = 100, delta_idle = 500, other deltas = 0
    // total delta = 800, non_idle = 300 -> usage_ratio = 300 / 800 = 0.375
    let second = collector
        .collect_from_content(MOCK_PROC_STAT_2)
        .expect("二次采样失败");
    assert_eq!(second.logical_count, 2);
    assert!((second.total.usage_ratio - 0.375).abs() < 1e-4);
    assert!((second.total.idle_ratio - 0.625).abs() < 1e-4);
    assert_eq!(second.cores.len(), 2);
    assert!((second.cores[0].usage_ratio - 0.375).abs() < 1e-4);
}

#[test]
fn test_linux_meminfo_with_memavailable() {
    let collector = LinuxMemoryCollector::new();
    let mem = collector
        .collect_from_content(MOCK_PROC_MEMINFO_WITH_AVAILABLE)
        .expect("解析 meminfo 失败");

    assert_eq!(mem.available_source, "kernel_memavailable");
    assert_eq!(mem.total_bytes, 16384000 * 1024);
    assert_eq!(mem.available_bytes, 10240000 * 1024);
    assert_eq!(mem.free_bytes, 4096000 * 1024);
    assert_eq!(mem.used_bytes, (16384000 - 10240000) * 1024);
    assert!((mem.usage_ratio - (6144000.0 / 16384000.0)).abs() < 1e-4);
    assert_eq!(mem.swap_total_bytes, 2048000 * 1024);
    assert_eq!(mem.swap_used_bytes, 1024000 * 1024);
    assert_eq!(mem.swap_usage_ratio, Some(0.5));
}

#[test]
fn test_linux_meminfo_fallback_calculation() {
    let collector = LinuxMemoryCollector::new();
    let mem = collector
        .collect_from_content(MOCK_PROC_MEMINFO_FALLBACK)
        .expect("解析 fallback meminfo 失败");

    // fallback 计算: (free + buffers + cached + sreclaimable) - shmem
    // = (4096000 + 512000 + 6144000 + 512000) - 256000 = 11264000 - 256000 = 11008000 kB
    assert_eq!(mem.available_source, "fallback_calculation");
    assert_eq!(mem.available_bytes, 11008000 * 1024);
    assert_eq!(mem.swap_total_bytes, 0);
    assert_eq!(mem.swap_usage_ratio, None);
}

#[test]
fn test_linux_mounts_parse() {
    use sprobe::collectors::linux::filesystem::parse_mounts;

    let mock_mounts = r#"sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
/dev/sda1 / ext4 rw,relatime,errors=remount-ro 0 0
/dev/sdb1 /data xfs rw,relatime 0 0
tmpfs /run tmpfs rw,nosuid,nodev,noexec,relatime,size=1638400k,mode=755 0 0
"#;

    let parsed = parse_mounts(mock_mounts);
    // sysfs 和 proc 应该被忽略，留下 / 和 /data 和 /run (tmpfs)
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].mountpoint, "/");
    assert_eq!(parsed[0].fstype, "ext4");
    assert_eq!(parsed[1].mountpoint, "/data");
    assert_eq!(parsed[1].fstype, "xfs");
}

#[test]
fn test_linux_net_dev_parse() {
    use sprobe::collectors::linux::network::parse_proc_net_dev;

    let mock_net_dev = r#"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1000       10    0    0    0     0          0         0     1000       10    0    0    0     0       0          0
  eth0: 5000000   4000    1    2    0     0          0         0  2000000     3000    0    0    0     0       0          0
"#;

    let entries = parse_proc_net_dev(mock_net_dev);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "lo");
    assert_eq!(entries[0].rx_bytes, 1000);
    assert_eq!(entries[1].name, "eth0");
    assert_eq!(entries[1].rx_bytes, 5000000);
    assert_eq!(entries[1].tx_bytes, 2000000);
    assert_eq!(entries[1].rx_errors, 1);
    assert_eq!(entries[1].rx_dropped, 2);
}

#[test]
fn test_linux_process_parse_and_aggregate() {
    use sprobe::collectors::linux::process::{LinuxProcessCollector, parse_proc_pid_stat};

    // 模拟真实 Linux /proc/1/stat (systemd)
    let sample_stat_1 = "1 (systemd) S 0 1 1 0 -1 4194560 21644 1913369 133 1354 756 1235 738 780 20 0 1 0 12 178716672 3120 18446744073709551615 1 1 0 0 0 0 671173123 4096 1260 0 0 0 17 0 0 0 0 0 0 0 0 0 0 0 0 0";
    let proc1 = parse_proc_pid_stat(sample_stat_1).expect("解析 /proc/1/stat 失败");
    assert_eq!(proc1.pid, 1);
    assert_eq!(proc1.name, "systemd");
    assert_eq!(proc1.state, 'S');
    assert_eq!(proc1.num_threads, 1);
    assert_eq!(proc1.rss_bytes, 3120 * 4096);

    let sample_stat_2 = "123 (kworker/0:1) R 2 0 0 0 -1 69238848 0 0 0 0 120 40 0 0 20 0 4 0 50 0 0 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 0 0 0 0 0 0 0 0 0 0 0 0 0";
    let proc2 = parse_proc_pid_stat(sample_stat_2).expect("解析 /proc/123/stat 失败");
    assert_eq!(proc2.pid, 123);
    assert_eq!(proc2.name, "kworker/0:1");
    assert_eq!(proc2.state, 'R');
    assert_eq!(proc2.num_threads, 4);

    let collector = LinuxProcessCollector::new();
    let watch = vec!["systemd".to_string(), "nginx".to_string()];
    let section = collector.aggregate(vec![proc1, proc2], 10, &watch);

    assert_eq!(section.count, 2);
    assert_eq!(section.thread_count, 5);
    assert_eq!(section.running_count, 1);
    assert_eq!(section.sleeping_count, 1);
    assert_eq!(section.top.len(), 2);
    assert_eq!(section.top[0].name, "systemd"); // RSS 较高排在前

    assert_eq!(section.watch.len(), 2);
    assert_eq!(section.watch[0].name, "systemd");
    assert!(section.watch[0].present);
    assert_eq!(section.watch[1].name, "nginx");
    assert!(!section.watch[1].present);
}
