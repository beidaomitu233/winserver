<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useServiceStore } from '../stores/useServiceStore'
import { getServiceMeta } from '../types'
import type { AppState, RuntimeManifest, SoftwareInfo } from '../types'
import { useToast } from '../composables/useToast'

function serviceLogo(service: { id: string; name: string }) {
  const id = service.id
  const name = service.name
  const logos: Record<string, string> = {
    apache: `<svg viewBox="0 0 48 48" role="img" aria-label="Apache"><path d="M36.5 5.5C27 9.2 20.1 17.3 16.2 28.7c-1.5 4.4-3.1 8.4-5.7 13.8 4.9-3.7 8.6-7.6 11.3-12.1 5-8.4 8.9-15.1 14.7-24.9Z" fill="currentColor" opacity=".96"/><path d="M14.7 32.7c5.5-2.4 10.1-5.9 14.1-10.4M18.3 24.9l9.1 1.7M21.5 18.9l9 1" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" opacity=".92"/></svg>`,
    nginx: `<svg viewBox="0 0 48 48" role="img" aria-label="Nginx"><path d="M24 3.8 41.2 13.7v20.1L24 44.2 6.8 34.3V13.7Z" fill="currentColor"/><path d="M15.3 33V15l17.4 18V15" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    mysql: `<svg viewBox="0 0 48 48" role="img" aria-label="MySQL"><path d="M6.5 30.5c5.6-8.3 13-12.7 21.4-12.7 6.3 0 10.8 2.3 13.6 6.7-4.4-1.9-7.8-2.4-10.1-1.7 3.8 1.3 6.2 4.5 7 9.5-5.1-4-9.2-5.3-12.5-3.9-2.4 1-4.6 3.3-6.6 7-3.4-3.8-7.7-5.4-12.8-4.9Z" fill="currentColor" opacity=".95"/><path d="M28.4 16.8c1.4-3.8 3.8-6.3 7.1-7.5-.1 3.6-1 6.4-2.8 8.4" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><circle cx="33.6" cy="22.2" r="1.3" fill="#fff"/></svg>`,
    mysql57: `<svg viewBox="0 0 48 48" role="img" aria-label="MySQL"><path d="M6.5 30.5c5.6-8.3 13-12.7 21.4-12.7 6.3 0 10.8 2.3 13.6 6.7-4.4-1.9-7.8-2.4-10.1-1.7 3.8 1.3 6.2 4.5 7 9.5-5.1-4-9.2-5.3-12.5-3.9-2.4 1-4.6 3.3-6.6 7-3.4-3.8-7.7-5.4-12.8-4.9Z" fill="currentColor" opacity=".95"/><path d="M28.4 16.8c1.4-3.8 3.8-6.3 7.1-7.5-.1 3.6-1 6.4-2.8 8.4" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><circle cx="33.6" cy="22.2" r="1.3" fill="#fff"/></svg>`,
    mysql80: `<svg viewBox="0 0 48 48" role="img" aria-label="MySQL"><path d="M6.5 30.5c5.6-8.3 13-12.7 21.4-12.7 6.3 0 10.8 2.3 13.6 6.7-4.4-1.9-7.8-2.4-10.1-1.7 3.8 1.3 6.2 4.5 7 9.5-5.1-4-9.2-5.3-12.5-3.9-2.4 1-4.6 3.3-6.6 7-3.4-3.8-7.7-5.4-12.8-4.9Z" fill="currentColor" opacity=".95"/><path d="M28.4 16.8c1.4-3.8 3.8-6.3 7.1-7.5-.1 3.6-1 6.4-2.8 8.4" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><circle cx="33.6" cy="22.2" r="1.3" fill="#fff"/></svg>`,
    php: `<svg viewBox="0 0 48 48" role="img" aria-label="PHP"><ellipse cx="24" cy="24" rx="21" ry="12.8" fill="currentColor" opacity=".96"/><text x="24" y="28.2" text-anchor="middle" class="brand-word" font-size="12.2" fill="#fff">PHP</text></svg>`,
    php73: `<svg viewBox="0 0 48 48" role="img" aria-label="PHP"><ellipse cx="24" cy="24" rx="21" ry="12.8" fill="currentColor" opacity=".96"/><text x="24" y="28.2" text-anchor="middle" class="brand-word" font-size="12.2" fill="#fff">PHP</text></svg>`,
    redis: `<svg viewBox="0 0 48 48" role="img" aria-label="Redis"><path d="m24 7 18 8-18 8L6 15Z" fill="currentColor"/><path d="m6 22 18 8 18-8v7l-18 8-18-8Z" fill="currentColor" opacity=".86"/><path d="m6 32 18 8 18-8v6l-18 7-18-7Z" fill="currentColor" opacity=".68"/><path d="m17 14 7-3 7 3-7 3Z" fill="#fff" opacity=".9"/></svg>`,
    minio: `<svg viewBox="0 0 48 48" role="img" aria-label="MinIO"><rect x="8" y="8" width="32" height="32" rx="6" fill="currentColor" opacity=".18"/><path d="M16 16h16v16H16z" fill="currentColor" opacity=".8"/><path d="M20 20h8M20 24h8M20 28h4" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round"/></svg>`,
    pgsql: `<svg viewBox="0 0 48 48" role="img" aria-label="PostgreSQL"><path d="M24 6c-7 0-12 5-12 12 0 4 2 8 5 10l-1 6h16l-1-6c3-2 5-6 5-10 0-7-5-12-12-12Z" fill="currentColor" opacity=".9"/><path d="M18 22h12M20 26h8" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round"/></svg>`
  }
  return `<span class="service-brand service-brand-${id}" title="${name}">${logos[id] || `<svg viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="currentColor" opacity=".14"/><text x="24" y="28" text-anchor="middle" class="brand-word" font-size="13" fill="currentColor">${String(name).slice(0, 2).toUpperCase()}</text></svg>`}</span>`
}

