<script setup lang="ts">
import { ref, provide, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AppIcons from './components/AppIcons.vue'
import AppSidebar from './components/layout/AppSidebar.vue'
import AppTopbar from './components/layout/AppTopbar.vue'
import AppStatusbar from './components/layout/AppStatusbar.vue'
import ConfigEditorModal from './components/modals/ConfigEditorModal.vue'
import PortCheckModal from './components/modals/PortCheckModal.vue'
import DashboardPage from './pages/DashboardPage.vue'
import SitesPage from './pages/SitesPage.vue'
import DatabasePage from './pages/DatabasePage.vue'
import SoftwarePage from './pages/SoftwarePage.vue'
import FilesPage from './pages/FilesPage.vue'
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
const { theme } = useTheme()
const { toasts } = useToast()
const serviceStore = useServiceStore()
const siteStore = useSiteStore()
const databaseStore = useDatabaseStore()
const settingsStore = useSettingsStore()

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

// Global port check state
const portCheckVisible = ref(false)

function openPortCheck() {
  portCheckVisible.value = true
}

provide('openPortCheck', openPortCheck)

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

let refreshTimer: number | null = null
let resourceTimer: number | null = null

onMounted(async () => {
  await refreshAll()
  refreshTimer = window.setInterval(() => {
    serviceStore.fetchState()
  }, 10000)
  resourceTimer = window.setInterval(fetchResource, 5000)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
  if (resourceTimer) clearInterval(resourceTimer)
})

function toastIcon(type: Toast['type']) {
  if (type === 'error' || type === 'warning') return 'alert'
  if (type === 'info') return 'info'
  return 'check'
}
</script>

<template>
  <AppIcons />
  <div class="app-window" id="appWindow">
    <AppSidebar />
    <main class="workspace">
      <AppTopbar />
      <section class="content" id="pageContent">
        <DashboardPage
          v-if="currentPage === 'dashboard'"
          @open-create-site="currentPage = 'sites'"
          @show-logs="currentPage = 'logs'"
          @show-ports="openPortCheck"
        />
        <SitesPage v-else-if="currentPage === 'sites'" />
        <DatabasePage v-else-if="currentPage === 'database'" />
        <SoftwarePage v-else-if="currentPage === 'software'" />
        <FilesPage v-else-if="currentPage === 'files'" />
        <LogsPage v-else-if="currentPage === 'logs'" />
        <SettingsPage v-else-if="currentPage === 'settings'" />
      </section>
      <AppStatusbar />
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

  <PortCheckModal
    :visible="portCheckVisible"
    @close="portCheckVisible = false"
  />
</template>

<style>
@import './styles/main.css';
</style>
