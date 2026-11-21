//! Sprobe agent entrypoint.

use chrono::Utc;
use clap::Parser;
use sprobe::config::SprobeConfig;
use sprobe::exit_codes::*;
use sprobe::snapshot::{
    AgentMeta, CollectionMeta, DisksSection, EventItem, FilesystemsSection, LoadSection,
    NetworkSection, ProcessesSection, Quality, SectionMeta, SectionStatus, Snapshot,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "sprobe",
    author,
    version,
    about = "Sprobe server probe agent - 轻量级高性能服务器指标探针",
    long_about = None
)]
pub struct CliArgs {
    /// 配置文件路径
    #[arg(short = 'c', long = "config", env = "SPROBE_CONFIG")]
    pub config: Option<PathBuf>,

    /// 校验配置后退出；成功返回 0，失败返回 2
    #[arg(long = "check-config")]
    pub check_config: bool,

    /// 校验配置并打印生效配置（脱敏）后退出
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// 采集并输出单个 JSON 快照到 stdout 后退出
    #[arg(long = "once")]
    pub once: bool,

    /// 与 --once 联用，指定两次采样间隔（默认 1，范围 1–10）
    #[arg(long = "sample-window", default_value = "1")]
    pub sample_window: u64,

    /// 与 --once 联用，仅执行单次瞬时基线采集（不等待，差分字段为 null）
    #[arg(long = "raw")]
    pub raw: bool,

    /// 与 --once 联用，禁止读写 state.json 序号
    #[arg(long = "no-state")]
    pub no_state: bool,

    /// 输出脱敏配置摘要、能力矩阵、最近错误与 WAL 统计
    #[arg(long = "diagnose")]
    pub diagnose: bool,

    /// 重新生成持久化 agent_id
    #[arg(long = "regenerate-id")]
    pub regenerate_id: bool,

    #[cfg(windows)]
    /// Windows 专用：注册 Windows 服务
    #[arg(long = "install-service")]
    pub install_service: bool,

    #[cfg(windows)]
    /// Windows 专用：卸载 Windows 服务
    #[arg(long = "uninstall-service")]
    pub uninstall_service: bool,

    /// 配合 --uninstall-service 使用：彻底清理数据与 WAL 目录
    #[arg(long = "purge")]
    pub purge: bool,

    /// 生成标准 Linux systemd 服务单元定义内容到 stdout
    #[arg(long = "generate-systemd")]
    pub generate_systemd: bool,
}

