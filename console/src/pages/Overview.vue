<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import type { NodeOverviewItem } from '../types/contract'
import ServerModal from '../components/ServerModal.vue'
import {
  getStoredNodes,
  createNode,
  updateNode,
  deleteNode,
  getDeletedIds,
  type NodeFormData,
} from '../utils/storage'
import {
  Server,
  Activity,
  AlertTriangle,
  WifiOff,
  Search,
  Filter,
  CheckCircle2,
  HelpCircle,
  HardDrive,
  RefreshCw,
  Radio,
  ChevronRight,
  LayoutGrid,
  List,
  ArrowUpRight,
  ArrowDownLeft,
  Cpu,
  Database,
  BarChart3,
  Plus,
  Edit2,
  Trash2,
  AlertCircle,
} from 'lucide-vue-next'

const router = useRouter()
const nodes = ref<NodeOverviewItem[]>(getStoredNodes())
const searchQuery = ref('')
const selectedOs = ref('all')
const selectedStatus = ref('all')
const isLive = ref(false)
const isLoading = ref(false)
const viewMode = ref<'cards' | 'table'>(
  (localStorage.getItem('sprobe_view_mode') as 'cards' | 'table') || 'cards'
)
let timer: number | null = null

// 弹窗状态
const isModalOpen = ref(false)
const modalMode = ref<'add' | 'edit'>('add')
const editingNode = ref<Partial<NodeFormData> | undefined>(undefined)

// 内置安全删除确认弹窗状态
const nodeToDelete = ref<{ id: string; hostname: string } | null>(null)

function setViewMode(mode: 'cards' | 'table') {
  viewMode.value = mode
  localStorage.setItem('sprobe_view_mode', mode)
}

// 异步拉取实时总览数据
async function fetchOverview() {
  isLoading.value = true
  try {
    const res = await fetch('/api/readonly/overview')
    if (res.ok) {
      const text = await res.text()
      // 防御 Cloudflare Pages SPA 回退返回的 HTML
      if (text.trim().startsWith('<')) {
        nodes.value = getStoredNodes()
        return
      }
      const data = JSON.parse(text)
      if (Array.isArray(data) && data.length > 0) {
        const deleted = getDeletedIds()
        const remoteNodes = data.filter((n: NodeOverviewItem) => !deleted.has(n.agent_id))
        const localNodes = getStoredNodes()
        
        // 远程真实上报节点置顶，本地其他节点保留
        const merged: NodeOverviewItem[] = [...remoteNodes]
        for (const loc of localNodes) {
          if (!merged.some(m => m.agent_id === loc.agent_id)) {
            merged.push(loc)
          }
        }
        nodes.value = merged
        isLive.value = true
      } else {
        nodes.value = getStoredNodes()
      }
    } else {
      nodes.value = getStoredNodes()
    }
  } catch (_) {
    nodes.value = getStoredNodes()
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  nodes.value = getStoredNodes()
  fetchOverview()
  timer = window.setInterval(fetchOverview, 5000)
})

onUnmounted(() => {
  if (timer) {
    clearInterval(timer)
  }
})

// 打开添加弹窗
function openAddModal() {
  modalMode.value = 'add'
  editingNode.value = undefined
  isModalOpen.value = true
}

// 打开编辑弹窗
function openEditModal(node: NodeOverviewItem, e?: Event) {
  if (e) e.stopPropagation()
  modalMode.value = 'edit'
  editingNode.value = {
    agent_id: node.agent_id,
    hostname: node.hostname,
    display_name: node.display_name,
    os: (node.os as 'linux' | 'windows' | 'darwin') || 'linux',
    arch: node.arch || 'x86_64',
    region: node.labels?.region || 'US-West',
    group: node.labels?.group || '默认集群',
    cycle_start_day: 1,
    monthly_limit_gib: node.cycle_limit_bytes ? Math.round(node.cycle_limit_bytes / (1024 * 1024 * 1024)) : 1024,
  }
  isModalOpen.value = true
}

