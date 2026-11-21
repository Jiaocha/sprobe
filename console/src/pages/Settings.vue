<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  Settings,
  Key,
  Bell,
  RotateCcw,
  Save,
  Check,
  Copy,
  Shield,
  Layers,
  AlertTriangle,
} from 'lucide-vue-next'
import { getStoredSettings, saveStoredSettings, resetNodesToDefault, type SiteSettings } from '../utils/storage'

const settings = ref<SiteSettings>(getStoredSettings())
const isSaved = ref(false)
const copiedToken = ref(false)
const resetConfirmed = ref(false)

onMounted(() => {
  settings.value = getStoredSettings()
})

const isTestingAlert = ref(false)
const testAlertResult = ref<string | null>(null)

async function handleSave() {
  saveStoredSettings(settings.value)
  // 同步提交到 Cloudflare 边缘端存储
  try {
    await fetch('/api/settings', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(settings.value),
    })
  } catch (_) {}

  isSaved.value = true
  setTimeout(() => {
    isSaved.value = false
  }, 2500)
}

async function handleTestAlert() {
  isTestingAlert.value = true
  testAlertResult.value = null
  try {
    const res = await fetch('/api/alerts/test', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ notifications: settings.value.notifications }),
    })
    if (res.ok) {
      testAlertResult.value = '✓ 测试告警已通过边缘节点成功发出！'
    } else {
      testAlertResult.value = '⚠️ 发送失败，请检查 Webhook 或 Bot 配置'
    }
  } catch (e: any) {
    testAlertResult.value = `⚠️ 发送出错: ${e.message}`
  } finally {
    isTestingAlert.value = false
    setTimeout(() => {
      testAlertResult.value = null
    }, 4000)
  }
}