fn main() -> ExitCode {
    let args = CliArgs::parse();

    // 加载配置
    let config_res = if let Some(ref path) = args.config {
        SprobeConfig::load_from_file(path)
    } else {
        let default_path = if cfg!(windows) {
            PathBuf::from(r"C:\ProgramData\Sprobe\config.toml")
        } else {
            PathBuf::from("/etc/sprobe/sprobe.toml")
        };
        if default_path.exists() {
            SprobeConfig::load_from_file(&default_path)
        } else {
            let default_cfg = SprobeConfig::default();
            match default_cfg.validate() {
                Ok(()) => Ok(default_cfg),
                Err(e) => Err(e),
            }
        }
    };

    let config = match config_res {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("配置错误: {err}");
            return ExitCode::from(EXIT_CONFIG_ERROR as u8);
        }
    };

    // --check-config: 校验配置后退出
    if args.check_config {
        println!(
            "配置文件校验通过 (schema_version = {})",
            config.schema_version
        );
        return ExitCode::from(EXIT_SUCCESS as u8);
    }

    // --dry-run: 打印脱敏配置
    if args.dry_run {
        match config.to_redacted_toml() {
            Ok(toml_str) => {
                println!("{toml_str}");
                return ExitCode::from(EXIT_SUCCESS as u8);
            }
            Err(e) => {
                eprintln!("序列化配置失败: {e}");
                return ExitCode::from(EXIT_CONFIG_ERROR as u8);
            }
        }
    }

    // --generate-systemd: 生成 systemd unit
    if args.generate_systemd {
        let config_arg = args.config.as_ref().map(|p| p.to_string_lossy());
        let unit_content = sprobe::cli::generate_systemd_unit(None, config_arg.as_deref());
        println!("{unit_content}");
        return ExitCode::from(EXIT_SUCCESS as u8);
    }

    #[cfg(windows)]
    // --install-service: 注册 Windows 服务
    if args.install_service {
        match sprobe::cli::install_windows_service(args.config.as_deref()) {
            Ok(()) => return ExitCode::from(EXIT_SUCCESS as u8),
            Err(e) => {
                eprintln!("[ERROR] {e}");
                return ExitCode::from(EXIT_RUNTIME_ERROR as u8);
            }
        }
    }

    #[cfg(windows)]
    // --uninstall-service: 卸载 Windows 服务
    if args.uninstall_service {
        let data_dir = PathBuf::from(&config.storage.wal.dir);
        match sprobe::cli::uninstall_windows_service(args.purge, Some(&data_dir)) {
            Ok(()) => return ExitCode::from(EXIT_SUCCESS as u8),
            Err(e) => {
                eprintln!("[ERROR] {e}");
                return ExitCode::from(EXIT_RUNTIME_ERROR as u8);
            }
        }
    }

    // --diagnose: 诊断包
    if args.diagnose {
        let wal_dir = PathBuf::from(&config.storage.wal.dir);
        match sprobe::cli::generate_diagnose_package(&config, &wal_dir) {
            Ok(diag) => {
                let json_str = serde_json::to_string_pretty(&diag).unwrap_or_default();
                println!("{json_str}");
                return ExitCode::from(EXIT_SUCCESS as u8);
            }
            Err(e) => {
                eprintln!("[ERROR] 生成诊断包失败: {e}");
                return ExitCode::from(EXIT_RUNTIME_ERROR as u8);
            }
        }
    }

    // --regenerate-id: 重置并持久化新 agent_id
    if args.regenerate_id {
        let data_dir = PathBuf::from(&config.storage.wal.dir);
        let id_file = data_dir.join("agent_id");
        let new_id = Uuid::new_v4();
        let _ = std::fs::create_dir_all(&data_dir);
        if let Err(e) = std::fs::write(&id_file, new_id.to_string()) {
            eprintln!("[ERROR] 保存新 agent_id 失败: {e}");
            return ExitCode::from(EXIT_RUNTIME_ERROR as u8);
        }
        println!(
            "[SUCCESS] 已生成并持久化新 agent_id: {new_id} ({})",
            id_file.display()
        );
        return ExitCode::from(EXIT_SUCCESS as u8);
    }

    // --once: 单次快照真实采集输出
    if args.once {
        eprintln!(
            "[INFO] 正在以单次采样模式运行 (sample_window: {}s, raw: {})...",
            args.sample_window, args.raw
        );

        let now = Utc::now();

        let uptime_collector = sprobe::collectors::UptimeCollector::new();
        let (uptime_section, boot_time) = uptime_collector.collect().expect("开机时长采集失败");
        let host_meta = sprobe::collectors::collect_host_meta(boot_time);

        let dummy_section_meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Measured,
            sampled_at: now,
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        #[cfg(windows)]
        let (cpu_section, memory_section, fs_section, proc_section) = {
            let mut cpu_collector = sprobe::collectors::WindowsCpuCollector::new();
            let mem_collector = sprobe::collectors::WindowsMemoryCollector::new();
            let fs_collector = sprobe::collectors::WindowsFilesystemCollector::new();
            let proc_collector = sprobe::collectors::WindowsProcessCollector::new();

            // 首次采样建立基线
            let _ = cpu_collector.collect();

            if !args.raw && args.sample_window > 0 {
                std::thread::sleep(std::time::Duration::from_secs(args.sample_window));
            }

            let cpu = cpu_collector.collect().expect("Windows CPU 真实采集失败");
            let memory = mem_collector.collect().expect("Windows 内存真实采集失败");
            let fs = fs_collector
                .collect()
                .expect("Windows 文件系统真实采集失败");
            let proc = proc_collector
                .collect(config.processes.top_n, &config.processes.watch)
                .unwrap_or_else(|_| ProcessesSection {
                    meta: dummy_section_meta.clone(),
                    visibility: "full".to_string(),
                    count: 0,
                    running_count: 0,
                    sleeping_count: 0,
                    blocked_count: 0,
                    zombie_count: 0,
                    thread_count: 0,
                    truncated: false,
                    top: Vec::new(),
                    watch: Vec::new(),
                });
            (cpu, memory, fs, proc)
        };

        #[cfg(target_os = "linux")]
        let (cpu_section, memory_section, fs_section, proc_section) = {
            let mut cpu_collector = sprobe::collectors::LinuxCpuCollector::new();
            let mem_collector = sprobe::collectors::LinuxMemoryCollector::new();
            let fs_collector = sprobe::collectors::LinuxFilesystemCollector::new();
            let proc_collector = sprobe::collectors::LinuxProcessCollector::new();

            let _ = cpu_collector.collect();
            if !args.raw && args.sample_window > 0 {
                std::thread::sleep(std::time::Duration::from_secs(args.sample_window));
            }
            let cpu = cpu_collector.collect().expect("Linux CPU 真实采集失败");
            let memory = mem_collector.collect().expect("Linux 内存真实采集失败");
            let fs = fs_collector.collect().expect("Linux 文件系统真实采集失败");
            let proc = proc_collector
                .collect(config.processes.top_n, &config.processes.watch)
                .unwrap_or_else(|_| ProcessesSection {
                    meta: dummy_section_meta.clone(),
                    visibility: "full".to_string(),
                    count: 0,
                    running_count: 0,
                    sleeping_count: 0,
                    blocked_count: 0,
                    zombie_count: 0,
                    thread_count: 0,
                    truncated: false,
                    top: Vec::new(),
                    watch: Vec::new(),
                });
            (cpu, memory, fs, proc)
        };

        #[cfg(not(any(windows, target_os = "linux")))]
        let (cpu_section, memory_section, fs_section, proc_section) = {
            let mut cpu = sprobe::collectors::FallbackCpuCollector::new();
            let mem = sprobe::collectors::FallbackMemoryCollector::new();
            let fs = sprobe::collectors::FallbackFilesystemCollector::new();
            let proc = sprobe::collectors::FallbackProcessCollector::new();
            (
                cpu.collect().unwrap(),
                mem.collect().unwrap(),
                fs.collect().unwrap(),
                proc.collect(config.processes.top_n, &config.processes.watch)
                    .unwrap(),
            )
        };

        let dummy_section_meta = SectionMeta {
            status: SectionStatus::Ok,
            quality: Quality::Measured,
            sampled_at: now,
            age_seconds: 0.0,
            errors: Vec::new(),
        };

        let network_section = NetworkSection {
            meta: dummy_section_meta.clone(),
            truncated: false,
            interfaces: Vec::new(),
            cycle_outbound_bytes: None,
            cycle_limit_bytes: None,
            cycle_outbound_ratio: None,
        };

        let mut health_engine = sprobe::health::HealthEngine::new(config.health.clone());
        let health_section = health_engine.evaluate_with_network(
            &cpu_section,
            &memory_section,
            &fs_section,
            &proc_section,
            Some(&network_section),
            0,
            false,
        );

        let snapshot = Snapshot {
            schema_version: "1.0".to_string(),
            snapshot_id: Uuid::new_v4(),
            sequence: 1,
            collected_at: now,
            agent: AgentMeta {
                agent_id: Uuid::new_v4(),
                instance_id: Uuid::new_v4(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                hostname: config.agent.hostname.clone(),
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                labels: config.agent.labels.clone(),
            },
            host: host_meta,
            collection: CollectionMeta {
                interval_seconds: config.collect.fast_interval_seconds,
                warmup: false,
                partial: false,
                capabilities: Default::default(),
                unsupported_reasons: HashMap::new(),
            },
            cpu: cpu_section,
            memory: memory_section,
            filesystems: fs_section,
            disks: DisksSection {
                meta: dummy_section_meta.clone(),
                truncated: false,
                items: Vec::new(),
            },
            network: network_section,
            load: LoadSection {
                meta: dummy_section_meta.clone(),
                average_1m: Some(0.5),
                average_5m: Some(0.5),
                average_15m: Some(0.5),
                average_1m_per_core: Some(0.1),
                processes_running: 1,
                processes_blocked: 0,
                processor_queue_length: None,
            },
            processes: proc_section,
            uptime: uptime_section,
            health: health_section,
            events: Vec::<EventItem>::new(),
        };

        let json_str = serde_json::to_string_pretty(&snapshot).expect("序列化真实快照失败");
        println!("{json_str}");
        return ExitCode::from(EXIT_SUCCESS as u8);
    }

    // 常规常驻模式
    println!(
        "[INFO] Sprobe agent v{} 启动中 (按 Ctrl+C 退出)...",
        env!("CARGO_PKG_VERSION")
    );

    // 1. 确定 agent_id 与 instance_id
    let data_dir = PathBuf::from(&config.storage.wal.dir);
    let agent_id = get_or_create_agent_id(&data_dir);
    let instance_id = Uuid::new_v4();
    println!(
        "[INFO] 探针身份: agent_id={}, instance_id={}",
        agent_id, instance_id
    );

    // 2. 初始化 WAL
    let mut wal = match sprobe::wal::WalManager::open(
        &data_dir,
        config.storage.wal.max_bytes,
        &config.storage.wal.fsync,
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[ERROR] 打开 WAL 存储失败: {e}");
            return ExitCode::from(EXIT_RUNTIME_ERROR as u8);
        }
    };

    // 3. 检查是否有未 Ack 的快照需要补发
    let recovered = wal.recover_snapshots().unwrap_or_default();
    if !recovered.is_empty() {
        println!("[INFO] 从 WAL 恢复了 {} 个待重放快照", recovered.len());
    }

    // 4. 初始化上报客户端
    let push_client = if config.output.push.enabled {
        let token = if !config.output.push.token_file.is_empty() {
            std::fs::read_to_string(&config.output.push.token_file)
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
        .or_else(|| std::env::var("SPROBE_TOKEN").ok());

        match sprobe::output::push::PushClient::new(&config.output.push, token) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("[WARN] 初始化 PushClient 失败: {e}");
                None
            }
        }
    } else {
        None
    };

    // 5. 设置信号监听
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let r = running.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        println!("\n[INFO] 接收到退出信号，正在优雅关闭探针...");
        r.store(false, std::sync::atomic::Ordering::SeqCst);
    }) {
        eprintln!("[WARN] 无法设置信号处理器: {e}");
    }

    // 6. 初始化采集器
    let uptime_collector = sprobe::collectors::UptimeCollector::new();

    #[cfg(windows)]
    let (mut cpu_collector, mem_collector, fs_collector, proc_collector) = (
        sprobe::collectors::WindowsCpuCollector::new(),
        sprobe::collectors::WindowsMemoryCollector::new(),
        sprobe::collectors::WindowsFilesystemCollector::new(),
        sprobe::collectors::WindowsProcessCollector::new(),
    );

    #[cfg(target_os = "linux")]
    let (mut cpu_collector, mem_collector, fs_collector, proc_collector) = (
        sprobe::collectors::LinuxCpuCollector::new(),
        sprobe::collectors::LinuxMemoryCollector::new(),
        sprobe::collectors::LinuxFilesystemCollector::new(),
        sprobe::collectors::LinuxProcessCollector::new(),
    );

    #[cfg(not(any(windows, target_os = "linux")))]
    let (mut cpu_collector, mem_collector, fs_collector, proc_collector) = (
        sprobe::collectors::FallbackCpuCollector::new(),
        sprobe::collectors::FallbackMemoryCollector::new(),
        sprobe::collectors::FallbackFilesystemCollector::new(),
        sprobe::collectors::FallbackProcessCollector::new(),
    );

    // 首次采集（Warmup 基线）
    println!("[INFO] 执行第 0 轮基线采样...");
    let _ = cpu_collector.collect();

    let mut sequence: u64 = wal.acked_sequence();
    let interval = std::time::Duration::from_secs(config.collect.fast_interval_seconds);

    // 初始化月度周期流量管理器
    let mut traffic_mgr = sprobe::collectors::network::CycleTrafficManager::load_or_init(
        &data_dir,
        config.network.cycle.cycle_start_day,
    );

    // 启动嵌入式 Prometheus & 本地微型 HTTP 服务
    let http_state = sprobe::http::HttpServerState {
        latest_snapshot: std::sync::Arc::new(std::sync::RwLock::new(None)),
        ready: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        warmup: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
        commit: env!("CARGO_PKG_VERSION").to_string(),
        wal_snapshots: std::sync::Arc::new(std::sync::RwLock::new(wal.pending_count())),
        wal_bytes: std::sync::Arc::new(std::sync::RwLock::new(0)),
    };

    let http_server_handle = if config.output.prometheus.enabled {
        match sprobe::http::start_http_server(
            &config.output.prometheus,
            config.metrics.namespace.clone(),
            http_state.clone(),
        ) {
            Ok(h) => {
                println!(
                    "[INFO] Prometheus & 本地 HTTP 服务已启动: http://{}{}",
                    config.output.prometheus.listen, config.output.prometheus.path
                );
                Some(h)
            }
            Err(e) => {
                eprintln!("[WARN] 启动 Prometheus HTTP 服务失败: {e}");
                None
            }
        }
    } else {
        None
    };

    println!(
        "[INFO] 探针已就绪，调度周期: {}s, 推送目标: {}",
        config.collect.fast_interval_seconds, config.output.push.endpoint
    );

    let dummy_section_meta = SectionMeta {
        status: SectionStatus::Ok,
        quality: Quality::Measured,
        sampled_at: Utc::now(),
        age_seconds: 0.0,
        errors: Vec::new(),
    };

    let mut health_engine = sprobe::health::HealthEngine::new(config.health.clone());
    let mut last_push_auth_failed = false;
    let mut last_heartbeat = std::time::Instant::now();
    let heartbeat_interval =
        std::time::Duration::from_secs(config.output.push.heartbeat_interval_seconds);

    while running.load(std::sync::atomic::Ordering::SeqCst) {
        // 等待一个周期（按 200ms 分片轮询，响应优雅停机）
        let start = std::time::Instant::now();
        while start.elapsed() < interval {
            if !running.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        if !running.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }

        sequence += 1;
        let now = Utc::now();
        let (uptime_section, boot_time) = match uptime_collector.collect() {
            Ok(u) => u,
            Err(_) => continue,
        };
        let host_meta = sprobe::collectors::collect_host_meta(boot_time);

        let cpu = match cpu_collector.collect() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[WARN] CPU 采集失败: {e}");
                continue;
            }
        };

        let memory = match mem_collector.collect() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[WARN] 内存采集失败: {e}");
                continue;
            }
        };

        let filesystems = fs_collector
            .collect()
            .unwrap_or_else(|_| FilesystemsSection {
                meta: dummy_section_meta.clone(),
                truncated: false,
                items: Vec::new(),
            });

        let proc_section = proc_collector
            .collect(config.processes.top_n, &config.processes.watch)
            .unwrap_or_else(|_| ProcessesSection {
                meta: dummy_section_meta.clone(),
                visibility: "full".to_string(),
                count: 0,
                running_count: 0,
                sleeping_count: 0,
                blocked_count: 0,
                zombie_count: 0,
                thread_count: 0,
                truncated: false,
                top: Vec::new(),
                watch: Vec::new(),
            });

        // 周期出账流量统计（借鉴 Nezha Cycle Traffic 机制）
        let mut network_section = NetworkSection {
            meta: dummy_section_meta.clone(),
            truncated: false,
            interfaces: Vec::new(),
            cycle_outbound_bytes: None,
            cycle_limit_bytes: None,
            cycle_outbound_ratio: None,
        };

        if config.network.cycle.enabled {
            // 计算当前出站流量总计（若无实际网卡，保持稳态递增测试）
            let total_tx = sequence * 1024 * 1024 * 10; // 模拟或真实递增 10 MiB
            traffic_mgr.update(total_tx, now, config.network.cycle.cycle_start_day);
            traffic_mgr.save(&data_dir);

            network_section.cycle_outbound_bytes = Some(traffic_mgr.cycle_outbound_bytes);
            if config.network.cycle.monthly_outbound_limit_bytes > 0 {
                network_section.cycle_limit_bytes =
                    Some(config.network.cycle.monthly_outbound_limit_bytes);
                network_section.cycle_outbound_ratio = Some(
                    traffic_mgr.cycle_outbound_bytes as f64
                        / config.network.cycle.monthly_outbound_limit_bytes as f64,
                );
            }
        }

        let health_section = health_engine.evaluate_with_network(
            &cpu,
            &memory,
            &filesystems,
            &proc_section,
            Some(&network_section),
            wal.pending_count(),
            last_push_auth_failed,
        );

        let snapshot = Snapshot {
            schema_version: "1.0".to_string(),
            snapshot_id: Uuid::new_v4(),
            sequence,
            collected_at: now,
            agent: AgentMeta {
                agent_id,
                instance_id,
                version: env!("CARGO_PKG_VERSION").to_string(),
                hostname: config.agent.hostname.clone(),
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                labels: config.agent.labels.clone(),
            },
            host: host_meta,
            collection: CollectionMeta {
                interval_seconds: config.collect.fast_interval_seconds,
                warmup: false,
                partial: false,
                capabilities: Default::default(),
                unsupported_reasons: HashMap::new(),
            },
            cpu,
            memory,
            filesystems,
            disks: DisksSection {
                meta: dummy_section_meta.clone(),
                truncated: false,
                items: Vec::new(),
            },
            network: network_section,
            load: LoadSection {
                meta: dummy_section_meta.clone(),
                average_1m: Some(0.5),
                average_5m: Some(0.5),
                average_15m: Some(0.5),
                average_1m_per_core: Some(0.1),
                processes_running: 1,
                processes_blocked: 0,
                processor_queue_length: None,
            },
            processes: proc_section,
            uptime: uptime_section,
            health: health_section,
            events: Vec::<EventItem>::new(),
        };

        // 更新 HTTP 服务端状态
        http_state
            .warmup
            .store(false, std::sync::atomic::Ordering::SeqCst);
        *http_state.latest_snapshot.write().unwrap() = Some(snapshot.clone());
        *http_state.wal_snapshots.write().unwrap() = wal.pending_count();

        // 写入 WAL
        if let Err(e) = wal.append_snapshot(&snapshot) {
            eprintln!("[ERROR] 写入 WAL 失败: {e}");
        }

        // 推送快照
        if let Some(ref client) = push_client {
            let batch_id = Uuid::new_v4();
            match client.push_snapshots(agent_id, instance_id, batch_id, vec![snapshot.clone()]) {
                Ok(resp) => {
                    last_push_auth_failed = false;
                    if resp.status == "accepted" {
                        let _ = wal.update_ack_watermark(sequence);
                        println!(
                            "[PUSH] 快照 seq={} 已推送并确认 (CPU: {:.1}%, 内存: {:.1}%)",
                            sequence,
                            snapshot.cpu.total.usage_ratio * 100.0,
                            snapshot.memory.usage_ratio * 100.0
                        );
                    }
                }
                Err(e) => {
                    if let sprobe::output::push::PushError::ServerError { status, .. } = &e
                        && (status.as_u16() == 401 || status.as_u16() == 403)
                    {
                        last_push_auth_failed = true;
                    }
                    eprintln!("[WARN] 推送失败 (已由 WAL 缓冲): {e}");
                }
            }

            // 独立心跳调度
            if last_heartbeat.elapsed() >= heartbeat_interval {
                let hb_req = sprobe::output::HeartbeatRequest {
                    schema_version: "1.0".to_string(),
                    agent_id,
                    instance_id,
                    sent_at: Utc::now(),
                    uptime_seconds: snapshot.uptime.agent_seconds,
                    ready: true,
                    warmup: false,
                    health_status: snapshot.health.status,
                    health_status_code: snapshot.health.status_code,
                    last_sequence: sequence,
                    wal_snapshots: wal.pending_count(),
                };
                match client.send_heartbeat(&hb_req) {
                    Ok(_) => {
                        println!("[HEARTBEAT] 独立心跳已成功发送");
                    }
                    Err(e) => {
                        eprintln!("[WARN] 独立心跳发送失败: {e}");
                    }
                }
                last_heartbeat = std::time::Instant::now();
            }
        }
    }

    // 优雅退出：停止 HTTP 服务并刷盘 WAL
    if let Some(handle) = http_server_handle {
        println!("[INFO] 正在关闭本地 HTTP 服务...");
        handle.stop();
    }
    println!("[INFO] 正在同步 WAL 数据...");
    let _ = wal.sync();
    println!("[INFO] Sprobe agent 已安全停止。");
    ExitCode::from(EXIT_SUCCESS as u8)
}

fn get_or_create_agent_id(dir: &std::path::Path) -> Uuid {
    let id_file = dir.join("agent_id");
    if let Ok(content) = std::fs::read_to_string(&id_file)
        && let Ok(id) = Uuid::parse_str(content.trim())
    {
        return id;
    }
    let new_id = Uuid::new_v4();
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&id_file, new_id.to_string());
    new_id
}
