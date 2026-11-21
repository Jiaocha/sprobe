use chrono::{Duration, Utc};
use sprobe::collectors::network::{CycleTrafficManager, matches_pattern, should_keep_interface};
use sprobe::health::HealthEngine;
use sprobe::snapshot::*;

#[test]
fn test_network_pattern_matching_and_filter() {
    // 1. 测试匹配函数
    assert!(matches_pattern("eth*", "eth0"));
    assert!(matches_pattern("eth*", "eth1"));
    assert!(!matches_pattern("eth*", "docker0"));
    assert!(matches_pattern("docker*", "docker0"));
    assert!(matches_pattern("*", "anything"));

    // 2. 测试 should_keep_interface
    let excludes = vec![
        "docker*".to_string(),
        "veth*".to_string(),
        "br-*".to_string(),
    ];
    let includes = vec!["eth*".to_string(), "ens*".to_string()];

    // 虚拟且 ignore_virtual = true
    assert!(!should_keep_interface(
        "veth123", true, &includes, &excludes, true
    ));
    // 命中排除列表
    assert!(!should_keep_interface(
        "docker0", false, &includes, &excludes, false
    ));
    // 物理网卡符合包含白名单
    assert!(should_keep_interface(
        "eth0", false, &includes, &excludes, true
    ));
    assert!(should_keep_interface(
        "ens33", false, &includes, &excludes, true
    ));
    // 物理网卡但不符合包含白名单
    assert!(!should_keep_interface(
        "wlan0", false, &includes, &excludes, true
    ));
}

#[test]
fn test_cycle_traffic_manager_and_rollover() {
    let now = Utc::now();
    let mut mgr = CycleTrafficManager::new(1);

    // 初始累计
    mgr.update(1000, now, 1);
    assert_eq!(mgr.cycle_outbound_bytes, 0); // 首次建立基线

    // 第二轮新增 5000 字节
    mgr.update(6000, now + Duration::seconds(10), 1);
    assert_eq!(mgr.cycle_outbound_bytes, 5000);

    // 第三轮重启计数器归零（从 2000 开始）
    mgr.update(2000, now + Duration::seconds(20), 1);
    assert_eq!(mgr.cycle_outbound_bytes, 7000);
}

#[test]
fn test_health_traffic_quota_alert() {
    let now = Utc::now();
    let mut engine = HealthEngine::new(Default::default());

    let dummy_meta = SectionMeta {
        status: SectionStatus::Ok,
        quality: Quality::Measured,
        sampled_at: now,
        age_seconds: 0.0,
        errors: Vec::new(),
    };

    let cpu = CpuSection {
        meta: dummy_meta.clone(),
        logical_count: 4,
        physical_count: 2,
        total: CpuTotal {
            usage_ratio: 0.1,
            idle_ratio: 0.9,
            iowait_ratio: 0.0,
            steal_ratio: 0.0,
            user_seconds_total: 10.0,
            nice_seconds_total: 0.0,
            system_seconds_total: 5.0,
            idle_seconds_total: 100.0,
            iowait_seconds_total: 0.0,
            irq_seconds_total: 0.0,
            softirq_seconds_total: 0.0,
            steal_seconds_total: 0.0,
        },
        cores: Vec::new(),
    };

    let memory = MemorySection {
        meta: dummy_meta.clone(),
        available_source: "MemAvailable".to_string(),
        total_bytes: 1000,
        available_bytes: 800,
        used_bytes: 200,
        free_bytes: 800,
        cached_bytes: 0,
        buffers_bytes: 0,
        usage_ratio: 0.2,
        dirty_bytes: 0,
        writeback_bytes: 0,
        swap_total_bytes: 0,
        swap_used_bytes: 0,
        swap_free_bytes: 0,
        swap_usage_ratio: None,
    };

    let filesystems = FilesystemsSection {
        meta: dummy_meta.clone(),
        truncated: false,
        items: Vec::new(),
    };

    let processes = ProcessesSection {
        meta: dummy_meta.clone(),
        visibility: "full".to_string(),
        count: 10,
        running_count: 1,
        sleeping_count: 9,
        blocked_count: 0,
        zombie_count: 0,
        thread_count: 20,
        truncated: false,
        top: Vec::new(),
        watch: Vec::new(),
    };

    // 构造超限的网络分区 (出站流量达到 88%)
    let net = NetworkSection {
        meta: dummy_meta,
        truncated: false,
        interfaces: Vec::new(),
        cycle_outbound_bytes: Some(880000000),
        cycle_limit_bytes: Some(1000000000),
        cycle_outbound_ratio: Some(0.88),
    };

    let health = engine.evaluate_with_network(
        &cpu,
        &memory,
        &filesystems,
        &processes,
        Some(&net),
        0,
        false,
    );
    assert_eq!(health.status, HealthStatus::Degraded);
    assert!(health.checks.iter().any(|c| c.id == "traffic_quota"));
}
