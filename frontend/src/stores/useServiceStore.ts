import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ServiceInfo, AppState, ConfigFileInfo, SoftwareInfo } from '../types'
import { useToast } from '../composables/useToast'

interface SuiteSummaryItem {
  serviceId: string
  name: string
  status: 'success' | 'failed' | 'skipped'
  message: string
}

export const useServiceStore = defineStore('services', () => {
  const services = ref<ServiceInfo[]>([])
  const software = ref<SoftwareInfo[]>([])
  const configFiles = ref<ConfigFileInfo[]>([])
  const isLoading = ref(false)
  const serviceLoading = ref<Record<string, boolean>>({})
  const suiteLoading = ref(false)
  const { show } = useToast()

  const runningCount = computed(() => services.value.filter(s => s.state === 'running').length)
  const totalCount = computed(() => services.value.length)
  const autoServices = computed(() => services.value.filter(s => s.auto))

  function serviceName(serviceId: string) {
    return services.value.find(s => s.id === serviceId)?.name || serviceId
  }

  function readableError(error: unknown) {
    if (typeof error === 'string') return error
    if (error && typeof error === 'object' && 'message' in error) {
      return String((error as { message?: unknown }).message || '操作失败')
    }
    return '操作失败'
  }

  function setServiceLoading(serviceId: string, value: boolean) {
    serviceLoading.value = { ...serviceLoading.value, [serviceId]: value }
  }

  function isServiceBusy(serviceId: string) {
    return Boolean(serviceLoading.value[serviceId])
  }

  function suiteSummaryText(summary: SuiteSummaryItem[] | undefined) {
    if (!summary?.length) return '没有符合条件的自动服务'
    const success = summary.filter(item => item.status === 'success').length
    const failed = summary.filter(item => item.status === 'failed').length
    const skipped = summary.filter(item => item.status === 'skipped').length
    return `成功 ${success} 项，失败 ${failed} 项，跳过 ${skipped} 项`
  }

  async function fetchState() {
    isLoading.value = true
    try {
      const state = await invoke<AppState>('get_state')
      services.value = state.services
      software.value = state.software || []
      configFiles.value = state.config_files || []
    } catch (e) {
      console.error('Failed to fetch state:', e)
    } finally {
      isLoading.value = false
    }
  }

  async function startService(serviceId: string) {
    setServiceLoading(serviceId, true)
    try {
      const result = await invoke<{ state: AppState }>('service_start', { serviceId })
      if (result.state) services.value = result.state.services
      show('启动成功', `${serviceName(serviceId)} 已通过真实检测`, 'success')
    } catch (e) {
      await fetchState()
      show('启动失败', readableError(e), 'error')
      throw e
    } finally {
      setServiceLoading(serviceId, false)
    }
  }

  async function stopService(serviceId: string) {
    setServiceLoading(serviceId, true)
    try {
      const result = await invoke<{ state: AppState }>('service_stop', { serviceId })
      if (result.state) services.value = result.state.services
      show('停止成功', `${serviceName(serviceId)} 已停止`, 'success')
    } catch (e) {
      await fetchState()
      show('停止失败', readableError(e), 'error')
      throw e
    } finally {
      setServiceLoading(serviceId, false)
    }
  }

  async function restartService(serviceId: string) {
    setServiceLoading(serviceId, true)
    try {
      const result = await invoke<{ state: AppState }>('service_restart', { serviceId })
      if (result.state) services.value = result.state.services
      show('重启成功', `${serviceName(serviceId)} 已通过真实检测`, 'success')
    } catch (e) {
      await fetchState()
      show('重启失败', readableError(e), 'error')
      throw e
    } finally {
      setServiceLoading(serviceId, false)
    }
  }

  async function startAll() {
    suiteLoading.value = true
    try {
      const result = await invoke<{ state: AppState, summary?: SuiteSummaryItem[] }>('suite_start')
      if (result.state) services.value = result.state.services
      const failed = result.summary?.some(item => item.status === 'failed')
      show(failed ? '启动完成但有失败项' : '启动完成', suiteSummaryText(result.summary), failed ? 'warning' : 'success')
    } catch (e) {
      console.error('Failed to start suite:', e)
      show('启动全部失败', readableError(e), 'error')
      throw e
    } finally {
      suiteLoading.value = false
    }
  }

  async function stopAll() {
    suiteLoading.value = true
    try {
      const result = await invoke<{ state: AppState, summary?: SuiteSummaryItem[] }>('suite_stop')
      if (result.state) services.value = result.state.services
      const failed = result.summary?.some(item => item.status === 'failed')
      show(failed ? '停止完成但有失败项' : '停止完成', suiteSummaryText(result.summary), failed ? 'warning' : 'success')
    } catch (e) {
      console.error('Failed to stop suite:', e)
      show('停止全部失败', readableError(e), 'error')
      throw e
    } finally {
      suiteLoading.value = false
    }
  }

  async function toggleAuto(serviceId: string, auto: boolean) {
    setServiceLoading(serviceId, true)
    try {
      const result = await invoke<{ state: AppState }>('service_toggle_auto', { serviceId, auto })
      if (result.state) services.value = result.state.services
    } catch (e) {
      console.error('Failed to toggle auto:', e)
      show('启动项保存失败', readableError(e), 'error')
      throw e
    } finally {
      setServiceLoading(serviceId, false)
    }
  }

  return {
    services, software, configFiles, isLoading, serviceLoading, suiteLoading, runningCount, totalCount, autoServices,
    isServiceBusy,
    fetchState, startService, stopService, restartService, startAll, stopAll, toggleAuto,
  }
})
