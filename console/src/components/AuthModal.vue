<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import {
  isAuthModalOpen,
  authModalMessage,
  verifyAndLogin,
  DEFAULT_ADMIN_PASSWORD,
} from '../utils/auth'
import { Lock, Eye, EyeOff, ShieldAlert, CheckCircle2, X } from 'lucide-vue-next'

const passwordInput = ref('')
const showPassword = ref(false)
const errorMessage = ref('')
const isSubmitting = ref(false)
const inputRef = ref<HTMLInputElement | null>(null)

watch(isAuthModalOpen, (open) => {
  if (open) {
    passwordInput.value = ''
    errorMessage.value = ''
    showPassword.value = false
    nextTick(() => {
      inputRef.value?.focus()
    })
  }
})

function closeModal() {
  isAuthModalOpen.value = false
}

function fillDemoPassword() {
  passwordInput.value = DEFAULT_ADMIN_PASSWORD
  errorMessage.value = ''
  nextTick(() => {
    inputRef.value?.focus()
  })
}

async function handleUnlock() {
  if (!passwordInput.value.trim()) {
    errorMessage.value = '请输入管理员密码'
    return
  }

  isSubmitting.value = true
  errorMessage.value = ''

  try {
    const success = await verifyAndLogin(passwordInput.value)
    if (!success) {
      errorMessage.value = '密码错误，请重新输入（默认口令: sprobe2026）'
    }
  } catch (err: any) {
    errorMessage.value = err.message || '验证失败'
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <div
    v-if="isAuthModalOpen"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm transition-opacity"
    @click.self="closeModal"
    @keydown.esc="closeModal"
  >
    <div
      class="relative w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl dark:bg-slate-900 border border-slate-200 dark:border-slate-800 transition-all transform animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- 关闭按钮 -->
      <button
        @click="closeModal"
        class="absolute top-4 right-4 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors"
      >
        <X class="h-5 w-5" />
      </button>

      <!-- 头部图标与标题 -->
      <div class="flex items-start gap-4">
        <div class="w-12 h-12 rounded-xl bg-amber-50 dark:bg-amber-950/60 border border-amber-200 dark:border-amber-900 flex items-center justify-center text-amber-600 dark:text-amber-400 shrink-0">
          <Lock class="h-6 w-6" />
        </div>
        <div class="space-y-1">
          <h3 class="text-base font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            安全验证 • 解锁管理员权限
          </h3>
          <p class="text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
            {{ authModalMessage }}
          </p>
        </div>
      </div>

      <!-- 密码输入表单 -->
      <form @submit.prevent="handleUnlock" class="mt-6 space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
            管理员密码 / Admin Password
          </label>
          <div class="relative">
            <input
              ref="inputRef"
              :type="showPassword ? 'text' : 'password'"
              v-model="passwordInput"
              placeholder="请输入管理密码"
              autocomplete="current-password"
              class="w-full rounded-lg border border-slate-300 bg-white px-3.5 py-2.5 text-sm text-slate-900 placeholder-slate-400 focus:border-brand-500 focus:outline-none focus:ring-2 focus:ring-brand-500/20 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100 dark:placeholder-slate-500"
              :class="{ 'border-rose-500 dark:border-rose-500 ring-2 ring-rose-500/20': errorMessage }"
            />
            <button
              type="button"
              @click="showPassword = !showPassword"
              class="absolute inset-y-0 right-0 flex items-center px-3 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
              tabindex="-1"
            >
              <EyeOff v-if="showPassword" class="h-4 w-4" />
              <Eye v-else class="h-4 w-4" />
            </button>
          </div>
          
          <!-- 错误消息 -->
          <div v-if="errorMessage" class="mt-2 flex items-center gap-1.5 text-xs text-rose-600 dark:text-rose-400">
            <ShieldAlert class="h-4 w-4 shrink-0" />
            <span>{{ errorMessage }}</span>
          </div>
        </div>

        <!-- 演示提示一键填入 -->
        <div class="rounded-lg bg-slate-50 p-3 dark:bg-slate-800/60 border border-slate-200 dark:border-slate-800 flex items-center justify-between text-xs">
          <div class="text-slate-600 dark:text-slate-400">
            <span>默认演示口令: </span>
            <code class="font-mono font-semibold text-brand-600 dark:text-brand-400 bg-brand-50 dark:bg-brand-950/60 px-1 py-0.5 rounded">
              {{ DEFAULT_ADMIN_PASSWORD }}
            </code>
          </div>
          <button
            type="button"
            @click="fillDemoPassword"
            class="text-xs font-semibold text-brand-600 dark:text-brand-400 hover:underline hover:text-brand-700 cursor-pointer"
          >
            一键填入
          </button>
        </div>

        <!-- 操作按钮 -->
        <div class="flex items-center justify-end gap-3 pt-2">
          <button
            type="button"
            @click="closeModal"
            class="rounded-lg border border-slate-300 px-4 py-2 text-xs font-semibold text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-300 dark:hover:bg-slate-800 transition-colors"
          >
            取消
          </button>
          <button
            type="submit"
            :disabled="isSubmitting"
            class="inline-flex items-center gap-1.5 rounded-lg bg-brand-600 px-5 py-2 text-xs font-semibold text-white shadow-sm hover:bg-brand-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-brand-600 disabled:opacity-50 transition-colors"
          >
            <CheckCircle2 class="h-4 w-4" />
            <span>{{ isSubmitting ? '验证中...' : '解锁管理员操作' }}</span>
          </button>
        </div>
      </form>
    </div>
  </div>
</template>
