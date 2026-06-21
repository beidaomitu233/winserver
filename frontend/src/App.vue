<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import AppIcons from './components/AppIcons.vue'
import InitSplash from './components/InitSplash.vue'
import AppSidebar from './components/layout/AppSidebar.vue'
import AppStatusbar from './components/layout/AppStatusbar.vue'
import ConfigEditorModal from './components/modals/ConfigEditorModal.vue'
import ServiceConfigModal from './components/modals/ServiceConfigModal.vue'
import PortCheckModal from './components/modals/PortCheckModal.vue'
import DashboardPage from './pages/DashboardPage.vue'
import SitesPage from './pages/SitesPage.vue'
import DatabasePage from './pages/DatabasePage.vue'
import SoftwarePage from './pages/SoftwarePage.vue'
import LogsPage from './pages/LogsPage.vue'
import SettingsPage from './pages/SettingsPage.vue'
import { useServiceStore } from './stores/useServiceStore'
import { useSiteStore } from './stores/useSiteStore'
import { useDatabaseStore } from './stores/useDatabaseStore'
import { useSettingsStore } from './stores/useSettingsStore'
import { useTheme } from './composables/useTheme'
import { useToast, type Toast } from './composables/useToast'
import type { SystemResource } from './types'

const currentPage = ref('dashboard')
const createSiteSignal = ref(0)
const { theme } = useTheme()
const { toasts } = useToast()
const tauriWindow = getCurrentWindow()
const serviceStore = useServiceStore()
const siteStore = useSiteStore()
const databaseStore = useDatabaseStore()
const settingsStore = useSettingsStore()

// App initialization status — the backend runs local-service detection and
// bundled runtime install on a background task. We show an InitSplash until it
// reports ready, so the window stays responsive instead of freezing.
const initPhase = ref('pending')
const appReady = ref(false)
let initTimer: number | null = null

const systemResource = ref<SystemResource>({
  cpu_percent: 0,
  cpu_count: 0,
  cpu_model: '',
  memory_percent: 0,
  total_memory_mb: 0,
  used_memory_mb: 0,
  disk: { percent: 0, used_gb: 0, total_gb: 0 },
  uptime_seconds: 0,
})

provide('currentPage', currentPage)
provide('theme', theme)
provide('systemResource', systemResource)

// Global config editor state
const configEditorVisible = ref(false)
const configEditorFileId = ref('')
const configEditorLabel = ref('')
const configEditorFilePath = ref('')

function openConfigEditor(fileId: string, label: string, filePath: string) {
  configEditorFileId.value = fileId
  configEditorLabel.value = label
  configEditorFilePath.value = filePath
  configEditorVisible.value = true
}

function closeConfigEditor() {
  configEditorVisible.value = false
}

provide('openConfigEditor', openConfigEditor)

// Global service config modal (dual-mode visual + file editor for redis/minio)
const serviceConfigVisible = ref(false)
const serviceConfigTarget = ref<{ id: string; name: string; config_file: string | null } | null>(null)

function openServiceConfigModal(svc: { id: string; name: string; config_file: string | null }) {
  serviceConfigTarget.value = svc
  serviceConfigVisible.value = true
}

function closeServiceConfig() {
  serviceConfigVisible.value = false
}

provide('openServiceConfigModal', openServiceConfigModal)

// Global port check state
const portCheckVisible = ref(false)

function openPortCheck() {
  portCheckVisible.value = true
}

provide('openPortCheck', openPortCheck)

function openCreateSite() {
  currentPage.value = 'sites'
  createSiteSignal.value += 1
}

async function fetchResource() {
  try {
    const data = await invoke<SystemResource>('get_system_resource')
    systemResource.value = data
  } catch {
    // silent
  }
}

async function refreshAll() {
  await Promise.all([
    serviceStore.fetchState(),
    siteStore.fetchState(),
    databaseStore.fetchState(),
    settingsStore.fetchSettings(),
    fetchResource(),
  ])
}

provide('refreshAll', refreshAll)

function toggleTheme() {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
}

async function minimizeWindow() {
  try {
    await tauriWindow.minimize()
  } catch {
    // Browser preview has no Tauri window; ignore.
  }
}

