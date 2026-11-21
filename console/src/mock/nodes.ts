import type { NodeOverviewItem } from '../types/contract'

export const mockNodes: NodeOverviewItem[] = [
  {
    agent_id: '04787d26-0209-4521-9fd2-e73759168169',
    hostname: 'SanJose-AMD2',
    display_name: 'Oracle 圣何塞 VPS (真实生产)',
    os: 'linux',
    arch: 'x86_64',
    labels: {
      region: 'US-SiliconValley',
      group: '生产业务',
      provider: 'Oracle Cloud / VPS',
      public_ip: '198.51.100.151',
    },
    health_status: 'ok',
    is_online: true,
    last_seen: new Date().toISOString(),
    cpu_usage_ratio: 0.185,
    memory_usage_ratio: 0.4287,
    disk_usage_ratio: 0.1596,
    load_1m: 0.50,
    network_rx_bytes_per_sec: 64500,
    network_tx_bytes_per_sec: 128900,
    cycle_outbound_bytes: 184500000000,
    cycle_limit_bytes: 1099511627776,
    cycle_outbound_ratio: 0.168,
  },
  {
    agent_id: '6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f',
    hostname: 'web-prod-01',
    display_name: '生产环境前端网关',
    os: 'linux',
    arch: 'x86_64',
    labels: { env: 'prod', role: 'web', region: 'shanghai' },
    health_status: 'ok',
    is_online: true,
    last_seen: new Date().toISOString(),
    cpu_usage_ratio: 0.2315,
    memory_usage_ratio: 0.4000,
    disk_usage_ratio: 0.4167,
    load_1m: 1.85,
    network_rx_bytes_per_sec: 12500000,
    network_tx_bytes_per_sec: 6250000,
    cycle_outbound_bytes: 322122547200, // 300 GiB
    cycle_limit_bytes: 1099511627776, // 1 TiB
    cycle_outbound_ratio: 0.293,
  },
  {
    agent_id: 'a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d',
    hostname: 'db-primary-01',
    display_name: '主数据库集群节点 A',
    os: 'linux',
    arch: 'x86_64',
    labels: { env: 'prod', role: 'db', region: 'shanghai' },
    health_status: 'degraded',
    is_online: true,
    last_seen: new Date().toISOString(),
    cpu_usage_ratio: 0.6520,
    memory_usage_ratio: 0.8840,
    disk_usage_ratio: 0.9120, // 高磁盘空间告警
    load_1m: 5.20,
    network_rx_bytes_per_sec: 4500000,
    network_tx_bytes_per_sec: 8900000,
    cycle_outbound_bytes: 989000000000, // 接近 1 TiB (约 90%)
    cycle_limit_bytes: 1099511627776,
    cycle_outbound_ratio: 0.899,
  },
  {
    agent_id: 'c3d4e5f6-a1b2-4c5d-8e9f-1a2b3c4d5e6f',
    hostname: 'win-app-worker',
    display_name: 'Windows 报表计算节点',
    os: 'windows',
    arch: 'x86_64',
    labels: { env: 'prod', role: 'worker', region: 'beijing' },
    health_status: 'ok',
    is_online: true,
    last_seen: new Date().toISOString(),
    cpu_usage_ratio: 0.4500,
    memory_usage_ratio: 0.5200,
    disk_usage_ratio: 0.3500,
    load_1m: 3.10,
    network_rx_bytes_per_sec: 1200000,
    network_tx_bytes_per_sec: 800000,
  },
  {
    agent_id: 'd4e5f6a1-b2c3-4d5e-9f0a-2b3c4d5e6f7a',
    hostname: 'edge-arm-gateway',
    display_name: '边缘 ARM 汇聚网关',
    os: 'linux',
    arch: 'aarch64',
    labels: { env: 'edge', role: 'gateway', region: 'guangzhou' },
    health_status: 'unknown',
    is_online: false,
    last_seen: new Date(Date.now() - 3600000).toISOString(), // 1 小时前失联
    cpu_usage_ratio: null,
    memory_usage_ratio: null,
    disk_usage_ratio: null,
    load_1m: null,
    network_rx_bytes_per_sec: null,
    network_tx_bytes_per_sec: null,
  },
]

import type { NodeDetailResponse, Snapshot } from '../types/contract'

