use chrono::Utc;
use sprobe::output::{PrometheusContext, encode_prometheus};
use sprobe::snapshot::*;
use std::collections::HashMap;
use uuid::Uuid;

fn make_mock_snapshot() -> Snapshot {
    let now = Utc::now();
    let meta = SectionMeta {
        status: SectionStatus::Ok,
        quality: Quality::Measured,
        sampled_at: now,
        age_seconds: 0.1,
        errors: Vec::new(),
    };

    Snapshot {
        schema_version: "1.0".to_string(),
        snapshot_id: Uuid::new_v4(),
        sequence: 1,
        collected_at: now,
        agent: AgentMeta {
            agent_id: Uuid::parse_str("6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f").unwrap(),
            instance_id: Uuid::new_v4(),
            version: "0.1.0".to_string(),
            hostname: "test-host".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            labels: HashMap::new(),
        },
        host: HostMeta {
            hostname: "test-host".to_string(),
            os_name: "Ubuntu".to_string(),
            kernel: "5.15.0".to_string(),
            architecture: "x86_64".to_string(),
            virtualization: None,
            cpu_model: Some("Test CPU".to_string()),
            boot_time: now,
        },
        collection: CollectionMeta {
            interval_seconds: 10,
            warmup: false,
            partial: false,
            capabilities: Default::default(),
            unsupported_reasons: HashMap::new(),
        },
        cpu: CpuSection {
            meta: meta.clone(),
            logical_count: 2,
            physical_count: 1,
            total: CpuTotal {
                usage_ratio: 0.25,
                idle_ratio: 0.75,
                iowait_ratio: 0.0,
                steal_ratio: 0.0,
                user_seconds_total: 100.5,
                nice_seconds_total: 0.0,
                system_seconds_total: 50.2,
                idle_seconds_total: 500.0,
                iowait_seconds_total: 0.0,
                irq_seconds_total: 0.0,
                softirq_seconds_total: 0.0,
                steal_seconds_total: 0.0,
            },
            cores: vec![
                CpuCore {
                    cpu: "0".to_string(),
                    usage_ratio: 0.30,
                    idle_ratio: 0.70,
                    iowait_ratio: 0.0,
                    steal_ratio: 0.0,
                    user_seconds_total: 60.0,
                    nice_seconds_total: 0.0,
                    system_seconds_total: 25.0,
                    idle_seconds_total: 250.0,
                    iowait_seconds_total: 0.0,
                    irq_seconds_total: 0.0,
                    softirq_seconds_total: 0.0,
                    steal_seconds_total: 0.0,
                    frequency_hertz: None,
                },
                CpuCore {
                    cpu: "1".to_string(),
                    usage_ratio: 0.20,
                    idle_ratio: 0.80,
                    iowait_ratio: 0.0,
                    steal_ratio: 0.0,
                    user_seconds_total: 40.5,
                    nice_seconds_total: 0.0,
                    system_seconds_total: 25.2,
                    idle_seconds_total: 250.0,
                    iowait_seconds_total: 0.0,
                    irq_seconds_total: 0.0,
                    softirq_seconds_total: 0.0,
                    steal_seconds_total: 0.0,
                    frequency_hertz: None,
                },
            ],
        },
        memory: MemorySection {
            meta: meta.clone(),
            available_source: "MemAvailable".to_string(),
            total_bytes: 16000000000,
            available_bytes: 8000000000,
            used_bytes: 8000000000,
            free_bytes: 4000000000,
            cached_bytes: 4000000000,
            buffers_bytes: 100000000,
            usage_ratio: 0.50,
            dirty_bytes: 0,
            writeback_bytes: 0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            swap_free_bytes: 0,
            swap_usage_ratio: None,
        },
        filesystems: FilesystemsSection {
            meta: meta.clone(),
            truncated: false,
            items: vec![FilesystemItem {
                device: "/dev/sda1".to_string(),
                volume_id: None,
                mountpoint: "/".to_string(),
                mountpoints: vec!["/".to_string()],
                fstype: "ext4".to_string(),
                read_only: false,
                total_bytes: 100000000000,
                used_bytes: 40000000000,
                free_bytes: 60000000000,
                available_bytes: 55000000000,
                reserved_bytes: 5000000000,
                usage_ratio: 0.40,
                inodes_total: 1000000,
                inodes_used: 100000,
                inodes_free: 900000,
                inodes_usage_ratio: 0.10,
                stale: false,
                quality: Quality::Measured,
            }],
        },
        disks: DisksSection {
            meta: meta.clone(),
            truncated: false,
            items: vec![DiskItem {
                device: "sda".to_string(),
                disk_class: "physical".to_string(),
                parent: None,
                read_bytes_total: 1234567,
                write_bytes_total: 7654321,
                read_operations_total: 100,
                write_operations_total: 200,
                io_time_seconds_total: 10.5,
                io_time_weighted_seconds_total: 12.0,
                in_flight_operations: 0,
                read_bytes_per_second: None,
                write_bytes_per_second: None,
                utilization_ratio: None,
                read_latency_seconds: None,
                write_latency_seconds: None,
                quality: Quality::Measured,
            }],
        },
        network: NetworkSection {
            meta: meta.clone(),
            truncated: false,
            interfaces: vec![NetworkInterfaceItem {
                name: "eth0".to_string(),
                class: "physical".to_string(),
                master: None,
                r#virtual: false,
                up: true,
                mtu: 1500,
                speed_bytes_per_second: None,
                receive_bytes_total: 999999,
                transmit_bytes_total: 888888,
                receive_packets_total: 1000,
                transmit_packets_total: 800,
                receive_errors_total: 0,
                transmit_errors_total: 0,
                receive_dropped_total: 0,
                transmit_dropped_total: 0,
                receive_bytes_per_second: None,
                transmit_bytes_per_second: None,
                receive_utilization_ratio: None,
                transmit_utilization_ratio: None,
            }],
            cycle_outbound_bytes: Some(1073741824),
            cycle_limit_bytes: Some(1099511627776),
            cycle_outbound_ratio: Some(0.000976),
        },
        load: LoadSection {
            meta: meta.clone(),
            average_1m: Some(1.23),
            average_5m: Some(1.10),
            average_15m: Some(0.95),
            average_1m_per_core: Some(0.615),
            processes_running: 2,
            processes_blocked: 0,
            processor_queue_length: None,
        },
        processes: ProcessesSection {
            meta: meta.clone(),
            visibility: "full".to_string(),
            count: 150,
            running_count: 2,
            sleeping_count: 148,
            blocked_count: 0,
            zombie_count: 0,
            thread_count: 450,
            truncated: false,
            top: Vec::new(),
            watch: Vec::new(),
        },
        uptime: UptimeSection {
            meta: meta.clone(),
            host_seconds: 3600.5,
            host_boot_time: now,
            agent_seconds: 120.0,
            clock_adjusted: false,
        },
        health: HealthSection {
            status: HealthStatus::Degraded,
            status_code: 1,
            since: Some(now),
            recovered_at: None,
            checks: vec![HealthCheckItem {
                id: "disk_space".to_string(),
                status: HealthStatus::Degraded,
                severity: "warning".to_string(),
                message: "Disk usage above threshold".to_string(),
                observed: Some(0.92),
                threshold: Some(0.90),
                since: now,
            }],
        },
        events: Vec::new(),
    }
}

