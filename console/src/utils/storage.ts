import type { NodeOverviewItem, NodeDetailResponse } from '../types/contract'
import { mockNodes, getMockNodeDetail } from '../mock/nodes'

const NODES_STORAGE_KEY = 'sprobe_managed_nodes'
const SETTINGS_STORAGE_KEY = 'sprobe_site_settings'
const DELETED_IDS_KEY = 'sprobe_deleted_node_ids'

export interface SiteSettings {
  siteTitle: string
  siteSubtitle: string
  defaultView: 'cards' | 'table'
  refreshInterval: number
  serverEndpoint: string
  globalToken: string
  alertThresholds: {
    cpuRatio: number
    memRatio: number
    diskRatio: number
    trafficRatio: number
  }
  notifications: {
    telegramEnabled: boolean
    telegramBotToken: string
    telegramChatId: string
    webhookEnabled: boolean
    webhookUrl: string
  }
}

export const defaultSettings: SiteSettings = {
  siteTitle: 'Sprobe 服务器集群监控',
  siteSubtitle: '轻量级、跨平台高精度探针集群控制台',
  defaultView: 'cards',
  refreshInterval: 5,
  serverEndpoint: 'https://monitor.example.com/api/v1/ingest',
  globalToken: 'sprobe-secret-token-key-2026',
  alertThresholds: {
    cpuRatio: 90,
    memRatio: 95,
    diskRatio: 90,
    trafficRatio: 80,
  },
  notifications: {
    telegramEnabled: false,
    telegramBotToken: '',
    telegramChatId: '',
    webhookEnabled: true,
    webhookUrl: 'https://hooks.slack.com/services/sprobe/alerts',
  },
}

// 获取已被删除的节点黑名单
export function getDeletedIds(): Set<string> {
  try {
    const raw = localStorage.getItem(DELETED_IDS_KEY)
    if (raw) {
      const arr = JSON.parse(raw)
      if (Array.isArray(arr)) {
        return new Set(arr)
      }
    }
  } catch (_) {}
  return new Set()
}

// 记录删除的节点 ID
export function markNodeAsDeleted(agentId: string): void {
  const ids = getDeletedIds()
  ids.add(agentId)
  localStorage.setItem(DELETED_IDS_KEY, JSON.stringify(Array.from(ids)))
}

// 获取节点列表
export function getStoredNodes(): NodeOverviewItem[] {
  const deletedIds = getDeletedIds()
  try {
    const raw = localStorage.getItem(NODES_STORAGE_KEY)
    if (raw !== null) {
      const parsed = JSON.parse(raw)
      if (Array.isArray(parsed)) {
        // 过滤掉任何被标记删除的节点
        let current = parsed.filter((n: NodeOverviewItem) => !deletedIds.has(n.agent_id))
        
        // 如果真实服务器未被用户显式删除，且不在当前列表中，则自动合并到首位
        const realNode = mockNodes.find(n => n.hostname === 'SanJose-AMD2')
        if (realNode && !deletedIds.has(realNode.agent_id) && !current.some(n => n.agent_id === realNode.agent_id)) {
          current = [realNode, ...current]
          localStorage.setItem(NODES_STORAGE_KEY, JSON.stringify(current))
        }
        return current
      }
    }
  } catch (_) {
    // 回退到默认
  }

  // 仅在首次使用 (raw === null) 时，将 mockNodes 写入，并排除已被删除节点
  const initial = mockNodes.filter((n) => !deletedIds.has(n.agent_id))
  localStorage.setItem(NODES_STORAGE_KEY, JSON.stringify(initial))
  return [...initial]
}

// 保存节点列表
export function saveStoredNodes(nodes: NodeOverviewItem[]): void {
  const deletedIds = getDeletedIds()
  const clean = nodes.filter((n) => !deletedIds.has(n.agent_id))
  localStorage.setItem(NODES_STORAGE_KEY, JSON.stringify(clean))
}

export interface NodeFormData {
  agent_id?: string
  hostname: string
  display_name: string
  os: 'linux' | 'windows' | 'darwin'
  arch: string
  region: string
  group: string
  cycle_start_day: number
  monthly_limit_gib: number
  token?: string
}

