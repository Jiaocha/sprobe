use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SectionStatus {
    Ok,
    Partial,
    Failed,
    Unsupported,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Measured,
    Derived,
    Estimated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Ok,
    Degraded,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionMeta {
    pub status: SectionStatus,
    pub quality: Quality,
    pub sampled_at: DateTime<Utc>,
    pub age_seconds: f64,
    #[serde(default)]
    pub errors: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMeta {
    pub agent_id: Uuid,
    pub instance_id: Uuid,
    pub version: String,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostMeta {
    pub hostname: String,
    pub os_name: String,
    pub kernel: String,
    pub architecture: String,
    pub virtualization: Option<String>,
    pub cpu_model: Option<String>,
    pub boot_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub cpu_modes: bool,
    pub cpu_frequency: bool,
    pub memory_available: bool,
    pub swap: bool,
    pub filesystem_inodes: bool,
    pub disk_io_counters: bool,
    pub network_speed: bool,
    pub process_top: bool,
    pub process_io: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            cpu_modes: true,
            cpu_frequency: true,
            memory_available: true,
            swap: true,
            filesystem_inodes: true,
            disk_io_counters: true,
            network_speed: true,
            process_top: true,
            process_io: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub interval_seconds: u64,
    pub warmup: bool,
    pub partial: bool,
    pub capabilities: Capabilities,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub unsupported_reasons: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTotal {
    pub usage_ratio: f64,
    pub idle_ratio: f64,
    pub iowait_ratio: f64,
    pub steal_ratio: f64,
    pub user_seconds_total: f64,
    pub nice_seconds_total: f64,
    pub system_seconds_total: f64,
    pub idle_seconds_total: f64,
    pub iowait_seconds_total: f64,
    pub irq_seconds_total: f64,
    pub softirq_seconds_total: f64,
    pub steal_seconds_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCore {
    pub cpu: String,
    pub usage_ratio: f64,
    pub idle_ratio: f64,
    pub iowait_ratio: f64,
    pub steal_ratio: f64,
    pub user_seconds_total: f64,
    pub nice_seconds_total: f64,
    pub system_seconds_total: f64,
    pub idle_seconds_total: f64,
    pub iowait_seconds_total: f64,
    pub irq_seconds_total: f64,
    pub softirq_seconds_total: f64,
    pub steal_seconds_total: f64,
    pub frequency_hertz: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub logical_count: usize,
    pub physical_count: usize,
    pub total: CpuTotal,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub cores: Vec<CpuCore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub available_source: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub cached_bytes: u64,
    pub buffers_bytes: u64,
    pub usage_ratio: f64,
    pub dirty_bytes: u64,
    pub writeback_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_free_bytes: u64,
    pub swap_usage_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemItem {
    pub device: String,
    pub volume_id: Option<String>,
    pub mountpoint: String,
    pub mountpoints: Vec<String>,
    pub fstype: String,
    pub read_only: bool,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub reserved_bytes: u64,
    pub usage_ratio: f64,
    pub inodes_total: u64,
    pub inodes_used: u64,
    pub inodes_free: u64,
    pub inodes_usage_ratio: f64,
    pub stale: bool,
    pub quality: Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemsSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub truncated: bool,
    pub items: Vec<FilesystemItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskItem {
    pub device: String,
    pub disk_class: String,
    pub parent: Option<String>,
    pub read_bytes_total: u64,
    pub write_bytes_total: u64,
    pub read_operations_total: u64,
    pub write_operations_total: u64,
    pub io_time_seconds_total: f64,
    pub io_time_weighted_seconds_total: f64,
    pub in_flight_operations: u64,
    pub read_bytes_per_second: Option<f64>,
    pub write_bytes_per_second: Option<f64>,
    pub utilization_ratio: Option<f64>,
    pub read_latency_seconds: Option<f64>,
    pub write_latency_seconds: Option<f64>,
    pub quality: Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisksSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub truncated: bool,
    pub items: Vec<DiskItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceItem {
    pub name: String,
    pub class: String,
    pub master: Option<String>,
    pub r#virtual: bool,
    pub up: bool,
    pub mtu: u32,
    pub speed_bytes_per_second: Option<u64>,
    pub receive_bytes_total: u64,
    pub transmit_bytes_total: u64,
    pub receive_packets_total: u64,
    pub transmit_packets_total: u64,
    pub receive_errors_total: u64,
    pub transmit_errors_total: u64,
    pub receive_dropped_total: u64,
    pub transmit_dropped_total: u64,
    pub receive_bytes_per_second: Option<f64>,
    pub transmit_bytes_per_second: Option<f64>,
    pub receive_utilization_ratio: Option<f64>,
    pub transmit_utilization_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub truncated: bool,
    pub interfaces: Vec<NetworkInterfaceItem>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cycle_outbound_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cycle_limit_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cycle_outbound_ratio: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub average_1m: Option<f64>,
    pub average_5m: Option<f64>,
    pub average_15m: Option<f64>,
    pub average_1m_per_core: Option<f64>,
    pub processes_running: u32,
    pub processes_blocked: u32,
    pub processor_queue_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTopItem {
    pub pid: u32,
    pub start_time: DateTime<Utc>,
    pub name: String,
    pub state: String,
    pub cpu_usage_ratio: f64,
    pub rss_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessWatchItem {
    pub name: String,
    pub present: bool,
    pub count: usize,
    pub pids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessesSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub visibility: String,
    pub count: usize,
    pub running_count: usize,
    pub sleeping_count: usize,
    pub blocked_count: usize,
    pub zombie_count: usize,
    pub thread_count: usize,
    pub truncated: bool,
    pub top: Vec<ProcessTopItem>,
    pub watch: Vec<ProcessWatchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UptimeSection {
    #[serde(flatten)]
    pub meta: SectionMeta,
    pub host_seconds: f64,
    pub host_boot_time: DateTime<Utc>,
    pub agent_seconds: f64,
    pub clock_adjusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckItem {
    pub id: String,
    pub status: HealthStatus,
    pub severity: String,
    pub message: String,
    pub observed: Option<f64>,
    pub threshold: Option<f64>,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSection {
    pub status: HealthStatus,
    pub status_code: u8,
    pub since: Option<DateTime<Utc>>,
    pub recovered_at: Option<DateTime<Utc>>,
    pub checks: Vec<HealthCheckItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventItem {
    pub event_id: Uuid,
    pub r#type: String,
    pub severity: String,
    pub occurred_at: DateTime<Utc>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: String,
    pub snapshot_id: Uuid,
    pub sequence: u64,
    pub collected_at: DateTime<Utc>,
    pub agent: AgentMeta,
    pub host: HostMeta,
    pub collection: CollectionMeta,
    pub cpu: CpuSection,
    pub memory: MemorySection,
    pub filesystems: FilesystemsSection,
    pub disks: DisksSection,
    pub network: NetworkSection,
    pub load: LoadSection,
    pub processes: ProcessesSection,
    pub uptime: UptimeSection,
    pub health: HealthSection,
    pub events: Vec<EventItem>,
}
