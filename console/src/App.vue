<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { Moon, Sun, Activity, Settings, Lock } from 'lucide-vue-next'
import { getStoredSettings } from './utils/storage'
import { isAdmin, logoutAdmin, checkAdmin } from './utils/auth'
import AuthModal from './components/AuthModal.vue'

const route = useRoute()
const isDark = ref(false)
const siteTitle = ref('Sprobe Console')

onMounted(() => {
  const s = getStoredSettings()
  if (s && s.siteTitle) {
    siteTitle.value = s.siteTitle
  }
})

function toggleDark() {
  isDark.value = !isDark.value
  if (isDark.value) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}
</script>

<template>
  <div class="min-h-screen bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100 flex flex-col">
    <!-- 顶部导航栏 -->
    <header class="sticky top-0 z-30 border-b border-slate-200 bg-white/80 backdrop-blur dark:border-slate-800 dark:bg-slate-900/80">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-14 flex items-center justify-between">
        <div class="flex items-center space-x-6">
          <router-link to="/overview" class="flex items-center space-x-3 hover:opacity-90 transition-opacity">
            <div class="w-8 h-8 rounded-lg bg-brand-600 flex items-center justify-center text-white font-bold text-sm shadow-sm">
              SP
            </div>
            <div>
              <div class="flex items-center space-x-2">
                <span class="font-bold text-base tracking-tight">Sprobe</span>
                <span class="rounded bg-brand-50 px-1.5 py-0.2 text-[10px] font-semibold text-brand-700 dark:bg-brand-950 dark:text-brand-300">
                  Console 1.0
                </span>
              </div>
            </div>
          </router-link>

          <!-- 主导航链接 -->
          <nav class="hidden md:flex items-center space-x-1 text-xs font-medium">
            <router-link
              to="/overview"
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-colors"
              :class="route.path.startsWith('/overview') || route.path.startsWith('/nodes')
                ? 'bg-brand-50 text-brand-700 font-semibold dark:bg-brand-950 dark:text-brand-300'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-100 dark:hover:bg-slate-800'"
            >
              <Activity class="h-3.5 w-3.5" />
              <span>集群总览</span>
            </router-link>

            <router-link
              to="/settings"
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-md transition-colors"
              :class="route.path === '/settings'
                ? 'bg-brand-50 text-brand-700 font-semibold dark:bg-brand-950 dark:text-brand-300'
                : 'text-slate-600 hover:text-slate-900 hover:bg-slate-100 dark:text-slate-400 dark:hover:text-slate-100 dark:hover:bg-slate-800'"
            >
              <Settings class="h-3.5 w-3.5" />
              <span>系统设置</span>
            </router-link>
          </nav>
        </div>

        <div class="flex items-center space-x-4">
          <!-- 移动端简单菜单 -->
          <div class="flex md:hidden items-center space-x-1 text-xs">
            <router-link
              to="/overview"
              class="p-1.5 rounded-md text-slate-600 hover:bg-slate-100 dark:text-slate-300"
              title="集群总览"
            >
              <Activity class="h-4 w-4" />
            </router-link>
            <router-link
              to="/settings"
              class="p-1.5 rounded-md text-slate-600 hover:bg-slate-100 dark:text-slate-300"
              title="系统设置"
            >
              <Settings class="h-4 w-4" />
            </router-link>
          </div>

          <!-- 安全模式身份状态胶囊 -->
          <div class="flex items-center">
            <!-- 已解锁管理员模式 -->
            <div
              v-if="isAdmin"
              class="flex items-center gap-1.5 rounded-full bg-emerald-50 py-1 pl-2.5 pr-1.5 text-xs font-medium text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800 shadow-xs"
            >
              <span class="inline-block w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
              <span class="hidden sm:inline">管理员模式</span>
              <button
                @click="logoutAdmin"
                class="ml-1 rounded-full p-1 hover:bg-emerald-100 dark:hover:bg-emerald-900 text-emerald-600 dark:text-emerald-400 transition-colors"
                title="退出管理模式 / 重新锁定"
              >
                <Lock class="h-3.5 w-3.5" />
              </button>
            </div>

            <!-- 访客演示模式 -->
            <button
              v-else
              @click="checkAdmin(undefined, '您正在以访客只读模式浏览。输入管理员密码即可解锁增删改与配置权限。')"
              class="group flex items-center gap-1.5 rounded-full bg-slate-100 py-1 px-2.5 text-xs font-medium text-slate-600 hover:bg-amber-50 hover:text-amber-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-amber-950/50 dark:hover:text-amber-300 border border-slate-200 dark:border-slate-700 hover:border-amber-300 dark:hover:border-amber-800 transition-all cursor-pointer"
              title="当前为访客演示只读模式，点击输入密码解锁管理员权限"
            >
              <Lock class="h-3.5 w-3.5 text-slate-400 group-hover:text-amber-500 transition-colors" />
              <span class="hidden sm:inline">演示访客 (只读)</span>
              <span class="text-[11px] underline text-slate-400 group-hover:text-amber-600 dark:group-hover:text-amber-400">解锁</span>
            </button>
          </div>

          <button
            @click="toggleDark"
            class="rounded-md p-1.5 text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
            title="切换主题"
          >
            <Sun v-if="isDark" class="h-4 w-4" />
            <Moon v-else class="h-4 w-4" />
          </button>
        </div>
      </div>
    </header>

    <!-- 主体内容 -->
    <main class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 flex-1 w-full">
      <router-view />
    </main>

    <!-- 底部版权信息 -->
    <footer class="border-t border-slate-200 py-4 text-center text-xs text-slate-400 dark:border-slate-800">
      <span>Sprobe 1.0 Server Probe & Console • 基于 Rust + Vue 3 极简高性能架构</span>
    </footer>

    <!-- 全局管理员安全验证弹窗 -->
    <AuthModal />
  </div>
</template>
