import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SiteInfo, AppState } from '../types'
import { useToast } from '../composables/useToast'

export const useSiteStore = defineStore('sites', () => {
  const sites = ref<SiteInfo[]>([])
  const { show } = useToast()

  function readableError(error: unknown) {
    if (typeof error === 'string') return error
    if (error && typeof error === 'object' && 'message' in error) {
      return String((error as { message?: unknown }).message || '操作失败')
    }
    return '操作失败'
  }

  async function fetchState() {
    try {
      const state = await invoke<AppState>('get_state')
      sites.value = state.sites
    } catch (e) {
      console.error('Failed to fetch sites:', e)
    }
  }

  async function createSite(domain: string, port: number, path: string, server: string, phpRuntimeId?: string) {
    try {
      const result = await invoke<{ state: AppState }>('site_create', {
        domain,
        port,
        path,
        server,
        phpRuntimeId: phpRuntimeId || null,
      })
      if (result.state) sites.value = result.state.sites
      show('站点已创建', `http://127.0.0.1:${port} 已通过健康检查`, 'success')
    } catch (e) {
      show('创建站点失败', readableError(e), 'error')
      throw e
    }
  }

  async function deleteSite(siteId: string) {
    try {
      const result = await invoke<{ state: AppState }>('site_delete', { siteId })
      if (result.state) sites.value = result.state.sites
      show('站点已删除', '源代码目录已保留', 'success')
    } catch (e) {
      show('删除站点失败', readableError(e), 'error')
      throw e
    }
  }

  async function updateSite(siteId: string, domain: string, port: number, path: string, server: string) {
    const result = await invoke<{ state: AppState }>('site_update', { siteId, domain, port, path, server })
    if (result.state) sites.value = result.state.sites
  }

  async function switchPhp(siteId: string, phpRuntimeId: string) {
    try {
      const result = await invoke<{ state: AppState }>('site_switch_php', { siteId, phpRuntimeId })
      if (result.state) sites.value = result.state.sites
      show('PHP 已切换', '站点已通过健康检查', 'success')
    } catch (e) {
      show('切换 PHP 失败', readableError(e), 'error')
      throw e
    }
  }

  async function enableSite(siteId: string) {
    try {
      const result = await invoke<{ state: AppState }>('site_enable', { siteId })
      if (result.state) sites.value = result.state.sites
      show('站点已启用', '站点已重新上线', 'success')
    } catch (e) {
      show('启用失败', readableError(e), 'error')
      throw e
    }
  }

  async function disableSite(siteId: string) {
    try {
      const result = await invoke<{ state: AppState }>('site_disable', { siteId })
      if (result.state) sites.value = result.state.sites
      show('站点已停用', '站点已下线', 'success')
    } catch (e) {
      show('停用失败', readableError(e), 'error')
      throw e
    }
  }

  return { sites, fetchState, createSite, deleteSite, updateSite, switchPhp, enableSite, disableSite }
})
