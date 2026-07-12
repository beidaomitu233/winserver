import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater'

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'latest'
  | 'available'
  | 'downloading'
  | 'installing'
  | 'error'

export const useUpdateStore = defineStore('app-update', () => {
  const currentVersion = ref('')
  const availableVersion = ref('')
  const releaseNotes = ref('')
  const releaseDate = ref('')
  const status = ref<UpdateStatus>('idle')
  const errorMessage = ref('')
  const progress = ref(0)
  const initialized = ref(false)

  let pendingUpdate: Update | null = null
  let contentLength = 0
  let downloaded = 0
  let activeCheck: Promise<void> | null = null

  const isBusy = computed(() =>
    status.value === 'checking' || status.value === 'downloading' || status.value === 'installing',
  )

  const statusText = computed(() => {
    switch (status.value) {
      case 'checking': return '正在检查更新…'
      case 'latest': return '当前已是最新版本'
      case 'available': return `发现新版本 v${availableVersion.value}`
      case 'downloading': return `正在下载更新 ${progress.value}%`
      case 'installing': return '下载完成，正在安装并准备重启…'
      case 'error': return errorMessage.value || '检查更新失败'
      default: return '可随时检查新版本'
    }
  })

  function readableError(error: unknown) {
    if (typeof error === 'string') return error
    if (error && typeof error === 'object' && 'message' in error) {
      return String((error as { message?: unknown }).message || '未知错误')
    }
    return String(error || '未知错误')
  }

  async function loadVersion() {
    if (currentVersion.value) return
    currentVersion.value = await getVersion()
  }

  async function initialize() {
    if (initialized.value) return
    initialized.value = true
    try {
      await loadVersion()
      await checkForUpdates()
    } catch (error) {
      status.value = 'error'
      errorMessage.value = readableError(error)
    }
  }

  async function checkForUpdates() {
    if (activeCheck) return activeCheck
    if (status.value === 'downloading' || status.value === 'installing') return

    activeCheck = (async () => {
      status.value = 'checking'
      errorMessage.value = ''
      progress.value = 0

      try {
        await loadVersion()
        const update = await check({ timeout: 15_000 })
        if (pendingUpdate && pendingUpdate !== update) {
          await pendingUpdate.close().catch(() => undefined)
        }
        pendingUpdate = update

        if (!update) {
          availableVersion.value = ''
          releaseNotes.value = ''
          releaseDate.value = ''
          status.value = 'latest'
          return
        }

        availableVersion.value = update.version
        releaseNotes.value = update.body || ''
        releaseDate.value = update.date || ''
        status.value = 'available'
      } catch (error) {
        if (pendingUpdate) {
          await pendingUpdate.close().catch(() => undefined)
        }
        pendingUpdate = null
        status.value = 'error'
        errorMessage.value = readableError(error)
      }
    })().finally(() => {
      activeCheck = null
    })

    return activeCheck
  }

  function handleDownloadEvent(event: DownloadEvent) {
    if (event.event === 'Started') {
      contentLength = event.data.contentLength || 0
      downloaded = 0
      progress.value = contentLength > 0 ? 0 : 5
      return
    }
    if (event.event === 'Progress') {
      downloaded += event.data.chunkLength
      progress.value = contentLength > 0
        ? Math.min(99, Math.round((downloaded / contentLength) * 100))
        : Math.min(95, progress.value + 1)
      return
    }
    progress.value = 100
    status.value = 'installing'
  }

  async function installUpdate() {
    if (!pendingUpdate || status.value !== 'available') return

    status.value = 'downloading'
    errorMessage.value = ''
    progress.value = 0
    try {
      await pendingUpdate.downloadAndInstall(handleDownloadEvent, { timeout: 10 * 60_000 })
      status.value = 'installing'
      progress.value = 100
      await relaunch()
    } catch (error) {
      status.value = 'error'
      errorMessage.value = readableError(error)
    }
  }

  return {
    currentVersion,
    availableVersion,
    releaseNotes,
    releaseDate,
    status,
    statusText,
    errorMessage,
    progress,
    isBusy,
    initialize,
    checkForUpdates,
    installUpdate,
  }
})
