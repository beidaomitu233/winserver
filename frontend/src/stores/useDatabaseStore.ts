import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DatabaseInfo, AppState } from '../types'

export const useDatabaseStore = defineStore('databases', () => {
  const databases = ref<DatabaseInfo[]>([])

  async function fetchState() {
    try {
      const state = await invoke<AppState>('get_state')
      databases.value = state.databases
    } catch (e) {
      console.error('Failed to fetch databases:', e)
    }
  }

  async function createDatabase(db: string, user: string, pass: string) {
    const result = await invoke<{ state: AppState }>('db_create', { db, user, pass })
    if (result.state) databases.value = result.state.databases
  }

  async function deleteDatabase(dbName: string) {
    const result = await invoke<{ state: AppState }>('db_delete', { dbName })
    if (result.state) databases.value = result.state.databases
  }

  async function changePassword(dbName: string, user: string, pass: string) {
    await invoke('db_change_password', { dbName, user, pass })
    await fetchState()
  }

  async function rootPassword(currentPass: string, newPass: string) {
    await invoke('db_root_password', { currentPass, newPass })
    await fetchState()
  }

  async function exportDatabase(dbName: string, path?: string) {
    await invoke('db_export', { dbName, path: path || null })
  }

  async function importDatabase(dbName: string, path: string) {
    await invoke('db_import', { dbName, path })
  }

  async function getBackups() {
    return invoke('db_backups')
  }

  async function deleteBackup(path: string) {
    await invoke('db_delete_backup', { path })
  }

  return {
    databases,
    fetchState,
    createDatabase,
    deleteDatabase,
    changePassword,
    rootPassword,
    exportDatabase,
    importDatabase,
    getBackups,
    deleteBackup,
  }
})