const serviceStore = useServiceStore()
const { show } = useToast()

const activeTab = ref('all')
const searchQuery = ref('')
const showImportModal = ref(false)
const isImporting = ref(false)
const importForm = ref({
  runtimeType: 'nginx' as 'nginx' | 'php' | 'mysql',
  installPath: '',
  port: 80,
})

// Download state: maps softwareId -> { percent, phase }
const downloadState = ref<Record<string, { percent: number; phase: string }>>({})
let progressTimers: Record<string, ReturnType<typeof setInterval>> = {}

const categories: Array<[string, string]> = [
  ['all', '全部'],
  ['web', 'Web'],
  ['database', '数据库'],
  ['runtime', '运行环境'],
  ['tools', '工具'],
]

const packageMap: Record<string, string> = {
  apache: 'web',
  nginx: 'web',
  mysql57: 'database',
  mysql80: 'database',
  pgsql: 'database',
  php73: 'runtime',
  php: 'runtime',
  redis: 'database',
  minio: 'tools',
}

// Merge software info from state into service list for download_url/status
const softwareMap = computed(() => {
  const map: Record<string, SoftwareInfo> = {}
  for (const sw of serviceStore.software) {
    map[sw.id] = sw
  }
  return map
})

const HIDDEN_SOFTWARE = new Set(['apache', 'pgsql', 'mc'])

const softwareList = computed(() => {
  const items = serviceStore.services
    .filter(s => !HIDDEN_SOFTWARE.has(s.id))
    .filter(s => {
      if (activeTab.value === 'all') return true
      return packageMap[s.id] === activeTab.value
    })
    .filter(s => {
      const q = searchQuery.value.toLowerCase()
      if (!q) return true
      return s.name.toLowerCase().includes(q)
    })
  // Installed first, then alphabetical — keeps the list scannable.
  return [...items].sort((a, b) => {
    if (a.installed !== b.installed) return a.installed ? -1 : 1
    if (a.state === 'running' && b.state !== 'running') return -1
    if (b.state === 'running' && a.state !== 'running') return 1
    return a.name.localeCompare(b.name, 'zh-CN')
  })
})

function openImportModal() {
  importForm.value = { runtimeType: 'nginx', installPath: '', port: 80 }
  showImportModal.value = true
}

