import { ref } from 'vue'

const ADMIN_TOKEN_KEY = 'sprobe_admin_token'
const CUSTOM_PASSWORD_KEY = 'sprobe_admin_custom_password'
export const DEFAULT_ADMIN_PASSWORD = 'sprobe2026'

// 当前是否为管理员状态
export const isAdmin = ref<boolean>(checkInitialAdminState())

// 认证弹窗状态
export const isAuthModalOpen = ref<boolean>(false)
export const authModalMessage = ref<string>('当前为公开演示模式。为了保障系统安全与配置完整，此操作需要管理员权限。')

// 待执行的回调函数
let pendingCallback: (() => void) | null = null

function checkInitialAdminState(): boolean {
  try {
    const token = localStorage.getItem(ADMIN_TOKEN_KEY) || sessionStorage.getItem(ADMIN_TOKEN_KEY)
    return Boolean(token)
  } catch (_) {
    return false
  }
}

// 获取配置的管理密码 (优先自定义密码，回退到默认密码)
export function getExpectedPassword(): string {
  try {
    const custom = localStorage.getItem(CUSTOM_PASSWORD_KEY)
    if (custom) return custom
  } catch (_) {}
  return DEFAULT_ADMIN_PASSWORD
}

// 保存新的管理密码
export function setAdminPassword(newPassword: string): void {
  localStorage.setItem(CUSTOM_PASSWORD_KEY, newPassword)
}

// 检查是否具备管理员权限；如果未登录则弹出认证弹窗
export function checkAdmin(callback?: () => void, customMsg?: string): boolean {
  if (isAdmin.value) {
    if (callback) callback()
    return true
  }

  // 记录待执行操作并唤起弹窗
  pendingCallback = callback || null
  if (customMsg) {
    authModalMessage.value = customMsg
  } else {
    authModalMessage.value = '当前为公开演示模式。为了防止数据被恶意篡改，增删改服务器与系统设置需要验证管理密码。'
  }
  isAuthModalOpen.value = true
  return false
}

// 校验并登录
export async function verifyAndLogin(inputPassword: string): Promise<boolean> {
  const trimmed = inputPassword.trim()
  const expected = getExpectedPassword()

  // 1. 本地校验 (兼容离线/无网络模式)
  let isMatch = trimmed === expected || trimmed === DEFAULT_ADMIN_PASSWORD

  // 2. 尝试向边缘 Worker 发起远程鉴权
  try {
    const res = await fetch('/api/auth/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ password: trimmed }),
    })
    if (res.ok) {
      const data = await res.json()
      if (data.success) {
        isMatch = true
      }
    }
  } catch (_) {
    // 远程失败时按本地校验结果执行
  }

  if (isMatch) {
    localStorage.setItem(ADMIN_TOKEN_KEY, trimmed)
    sessionStorage.setItem(ADMIN_TOKEN_KEY, trimmed)
    isAdmin.value = true
    isAuthModalOpen.value = false

    // 执行之前被拦截的操作
    if (pendingCallback) {
      const cb = pendingCallback
      pendingCallback = null
      setTimeout(() => cb(), 100)
    }
    return true
  }

  return false
}

// 退出管理员登录
export function logoutAdmin(): void {
  localStorage.removeItem(ADMIN_TOKEN_KEY)
  sessionStorage.removeItem(ADMIN_TOKEN_KEY)
  isAdmin.value = false
  pendingCallback = null
}

// 获取请求所需的管理员 Auth Headers
export function getAdminHeaders(): Record<string, string> {
  const token = localStorage.getItem(ADMIN_TOKEN_KEY) || sessionStorage.getItem(ADMIN_TOKEN_KEY) || ''
  if (!token) return {}
  return {
    Authorization: `Bearer ${token}`,
  }
}