// 添加新节点
export function createNode(data: NodeFormData): NodeOverviewItem {
  const nodes = getStoredNodes()
  const agent_id = data.agent_id || `agent-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 6)}`
  const limitBytes = data.monthly_limit_gib ? data.monthly_limit_gib * 1024 * 1024 * 1024 : 1099511627776

  const newNode: NodeOverviewItem = {
    agent_id,
    hostname: data.hostname || 'new-host',
    display_name: data.display_name || data.hostname || '未命名节点',
    os: data.os || 'linux',
    arch: data.arch || 'x86_64',
    labels: {
      region: data.region || 'US',
      group: data.group || '默认分组',
      env: 'production',
    },
    health_status: 'ok',
    is_online: true,
    last_seen: new Date().toISOString(),
    cpu_usage_ratio: 0.12,
    memory_usage_ratio: 0.35,
    disk_usage_ratio: 0.28,
    load_1m: 0.45,
    network_rx_bytes_per_sec: 125000,
    network_tx_bytes_per_sec: 85000,
    cycle_outbound_bytes: Math.floor(limitBytes * 0.15),
    cycle_limit_bytes: limitBytes,
    cycle_outbound_ratio: 0.15,
  }

  nodes.unshift(newNode)
  saveStoredNodes(nodes)
  return newNode
}

// 修改节点
export function updateNode(agentId: string, data: Partial<NodeFormData>): NodeOverviewItem | null {
  const nodes = getStoredNodes()
  const idx = nodes.findIndex((n) => n.agent_id === agentId)
  if (idx === -1) return null

  const target = nodes[idx]!
  if (data.hostname !== undefined) target.hostname = data.hostname
  if (data.display_name !== undefined) target.display_name = data.display_name
  if (data.os !== undefined) target.os = data.os
  if (data.arch !== undefined) target.arch = data.arch
  
  if (data.region !== undefined || data.group !== undefined) {
    target.labels = {
      ...target.labels,
      ...(data.region !== undefined ? { region: data.region } : {}),
      ...(data.group !== undefined ? { group: data.group } : {}),
    }
  }

  if (data.monthly_limit_gib !== undefined) {
    const limitBytes = data.monthly_limit_gib * 1024 * 1024 * 1024
    target.cycle_limit_bytes = limitBytes
    if (target.cycle_outbound_bytes !== null && target.cycle_outbound_bytes !== undefined) {
      target.cycle_outbound_ratio = limitBytes > 0 ? target.cycle_outbound_bytes / limitBytes : 0
    }
  }

  nodes[idx] = target
  saveStoredNodes(nodes)
  return target
}

// 删除节点
export function deleteNode(agentId: string): boolean {
  markNodeAsDeleted(agentId)
  const nodes = getStoredNodes()
  const filtered = nodes.filter((n) => n.agent_id !== agentId)
  saveStoredNodes(filtered)
  return true
}

// 恢复节点出厂默认数据
export function resetNodesToDefault(): NodeOverviewItem[] {
  localStorage.removeItem(DELETED_IDS_KEY)
  localStorage.setItem(NODES_STORAGE_KEY, JSON.stringify(mockNodes))
  return [...mockNodes]
}

// 获取系统配置
export function getStoredSettings(): SiteSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY)
    if (raw) {
      return { ...defaultSettings, ...JSON.parse(raw) }
    }
  } catch (_) {
    // 降级
  }
  return { ...defaultSettings }
}

// 保存系统配置
export function saveStoredSettings(settings: SiteSettings): void {
  localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings))
}

// 获取详情快照（带本地修改合并）
export function getManagedNodeDetail(agentId: string): NodeDetailResponse {
  const baseDetail = getMockNodeDetail(agentId)
  const nodes = getStoredNodes()
  const node = nodes.find((n) => n.agent_id === agentId)
  if (!node) return baseDetail

  const copy: NodeDetailResponse = JSON.parse(JSON.stringify(baseDetail))
  copy.snapshot.agent.agent_id = node.agent_id
  copy.snapshot.agent.hostname = node.hostname
  copy.snapshot.host.hostname = node.display_name || node.hostname
  copy.snapshot.agent.labels = { ...node.labels }
  copy.snapshot.agent.os = node.os

  if (node.cycle_limit_bytes !== undefined && node.cycle_limit_bytes !== null) {
    copy.snapshot.network.cycle_limit_bytes = node.cycle_limit_bytes
    copy.snapshot.network.cycle_outbound_bytes = node.cycle_outbound_bytes ?? 0
    copy.snapshot.network.cycle_outbound_ratio = node.cycle_outbound_ratio ?? 0
  }

  return copy
}