function openImportForService(serviceId: string) {
  let runtimeType: 'nginx' | 'php' | 'mysql' = 'nginx'
  let port = 80
  if (serviceId.startsWith('php')) {
    runtimeType = 'php'
    port = 9073
  } else if (serviceId.startsWith('mysql')) {
    runtimeType = 'mysql'
    port = 3306
  }
  importForm.value = { runtimeType, installPath: '', port }
  showImportModal.value = true
}

async function browsePath() {
  try {
    const selected = await open({ directory: true, title: '选择安装目录' })
    if (selected) {
      importForm.value.installPath = selected
    }
  } catch {
    // user cancelled
  }
}

function setImportType(type: 'nginx' | 'php' | 'mysql') {
  importForm.value.runtimeType = type as any
  if (type === 'nginx') importForm.value.port = 80
  else if (type === 'php') importForm.value.port = 9073
  else importForm.value.port = 3306
}

function readableError(error: unknown) {
  if (typeof error === 'string') return error
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message?: unknown }).message || '操作失败')
  }
  return '操作失败'
}

async function importRuntime() {
  const installPath = importForm.value.installPath.trim()
  if (!installPath) {
    show('导入失败', '请填写本地运行环境目录', 'warning')
    return
  }

  isImporting.value = true
  try {
    const result = await invoke<{ runtime: RuntimeManifest; state: AppState; message?: string }>('runtime_import', {
      runtimeType: importForm.value.runtimeType,
      installPath,
      port: Number(importForm.value.port),
    })
    if (result.state) {
      serviceStore.services = result.state.services
      if (result.state.software) serviceStore.software = result.state.software
    } else {
      await serviceStore.fetchState()
    }
    const label = result.runtime
      ? `${String(result.runtime.runtime_type).toUpperCase()} ${result.runtime.version}`
      : importForm.value.runtimeType.toUpperCase()
    show('导入成功', result.message || `${label} 已可用`, 'success')
    showImportModal.value = false
  } catch (e) {
    show('导入失败', readableError(e), 'error')
  } finally {
    isImporting.value = false
  }
}

async function toggleService(id: string) {
  const svc = serviceStore.services.find(s => s.id === id)
  if (!svc) return
  try {
    if (svc.state === 'running') {
      await serviceStore.stopService(id)
    } else {
      await serviceStore.startService(id)
    }
  } catch (e: any) {
    console.error('Toggle service failed:', e)
  }
}

async function uninstallSoftware(softwareId: string) {
  if (!confirm('确定要卸载此软件吗？')) return
  try {
    const result = await invoke<{ state: any }>('software_uninstall', { softwareId })
    if (result.state) {
      await serviceStore.fetchState()
    }
  } catch (e) {
    console.error('Uninstall software failed:', e)
    show('卸载失败', readableError(e), 'error')
  }
}

function isDownloading(softwareId: string): boolean {
  const ds = downloadState.value[softwareId]
  return ds != null && ds.phase !== 'idle' && ds.phase !== 'done' && ds.phase !== 'failed'
}

function getDownloadPercent(softwareId: string): number {
  return downloadState.value[softwareId]?.percent ?? 0
}

function getDownloadPhaseText(softwareId: string): string {
  const phase = downloadState.value[softwareId]?.phase ?? ''
  const pct = getDownloadPercent(softwareId)
  switch (phase) {
    case 'downloading':
      return pct > 0 ? '下载中' : '连接中'
    case 'installing': return '安装中'
    case 'done': return '完成'
    case 'failed': return '失败'
    default: return ''
  }
}

