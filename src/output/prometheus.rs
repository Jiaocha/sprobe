use crate::snapshot::Snapshot;
use std::fmt::Write;

pub struct PrometheusContext {
    pub namespace: String,
    pub commit: String,
    pub wal_snapshots: u64,
    pub wal_bytes: u64,
}

impl Default for PrometheusContext {
    fn default() -> Self {
        Self {
            namespace: "sprobe".to_string(),
            commit: "unknown".to_string(),
            wal_snapshots: 0,
            wal_bytes: 0,
        }
    }
}

/// 格式化标签值为 Prometheus 安全转义字符串
fn escape_label_value(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            _ => out.push(c),
        }
    }
    out
}

/// 编码当前快照为 Prometheus 0.0.4 文本格式
pub fn encode_prometheus(snapshot: Option<&Snapshot>, ctx: &PrometheusContext) -> String {
    let mut buffer = String::with_capacity(2048);
    let ns = &ctx.namespace;

    if snapshot.is_none() {
        // 快照不存在时（预热期前）
        let _ = writeln!(buffer, "# HELP {ns}_agent_up Agent process is running");
        let _ = writeln!(buffer, "# TYPE {ns}_agent_up gauge");
        let _ = writeln!(buffer, "{ns}_agent_up 0");
        let _ = writeln!(buffer, "# HELP {ns}_agent_warmup Agent warmup status");
        let _ = writeln!(buffer, "# TYPE {ns}_agent_warmup gauge");
        let _ = writeln!(buffer, "{ns}_agent_warmup 1");
        return buffer;
    }

    let snap = snapshot.unwrap();

    // 1. 自监控指标
    let _ = writeln!(buffer, "# HELP {ns}_agent_up Agent process is running");
    let _ = writeln!(buffer, "# TYPE {ns}_agent_up gauge");
    let _ = writeln!(buffer, "{ns}_agent_up 1");

    let _ = writeln!(buffer, "# HELP {ns}_agent_info Agent build information");
    let _ = writeln!(buffer, "# TYPE {ns}_agent_info gauge");
    let _ = writeln!(
        buffer,
        r#"{ns}_agent_info{{version="{}",commit="{}",os="{}",arch="{}",agent_id="{}"}} 1"#,
        escape_label_value(&snap.agent.version),
        escape_label_value(&ctx.commit),
        escape_label_value(&snap.agent.os),
        escape_label_value(&snap.agent.arch),
        snap.agent.agent_id
    );

    let _ = writeln!(buffer, "# HELP {ns}_agent_warmup Agent warmup status");
    let _ = writeln!(buffer, "# TYPE {ns}_agent_warmup gauge");
    let _ = writeln!(
        buffer,
        "{ns}_agent_warmup {}",
        if snap.collection.warmup { 1 } else { 0 }
    );

    let _ = writeln!(
        buffer,
        "# HELP {ns}_agent_uptime_seconds Agent process uptime"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_agent_uptime_seconds gauge");
    let _ = writeln!(
        buffer,
        "{ns}_agent_uptime_seconds {:.2}",
        snap.uptime.agent_seconds
    );

    // 2. CPU 使用率与计数器
    let _ = writeln!(
        buffer,
        "# HELP {ns}_cpu_usage_ratio CPU usage ratio excluding idle, iowait and steal"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_cpu_usage_ratio gauge");
    if !snap.cpu.cores.is_empty() {
        for core in &snap.cpu.cores {
            let _ = writeln!(
                buffer,
                r#"{ns}_cpu_usage_ratio{{cpu="{}"}} {:.4}"#,
                escape_label_value(&core.cpu),
                core.usage_ratio
            );
        }
    } else {
        // 无分核数据时输出整体
        let _ = writeln!(
            buffer,
            "{ns}_cpu_usage_ratio {:.4}",
            snap.cpu.total.usage_ratio
        );
    }

    let _ = writeln!(
        buffer,
        "# HELP {ns}_cpu_user_seconds_total Total user CPU time spent in seconds"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_cpu_user_seconds_total counter");
    if !snap.cpu.cores.is_empty() {
        for core in &snap.cpu.cores {
            let _ = writeln!(
                buffer,
                r#"{ns}_cpu_user_seconds_total{{cpu="{}"}} {:.2}"#,
                escape_label_value(&core.cpu),
                core.user_seconds_total
            );
        }
    } else {
        let _ = writeln!(
            buffer,
            "{ns}_cpu_user_seconds_total {:.2}",
            snap.cpu.total.user_seconds_total
        );
    }

    // 3. 内存
    let _ = writeln!(
        buffer,
        "# HELP {ns}_memory_available_bytes Estimated memory available for new processes"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_memory_available_bytes gauge");
    let _ = writeln!(
        buffer,
        "{ns}_memory_available_bytes {}",
        snap.memory.available_bytes
    );

    // 4. 文件系统
    if !snap.filesystems.items.is_empty() {
        let _ = writeln!(
            buffer,
            "# HELP {ns}_filesystem_available_bytes Filesystem space available to non-root users"
        );
        let _ = writeln!(buffer, "# TYPE {ns}_filesystem_available_bytes gauge");
        for fs in &snap.filesystems.items {
            let _ = writeln!(
                buffer,
                r#"{ns}_filesystem_available_bytes{{device="{}",mountpoint="{}",fstype="{}"}} {}"#,
                escape_label_value(&fs.device),
                escape_label_value(&fs.mountpoint),
                escape_label_value(&fs.fstype),
                fs.available_bytes
            );
        }
    }

    // 5. 磁盘 I/O（若有）
    if !snap.disks.items.is_empty() {
        let _ = writeln!(
            buffer,
            "# HELP {ns}_disk_read_bytes_total Total bytes read from disk"
        );
        let _ = writeln!(buffer, "# TYPE {ns}_disk_read_bytes_total counter");
        for disk in &snap.disks.items {
            let _ = writeln!(
                buffer,
                r#"{ns}_disk_read_bytes_total{{device="{}",disk_class="{}"}} {}"#,
                escape_label_value(&disk.device),
                escape_label_value(&disk.disk_class),
                disk.read_bytes_total
            );
        }
    }

    // 6. 网络流量
    if !snap.network.interfaces.is_empty() {
        let _ = writeln!(
            buffer,
            "# HELP {ns}_network_receive_bytes_total Total bytes received"
        );
        let _ = writeln!(buffer, "# TYPE {ns}_network_receive_bytes_total counter");
        for iface in &snap.network.interfaces {
            let _ = writeln!(
                buffer,
                r#"{ns}_network_receive_bytes_total{{interface="{}",class="{}"}} {}"#,
                escape_label_value(&iface.name),
                escape_label_value(&iface.class),
                iface.receive_bytes_total
            );
        }
    }

    if let Some(cycle_bytes) = snap.network.cycle_outbound_bytes {
        let _ = writeln!(
            buffer,
            "# HELP {ns}_network_cycle_outbound_bytes Current cycle outbound traffic in bytes"
        );
        let _ = writeln!(buffer, "# TYPE {ns}_network_cycle_outbound_bytes counter");
        let _ = writeln!(buffer, "{ns}_network_cycle_outbound_bytes {cycle_bytes}");
    }

    // 7. 负载（如果有平均负载）
    if let Some(load1) = snap.load.average_1m {
        let _ = writeln!(buffer, "# HELP {ns}_load_average_1m System load average 1m");
        let _ = writeln!(buffer, "# TYPE {ns}_load_average_1m gauge");
        let _ = writeln!(buffer, "{ns}_load_average_1m {:.2}", load1);
    }

    // 8. 进程数
    let _ = writeln!(buffer, "# HELP {ns}_process_count Total process count");
    let _ = writeln!(buffer, "# TYPE {ns}_process_count gauge");
    let _ = writeln!(buffer, "{ns}_process_count {}", snap.processes.count);

    // 9. 主机开机时间
    let _ = writeln!(
        buffer,
        "# HELP {ns}_host_uptime_seconds Host uptime in seconds"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_host_uptime_seconds gauge");
    let _ = writeln!(
        buffer,
        "{ns}_host_uptime_seconds {:.2}",
        snap.uptime.host_seconds
    );

    // 10. 健康状态与检查项
    let _ = writeln!(
        buffer,
        "# HELP {ns}_health_status_code Health status: 0=ok 1=degraded 2=critical 3=unknown"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_health_status_code gauge");
    let _ = writeln!(
        buffer,
        "{ns}_health_status_code {}",
        snap.health.status_code
    );

    if !snap.health.checks.is_empty() {
        let _ = writeln!(
            buffer,
            "# HELP {ns}_health_check_status Health check status (1 = firing)"
        );
        let _ = writeln!(buffer, "# TYPE {ns}_health_check_status gauge");
        for check in &snap.health.checks {
            let _ = writeln!(
                buffer,
                r#"{ns}_health_check_status{{check="{}",severity="{}"}} 1"#,
                escape_label_value(&check.id),
                escape_label_value(&check.severity)
            );
        }
    }

    // 11. WAL 状态指标
    let _ = writeln!(
        buffer,
        "# HELP {ns}_wal_snapshots Number of pending snapshots in WAL"
    );
    let _ = writeln!(buffer, "# TYPE {ns}_wal_snapshots gauge");
    let _ = writeln!(buffer, "{ns}_wal_snapshots {}", ctx.wal_snapshots);

    let _ = writeln!(buffer, "# HELP {ns}_wal_bytes Total size of WAL in bytes");
    let _ = writeln!(buffer, "# TYPE {ns}_wal_bytes gauge");
    let _ = writeln!(buffer, "{ns}_wal_bytes {}", ctx.wal_bytes);

    buffer
}
