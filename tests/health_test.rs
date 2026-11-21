//! 健康状态机与评估规则测试

use chrono::Utc;
use sprobe::config::HealthConfig;
use sprobe::health::HealthEngine;
use sprobe::snapshot::{
    CpuSection, CpuTotal, FilesystemItem, FilesystemsSection, HealthStatus, MemorySection,
    ProcessesSection, Quality, SectionMeta, SectionStatus,
};

fn mock_meta() -> SectionMeta {
    SectionMeta {
        status: SectionStatus::Ok,
        quality: Quality::Measured,
        sampled_at: Utc::now(),
        age_seconds: 0.0,
        errors: Vec::new(),
    }
}

fn mock_cpu(usage: f64) -> CpuSection {
    CpuSection {
        meta: mock_meta(),
        logical_count: 4,
        physical_count: 4,
        total: CpuTotal {
            usage_ratio: usage,
            idle_ratio: 1.0 - usage,
            iowait_ratio: 0.0,
            steal_ratio: 0.0,
            user_seconds_total: 100.0,
            nice_seconds_total: 0.0,
            system_seconds_total: 100.0,
            idle_seconds_total: 800.0,
            iowait_seconds_total: 0.0,
            irq_seconds_total: 0.0,
            softirq_seconds_total: 0.0,
            steal_seconds_total: 0.0,
        },
        cores: Vec::new(),
    }
}

fn mock_memory(usage: f64) -> MemorySection {
    MemorySection {
        meta: mock_meta(),
        available_source: "mock".to_string(),
        total_bytes: 1000,
        available_bytes: ((1.0 - usage) * 1000.0) as u64,
        used_bytes: (usage * 1000.0) as u64,
        free_bytes: ((1.0 - usage) * 1000.0) as u64,
        cached_bytes: 0,
        buffers_bytes: 0,
        usage_ratio: usage,
        dirty_bytes: 0,
        writeback_bytes: 0,
        swap_total_bytes: 0,
        swap_used_bytes: 0,
        swap_free_bytes: 0,
        swap_usage_ratio: None,
    }
}

fn mock_filesystems(usage: f64) -> FilesystemsSection {
    FilesystemsSection {
        meta: mock_meta(),
        truncated: false,
        items: vec![FilesystemItem {
            device: "C:".to_string(),
            volume_id: None,
            mountpoint: "C:\\".to_string(),
            mountpoints: vec!["C:\\".to_string()],
            fstype: "NTFS".to_string(),
            read_only: false,
            total_bytes: 1000,
            used_bytes: (usage * 1000.0) as u64,
            free_bytes: ((1.0 - usage) * 1000.0) as u64,
            available_bytes: ((1.0 - usage) * 1000.0) as u64,
            reserved_bytes: 0,
            usage_ratio: usage,
            inodes_total: 0,
            inodes_used: 0,
            inodes_free: 0,
            inodes_usage_ratio: 0.0,
            stale: false,
            quality: Quality::Measured,
        }],
    }
}

fn mock_processes() -> ProcessesSection {
    ProcessesSection {
        meta: mock_meta(),
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
    }
}

#[test]
fn test_health_state_transitions() {
    let mut engine = HealthEngine::new(HealthConfig::default());

    // 1. 全部正常
    let h1 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.5),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(h1.status, HealthStatus::Ok);
    assert_eq!(h1.status_code, 0);
    assert!(h1.checks.is_empty());

    // 2. 内存升高至 92% (Warning -> Degraded)
    let h2 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.92),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(h2.status, HealthStatus::Degraded);
    assert_eq!(h2.status_code, 1);
    assert_eq!(h2.checks.len(), 1);
    assert_eq!(h2.checks[0].id, "memory_pressure");

    // 3. 内存升高至 97% (Critical)
    let h3 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.97),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(h3.status, HealthStatus::Critical);
    assert_eq!(h3.status_code, 2);

    // 4. 出现 push_auth_failed，立即锁定 critical
    let h4 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.5),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        true,
    );
    assert_eq!(h4.status, HealthStatus::Critical);
    assert!(h4.checks.iter().any(|c| c.id == "push_auth_failed"));

    // 5. 故障恢复：指标恢复正常后需连续 3 周期确认
    // 第 1 周期正常
    let r1 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.5),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(r1.status, HealthStatus::Critical); // 滞后未满足 3 周期，仍为旧状态

    // 第 2 周期正常
    let r2 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.5),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(r2.status, HealthStatus::Critical);

    // 第 3 周期正常 -> 满足 3 周期，正式恢复为 Ok！
    let r3 = engine.evaluate(
        &mock_cpu(0.2),
        &mock_memory(0.5),
        &mock_filesystems(0.4),
        &mock_processes(),
        0,
        false,
    );
    assert_eq!(r3.status, HealthStatus::Ok);
    assert_eq!(r3.status_code, 0);
    assert!(r3.recovered_at.is_some());
}