// 保存节点 (添加或更新)
function handleSaveNode(data: NodeFormData) {
  if (modalMode.value === 'add') {
    const newNode = createNode(data)
    nodes.value.unshift(newNode)
  } else if (data.agent_id) {
    const updated = updateNode(data.agent_id, data)
    if (updated) {
      const idx = nodes.value.findIndex((n) => n.agent_id === data.agent_id)
      if (idx !== -1) {
        nodes.value[idx] = updated
      }
    }
  }
}

// 触发内置删除弹窗
function confirmDelete(agentId: string, hostname: string, e?: Event) {
  if (e) e.stopPropagation()
  nodeToDelete.value = { id: agentId, hostname }
}

// 确认执行删除
function executeDelete() {
  if (!nodeToDelete.value) return
  const { id } = nodeToDelete.value
  deleteNode(id)
  nodes.value = nodes.value.filter((n) => n.agent_id !== id)
  nodeToDelete.value = null
}

// 格式化比率
function formatRatio(ratio: number | null | undefined): string {
  if (ratio === null || ratio === undefined) return '—'
  return `${(ratio * 100).toFixed(1)}%`
}

// 格式化网络速率
function formatBytes(bytesPerSec: number | null | undefined): string {
  if (bytesPerSec === null || bytesPerSec === undefined) return '—'
  if (bytesPerSec < 1024) return `${bytesPerSec.toFixed(0)} B/s`
  if (bytesPerSec < 1024 * 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`
  return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`
}

// 格式化容量大小
function formatDataSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return '—'
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KiB', 'MiB', 'GiB', 'TiB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

// 根据标签提取国旗与地区徽标
function getRegionBadge(labels?: Record<string, string>): { flag: string; name: string } {
  if (!labels) return { flag: '🌐', name: '全局节点' }
  const r = (labels.region || labels.country || labels.location || '').toLowerCase()
  if (
    r.includes('shanghai') ||
    r.includes('beijing') ||
    r.includes('cn') ||
    r.includes('guangzhou') ||
    r.includes('hangzhou')
  ) {
    return { flag: '🇨🇳', name: labels.region || '中国' }
  }
  if (
    r.includes('us') ||
    r.includes('la') ||
    r.includes('silicon') ||
    r.includes('virginia') ||
    r.includes('america')
  ) {
    return { flag: '🇺🇸', name: labels.region || '美西' }
  }
  if (r.includes('jp') || r.includes('tokyo') || r.includes('osaka')) {
    return { flag: '🇯🇵', name: labels.region || '日本' }
  }
  if (r.includes('hk') || r.includes('hongkong')) {
    return { flag: '🇭🇰', name: labels.region || '香港' }
  }
  if (r.includes('sg') || r.includes('singapore')) {
    return { flag: '🇸🇬', name: labels.region || '新加坡' }
  }
  if (r.includes('de') || r.includes('frankfurt') || r.includes('germany')) {
    return { flag: '🇩🇪', name: labels.region || '德国' }
  }
  return { flag: '🌐', name: labels.region || '通用' }
}

// 统计指标
const stats = computed(() => {
  const total = nodes.value.length
  const online = nodes.value.filter((n) => n.is_online).length
  const offline = total - online
  const degraded = nodes.value.filter(
    (n) => n.is_online && (n.health_status === 'degraded' || n.health_status === 'critical')
  ).length
  const highDisk = nodes.value.filter(
    (n) => n.is_online && n.disk_usage_ratio !== null && n.disk_usage_ratio >= 0.9
  ).length
  const trafficWarning = nodes.value.filter(
    (n) => n.is_online && n.cycle_outbound_ratio !== null && n.cycle_outbound_ratio !== undefined && n.cycle_outbound_ratio >= 0.8
  ).length
  return { total, online, offline, degraded, highDisk, trafficWarning }
})

// 过滤后的节点清单
const filteredNodes = computed(() => {
  return nodes.value.filter((node) => {
    // OS 过滤
    if (selectedOs.value !== 'all' && node.os !== selectedOs.value) return false

    // 状态过滤
    if (selectedStatus.value === 'online' && !node.is_online) return false
    if (selectedStatus.value === 'offline' && node.is_online) return false
    if (
      selectedStatus.value === 'degraded' &&
      node.health_status !== 'degraded' &&
      node.health_status !== 'critical'
    ) {
      return false
    }

    // 搜索匹配
    if (searchQuery.value.trim()) {
      const q = searchQuery.value.toLowerCase()
      const matchHost = node.hostname.toLowerCase().includes(q)
      const matchName = node.display_name.toLowerCase().includes(q)
      const matchId = node.agent_id.toLowerCase().includes(q)
      const matchRegion = (node.labels?.region || '').toLowerCase().includes(q)
      return matchHost || matchName || matchId || matchRegion
    }

    return true
  })
})
</script>

<template>
  <div class="space-y-6">
    <!-- 顶部汇总统计卡片 -->
    <div class="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-6">
      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-slate-500">全部节点</span>
          <Server class="h-4 w-4 text-slate-400" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight">{{ stats.total }}</div>
      </div>

      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-emerald-600 dark:text-emerald-400">在线活跃</span>
          <Activity class="h-4 w-4 text-emerald-500" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight text-emerald-600 dark:text-emerald-400">{{ stats.online }}</div>
      </div>

      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-amber-600 dark:text-amber-400">异常/降级</span>
          <AlertTriangle class="h-4 w-4 text-amber-500" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight text-amber-600 dark:text-amber-400">{{ stats.degraded }}</div>
      </div>

      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-rose-600 dark:text-rose-400">磁盘告警 (≥90%)</span>
          <HardDrive class="h-4 w-4 text-rose-500" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight text-rose-600 dark:text-rose-400">{{ stats.highDisk }}</div>
      </div>

      <!-- 借鉴 Nezha：周期月结流量超限告警统计 -->
      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-indigo-600 dark:text-indigo-400">月流量预警 (≥80%)</span>
          <BarChart3 class="h-4 w-4 text-indigo-500" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight text-indigo-600 dark:text-indigo-400">{{ stats.trafficWarning }}</div>
      </div>

      <div class="rounded-lg border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="text-xs font-medium text-slate-400">离线节点</span>
          <WifiOff class="h-4 w-4 text-slate-400" />
        </div>
        <div class="mt-2 text-2xl font-bold tracking-tight text-slate-500">{{ stats.offline }}</div>
      </div>
    </div>

    <!-- 筛选、搜索与视图切换工具条 -->
    <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between rounded-lg border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-900">
      <div class="relative flex-1 max-w-md">
        <Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索主机名、显示名、地区或 Agent ID..."
          class="w-full rounded-md border border-slate-200 bg-slate-50 pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:border-brand-500 focus:bg-white focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100 dark:placeholder-slate-500"
        />
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <!-- 添加服务器主按钮 -->
        <button
          @click="openAddModal"
          class="inline-flex items-center gap-1.5 rounded-md bg-brand-600 px-3 py-1.5 text-xs font-semibold text-white shadow-sm hover:bg-brand-500 transition-colors"
        >
          <Plus class="h-3.5 w-3.5" />
          <span>添加服务器</span>
        </button>

        <!-- OS 筛选 -->
        <div class="flex items-center space-x-1.5 text-xs text-slate-500">
          <Filter class="h-3.5 w-3.5" />
          <select
            v-model="selectedOs"
            class="rounded border border-slate-200 bg-white px-2 py-1 text-xs dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200"
          >
            <option value="all">全部平台</option>
            <option value="linux">Linux</option>
            <option value="windows">Windows</option>
          </select>
        </div>

        <!-- 状态筛选 -->
        <div class="text-xs text-slate-500">
          <select
            v-model="selectedStatus"
            class="rounded border border-slate-200 bg-white px-2 py-1 text-xs dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200"
          >
            <option value="all">全部状态</option>
            <option value="online">仅在线</option>
            <option value="degraded">异常/降级</option>
            <option value="offline">仅离线</option>
          </select>
        </div>

        <!-- 视图切换 (卡片网格 vs 表格列表) -->
        <div class="flex items-center rounded-md border border-slate-200 bg-slate-50 p-0.5 dark:border-slate-800 dark:bg-slate-950">
          <button
            @click="setViewMode('cards')"
            :class="[
              'rounded p-1 transition-colors',
              viewMode === 'cards'
                ? 'bg-white text-brand-600 shadow-sm dark:bg-slate-800 dark:text-brand-400'
                : 'text-slate-400 hover:text-slate-700 dark:hover:text-slate-300',
            ]"
            title="紧凑服务器卡片网格视图"
          >
            <LayoutGrid class="h-3.5 w-3.5" />
          </button>
          <button
            @click="setViewMode('table')"
            :class="[
              'rounded p-1 transition-colors',
              viewMode === 'table'
                ? 'bg-white text-brand-600 shadow-sm dark:bg-slate-800 dark:text-brand-400'
                : 'text-slate-400 hover:text-slate-700 dark:hover:text-slate-300',
            ]"
            title="详细指标表格视图"
          >
            <List class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- 实时指示器与刷新按钮 -->
        <div class="flex items-center space-x-2 border-l border-slate-200 pl-3 dark:border-slate-800">
          <span
            :class="[
              'inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium',
              isLive
                ? 'bg-emerald-50 text-emerald-700 ring-1 ring-inset ring-emerald-600/20 dark:bg-emerald-950 dark:text-emerald-400'
                : 'bg-slate-100 text-slate-600 ring-1 ring-inset ring-slate-500/10 dark:bg-slate-800 dark:text-slate-400',
            ]"
          >
            <Radio class="mr-1 h-3 w-3 animate-pulse" v-if="isLive" />
            {{ isLive ? '实时 D1' : '本地存储' }}
          </span>

          <button
            @click="fetchOverview"
            :disabled="isLoading"
            title="刷新数据"
            class="rounded p-1 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-300 transition-colors"
          >
            <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': isLoading }" />
          </button>
        </div>
      </div>
    </div>

    <!-- 模式 1：紧凑卡片网格视图 -->
    <div v-if="viewMode === 'cards'" class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="node in filteredNodes"
        :key="node.agent_id"
        @click="router.push('/nodes/' + node.agent_id)"
        class="group relative rounded-lg border border-slate-200 bg-white p-4 shadow-sm hover:border-brand-500/50 hover:shadow-md transition-all cursor-pointer dark:border-slate-800 dark:bg-slate-900 dark:hover:border-brand-500/50"
      >
        <!-- 卡片头部：国旗、主机名、在线灯、系统与快捷操作 -->
        <div class="flex items-start justify-between">
          <div class="flex items-center space-x-2.5">
            <span class="text-xl select-none" :title="getRegionBadge(node.labels).name">
              {{ getRegionBadge(node.labels).flag }}
            </span>
            <div>
              <div class="flex items-center space-x-2">
                <span class="font-bold text-sm text-slate-900 dark:text-slate-100 group-hover:text-brand-600 transition-colors">
                  {{ node.hostname }}
                </span>
                <span class="rounded bg-slate-100 px-1 py-0.2 text-[9px] font-mono text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                  {{ node.os }}
                </span>
              </div>
              <div class="text-[11px] text-slate-500 mt-0.5 truncate max-w-[160px]">
                {{ node.display_name }}
              </div>
            </div>
          </div>

          <!-- 右上角快捷操作与状态徽章 -->
          <div class="flex items-center space-x-1">
            <!-- 快捷编辑按钮 -->
            <button
              @click.stop="openEditModal(node, $event)"
              class="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200 transition-all"
              title="编辑服务器设置"
            >
              <Edit2 class="h-3.5 w-3.5" />
            </button>

            <!-- 快捷删除按钮 -->
            <button
              @click.stop="confirmDelete(node.agent_id, node.hostname, $event)"
              class="rounded p-1 text-slate-400 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950 transition-all"
              title="删除/下线此服务器"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>

            <!-- 在线状态徽章 -->
            <span
              v-if="!node.is_online"
              class="inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400"
            >
              离线
            </span>
            <span
              v-else-if="node.health_status === 'ok'"
              class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300"
            >
              <span class="h-1.5 w-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
              正常
            </span>
            <span
              v-else
              class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-300"
            >
              <AlertTriangle class="h-3 w-3" />
              降级
            </span>
          </div>
        </div>

        <!-- 卡片核心资源紧凑条（CPU、内存、磁盘、月度流量） -->
        <div class="mt-4 space-y-2.5 text-xs">
          <!-- CPU -->
          <div>
            <div class="flex justify-between text-[11px] text-slate-500 mb-1">
              <span class="flex items-center gap-1"><Cpu class="h-3 w-3" /> CPU</span>
              <span class="font-mono font-medium">{{ formatRatio(node.cpu_usage_ratio) }}</span>
            </div>
            <div class="h-1.5 w-full rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
              <div
                class="h-full rounded-full transition-all"
                :class="(node.cpu_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-brand-500'"
                :style="{ width: `${Math.min((node.cpu_usage_ratio || 0) * 100, 100)}%` }"
              ></div>
            </div>
          </div>

          <!-- 内存 -->
          <div>
            <div class="flex justify-between text-[11px] text-slate-500 mb-1">
              <span class="flex items-center gap-1"><Database class="h-3 w-3" /> 内存</span>
              <span class="font-mono font-medium">{{ formatRatio(node.memory_usage_ratio) }}</span>
            </div>
            <div class="h-1.5 w-full rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
              <div
                class="h-full rounded-full transition-all"
                :class="(node.memory_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-indigo-500'"
                :style="{ width: `${Math.min((node.memory_usage_ratio || 0) * 100, 100)}%` }"
              ></div>
            </div>
          </div>

          <!-- 磁盘容量 -->
          <div>
            <div class="flex justify-between text-[11px] text-slate-500 mb-1">
              <span class="flex items-center gap-1"><HardDrive class="h-3 w-3" /> 磁盘</span>
              <span class="font-mono font-medium">{{ formatRatio(node.disk_usage_ratio) }}</span>
            </div>
            <div class="h-1.5 w-full rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
              <div
                class="h-full rounded-full transition-all"
                :class="(node.disk_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-slate-400 dark:bg-slate-600'"
                :style="{ width: `${Math.min((node.disk_usage_ratio || 0) * 100, 100)}%` }"
              ></div>
            </div>
          </div>

          <!-- 月度周期流量进度 -->
          <div>
            <div class="flex justify-between text-[11px] text-slate-500 mb-1">
              <span class="flex items-center gap-1"><BarChart3 class="h-3 w-3" /> 月流量</span>
              <span class="font-mono font-medium">
                {{ formatDataSize(node.cycle_outbound_bytes) }} / {{ formatDataSize(node.cycle_limit_bytes || 1099511627776) }}
                ({{ formatRatio(node.cycle_outbound_ratio || 0.25) }})
              </span>
            </div>
            <div class="h-1.5 w-full rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
              <div
                class="h-full rounded-full transition-all"
                :class="(node.cycle_outbound_ratio || 0) >= 0.85 ? 'bg-rose-500' : 'bg-emerald-500'"
                :style="{ width: `${Math.min((node.cycle_outbound_ratio || 0.25) * 100, 100)}%` }"
              ></div>
            </div>
          </div>
        </div>

        <!-- 卡片底部：实时上下行速率与负载 -->
        <div class="mt-4 pt-3 border-t border-slate-100 dark:border-slate-800/80 flex items-center justify-between text-[11px] text-slate-500 font-mono">
          <div class="flex items-center space-x-2">
            <span class="flex items-center text-emerald-600 dark:text-emerald-400">
              <ArrowDownLeft class="h-3 w-3 mr-0.5" />
              {{ formatBytes(node.network_rx_bytes_per_sec) }}
            </span>
            <span class="flex items-center text-brand-600 dark:text-brand-400">
              <ArrowUpRight class="h-3 w-3 mr-0.5" />
              {{ formatBytes(node.network_tx_bytes_per_sec) }}
            </span>
          </div>

          <div>负载: {{ node.load_1m !== null ? node.load_1m.toFixed(2) : '—' }}</div>
        </div>
      </div>
    </div>

    <!-- 模式 2：详细表格视图 -->
    <div v-else class="overflow-hidden rounded-lg border border-slate-200 bg-white shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <table class="min-w-full divide-y divide-slate-200 dark:divide-slate-800 text-left text-sm">
        <thead class="bg-slate-50 dark:bg-slate-950 text-xs font-semibold text-slate-500">
          <tr>
            <th scope="col" class="py-3.5 pl-4 pr-3 sm:pl-6">节点信息</th>
            <th scope="col" class="px-3 py-3.5">健康状态</th>
            <th scope="col" class="px-3 py-3.5">CPU 使用率</th>
            <th scope="col" class="px-3 py-3.5">内存使用率</th>
            <th scope="col" class="px-3 py-3.5">根磁盘容量</th>
            <th scope="col" class="px-3 py-3.5">月度出站流量</th>
            <th scope="col" class="px-3 py-3.5">实时吞吐 (↓/↑)</th>
            <th scope="col" class="px-3 py-3.5">最近活跃</th>
            <th scope="col" class="py-3.5 pl-3 pr-4 sm:pr-6 text-right">操作</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
          <tr
            v-for="node in filteredNodes"
            :key="node.agent_id"
            @click="router.push('/nodes/' + node.agent_id)"
            class="hover:bg-slate-50/80 dark:hover:bg-slate-800/30 transition-colors cursor-pointer group"
          >
            <!-- 节点基础信息（带旗帜） -->
            <td class="py-4 pl-4 pr-3 sm:pl-6">
              <div class="flex items-center space-x-2">
                <span class="text-lg select-none">{{ getRegionBadge(node.labels).flag }}</span>
                <div>
                  <div class="font-medium text-slate-900 dark:text-slate-100 flex items-center space-x-2">
                    <span class="group-hover:text-brand-600 transition-colors">{{ node.hostname }}</span>
                    <span class="rounded bg-slate-100 px-1.5 py-0.5 text-[10px] font-mono text-slate-600 dark:bg-slate-800 dark:text-slate-300">
                      {{ node.os }}/{{ node.arch }}
                    </span>
                  </div>
                  <div class="text-xs text-slate-500 mt-0.5">{{ node.display_name }}</div>
                </div>
              </div>
            </td>

            <!-- 健康状态 -->
            <td class="px-3 py-4">
              <span
                v-if="!node.is_online"
                class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400"
              >
                <HelpCircle class="h-3 w-3" />
                离线
              </span>
              <span
                v-else-if="node.health_status === 'ok'"
                class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-emerald-50 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300"
              >
                <CheckCircle2 class="h-3 w-3" />
                健康
              </span>
              <span
                v-else
                class="inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium bg-amber-50 text-amber-700 dark:bg-amber-950 dark:text-amber-300"
              >
                <AlertTriangle class="h-3 w-3" />
                降级
              </span>
            </td>

            <!-- CPU -->
            <td class="px-3 py-4">
              <div class="text-xs font-mono font-medium">{{ formatRatio(node.cpu_usage_ratio) }}</div>
              <div class="mt-1 h-1.5 w-20 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                <div
                  class="h-full rounded-full transition-all"
                  :class="(node.cpu_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-brand-500'"
                  :style="{ width: `${Math.min((node.cpu_usage_ratio || 0) * 100, 100)}%` }"
                ></div>
              </div>
            </td>

            <!-- 内存 -->
            <td class="px-3 py-4">
              <div class="text-xs font-mono font-medium">{{ formatRatio(node.memory_usage_ratio) }}</div>
              <div class="mt-1 h-1.5 w-20 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                <div
                  class="h-full rounded-full transition-all"
                  :class="(node.memory_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-indigo-500'"
                  :style="{ width: `${Math.min((node.memory_usage_ratio || 0) * 100, 100)}%` }"
                ></div>
              </div>
            </td>

            <!-- 磁盘 -->
            <td class="px-3 py-4">
              <div class="text-xs font-mono font-medium">{{ formatRatio(node.disk_usage_ratio) }}</div>
              <div class="mt-1 h-1.5 w-20 rounded-full bg-slate-100 dark:bg-slate-800 overflow-hidden">
                <div
                  class="h-full rounded-full transition-all"
                  :class="(node.disk_usage_ratio || 0) >= 0.90 ? 'bg-rose-500' : 'bg-slate-400 dark:bg-slate-600'"
                  :style="{ width: `${Math.min((node.disk_usage_ratio || 0) * 100, 100)}%` }"
                ></div>
              </div>
            </td>

            <!-- 月度出站流量 -->
            <td class="px-3 py-4">
              <div class="text-xs font-mono font-medium">
                {{ formatDataSize(node.cycle_outbound_bytes) }}
              </div>
              <div class="text-[10px] text-slate-400 font-mono">
                / {{ formatDataSize(node.cycle_limit_bytes || 1099511627776) }}
              </div>
            </td>

            <!-- 实时吞吐 -->
            <td class="px-3 py-4 text-xs font-mono text-slate-500">
              <div class="text-emerald-600 dark:text-emerald-400">↓ {{ formatBytes(node.network_rx_bytes_per_sec) }}</div>
              <div class="text-brand-600 dark:text-brand-400">↑ {{ formatBytes(node.network_tx_bytes_per_sec) }}</div>
            </td>

            <!-- 最近活跃 -->
            <td class="px-3 py-4 text-xs text-slate-400">
              {{ new Date(node.last_seen).toLocaleTimeString() }}
            </td>

            <!-- 操作按钮组 -->
            <td class="py-4 pl-3 pr-4 sm:pr-6 text-right whitespace-nowrap">
              <div class="flex items-center justify-end space-x-1" @click.stop>
                <button
                  @click="openEditModal(node, $event)"
                  class="rounded p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200 transition-colors"
                  title="修改设置"
                >
                  <Edit2 class="h-3.5 w-3.5" />
                </button>
                <button
                  @click="confirmDelete(node.agent_id, node.hostname, $event)"
                  class="rounded p-1 text-slate-400 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950 transition-colors"
                  title="删除节点"
                >
                  <Trash2 class="h-3.5 w-3.5" />
                </button>
                <button
                  @click="router.push('/nodes/' + node.agent_id)"
                  class="rounded p-1 text-slate-400 hover:text-brand-600 transition-colors"
                  title="查看详情"
                >
                  <ChevronRight class="h-4 w-4" />
                </button>
              </div>
            </td>
          </tr>

          <!-- 空状态 -->
          <tr v-if="filteredNodes.length === 0">
            <td colspan="9" class="py-12 text-center text-sm text-slate-500">
              <div class="flex flex-col items-center justify-center space-y-2">
                <Server class="h-8 w-8 text-slate-300" />
                <p>未找到匹配条件的探针节点</p>
                <button
                  @click="searchQuery = ''; selectedOs = 'all'; selectedStatus = 'all'"
                  class="text-xs text-brand-600 hover:underline"
                >
                  重置筛选条件
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 服务器添加 / 修改弹窗组件 -->
    <ServerModal
      :is-open="isModalOpen"
      :mode="modalMode"
      :initial-data="editingNode"
      @close="isModalOpen = false"
      @save="handleSaveNode"
    />

    <!-- 内置现代化删除确认对话框 -->
    <div v-if="nodeToDelete" class="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div class="fixed inset-0 bg-slate-950/60 backdrop-blur-sm transition-opacity" @click="nodeToDelete = null"></div>
      <div class="relative w-full max-w-md overflow-hidden rounded-xl bg-white p-6 shadow-2xl ring-1 ring-slate-900/10 dark:bg-slate-900 dark:ring-slate-800">
        <div class="flex items-start space-x-3">
          <div class="rounded-full bg-rose-100 p-2 text-rose-600 dark:bg-rose-950/60 dark:text-rose-400 shrink-0">
            <AlertCircle class="h-5 w-5" />
          </div>
          <div class="flex-1">
            <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100">确认删除服务器节点？</h3>
            <p class="mt-1 text-xs text-slate-500">
              确定要下线并删除服务器 <strong class="font-mono text-slate-800 dark:text-slate-200">[{{ nodeToDelete.hostname }}]</strong> 吗？
              删除后该节点将从列表和持久化存储中彻底移除，<strong>页面刷新后不会再次恢复</strong>。
            </p>
          </div>
        </div>

        <div class="mt-5 flex items-center justify-end space-x-3">
          <button
            @click="nodeToDelete = null"
            class="rounded-lg border border-slate-300 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800 transition-colors"
          >
            取消
          </button>
          <button
            @click="executeDelete"
            class="rounded-lg bg-rose-600 px-3 py-1.5 text-xs font-semibold text-white shadow-sm hover:bg-rose-500 transition-colors"
          >
            确认永久删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