function startProgressPolling(softwareId: string) {
  if (progressTimers[softwareId]) return
  progressTimers[softwareId] = setInterval(async () => {
    try {
      const result = await invoke<{
        percent: number
        phase: string
        total?: number
        downloaded?: number
      }>('software_download_progress', { softwareId })
      // When Content-Length is missing, backend may report total=0 — keep a
      // soft indeterminate progress so the bar is not stuck at 0 forever.
      let percent = Number(result.percent) || 0
      if (result.phase === 'downloading' && percent <= 0 && (result.downloaded || 0) > 0) {
        const dl = Number(result.downloaded) || 0
        // log-scale soft progress up to 90% until total is known
        percent = Math.min(90, Math.max(5, Math.round(Math.log10(dl + 10) * 12)))
      }
      if (result.phase === 'installing' && percent < 92) percent = 92
      downloadState.value[softwareId] = { percent, phase: result.phase }
      if (result.phase === 'done' || result.phase === 'failed' || result.phase === 'idle') {
        stopProgressPolling(softwareId)
        if (result.phase === 'done') {
          downloadState.value[softwareId] = { percent: 100, phase: 'done' }
          await serviceStore.fetchState()
        }
      }
    } catch {
      stopProgressPolling(softwareId)
    }
  }, 400)
}

function stopProgressPolling(softwareId: string) {
  if (progressTimers[softwareId]) {
    clearInterval(progressTimers[softwareId])
    delete progressTimers[softwareId]
  }
}

/** One-click install: backend prefers bundled package, then catalog download. */
async function oneClickInstall(softwareId: string) {
  downloadState.value[softwareId] = { percent: 0, phase: 'installing' }
  startProgressPolling(softwareId)
  try {
    const result = await invoke<{ state: any; message: string }>('software_install', { softwareId })
    if (result.state) {
      await serviceStore.fetchState()
    } else {
      await serviceStore.fetchState()
    }
    show('安装成功', result.message || `${softwareId} 已安装并完成配置`, 'success')
    downloadState.value[softwareId] = { percent: 100, phase: 'done' }
  } catch (e) {
    // Fallbacks for older command shapes
    try {
      if (softwareMap.value[softwareId]?.has_bundled) {
        await invoke('software_install_bundled', { softwareId })
      } else if (softwareMap.value[softwareId]?.download_url) {
        await invoke('software_download_install', { softwareId })
      } else {
        throw e
      }
      await serviceStore.fetchState()
      show('安装成功', `${softwareId} 已安装并完成配置`, 'success')
      downloadState.value[softwareId] = { percent: 100, phase: 'done' }
    } catch (e2) {
      show('安装失败', readableError(e2), 'error')
      downloadState.value[softwareId] = { percent: 0, phase: 'failed' }
    }
  } finally {
    stopProgressPolling(softwareId)
  }
}

onMounted(() => {
  serviceStore.fetchState()
})

