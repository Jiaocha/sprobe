use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("配置文件不存在: {0}")]
    NotFound(PathBuf),
    #[error("配置文件读取失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML 解析失败: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("配置校验失败: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprobeConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub collect: CollectConfig,
    #[serde(default)]
    pub cpu: CpuConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub filesystems: FilesystemsConfig,
    #[serde(default)]
    pub disks: DisksConfig,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub processes: ProcessesConfig,
    #[serde(default)]
    pub load: LoadConfig,
    #[serde(default)]
    pub health: HealthConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

impl Default for SprobeConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            agent: AgentConfig::default(),
            collect: CollectConfig::default(),
            cpu: CpuConfig::default(),
            memory: MemoryConfig::default(),
            filesystems: FilesystemsConfig::default(),
            disks: DisksConfig::default(),
            network: NetworkConfig::default(),
            processes: ProcessesConfig::default(),
            load: LoadConfig::default(),
            health: HealthConfig::default(),
            metrics: MetricsConfig::default(),
            output: OutputConfig::default(),
            storage: StorageConfig::default(),
            security: SecurityConfig::default(),
            log: LogConfig::default(),
            limits: LimitsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    #[serde(default = "default_state_file")]
    pub state_file: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub proc_prefix: String,
    #[serde(default)]
    pub sys_prefix: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

fn default_data_dir() -> String {
    if cfg!(windows) {
        r"C:\ProgramData\Sprobe\data".to_string()
    } else {
        "/var/lib/sprobe".to_string()
    }
}

