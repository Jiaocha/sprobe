// Sprobe Contract v1.0 TypeScript Definitions
// 对齐 contract/sprobe_contract_v1_0.schema.json 与 docs/backend/rust_v1_3.md 第 6 章

export type SectionStatus = 'ok' | 'partial' | 'failed' | 'unsupported' | 'timeout'
export type Quality = 'measured' | 'derived' | 'estimated'
export type HealthStatus = 'ok' | 'degraded' | 'critical' | 'unknown'

export interface SectionMeta {
  status: SectionStatus
  quality: Quality
  sampled_at: string
  age_seconds: number
  errors: Array<Record<string, unknown>>
}

export interface Capabilities {
  cpu_modes: boolean
  cpu_frequency: boolean
  memory_available: boolean
  swap: boolean
  filesystem_inodes: boolean
  disk_io_counters: boolean
  network_speed: boolean
  process_top: boolean
  process_io: boolean
}

export interface AgentMeta {
  agent_id: string
  instance_id: string
  version: string
  hostname: string
  os: string
  arch: string
  labels: Record<string, string>
}

export interface HostMeta {
  hostname: string
  os_name: string
  kernel: string
  architecture: string
  virtualization: string | null
  cpu_model: string | null
  boot_time: string
}

export interface CollectionMeta {
  interval_seconds: number
  warmup: boolean
  partial: boolean
  capabilities: Capabilities
  unsupported_reasons?: Record<string, string>
}

export interface CpuCore {
  cpu: string
  usage_ratio: number
  idle_ratio: number
  iowait_ratio: number
  steal_ratio: number
  user_seconds_total: number
  nice_seconds_total: number
  system_seconds_total: number
  idle_seconds_total: number
  iowait_seconds_total: number
  irq_seconds_total: number
  softirq_seconds_total: number
  steal_seconds_total: number
  frequency_hertz?: number | null
}

export interface CpuTotal {
  usage_ratio: number
  idle_ratio: number
  iowait_ratio: number
  steal_ratio: number
  user_seconds_total: number
  nice_seconds_total: number
  system_seconds_total: number
  idle_seconds_total: number
  iowait_seconds_total: number
  irq_seconds_total: number
  softirq_seconds_total: number
  steal_seconds_total: number
}

export interface CpuSection extends SectionMeta {
  logical_count: number
  physical_count: number
  total: CpuTotal
  cores?: CpuCore[]
}

export interface MemorySection extends SectionMeta {
  available_source: string
  total_bytes: number
  available_bytes: number
  used_bytes: number
  free_bytes: number
  cached_bytes: number
  buffers_bytes: number
  usage_ratio: number
  dirty_bytes: number
  writeback_bytes: number
  swap_total_bytes: number
  swap_used_bytes: number
  swap_free_bytes: number
  swap_usage_ratio: number | null
}

export interface FilesystemItem {
  device: string
  volume_id?: string | null
  mountpoint: string
  mountpoints: string[]
  fstype: string
  read_only: boolean
  total_bytes: number
  used_bytes: number
  free_bytes: number
  available_bytes: number
  reserved_bytes: number
  usage_ratio: number
  inodes_total: number
  inodes_used: number
  inodes_free: number
  inodes_usage_ratio: number
  stale: boolean
  quality: Quality
}

export interface FilesystemsSection extends SectionMeta {
  truncated: boolean
  items: FilesystemItem[]
}

export interface DiskItem {
  device: string
  disk_class: 'physical' | 'virtual' | 'partition'
  parent: string | null
  read_bytes_total: number
  write_bytes_total: number
  read_operations_total: number
  write_operations_total: number
  io_time_seconds_total: number
  io_time_weighted_seconds_total: number
  in_flight_operations: number
  read_bytes_per_second: number | null
  write_bytes_per_second: number | null
  utilization_ratio: number | null
  read_latency_seconds: number | null
  write_latency_seconds: number | null
  quality: Quality
}

export interface DisksSection extends SectionMeta {
  truncated: boolean
  items: DiskItem[]
}

export interface NetworkInterfaceItem {
  name: string
  class: 'physical' | 'virtual' | 'loopback'
  master: string | null
  virtual: boolean
  up: boolean
  mtu: number
  speed_bytes_per_second: number | null
  receive_bytes_total: number
  transmit_bytes_total: number
  receive_packets_total: number
  transmit_packets_total: number
  receive_errors_total: number
  transmit_errors_total: number
  receive_dropped_total: number
  transmit_dropped_total: number
  receive_bytes_per_second: number | null
  transmit_bytes_per_second: number | null
  receive_utilization_ratio: number | null
  transmit_utilization_ratio: number | null
}