onUnmounted(() => {
  for (const id of Object.keys(progressTimers)) {
    stopProgressPolling(id)
  }
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">软件</h1>
      </div>
      <div class="page-actions">
        <button class="btn primary" @click="openImportModal">
          <svg class="icon icon-sm"><use href="#i-upload" /></svg>导入本地环境
        </button>
      </div>
    </div>

    <div class="card">
      <div class="tabs">
        <button
          v-for="cat in categories"
          :key="cat[0]"
          class="tab"
          :class="{ active: activeTab === cat[0] }"
          @click="activeTab = cat[0]"
        >
          {{ cat[1] }}
        </button>
        <div class="search-box" style="margin-left: auto">
          <svg class="icon icon-sm"><use href="#i-search" /></svg>
          <input v-model="searchQuery" class="input" placeholder="搜索" />
        </div>
      </div>

      <div class="software-list">
        <template v-if="softwareList.length > 0">
          <div
            v-for="svc in softwareList"
            :key="svc.id"
            class="software-row"
            :class="{ 'is-running': svc.state === 'running', 'is-installed': svc.installed }"
          >
            <div class="software-main">
              <div
                class="service-logo"
                :style="{ '--logo': getServiceMeta(svc.id).color }"
                v-html="serviceLogo({ id: svc.id, name: svc.name })"
              />
              <div class="software-meta">
                <div class="table-title">
                  {{ svc.name }}
                  <span v-if="svc.state === 'running'" class="run-pill">运行中</span>
                </div>
                <div class="subline">
                  <template v-if="svc.installed">
                    {{ svc.port ? `端口 ${svc.port}` : '已就绪' }}
                    <span v-if="svc.state === 'failed' && svc.error_message"> · {{ svc.error_message }}</span>
                  </template>
                  <template v-else>
                    {{ softwareMap[svc.id]?.has_local ? '本机已检测到，可导入' : '未安装' }}
                  </template>
                </div>
              </div>
            </div>
            <div class="software-actions">
              <template v-if="svc.installed">
                <button
                  class="btn small"
                  :class="svc.state !== 'running' ? 'primary' : ''"
                  @click="toggleService(svc.id)"
                >
                  {{ svc.state === 'running' ? '停止' : '启动' }}
                </button>
                <button class="btn small ghost" @click="uninstallSoftware(svc.id)">卸载</button>
              </template>
              <template v-else-if="isDownloading(svc.id)">
                <div class="download-progress">
                  <div class="progress-bar">
                    <div class="progress-fill" :style="{ width: getDownloadPercent(svc.id) + '%' }"></div>
                  </div>
                  <span class="progress-text">{{ getDownloadPhaseText(svc.id) }} {{ getDownloadPercent(svc.id) }}%</span>
                </div>
              </template>
              <template v-else>
                <button
                  class="btn small primary"
                  :disabled="isDownloading(svc.id)"
                  @click="oneClickInstall(svc.id)"
                >安装</button>
                <button class="btn small ghost" @click="openImportForService(svc.id)">导入</button>
              </template>
            </div>
          </div>
        </template>
        <div v-else style="padding: 24px; color: var(--text-3)">暂无软件</div>
      </div>
    </div>

    <Teleport to="body">
      <div class="overlay" :class="{ show: showImportModal }" @click.self="showImportModal = false">
        <div class="modal">
          <div class="modal-head">
            <div class="modal-title">导入本地运行环境</div>
            <button class="icon-btn" @click="showImportModal = false">
              <svg class="icon icon-sm"><use href="#i-stop" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-grid">
              <div class="form-row">
                <label class="form-label">类型</label>
                <select
                  class="select"
                  :value="importForm.runtimeType"
                  @change="setImportType(($event.target as HTMLSelectElement).value as 'nginx' | 'php' | 'mysql')"
                >
                  <option value="nginx">Nginx</option>
                  <option value="php">PHP</option>
                  <option value="mysql">MySQL</option>
                </select>
              </div>
              <div class="form-row">
                <label class="form-label">端口</label>
                <input v-model.number="importForm.port" class="input" type="number" min="1" max="65535" />
              </div>
              <div class="form-row full">
                <label class="form-label">安装目录</label>
                <div style="display: flex; gap: 8px;">
                  <input
                    v-model="importForm.installPath"
                    class="input"
                    style="flex: 1"
                    placeholder="例如 D:\runtime\nginx-1.26.3 或 D:\runtime\php-8.2.12"
                  />
                  <button class="btn" @click="browsePath" type="button">浏览</button>
                </div>
                <div class="form-hint">
                  Nginx：nginx.exe + conf\nginx.conf；PHP：php.exe / php-cgi.exe / php.ini；MySQL：bin\mysqld.exe（端口固定 3306，5.7/8.0 不同时运行）。
                </div>
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" @click="showImportModal = false">取消</button>
            <button class="btn primary" :disabled="isImporting" @click="importRuntime">
              {{ isImporting ? '导入中' : '导入' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.software-meta {
  min-width: 0;
}
.software-meta .subline {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.run-pill {
  display: inline-flex;
  align-items: center;
  margin-left: 8px;
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 650;
  color: var(--success);
  background: var(--success-soft);
  vertical-align: middle;
}
.software-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-wrap: wrap;
}
.btn.ghost {
  background: transparent;
  color: var(--text-3);
}
.btn.ghost:hover {
  color: var(--danger, #e11d48);
  border-color: color-mix(in srgb, var(--danger, #e11d48) 35%, var(--line));
}
.download-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 140px;
}

.progress-bar {
  flex: 1;
  height: 6px;
  background: var(--bg-2, #e2e8f0);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: var(--primary, #3b82f6);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.progress-text {
  font-size: 12px;
  color: var(--text-2, #64748b);
  white-space: nowrap;
}


.software-status {
  display: flex;
  align-items: center;
  gap: 6px;
}

.installed-tag {
  background: var(--success, #22c55e);
  color: #fff;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  white-space: nowrap;
}
</style>
