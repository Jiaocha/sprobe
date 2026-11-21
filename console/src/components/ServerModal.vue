<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { X, Copy, Check, Terminal, Settings, Server } from 'lucide-vue-next'
import type { NodeFormData } from '../utils/storage'
import { getStoredSettings } from '../utils/storage'

const props = defineProps<{
  isOpen: boolean
  mode: 'add' | 'edit'
  initialData?: Partial<NodeFormData>
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', data: NodeFormData): void
}>()

const activeTab = ref<'form' | 'install'>('form')
const copiedCommand = ref<string | null>(null)

const form = ref<NodeFormData>({
  hostname: '',
  display_name: '',
  os: 'linux',
  arch: 'x86_64',
  region: 'US-West',
  group: '核心业务',
  cycle_start_day: 1,
  monthly_limit_gib: 1024,
})

const regionOptions = [
  { flag: '🇨🇳', label: '中国 (CN-Shanghai)', value: 'CN-Shanghai' },
  { flag: '🇨🇳', label: '中国 (CN-Beijing)', value: 'CN-Beijing' },
  { flag: '🇺🇸', label: '美国西海岸 (US-West)', value: 'US-West' },
  { flag: '🇺🇸', label: '美国硅谷 (US-SiliconValley)', value: 'US-SiliconValley' },
  { flag: '🇯🇵', label: '日本东京 (JP-Tokyo)', value: 'JP-Tokyo' },
  { flag: '🇭🇰', label: '中国香港 (HK-HongKong)', value: 'HK-HongKong' },
  { flag: '🇸🇬', label: '新加坡 (SG-Singapore)', value: 'SG-Singapore' },
  { flag: '🇩🇪', label: '德国法兰克福 (DE-Frankfurt)', value: 'DE-Frankfurt' },
  { flag: '🌐', label: '其他/通用 (Global)', value: 'Global' },
]

watch(
  () => props.initialData,
  (val) => {
    if (val) {
      form.value = {
        hostname: val.hostname || '',
        display_name: val.display_name || val.hostname || '',
        os: val.os || 'linux',
        arch: val.arch || 'x86_64',
        region: val.region || 'US-West',
        group: val.group || '默认分组',
        cycle_start_day: val.cycle_start_day || 1,
        monthly_limit_gib: val.monthly_limit_gib ?? 1024,
        agent_id: val.agent_id,
      }
    } else {
      form.value = {
        hostname: `vps-${Math.random().toString(36).substring(2, 6)}`,
        display_name: '新接入服务器',
        os: 'linux',
        arch: 'x86_64',
        region: 'US-West',
        group: '默认集群',
        cycle_start_day: 1,
        monthly_limit_gib: 1024,
      }
    }
    activeTab.value = 'form'
  },
  { immediate: true }
)

const installCommands = computed(() => {
  const settings = getStoredSettings()
  const endpoint = settings.serverEndpoint
  const token = settings.globalToken
  const host = form.value.hostname || 'my-node'

  return {
    linux: `curl -fsSL https://sprobe-console.pages.dev/install.sh | bash -s -- --endpoint "${endpoint}" --token "${token}" --name "${host}"`,
    windows: `iwr -useb https://sprobe-console.pages.dev/install.ps1 | iex; Start-Sprobe -Endpoint "${endpoint}" -Token "${token}" -Hostname "${host}"`,
    docker: `docker run -d --name sprobe --restart always --net host --pid host -v /:/rootfs:ro -e SPROBE_ENDPOINT="${endpoint}" -e SPROBE_TOKEN="${token}" sprobe/agent:latest`,
  }
})

function copyText(cmd: string, key: string) {
  navigator.clipboard.writeText(cmd)
  copiedCommand.value = key
  setTimeout(() => {
    copiedCommand.value = null
  }, 2000)
}