async function toggleMaximizeWindow() {
  try {
    if (await tauriWindow.isMaximized()) {
      await tauriWindow.unmaximize()
    } else {
      await tauriWindow.maximize()
    }
  } catch {
    // Browser preview has no Tauri window; ignore.
  }
}

async function closeWindow() {
  try {
    await tauriWindow.close()
  } catch {
    // Browser preview has no Tauri window; ignore.
  }
}

let refreshTimer: number | null = null
let resourceTimer: number | null = null

onMounted(async () => {
  // Poll the backend init status until the background auto-setup completes.
  initTimer = window.setInterval(async () => {
    try {
      const status = await invoke<{ phase: string; ready: boolean }>('app_init_status')
      initPhase.value = status.phase || 'pending'
      if (status.ready) {
        appReady.value = true
        if (initTimer) {
          clearInterval(initTimer)
          initTimer = null
        }
        await refreshAll()
      }
    } catch {
      // Command may briefly fail before state is managed; retry on next tick.
    }
  }, 300)

  // Best-effort early refresh so the UI can populate as soon as it renders.
  await refreshAll()
  refreshTimer = window.setInterval(() => {
    serviceStore.fetchState()
  }, 10000)
  resourceTimer = window.setInterval(fetchResource, 5000)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
  if (resourceTimer) clearInterval(resourceTimer)
  if (initTimer) clearInterval(initTimer)
})

function toastIcon(type: Toast['type']) {
  if (type === 'error' || type === 'warning') return 'alert'
  if (type === 'info') return 'info'
  return 'check'
}
</script>

<template>
  <AppIcons />
  <InitSplash v-if="!appReady" :phase="initPhase" :ready="appReady" />
  <div class="app-window" id="appWindow" v-else>
    <AppSidebar />
    <main class="workspace">
      <div class="window-chrome">
        <div class="window-drag-region"></div>
        <div class="chrome-actions">
          <button class="icon-btn" title="刷新状态" @click="refreshAll">
            <svg class="icon"><use href="#i-refresh"></use></svg>
          </button>
          <button class="icon-btn" title="切换主题" @click="toggleTheme">
            <svg class="icon"><use :href="theme === 'dark' ? '#i-sun' : '#i-moon'"></use></svg>
          </button>
        </div>
        <div class="window-controls" aria-label="窗口控制">
          <button class="window-btn" title="最小化" @click="minimizeWindow"><span class="window-glyph minimize"></span></button>
          <button class="window-btn" title="最大化/还原" @click="toggleMaximizeWindow"><span class="window-glyph maximize"></span></button>
          <button class="window-btn close" title="关闭" @click="closeWindow"><span class="window-glyph close"></span></button>
        </div>
      </div>
      <section class="content" id="pageContent">
        <DashboardPage
          v-if="currentPage === 'dashboard'"
          @open-create-site="openCreateSite"
          @show-logs="currentPage = 'logs'"
          @show-ports="openPortCheck"
        />
        <SitesPage v-else-if="currentPage === 'sites'" :create-signal="createSiteSignal" />
        <DatabasePage v-else-if="currentPage === 'database'" />
        <SoftwarePage v-else-if="currentPage === 'software'" />
        <LogsPage v-else-if="currentPage === 'logs'" />
        <SettingsPage v-else-if="currentPage === 'settings'" />
      </section>
      <AppStatusbar v-if="currentPage !== 'dashboard'" />
    </main>
  </div>

  <div class="toast-stack" id="toastStack">
    <div v-for="toast in toasts" :key="toast.id" class="toast" :class="toast.type">
      <div class="toast-icon">
        <svg class="icon icon-sm"><use :href="`#i-${toastIcon(toast.type)}`" /></svg>
      </div>
      <div>
        <div class="toast-title">{{ toast.title }}</div>
        <div v-if="toast.message" class="toast-message">{{ toast.message }}</div>
      </div>
    </div>
  </div>

  <ConfigEditorModal
    :visible="configEditorVisible"
    :fileId="configEditorFileId"
    :label="configEditorLabel"
    :filePath="configEditorFilePath"
    @close="closeConfigEditor"
  />

  <ServiceConfigModal
    :visible="serviceConfigVisible"
    :service="serviceConfigTarget"
    @close="closeServiceConfig"
  />

  <PortCheckModal
    :visible="portCheckVisible"
    @close="portCheckVisible = false"
  />
</template>

<style>
@import './styles/main.css';
</style>