export interface NetworkSection extends SectionMeta {
  truncated: boolean
  interfaces: NetworkInterfaceItem[]
  cycle_outbound_bytes?: number | null
  cycle_limit_bytes?: number | null
  cycle_outbound_ratio?: number | null
}

export interface LoadSection extends SectionMeta {
  average_1m: number | null
  average_5m: number | null
  average_15m: number | null
  average_1m_per_core: number | null
  processes_running: number
  processes_blocked: number
  processor_queue_length: number | null
}

export interface ProcessTopItem {
  pid: number
  start_time: string
  name: string
  state: string
  cpu_usage_ratio: number
  rss_bytes: number
}

export interface ProcessWatchItem {
  name: string
  present: boolean
  count: number
  pids: number[]
}

export interface ProcessesSection extends SectionMeta {
  visibility: 'full' | 'public'
  count: number
  running_count: number
  sleeping_count: number
  blocked_count: number
  zombie_count: number
  thread_count: number
  truncated: boolean
  top: ProcessTopItem[]
  watch: ProcessWatchItem[]
}

export interface UptimeSection extends SectionMeta {
  host_seconds: number
  host_boot_time: string
  agent_seconds: number
  clock_adjusted: boolean
}

export interface HealthCheckItem {
  id: string
  status: HealthStatus
  severity: 'info' | 'warning' | 'critical'
  message: string
  observed: number | string | null
  threshold: number | string | null
  since: string
}

export interface HealthSection {
  status: HealthStatus
  status_code: number
  since: string | null
  recovered_at: string | null
  checks: HealthCheckItem[]
}

export interface EventItem {
  event_id: string
  type: string
  severity: 'info' | 'warning' | 'critical'
  occurred_at: string
  details: Record<string, unknown>
}

export interface Snapshot {
  schema_version: '1.0'
  snapshot_id: string
  sequence: number
  collected_at: string
  agent: AgentMeta
  host: HostMeta
  collection: CollectionMeta
  cpu: CpuSection
  memory: MemorySection
  filesystems: FilesystemsSection
  disks: DisksSection
  network: NetworkSection
  load: LoadSection
  processes: ProcessesSection
  uptime: UptimeSection
  health: HealthSection
  events: EventItem[]
}

export interface IngestBatch {
  schema_version: '1.0'
  batch_id: string
  agent_id: string
  instance_id: string
  created_at: string
  sequence_first: number
  sequence_last: number
  snapshot_count: number
  snapshots: Snapshot[]
}

export interface HeartbeatRequest {
  schema_version: '1.0'
  agent_id: string
  instance_id: string
  sent_at: string
  uptime_seconds: number
  ready: boolean
  warmup: boolean
  health_status: HealthStatus
  health_status_code: number
  last_sequence: number
  wal_snapshots: number
}

export interface IngestSuccessResponse {
  schema_version: '1.0'
  status: 'accepted' | 'partial'
  batch_id: string
  accepted: number
  duplicates: number
  rejected: Array<{
    sequence: number
    reason: string
  }>
  server_time: string
}

export interface ErrorResponse {
  error: {
    code:
      | 'invalid_json'
      | 'invalid_gzip'
      | 'unsupported_schema'
      | 'invalid_query'
      | 'unauthorized'
      | 'unauthenticated'
      | 'forbidden'
      | 'public_detail_disabled'
      | 'not_found'
      | 'request_timeout'
      | 'conflict'
      | 'payload_too_large'
      | 'validation_failed'
      | 'rate_limited'
      | 'internal'
      | 'upstream_unavailable'
    message: string
    retryable: boolean
    request_id: string
    server_time: string
  }
}

// Console 只读 API 节点摘要定义
export interface NodeOverviewItem {
  agent_id: string
  hostname: string
  display_name: string
  os: string
  arch: string
  labels: Record<string, string>
  health_status: HealthStatus
  is_online: boolean
  last_seen: string
  cpu_usage_ratio: number | null
  memory_usage_ratio: number | null
  disk_usage_ratio: number | null
  load_1m: number | null
  network_rx_bytes_per_sec: number | null
  network_tx_bytes_per_sec: number | null
  cycle_outbound_bytes?: number | null
  cycle_limit_bytes?: number | null
  cycle_outbound_ratio?: number | null
}

export interface AgentRuntime {
  online: boolean
  last_push_at: string
  last_heartbeat_at: string
  wal_snapshots: number
}

export interface NodeDetailResponse {
  schema_version: '1.0'
  generated_at: string
  view: 'public' | 'full'
  agent_runtime: AgentRuntime
  snapshot: Snapshot
}