#[test]
fn test_prometheus_warmup_none_snapshot() {
    let ctx = PrometheusContext {
        namespace: "sprobe".to_string(),
        commit: "abc1234".to_string(),
        wal_snapshots: 0,
        wal_bytes: 0,
    };
    let output = encode_prometheus(None, &ctx);
    assert!(output.contains("sprobe_agent_up 0"));
    assert!(output.contains("sprobe_agent_warmup 1"));
    assert!(!output.contains("sprobe_memory_available_bytes"));
}

#[test]
fn test_prometheus_valid_snapshot() {
    let snap = make_mock_snapshot();
    let ctx = PrometheusContext {
        namespace: "sprobe".to_string(),
        commit: "testcomm".to_string(),
        wal_snapshots: 5,
        wal_bytes: 10240,
    };
    let output = encode_prometheus(Some(&snap), &ctx);

    assert!(output.contains("sprobe_agent_up 1"));
    assert!(output.contains(r#"sprobe_agent_info{version="0.1.0",commit="testcomm",os="linux",arch="x86_64",agent_id="6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f"} 1"#));
    assert!(output.contains("sprobe_agent_warmup 0"));
    assert!(output.contains("sprobe_agent_uptime_seconds 120.00"));
    assert!(output.contains(r#"sprobe_cpu_usage_ratio{cpu="0"} 0.3000"#));
    assert!(output.contains(r#"sprobe_cpu_usage_ratio{cpu="1"} 0.2000"#));
    assert!(output.contains("sprobe_memory_available_bytes 8000000000"));
    assert!(output.contains(r#"sprobe_filesystem_available_bytes{device="/dev/sda1",mountpoint="/",fstype="ext4"} 55000000000"#));
    assert!(
        output.contains(
            r#"sprobe_disk_read_bytes_total{device="sda",disk_class="physical"} 1234567"#
        )
    );
    assert!(output.contains(
        r#"sprobe_network_receive_bytes_total{interface="eth0",class="physical"} 999999"#
    ));
    assert!(output.contains("sprobe_load_average_1m 1.23"));
    assert!(output.contains("sprobe_process_count 150"));
    assert!(output.contains("sprobe_host_uptime_seconds 3600.50"));
    assert!(output.contains("sprobe_health_status_code 1"));
    assert!(
        output.contains(r#"sprobe_health_check_status{check="disk_space",severity="warning"} 1"#)
    );
    assert!(output.contains("sprobe_wal_snapshots 5"));
    assert!(output.contains("sprobe_wal_bytes 10240"));
    assert!(output.contains("sprobe_network_cycle_outbound_bytes 1073741824"));
}

#[test]
fn test_prometheus_omit_empty_load() {
    let mut snap = make_mock_snapshot();
    snap.load.average_1m = None;
    let ctx = PrometheusContext::default();
    let output = encode_prometheus(Some(&snap), &ctx);
    // 空值指标不输出序列
    assert!(!output.contains("sprobe_load_average_1m"));
}