export function getMockNodeDetail(agentId: string): NodeDetailResponse {
  const node = mockNodes.find((n) => n.agent_id === agentId) || mockNodes[0]!
  const now = new Date().toISOString()

  const isRealHost = node.hostname === 'SanJose-AMD2' || node.agent_id === '04787d26-0209-4521-9fd2-e73759168169'

  const hostOs = isRealHost ? 'Debian GNU/Linux 13 (trixie)' : node.os === 'windows' ? 'Windows 11 Pro 23H2' : 'Ubuntu 24.04.1 LTS'
  const hostKernel = isRealHost ? '6.12.107+deb13-cloud-amd64' : node.os === 'windows' ? '10.0.22631' : '6.8.0-45-generic'
  const cpuModel = isRealHost ? 'AMD EPYC (2 Cores / Oracle Cloud VPS)' : 'AMD Ryzen / EPYC High Performance Processor'
  const bootTime = isRealHost ? '2026-09-20T03:51:23.270Z' : new Date(Date.now() - 86400000 * 3).toISOString()

  const snapshot: Snapshot = {
    schema_version: '1.0',
    snapshot_id: isRealHost ? '0060cf10-352a-49a0-a192-73838f5a8ccc' : '5b0c2f7e-6d1a-4c2e-9b7f-2f3a1c9d0e11',
    sequence: 142,
    collected_at: now,
    agent: {
      agent_id: node.agent_id,
      instance_id: isRealHost ? '64486fd6-df57-4aec-9f93-5c472a298b49' : '2a7d9c1e-4b6f-4e2a-8c1d-9f0b3a6e5d22',
      version: '0.1.0',
      hostname: node.hostname,
      os: node.os,
      arch: node.arch,
      labels: node.labels,
    },
    host: {
      hostname: node.hostname,
      os_name: hostOs,
      kernel: hostKernel,
      architecture: node.arch,
      virtualization: isRealHost ? 'kvm' : 'kvm',
      cpu_model: cpuModel,
      boot_time: bootTime,
    },
    collection: {
      interval_seconds: 5,
      warmup: false,
      partial: false,
      capabilities: {
        cpu_modes: true,
        cpu_frequency: true,
        memory_available: true,
        swap: true,
        filesystem_inodes: true,
        disk_io_counters: true,
        network_speed: true,
        process_top: true,
        process_io: false,
      },
    },
    cpu: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      logical_count: isRealHost ? 2 : 8,
      physical_count: isRealHost ? 2 : 4,
      total: {
        usage_ratio: node.cpu_usage_ratio ?? 0.185,
        idle_ratio: 1.0 - (node.cpu_usage_ratio ?? 0.185),
        iowait_ratio: isRealHost ? 0.088 : 0.02,
        steal_ratio: isRealHost ? 0.61 : 0.0,
        user_seconds_total: isRealHost ? 1541596.1 : 1205.4,
        nice_seconds_total: isRealHost ? 144.5 : 0.0,
        system_seconds_total: isRealHost ? 187350.5 : 312.8,
        idle_seconds_total: isRealHost ? 5654869.6 : 5410.2,
        iowait_seconds_total: isRealHost ? 30289.5 : 15.0,
        irq_seconds_total: 0.0,
        softirq_seconds_total: isRealHost ? 13686.7 : 8.2,
        steal_seconds_total: isRealHost ? 3654872.9 : 0.0,
      },
    },
    memory: isRealHost
      ? {
          status: 'ok',
          quality: 'measured',
          sampled_at: now,
          age_seconds: 0.1,
          errors: [],
          available_source: 'kernel_memavailable',
          total_bytes: 1009438720, // 962.6 MiB
          available_bytes: 576679936, // 550 MiB
          used_bytes: 432758784, // 412.7 MiB
          free_bytes: 156000256,
          cached_bytes: 537939968,
          buffers_bytes: 4673536,
          usage_ratio: 0.4287,
          dirty_bytes: 733184,
          writeback_bytes: 0,
          swap_total_bytes: 2684346368, // 2.5 GiB
          swap_used_bytes: 162627584,
          swap_free_bytes: 2521718784,
          swap_usage_ratio: 0.0605,
        }
      : {
          status: 'ok',
          quality: 'measured',
          sampled_at: now,
          age_seconds: 0.1,
          errors: [],
          available_source: 'MemAvailable',
          total_bytes: 17179869184, // 16 GiB
          available_bytes: 10737418240, // 10 GiB
          used_bytes: 6442450944, // 6 GiB
          free_bytes: 4294967296,
          cached_bytes: 5368709120,
          buffers_bytes: 1073741824,
          usage_ratio: node.memory_usage_ratio ?? 0.375,
          dirty_bytes: 1048576,
          writeback_bytes: 0,
          swap_total_bytes: 4294967296,
          swap_used_bytes: 0,
          swap_free_bytes: 4294967296,
          swap_usage_ratio: 0.0,
        },
    filesystems: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      truncated: false,
      items: isRealHost
        ? [
            {
              device: '/dev/sda2',
              mountpoint: '/',
              mountpoints: ['/'],
              fstype: 'ext4',
              read_only: false,
              total_bytes: 51991019520, // 48.4 GiB
              used_bytes: 8297451520, // 7.72 GiB
              free_bytes: 43693568000,
              available_bytes: 41019387904, // 38.2 GiB
              reserved_bytes: 2674180096,
              usage_ratio: 0.1596,
              inodes_total: 3244032,
              inodes_used: 220426,
              inodes_free: 3023606,
              inodes_usage_ratio: 0.0679,
              stale: false,
              quality: 'measured',
            },
            {
              device: '/dev/sda1',
              mountpoint: '/boot/efi',
              mountpoints: ['/boot/efi'],
              fstype: 'vfat',
              read_only: false,
              total_bytes: 535805952,
              used_bytes: 9469952,
              free_bytes: 526336000,
              available_bytes: 526336000,
              reserved_bytes: 0,
              usage_ratio: 0.0176,
              inodes_total: 0,
              inodes_used: 0,
              inodes_free: 0,
              inodes_usage_ratio: 0.0,
              stale: false,
              quality: 'measured',
            },
          ]
        : [
            {
              device: node.os === 'windows' ? 'C:\\' : '/dev/nvme0n1p2',
              mountpoint: node.os === 'windows' ? 'C:\\' : '/',
              mountpoints: [node.os === 'windows' ? 'C:\\' : '/'],
              fstype: node.os === 'windows' ? 'NTFS' : 'ext4',
              read_only: false,
              total_bytes: 536870912000,
              used_bytes: Math.floor(536870912000 * (node.disk_usage_ratio ?? 0.45)),
              free_bytes: Math.floor(536870912000 * (1 - (node.disk_usage_ratio ?? 0.45))),
              available_bytes: Math.floor(512000000000 * (1 - (node.disk_usage_ratio ?? 0.45))),
              reserved_bytes: 24870912000,
              usage_ratio: node.disk_usage_ratio ?? 0.45,
              inodes_total: 32768000,
              inodes_used: 1250000,
              inodes_free: 31518000,
              inodes_usage_ratio: 0.038,
              stale: false,
              quality: 'measured',
            },
          ],
    },
    disks: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      truncated: false,
      items: [
        {
          device: isRealHost ? 'sda' : node.os === 'windows' ? 'C:' : 'nvme0n1',
          disk_class: 'physical',
          parent: null,
          read_bytes_total: isRealHost ? 15200000000 : 42949672960,
          write_bytes_total: isRealHost ? 8900000000 : 17179869184,
          read_operations_total: 210000,
          write_operations_total: 154000,
          io_time_seconds_total: 120.5,
          io_time_weighted_seconds_total: 145.2,
          in_flight_operations: 0,
          read_bytes_per_second: 420000,
          write_bytes_per_second: 310000,
          utilization_ratio: 0.012,
          read_latency_seconds: 0.0008,
          write_latency_seconds: 0.0015,
          quality: 'measured',
        },
      ],
    },
    network: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      truncated: false,
      interfaces: [
        {
          name: isRealHost ? 'ens3' : node.os === 'windows' ? 'Ethernet 1' : 'eth0',
          class: 'physical',
          master: null,
          virtual: false,
          up: true,
          mtu: 1500,
          speed_bytes_per_second: 125000000, // 1Gbps
          receive_bytes_total: isRealHost ? 45200000000 : 10485760000,
          transmit_bytes_total: isRealHost ? 89200000000 : 5242880000,
          receive_packets_total: 35000000,
          transmit_packets_total: 42000000,
          receive_errors_total: 0,
          transmit_errors_total: 0,
          receive_dropped_total: 0,
          transmit_dropped_total: 0,
          receive_bytes_per_second: node.network_rx_bytes_per_sec ?? 64500,
          transmit_bytes_per_second: node.network_tx_bytes_per_sec ?? 128900,
          receive_utilization_ratio: 0.008,
          transmit_utilization_ratio: 0.015,
        },
      ],
      cycle_outbound_bytes: node.cycle_outbound_bytes ?? (isRealHost ? 184500000000 : 322122547200),
      cycle_limit_bytes: node.cycle_limit_bytes ?? 1099511627776,
      cycle_outbound_ratio: node.cycle_outbound_ratio ?? 0.168,
    },
    load: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      average_1m: node.load_1m,
      average_5m: (node.load_1m ?? 0.5) * 0.9,
      average_15m: (node.load_1m ?? 0.5) * 0.8,
      average_1m_per_core: (node.load_1m ?? 0.5) / (isRealHost ? 2 : 8),
      processes_running: 1,
      processes_blocked: 0,
      processor_queue_length: null,
    },
    processes: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      visibility: 'full',
      count: isRealHost ? 104 : 248,
      running_count: 1,
      sleeping_count: isRealHost ? 103 : 246,
      blocked_count: 0,
      zombie_count: 0,
      thread_count: isRealHost ? 380 : 1420,
      truncated: false,
      top: isRealHost
        ? [
            {
              pid: 1045178,
              start_time: '2026-11-21T08:10:00Z',
              name: 'terraform-provi',
              state: 'S',
              cpu_usage_ratio: 0.002,
              rss_bytes: 200278016, // 191 MiB
            },
            {
              pid: 1045153,
              start_time: '2026-11-21T08:09:50Z',
              name: 'terraform',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 81788928, // 78 MiB
            },
            {
              pid: 1045148,
              start_time: '2026-11-21T08:09:40Z',
              name: 'python',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 28311552, // 27 MiB
            },
            {
              pid: 1848913,
              start_time: '2026-11-15T12:00:00Z',
              name: 'xray',
              state: 'S',
              cpu_usage_ratio: 0.008,
              rss_bytes: 15728640, // 15 MiB
            },
            {
              pid: 819,
              start_time: '2026-09-20T03:52:00Z',
              name: 'fail2ban-server',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 14680064, // 14 MiB
            },
            {
              pid: 822,
              start_time: '2026-09-20T03:52:05Z',
              name: 'mtg',
              state: 'S',
              cpu_usage_ratio: 0.003,
              rss_bytes: 9437184, // 9 MiB
            },
            {
              pid: 820,
              start_time: '2026-09-20T03:52:02Z',
              name: 'frpc',
              state: 'S',
              cpu_usage_ratio: 0.002,
              rss_bytes: 9437184, // 9 MiB
            },
            {
              pid: 723,
              start_time: '2026-09-20T03:51:50Z',
              name: 'BT-Task',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 5242880, // 5 MiB
            },
            {
              pid: 1044973,
              start_time: '2026-11-21T08:13:36Z',
              name: 'sprobe',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 4194304, // 4 MiB (Sprobe 超轻量探针常驻)
            },
            {
              pid: 1,
              start_time: '2026-09-20T03:51:23Z',
              name: 'systemd',
              state: 'S',
              cpu_usage_ratio: 0.0,
              rss_bytes: 7340032, // 7 MiB
            },
          ]
        : [
            {
              pid: 1024,
              start_time: new Date(Date.now() - 86400000).toISOString(),
              name: 'sprobe',
              state: 'R',
              cpu_usage_ratio: 0.005,
              rss_bytes: 18450000,
            },
            {
              pid: 884,
              start_time: new Date(Date.now() - 86400000 * 2).toISOString(),
              name: 'systemd / init',
              state: 'S',
              cpu_usage_ratio: 0.001,
              rss_bytes: 32500000,
            },
            {
              pid: 2410,
              start_time: new Date(Date.now() - 3600000 * 4).toISOString(),
              name: 'nginx-worker',
              state: 'S',
              cpu_usage_ratio: 0.032,
              rss_bytes: 84500000,
            },
          ],
      watch: [
        {
          name: 'sprobe',
          present: true,
          count: 1,
          pids: [isRealHost ? 1044973 : 1024],
        },
      ],
    },
    uptime: {
      status: 'ok',
      quality: 'measured',
      sampled_at: now,
      age_seconds: 0.1,
      errors: [],
      host_seconds: isRealHost ? 5372568.0 : 259200.0,
      host_boot_time: bootTime,
      agent_seconds: isRealHost ? 120.0 : 1420.0,
      clock_adjusted: false,
    },
    health: {
      status: node.health_status,
      status_code: node.health_status === 'ok' ? 0 : node.health_status === 'degraded' ? 1 : 2,
      since: now,
      recovered_at: null,
      checks:
        node.health_status === 'degraded'
          ? [
              {
                id: 'disk_space',
                status: 'degraded',
                severity: 'warning',
                message: '文件系统空间超过 90% 警戒阈值',
                observed: node.disk_usage_ratio,
                threshold: 0.9,
                since: now,
              },
            ]
          : [],
    },
    events: [],
  }

  return {
    schema_version: '1.0',
    generated_at: now,
    view: 'full',
    agent_runtime: {
      online: node.is_online,
      last_push_at: now,
      last_heartbeat_at: now,
      wal_snapshots: 0,
    },
    snapshot,
  }
}
