import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SystemSettings } from '../types'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<SystemSettings>({
    autostart: false,
    start_suite_on_launch: false,
    php_my_admin_url: 'http://127.0.0.1/phpmyadmin',
    port: 18113,
    data_dir: '',
    config_path: '',
  })

  async function fetchSettings() {
    try {
      settings.value = await invoke<SystemSettings>('get_settings')
    } catch (e) {
      console.error('Failed to fetch settings:', e)
    }
  }

  async function updateSettings(updates: Partial<SystemSettings>) {
    await invoke('update_settings', { params: JSON.stringify(updates) })
    await fetchSettings()
  }

  return { settings, fetchSettings, updateSettings }
})