function generateRandomToken() {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
  let result = 'sprobe-'
  for (let i = 0; i < 24; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  settings.value.globalToken = result
}

function copyToken() {
  navigator.clipboard.writeText(settings.value.globalToken)
  copiedToken.value = true
  setTimeout(() => {
    copiedToken.value = false
  }, 2000)
}

function handleResetData() {
  if (confirm('确定要恢复默认预设的服务器节点与模拟数据吗？您自建的测试节点将被重置。')) {
    resetNodesToDefault()
    resetConfirmed.value = true
    setTimeout(() => {
      resetConfirmed.value = false
    }, 2500)
  }
}
</script>

<template>
  <div class="space-y-6 max-w-4xl mx-auto pb-12">
    <!-- 标题与保存控制栏 -->
    <div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between border-b border-slate-200 pb-5 dark:border-slate-800">
      <div>
        <div class="flex items-center space-x-2">
          <Settings class="h-6 w-6 text-brand-600" />
          <h1 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
            控制台与系统设置
          </h1>
        </div>
        <p class="text-xs text-slate-500 mt-1">
          管理站点标题、通信密钥、默认视图模式、VPS 流量告警阈值及通知渠道
        </p>
      </div>

      <div class="flex items-center space-x-3">
        <button
          @click="handleSave"
          class="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-4 py-2 text-xs font-semibold text-white shadow-sm hover:bg-brand-500 transition-colors"
        >
          <Check v-if="isSaved" class="h-4 w-4 text-emerald-300" />
          <Save v-else class="h-4 w-4" />
          <span>{{ isSaved ? '设置已保存' : '保存所有设置' }}</span>
        </button>
      </div>
    </div>

    <!-- 1. 站点与偏好设置 -->
    <div class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4">
      <div class="flex items-center space-x-2 pb-3 border-b border-slate-100 dark:border-slate-800">
        <Layers class="h-4 w-4 text-brand-600" />
        <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">站点与显示偏好</h2>
      </div>

      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 text-xs">
        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">站点名称</label>
          <input
            v-model="settings.siteTitle"
            type="text"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
          />
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">站点副标题 / 说明</label>
          <input
            v-model="settings.siteSubtitle"
            type="text"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
          />
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">默认总览呈现视图</label>
          <select
            v-model="settings.defaultView"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
          >
            <option value="cards">哪吒经典服务器卡片网格 (Cards Grid)</option>
            <option value="table">紧凑运维详细表格视图 (Table Mode)</option>
          </select>
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">数据自动轮询刷新频率 (秒)</label>
          <input
            v-model.number="settings.refreshInterval"
            type="number"
            min="1"
            max="60"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>
      </div>
    </div>

    <!-- 2. 通信密钥与服务端点 -->
    <div class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4">
      <div class="flex items-center space-x-2 pb-3 border-b border-slate-100 dark:border-slate-800">
        <Key class="h-4 w-4 text-brand-600" />
        <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">探针上报端点与鉴权密钥</h2>
      </div>

      <div class="space-y-4 text-xs">
        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">服务端接收 Ingest 端点 URL</label>
          <input
            v-model="settings.serverEndpoint"
            type="text"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
          <p class="text-[11px] text-slate-400 mt-1">探针启动时将向此 HTTPS 地址批量推送系统快照与心跳包。</p>
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">全局通信 Token 密钥 (Secret Token)</label>
          <div class="flex items-center space-x-2">
            <input
              v-model="settings.globalToken"
              type="text"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
            <button
              @click="copyToken"
              class="inline-flex items-center gap-1 shrink-0 rounded-md border border-slate-300 px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800 transition-colors"
            >
              <Check v-if="copiedToken" class="h-3.5 w-3.5 text-emerald-600" />
              <Copy v-else class="h-3.5 w-3.5" />
              <span>{{ copiedToken ? '已复制' : '复制' }}</span>
            </button>
            <button
              @click="generateRandomToken"
              class="inline-flex items-center gap-1 shrink-0 rounded-md border border-slate-300 px-3 py-2 text-xs text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800 transition-colors"
            >
              <Shield class="h-3.5 w-3.5" />
              <span>随机生成</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. 告警阈值设置 -->
    <div class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4">
      <div class="flex items-center space-x-2 pb-3 border-b border-slate-100 dark:border-slate-800">
        <AlertTriangle class="h-4 w-4 text-amber-600" />
        <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">全局健康告警阈值 (Alert Thresholds)</h2>
      </div>

      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 text-xs">
        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">CPU 使用率告警水位 (%)</label>
          <input
            v-model.number="settings.alertThresholds.cpuRatio"
            type="number"
            min="50"
            max="100"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">内存使用率告警水位 (%)</label>
          <input
            v-model.number="settings.alertThresholds.memRatio"
            type="number"
            min="50"
            max="100"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">磁盘容量告警水位 (%)</label>
          <input
            v-model.number="settings.alertThresholds.diskRatio"
            type="number"
            min="50"
            max="100"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>

        <div>
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">VPS 月结出账流量警戒水位 (%)</label>
          <input
            v-model.number="settings.alertThresholds.trafficRatio"
            type="number"
            min="50"
            max="100"
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>
      </div>
    </div>

    <!-- 4. 通知渠道 -->
    <div class="rounded-xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-800 dark:bg-slate-900 space-y-4">
      <div class="flex items-center space-x-2 pb-3 border-b border-slate-100 dark:border-slate-800">
        <Bell class="h-4 w-4 text-brand-600" />
        <h2 class="text-sm font-semibold text-slate-900 dark:text-slate-100">通知渠道配置 (Alert Notifications)</h2>
      </div>

      <div class="space-y-4 text-xs">
        <!-- Webhook -->
        <div class="flex items-center justify-between">
          <div>
            <span class="font-medium text-slate-800 dark:text-slate-200">启用 Webhook 警报推送 (Slack / 飞书 / 钉钉 / 企业微信)</span>
            <p class="text-[11px] text-slate-400">当节点离线或触发 CPU / 流量告警时，发送 HTTP POST JSON 通知</p>
          </div>
          <input
            v-model="settings.notifications.webhookEnabled"
            type="checkbox"
            class="h-4 w-4 rounded border-slate-300 text-brand-600 focus:ring-brand-500"
          />
        </div>

        <div v-if="settings.notifications.webhookEnabled">
          <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">Webhook URL</label>
          <input
            v-model="settings.notifications.webhookUrl"
            type="text"
            placeholder="https://oapi.dingtalk.com/robot/send?access_token=..."
            class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
          />
        </div>

        <!-- Telegram Bot -->
        <div class="pt-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between">
          <div>
            <span class="font-medium text-slate-800 dark:text-slate-200">启用 Telegram Bot 告警通知</span>
            <p class="text-[11px] text-slate-400">由 Cloudflare 边缘节点直接向您的 Telegram 发起离线或过载告警</p>
          </div>
          <input
            v-model="settings.notifications.telegramEnabled"
            type="checkbox"
            class="h-4 w-4 rounded border-slate-300 text-brand-600 focus:ring-brand-500"
          />
        </div>

        <div v-if="settings.notifications.telegramEnabled" class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">Telegram Bot Token</label>
            <input
              v-model="settings.notifications.telegramBotToken"
              type="text"
              placeholder="123456789:ABCdefGhIJKlmNoPQRsTUVwxyZ"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
          </div>
          <div>
            <label class="block font-medium text-slate-700 dark:text-slate-300 mb-1">Telegram Chat ID / User ID</label>
            <input
              v-model="settings.notifications.telegramChatId"
              type="text"
              placeholder="例如: 987654321"
              class="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-slate-900 shadow-sm focus:border-brand-500 focus:outline-none dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 font-mono"
            />
          </div>
        </div>

        <!-- 测试通知控制栏 -->
        <div class="pt-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between">
          <div class="text-[11px] text-slate-500">
            <span v-if="testAlertResult" :class="testAlertResult.startsWith('✓') ? 'text-emerald-600 font-bold' : 'text-amber-600 font-medium'">
              {{ testAlertResult }}
            </span>
            <span v-else>配置后可点击右侧按钮立即测试通知连通性</span>
          </div>

          <button
            type="button"
            @click="handleTestAlert"
            :disabled="isTestingAlert || (!settings.notifications.webhookEnabled && !settings.notifications.telegramEnabled)"
            class="inline-flex items-center gap-1.5 rounded border border-slate-300 bg-white px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50 disabled:opacity-50 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 transition-colors"
          >
            <Bell class="h-3.5 w-3.5" :class="{ 'animate-bounce': isTestingAlert }" />
            <span>{{ isTestingAlert ? '发送中...' : '测试告警连通' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 5. 数据管理与恢复出厂 -->
    <div class="rounded-xl border border-rose-200 bg-rose-50/30 p-6 shadow-sm dark:border-rose-950 dark:bg-rose-950/20 space-y-4">
      <div class="flex items-center space-x-2 pb-3 border-b border-rose-100 dark:border-rose-900/40">
        <RotateCcw class="h-4 w-4 text-rose-600 dark:text-rose-400" />
        <h2 class="text-sm font-semibold text-rose-900 dark:text-rose-200">数据与出厂预设重置</h2>
      </div>

      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 text-xs">
        <div>
          <p class="text-slate-700 dark:text-slate-300 font-medium">重置本地服务器节点数据</p>
          <p class="text-slate-500 text-[11px] mt-0.5">
            如果您在测试过程中添加或删除了节点，点击此按钮可一键恢复初始的 4 台各大洲演示服务器。
          </p>
        </div>

        <button
          @click="handleResetData"
          class="inline-flex items-center gap-1.5 rounded-lg border border-rose-300 bg-white px-4 py-2 text-xs font-semibold text-rose-700 hover:bg-rose-50 dark:border-rose-900 dark:bg-slate-900 dark:text-rose-300 dark:hover:bg-rose-950 transition-colors shrink-0"
        >
          <RotateCcw class="h-3.5 w-3.5" />
          <span>{{ resetConfirmed ? '数据已恢复默认' : '恢复默认预设节点' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