fn default_state_file() -> String {
    "state.json".to_string()
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            state_file: default_state_file(),
            hostname: String::new(),
            proc_prefix: String::new(),
            sys_prefix: String::new(),
            labels: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectConfig {
    #[serde(default = "default_fast_interval")]
    pub fast_interval_seconds: u64,
    #[serde(default = "default_medium_interval")]
    pub medium_interval_seconds: u64,
    #[serde(default = "default_slow_interval")]
    pub slow_interval_seconds: u64,
    #[serde(default = "default_collector_timeout")]
    pub collector_timeout_seconds: f64,
    #[serde(default = "default_snapshot_timeout")]
    pub snapshot_timeout_seconds: f64,
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_collectors: usize,
    #[serde(default = "default_jitter_ratio")]
    pub jitter_ratio: f64,
}

fn default_fast_interval() -> u64 {
    10
}
fn default_medium_interval() -> u64 {
    30
}
fn default_slow_interval() -> u64 {
    300
}
fn default_collector_timeout() -> f64 {
    2.0
}
fn default_snapshot_timeout() -> f64 {
    5.0
}
fn default_max_concurrent() -> usize {
    4
}
fn default_jitter_ratio() -> f64 {
    0.2
}

impl Default for CollectConfig {
    fn default() -> Self {
        Self {
            fast_interval_seconds: default_fast_interval(),
            medium_interval_seconds: default_medium_interval(),
            slow_interval_seconds: default_slow_interval(),
            collector_timeout_seconds: default_collector_timeout(),
            snapshot_timeout_seconds: default_snapshot_timeout(),
            max_concurrent_collectors: default_max_concurrent(),
            jitter_ratio: default_jitter_ratio(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub per_core: bool,
    #[serde(default = "default_true")]
    pub frequency: bool,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            per_core: true,
            frequency: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub include_swap: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            include_swap: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub root_prefix: String,
    #[serde(default = "default_include_fstypes")]
    pub include_fstypes: Vec<String>,
    #[serde(default = "default_exclude_fstypes")]
    pub exclude_fstypes: Vec<String>,
    #[serde(default)]
    pub include_mountpoints: Vec<String>,
    #[serde(default = "default_exclude_mountpoints")]
    pub exclude_mountpoints: Vec<String>,
    #[serde(default = "default_fs_network_timeout")]
    pub network_timeout_seconds: f64,
}

fn default_include_fstypes() -> Vec<String> {
    vec![
        "ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "ntfs", "apfs", "vfat", "exfat",
        "nfs", "nfs4", "cifs",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_exclude_fstypes() -> Vec<String> {
    vec![
        "proc",
        "sysfs",
        "devtmpfs",
        "devpts",
        "cgroup",
        "cgroup2",
        "securityfs",
        "debugfs",
        "tracefs",
        "pstore",
        "bpf",
        "configfs",
        "fusectl",
        "mqueue",
        "hugetlbfs",
        "autofs",
        "binfmt_misc",
        "rpc_pipefs",
        "nsfs",
        "squashfs",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn default_exclude_mountpoints() -> Vec<String> {
    vec!["/snap".to_string(), "/var/lib/docker/overlay2".to_string()]
}

fn default_fs_network_timeout() -> f64 {
    2.0
}

impl Default for FilesystemsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            root_prefix: String::new(),
            include_fstypes: default_include_fstypes(),
            exclude_fstypes: default_exclude_fstypes(),
            include_mountpoints: Vec::new(),
            exclude_mountpoints: default_exclude_mountpoints(),
            network_timeout_seconds: default_fs_network_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisksConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub include_partitions: bool,
    #[serde(default = "default_true")]
    pub include_virtual: bool,
}

impl Default for DisksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            include_partitions: false,
            include_virtual: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub include_loopback: bool,
    #[serde(default)]
    pub include_slaves: bool,
    #[serde(default = "default_true")]
    pub include_virtual: bool,
    #[serde(default)]
    pub aggregate_interfaces: Vec<String>,
    #[serde(default)]
    pub include_interfaces: Vec<String>,
    #[serde(default = "default_exclude_interfaces")]
    pub exclude_interfaces: Vec<String>,
    #[serde(default = "default_true")]
    pub ignore_virtual: bool,
    #[serde(default)]
    pub cycle: NetworkCycleConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkCycleConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_cycle_start_day")]
    pub cycle_start_day: u8,
    #[serde(default)]
    pub monthly_outbound_limit_bytes: u64,
    #[serde(default = "default_warning_ratio")]
    pub warning_ratio: f64,
}

fn default_cycle_start_day() -> u8 {
    1
}

fn default_warning_ratio() -> f64 {
    0.80
}

fn default_exclude_interfaces() -> Vec<String> {
    vec![
        "docker*".to_string(),
        "veth*".to_string(),
        "br-*".to_string(),
        "cni*".to_string(),
        "tailscale*".to_string(),
    ]
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            include_loopback: false,
            include_slaves: false,
            include_virtual: true,
            aggregate_interfaces: Vec::new(),
            include_interfaces: Vec::new(),
            exclude_interfaces: default_exclude_interfaces(),
            ignore_virtual: true,
            cycle: NetworkCycleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessesConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_top_n")]
    pub top_n: usize,
    #[serde(default)]
    pub include_exe: bool,
    #[serde(default)]
    pub watch: Vec<String>,
}

fn default_top_n() -> usize {
    20
}

impl Default for ProcessesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            top_n: default_top_n(),
            include_exe: false,
            watch: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub windows_synthetic: bool,
}

impl Default for LoadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            windows_synthetic: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub disabled_checks: Vec<String>,
    #[serde(default)]
    pub thresholds: HealthThresholdsConfig,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            disabled_checks: Vec::new(),
            thresholds: HealthThresholdsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholdsConfig {
    #[serde(default = "default_ratio_90")]
    pub cpu_usage_ratio: f64,
    #[serde(default = "default_ratio_20")]
    pub cpu_iowait_ratio: f64,
    #[serde(default = "default_ratio_90")]
    pub memory_usage_warning: f64,
    #[serde(default = "default_ratio_95")]
    pub memory_usage_critical: f64,
    #[serde(default = "default_ratio_90")]
    pub disk_usage_warning: f64,
    #[serde(default = "default_ratio_95")]
    pub disk_usage_critical: f64,
    #[serde(default = "default_ratio_90")]
    pub inode_usage_ratio: f64,
    #[serde(default = "default_disk_latency")]
    pub disk_latency_seconds: f64,
    #[serde(default = "default_network_error_rate")]
    pub network_error_rate: f64,
    #[serde(default = "default_load_per_core")]
    pub load_per_core: f64,
    #[serde(default = "default_backlog_snapshots")]
    pub agent_backlog_snapshots: u64,
    #[serde(default = "default_backlog_age")]
    pub agent_backlog_age_seconds: u64,
}

fn default_ratio_90() -> f64 {
    0.90
}
fn default_ratio_95() -> f64 {
    0.95
}
fn default_ratio_20() -> f64 {
    0.20
}
fn default_disk_latency() -> f64 {
    0.100
}
fn default_network_error_rate() -> f64 {
    1.0
}
fn default_load_per_core() -> f64 {
    2.0
}
fn default_backlog_snapshots() -> u64 {
    1000
}
fn default_backlog_age() -> u64 {
    1800
}

impl Default for HealthThresholdsConfig {
    fn default() -> Self {
        Self {
            cpu_usage_ratio: default_ratio_90(),
            cpu_iowait_ratio: default_ratio_20(),
            memory_usage_warning: default_ratio_90(),
            memory_usage_critical: default_ratio_95(),
            disk_usage_warning: default_ratio_90(),
            disk_usage_critical: default_ratio_95(),
            inode_usage_ratio: default_ratio_90(),
            disk_latency_seconds: default_disk_latency(),
            network_error_rate: default_network_error_rate(),
            load_per_core: default_load_per_core(),
            agent_backlog_snapshots: default_backlog_snapshots(),
            agent_backlog_age_seconds: default_backlog_age(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_namespace")]
    pub namespace: String,
    #[serde(default)]
    pub expose_quality: bool,
}

fn default_namespace() -> String {
    "sprobe".to_string()
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            namespace: default_namespace(),
            expose_quality: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub push: PushConfig,
    #[serde(default)]
    pub prometheus: PrometheusConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_push_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_heartbeat_endpoint")]
    pub heartbeat_endpoint: String,
    #[serde(default = "default_token_file")]
    pub token_file: String,
    #[serde(default = "default_medium_interval")]
    pub interval_seconds: u64,
    #[serde(default = "default_medium_interval")]
    pub heartbeat_interval_seconds: u64,
    #[serde(default = "default_batch_max_snapshots")]
    pub batch_max_snapshots: usize,
    #[serde(default = "default_batch_max_bytes")]
    pub batch_max_bytes: usize,
    #[serde(default)]
    pub allow_out_of_order: bool,
    #[serde(default = "default_one")]
    pub max_in_flight_batches: usize,
    #[serde(default = "default_request_timeout")]
    pub request_timeout_seconds: f64,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_seconds: f64,
    #[serde(default = "default_true")]
    pub gzip: bool,
    #[serde(default)]
    pub tls_ca_file: String,
    #[serde(default)]
    pub tls_insecure_skip_verify: bool,
    #[serde(default)]
    pub allow_insecure: bool,
    #[serde(default)]
    pub client_cert_file: String,
    #[serde(default)]
    pub client_key_file: String,
}

fn default_push_endpoint() -> String {
    "https://monitor.example.com/api/v1/ingest".to_string()
}
fn default_heartbeat_endpoint() -> String {
    "https://monitor.example.com/api/v1/heartbeat".to_string()
}
fn default_token_file() -> String {
    if cfg!(windows) {
        r"C:\ProgramData\Sprobe\token".to_string()
    } else {
        "/etc/sprobe/token".to_string()
    }
}
fn default_batch_max_snapshots() -> usize {
    6
}
fn default_batch_max_bytes() -> usize {
    1048576
}
fn default_one() -> usize {
    1
}
fn default_request_timeout() -> f64 {
    10.0
}
fn default_connect_timeout() -> f64 {
    3.0
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: default_push_endpoint(),
            heartbeat_endpoint: default_heartbeat_endpoint(),
            token_file: default_token_file(),
            interval_seconds: default_medium_interval(),
            heartbeat_interval_seconds: default_medium_interval(),
            batch_max_snapshots: default_batch_max_snapshots(),
            batch_max_bytes: default_batch_max_bytes(),
            allow_out_of_order: false,
            max_in_flight_batches: default_one(),
            request_timeout_seconds: default_request_timeout(),
            connect_timeout_seconds: default_connect_timeout(),
            gzip: true,
            tls_ca_file: String::new(),
            tls_insecure_skip_verify: false,
            allow_insecure: false,
            client_cert_file: String::new(),
            client_key_file: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_prom_listen")]
    pub listen: String,
    #[serde(default = "default_prom_path")]
    pub path: String,
    #[serde(default)]
    pub allow_remote: bool,
}

fn default_prom_listen() -> String {
    "127.0.0.1:9101".to_string()
}
fn default_prom_path() -> String {
    "/metrics".to_string()
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen: default_prom_listen(),
            path: default_prom_path(),
            allow_remote: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default)]
    pub wal: WalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalConfig {
    #[serde(default = "default_wal_dir")]
    pub dir: String,
    #[serde(default = "default_wal_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_wal_max_age")]
    pub max_age_seconds: u64,
    #[serde(default = "default_fsync")]
    pub fsync: String,
}

fn default_wal_dir() -> String {
    if cfg!(windows) {
        r"C:\ProgramData\Sprobe\data\wal".to_string()
    } else {
        "/var/lib/sprobe/wal".to_string()
    }
}
fn default_wal_max_bytes() -> usize {
    100663296
} // 96 MiB
fn default_wal_max_age() -> u64 {
    3600
}
fn default_fsync() -> String {
    "batch".to_string()
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            dir: default_wal_dir(),
            max_bytes: default_wal_max_bytes(),
            max_age_seconds: default_wal_max_age(),
            fsync: default_fsync(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub allow_loose_token_permissions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default)]
    pub file: String,
    #[serde(default = "default_log_max_size")]
    pub max_size_bytes: usize,
    #[serde(default = "default_log_max_files")]
    pub max_files: usize,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_log_max_size() -> usize {
    10485760
}
fn default_log_max_files() -> usize {
    5
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file: String::new(),
            max_size_bytes: default_log_max_size(),
            max_files: default_log_max_files(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitsConfig {
    #[serde(default = "default_max_events")]
    pub max_events_per_snapshot: usize,
    #[serde(default = "default_max_string")]
    pub max_string_bytes: usize,
}

fn default_max_events() -> usize {
    100
}
fn default_max_string() -> usize {
    256
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_events_per_snapshot: default_max_events(),
            max_string_bytes: default_max_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

impl SprobeConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path_ref = path.as_ref();
        if !path_ref.exists() {
            return Err(ConfigError::NotFound(path_ref.to_path_buf()));
        }
        let content = std::fs::read_to_string(path_ref)?;
        let config: SprobeConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != "1.0" {
            return Err(ConfigError::Validation(format!(
                "不支持的 schema_version: {}, 仅支持 1.0",
                self.schema_version
            )));
        }

        if self.collect.fast_interval_seconds == 0 {
            return Err(ConfigError::Validation(
                "fast_interval_seconds 不能为 0".into(),
            ));
        }

        if self.collect.jitter_ratio < 0.0 || self.collect.jitter_ratio > 0.5 {
            return Err(ConfigError::Validation(
                "jitter_ratio 必须在 0.0 到 0.5 之间".into(),
            ));
        }

        if self.output.push.enabled {
            if self.output.push.endpoint.is_empty() {
                return Err(ConfigError::Validation("push.endpoint 不能为空".into()));
            }
            if self.output.push.endpoint.starts_with("http://")
                && (!self.output.push.allow_insecure || !self.output.push.tls_insecure_skip_verify)
            {
                return Err(ConfigError::Validation(
                    "明文 http:// 推送仅在显式设置 allow_insecure=true 和 tls_insecure_skip_verify=true 时允许".into(),
                ));
            }
        }

        match self.storage.wal.fsync.as_str() {
            "always" | "batch" | "never" => {}
            other => {
                return Err(ConfigError::Validation(format!(
                    "wal.fsync 无效: {}, 仅允许 always | batch | never",
                    other
                )));
            }
        }

        Ok(())
    }

    /// 输出脱敏后的配置字符串（不含敏感信息）
    pub fn to_redacted_toml(&self) -> Result<String, toml::ser::Error> {
        let mut cloned = self.clone();
        // 如果存在敏感文件配置或路径，在此进行展示屏蔽
        if !cloned.output.push.token_file.is_empty() {
            cloned.output.push.token_file = "[CONFIGURED]".to_string();
        }
        toml::to_string_pretty(&cloned)
    }
}