function handleSave() {
  if (!form.value.hostname.trim()) {
    form.value.hostname = `vps-${Date.now().toString(36).substring(4)}`
  }
  if (!form.value.display_name.trim()) {
    form.value.display_name = form.value.hostname
  }
  emit('save', { ...form.value })
  emit('close')
}
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <!-- 背景遮罩 -->
    <div class="fixed inset-0 bg-slate-950/60 backdrop-blur-sm transition-opacity" @click="emit('close')"></div>

    <!-- 弹窗主体 -->
    <div
      class="relative w-full max-w-xl overflow-hidden rounded-xl bg-white shadow-2xl ring-1 ring-slate-900/10 dark:bg-slate-900 dark:ring-slate-800"
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between border-b border-slate-200 px-6 py-4 dark:border-slate-800">
        <div class="flex items-center space-x-2">
          <Server class="h-5 w-5 text-brand-600" />
          <h3 class="text-base font-bold text-slate-900 dark:text-slate-100">
            {{ mode === 'add' ? '接入新服务器探针' : '编辑服务器设置' }}
          </h3>
        </div>
        <button
          @click="emit('close')"
          class="rounded-lg p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-500 dark:hover:bg-slate-800"
        >
          <X class="h-5 w-5" />
        </button>
      </div>

      <!-- 标签页切换 (添加模式支持查看一键安装指令) -->
      <div v-if="mode === 'add'" class="flex border-b border-slate-200 px-6 text-xs font-medium dark:border-slate-800">
        <button
          @click="activeTab = 'form'"
          class="flex items-center gap-1.5 border-b-2 py-3 px-3 transition-colors"
          :class="activeTab === 'form' ? 'border-brand-600 text-brand-600 font-semibold' : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <Settings class="h-3.5 w-3.5" />
          <span>基础配置参数</span>
        </button>
        <button
          @click="activeTab = 'install'"
          class="flex items-center gap-1.5 border-b-2 py-3 px-3 transition-colors"
          :class="activeTab === 'install' ? 'border-brand-600 text-brand-600 font-semibold' : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <Terminal class="h-3.5 w-3.5" />
          <span>一键安装命令</span>
        </button>
      </div>

      <!-- 选项 1: 表单编辑 -->
      <div v-if="activeTab === 'form'" class="max-h-[70vh] overflow-y-auto px-6 py-5 space-y-4 text-xs">
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <!-- 主机名 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">主机名 (Hostname) *</label>
            <input
              v-model="form.hostname"
              type="text"
              placeholder="例如: vps-us-prod-01"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
          </div>

          <!-- 自定义显示别名 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">显示别名 (Display Name)</label>
            <input
              v-model="form.display_name"
              type="text"
              placeholder="例如: 硅谷主节点"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
          </div>

          <!-- 地区与国旗选择 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">节点地理位置 (匹配国旗)</label>
            <select
              v-model="form.region"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            >
              <option v-for="opt in regionOptions" :key="opt.value" :value="opt.value">
                {{ opt.flag }} {{ opt.label }}
              </option>
            </select>
          </div>

          <!-- 所属分组 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">集群分组标签 (Group)</label>
            <input
              v-model="form.group"
              type="text"
              placeholder="例如: 边缘网关 / 核心业务"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            />
          </div>

          <!-- 操作系统 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">系统平台</label>
            <select
              v-model="form.os"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
            >
              <option value="linux">Linux (Ubuntu / Debian / CentOS / Alpine)</option>
              <option value="windows">Windows Server / Windows 11</option>
              <option value="darwin">macOS (Darwin)</option>
            </select>
          </div>

          <!-- CPU 架构 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">CPU 架构</label>
            <select
              v-model="form.arch"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            >
              <option value="x86_64">x86_64 (amd64)</option>
              <option value="aarch64">aarch64 (arm64)</option>
            </select>
          </div>

          <!-- 月结出账日 -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">VPS 月结出账日 (1-31 日)</label>
            <input
              v-model.number="form.cycle_start_day"
              type="number"
              min="1"
              max="31"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
          </div>

          <!-- 月度出站流量配额 (GB) -->
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">月度出账流量配额 (GiB)</label>
            <input
              v-model.number="form.monthly_limit_gib"
              type="number"
              min="0"
              step="100"
              placeholder="例如: 1024"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
          </div>
        </div>

        <p class="text-[11px] text-slate-400">
          * 提示：保存后数据将立即在控制台生效并持久化在本地浏览器存储中。
        </p>
      </div>

      <!-- 选项 2: 一键安装脚本展示 -->
      <div v-else class="max-h-[70vh] overflow-y-auto px-6 py-5 space-y-4 text-xs">
        <p class="text-slate-500">
          在您的目标 VPS 服务器终端执行以下任一命令，探针即可一键完成拉取、初始化并自动接入此控制台：
        </p>

        <!-- Linux 命令 -->
        <div class="space-y-1.5">
          <div class="flex items-center justify-between font-medium text-slate-700 dark:text-slate-300">
            <span>Linux (curl 一键安装服务)</span>
            <button
              @click="copyText(installCommands.linux, 'linux')"
              class="inline-flex items-center gap-1 text-[11px] text-brand-600 hover:text-brand-700 font-semibold"
            >
              <Check v-if="copiedCommand === 'linux'" class="h-3 w-3 text-emerald-600" />
              <Copy v-else class="h-3 w-3" />
              <span>{{ copiedCommand === 'linux' ? '已复制' : '复制命令' }}</span>
            </button>
          </div>
          <pre class="overflow-x-auto rounded-lg bg-slate-900 p-3 font-mono text-[11px] text-emerald-400"><code>{{ installCommands.linux }}</code></pre>
        </div>

        <!-- Windows 命令 -->
        <div class="space-y-1.5">
          <div class="flex items-center justify-between font-medium text-slate-700 dark:text-slate-300">
            <span>Windows (PowerShell 管理员一键启动)</span>
            <button
              @click="copyText(installCommands.windows, 'windows')"
              class="inline-flex items-center gap-1 text-[11px] text-brand-600 hover:text-brand-700 font-semibold"
            >
              <Check v-if="copiedCommand === 'windows'" class="h-3 w-3 text-emerald-600" />
              <Copy v-else class="h-3 w-3" />
              <span>{{ copiedCommand === 'windows' ? '已复制' : '复制命令' }}</span>
            </button>
          </div>
          <pre class="overflow-x-auto rounded-lg bg-slate-900 p-3 font-mono text-[11px] text-emerald-400"><code>{{ installCommands.windows }}</code></pre>
        </div>

        <!-- Docker 命令 -->
        <div class="space-y-1.5">
          <div class="flex items-center justify-between font-medium text-slate-700 dark:text-slate-300">
            <span>Docker (容器化 host 模式运行)</span>
            <button
              @click="copyText(installCommands.docker, 'docker')"
              class="inline-flex items-center gap-1 text-[11px] text-brand-600 hover:text-brand-700 font-semibold"
            >
              <Check v-if="copiedCommand === 'docker'" class="h-3 w-3 text-emerald-600" />
              <Copy v-else class="h-3 w-3" />
              <span>{{ copiedCommand === 'docker' ? '已复制' : '复制命令' }}</span>
            </button>
          </div>
          <pre class="overflow-x-auto rounded-lg bg-slate-900 p-3 font-mono text-[11px] text-emerald-400"><code>{{ installCommands.docker }}</code></pre>
        </div>
      </div>

      <!-- 底部操作按钮 -->
      <div class="flex items-center justify-end space-x-3 border-t border-slate-200 px-6 py-4 dark:border-slate-800">
        <button
          @click="emit('close')"
          class="rounded-lg border border-slate-300 px-4 py-2 text-xs font-medium text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800 transition-colors"
        >
          取消
        </button>
        <button
          @click="handleSave"
          class="rounded-lg bg-brand-600 px-4 py-2 text-xs font-semibold text-white shadow-sm hover:bg-brand-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 transition-colors"
        >
          {{ mode === 'add' ? '立即添加' : '保存修改' }}
        </button>
      </div>
    </div>
  </div>
</template>
