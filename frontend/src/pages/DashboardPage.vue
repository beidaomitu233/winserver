<script setup lang="ts">
import { ref, computed, inject, onMounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useServiceStore } from '../stores/useServiceStore'
import { useSiteStore } from '../stores/useSiteStore'
import { useDatabaseStore } from '../stores/useDatabaseStore'
import { useSettingsStore } from '../stores/useSettingsStore'
import { getServiceMeta, statusText, stateToStatus } from '../types'
import type { SystemResource } from '../types'

const icon = (name: string, cls = '') => `<svg class="icon ${cls}"><use href="#i-${name}"></use></svg>`

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
const siteStore = useSiteStore()
const databaseStore = useDatabaseStore()
const settingsStore = useSettingsStore()
const currentPage = inject<Ref<string>>('currentPage')!
const resources = inject<Ref<SystemResource>>('systemResource')!
const openConfigEditor = inject<(fileId: string, label: string, filePath: string) => void>('openConfigEditor')!
const openServiceConfigModal = inject<(svc: { id: string; name: string; config_file: string | null }) => void>('openServiceConfigModal')!

const collapsed = ref(localStorage.getItem('ws-svc-collapsed') === '1')
const logs = ref<Array<{ time: string; type: string; text: string }>>([])

const dashboardServices = computed(() => {
  const primaryTypes = new Set(['nginx', 'apache', 'php', 'php73', 'mysql57', 'mysql80', 'redis', 'pgsql', 'minio'])
  return serviceStore.services.filter(service =>
    primaryTypes.has(service.id) || primaryTypes.has(service.service_type)
  )
})
const runningCount = computed(() => dashboardServices.value.filter(service => service.state === 'running').length)
const totalCount = computed(() => dashboardServices.value.length)
const siteCount = computed(() => siteStore.sites.length)
const dbCount = computed(() => databaseStore.databases.length)

function formatUptime(seconds: number) {
  const total = Math.max(0, Number(seconds || 0))
  const days = Math.floor(total / 86400)
  const hours = Math.floor((total % 86400) / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  if (days > 0) return `${days}天 ${hours}小时`
  if (hours > 0) return `${hours}小时 ${minutes}分`
  return `${minutes}分`
}

const uptimeText = computed(() => formatUptime(resources.value.uptime_seconds))

function toggleCollapse() {
  collapsed.value = !collapsed.value
  localStorage.setItem('ws-svc-collapsed', collapsed.value ? '1' : '0')
}

async function startAll() {
  try {
    await serviceStore.startAll()
  } catch (e: any) {
    console.error('Start all failed:', e)
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

async function restartService(id: string) {
  try {
    await serviceStore.restartService(id)
  } catch (e: any) {
    console.error('Restart service failed:', e)
  }
}

function openServiceConfig(svc: { id: string; name: string; config_file: string | null }) {
  // Redis/MinIO get a dedicated dual-mode config modal; others fall back to
  // the generic config-file text editor.
  if (svc.id === 'redis' || svc.id === 'minio') {
    openServiceConfigModal?.(svc)
    return
  }
  const configFile = serviceStore.configFiles.find(f => f.id === svc.id || f.id === `service:${svc.id}`)
  if (configFile) {
    openConfigEditor(configFile.id, configFile.label, configFile.path)
  } else if (svc.config_file) {
    openConfigEditor(`service:${svc.id}`, svc.name + ' 配置', svc.config_file)
  }
}

/// Status text tailored to the home card, including the "未安装" (not installed)
/// state that the base statusText() does not cover.
function homeStatusText(svc: { installed: boolean; state: string }): string {
  if (!svc.installed) return '未安装'
  return statusText(svc.state as any)
}

/// Resolve the action buttons for a service per PRD §1.1 status→button mapping.
function serviceActionMode(svc: { installed: boolean; state: string }): {
  primaryLabel: string
  primaryAction: 'install' | 'toggle' | 'viewProblem' | 'none'
  primaryDisabled: boolean
  showRestart: boolean
  showConfig: boolean
  showLog: boolean
} {
  if (!svc.installed) {
    return { primaryLabel: '安装/导入', primaryAction: 'install', primaryDisabled: false, showRestart: false, showConfig: false, showLog: false }
  }
  switch (svc.state) {
    case 'starting':
      return { primaryLabel: '启动中', primaryAction: 'none', primaryDisabled: true, showRestart: false, showConfig: false, showLog: true }
    case 'stopping':
      return { primaryLabel: '停止中', primaryAction: 'none', primaryDisabled: true, showRestart: false, showConfig: false, showLog: true }
    case 'running':
      return { primaryLabel: '停止', primaryAction: 'toggle', primaryDisabled: false, showRestart: true, showConfig: true, showLog: false }
    case 'failed':
    case 'degraded':
      return { primaryLabel: '查看问题', primaryAction: 'viewProblem', primaryDisabled: false, showRestart: true, showConfig: false, showLog: true }
    default: // stopped, installed, unknown
      return { primaryLabel: '启动', primaryAction: 'toggle', primaryDisabled: false, showRestart: false, showConfig: true, showLog: false }
  }
}

async function installBundled(serviceId: string) {
  try {
    await invoke('software_install_bundled', { softwareId: serviceId })
    await serviceStore.fetchState()
  } catch (e: any) {
    console.error('Install bundled failed:', e)
  }
}

function openSoftwarePage() {
  currentPage.value = 'software'
}

async function fetchLogs() {
  try {
    const data = await invoke<{ logs: string[] }>('get_logs', { source: 'operation', search: '' })
    logs.value = (data.logs || []).slice(0, 6).map(line => {
      const match = line.match(/^\[([^\]]+)\]\s+(\w+)\s+(.*)$/)
      return {
        time: match?.[1]?.slice(11, 19) || '',
        type: line.includes('FAIL') ? 'error' : 'success',
        text: match?.[3] || line,
      }
    })
  } catch {
    // silent
  }
}

onMounted(async () => {
  await Promise.all([serviceStore.fetchState(), siteStore.fetchState(), databaseStore.fetchState(), settingsStore.fetchSettings()])
  await fetchLogs()
})
</script>

<template>
  <div class="page-enter dashboard-page dashboard-v2">
    <!-- Hero -->
    <section class="home-hero home-hero-v2">
      <div class="home-hero-top">
        <div class="home-heading">
          <div class="home-heading-mark" v-html="icon('server')" />
          <div>
            <div class="home-title">本地开发环境</div>
          </div>
        </div>
        <div class="home-hero-actions">
          <span class="home-state-pill">
            <span class="status-dot running" />
            {{ runningCount }} 项服务在线
          </span>
          <button class="btn primary" :disabled="serviceStore.suiteLoading" @click="startAll">
            <svg class="icon icon-sm"><use href="#i-play" /></svg>
            {{ serviceStore.suiteLoading ? '启动中' : '一键启动' }}
          </button>
        </div>
      </div>

      <!-- KPIs -->
      <div class="home-kpis home-kpis-v2">
        <div class="home-kpi">
          <div class="home-kpi-value">{{ runningCount }}<small>/ {{ totalCount }}</small></div>
          <div class="home-kpi-label">服务</div>
        </div>
        <div class="home-kpi">
          <div class="home-kpi-value">{{ siteCount }}<small>个</small></div>
          <div class="home-kpi-label">站点</div>
        </div>
        <div class="home-kpi">
          <div class="home-kpi-value">{{ dbCount }}<small>个</small></div>
          <div class="home-kpi-label">数据库</div>
        </div>
        <div class="home-kpi">
          <div class="home-kpi-value">{{ uptimeText }}</div>
          <div class="home-kpi-label">运行时间</div>
        </div>
      </div>

      <div class="home-quickbar home-quickbar-v2">
        <button class="home-quick" @click="$emit('open-create-site')">
          <svg class="icon icon-sm"><use href="#i-plus" /></svg>
          新建站点
        </button>
        <button class="home-quick" @click="currentPage = 'sites'">
          <svg class="icon icon-sm"><use href="#i-globe" /></svg>
          网站
        </button>
        <button class="home-quick" @click="currentPage = 'database'">
          <svg class="icon icon-sm"><use href="#i-database" /></svg>
          数据库
        </button>
        <button class="home-quick" @click="currentPage = 'software'">
          <svg class="icon icon-sm"><use href="#i-box" /></svg>
          环境
        </button>
        <button class="home-quick" @click="$emit('show-ports')">
          <svg class="icon icon-sm"><use href="#i-network" /></svg>
          端口
        </button>
        <button class="home-quick" @click="$emit('show-logs')">
          <svg class="icon icon-sm"><use href="#i-file" /></svg>
          日志
        </button>
      </div>
    </section>

    <!-- Workspace -->
    <section class="home-workspace">
      <!-- Left: services + logs -->
      <div class="home-workspace-main">
        <!-- Service section header -->
        <div class="home-section-head">
          <div
            class="home-section-title-wrap"
            @click="toggleCollapse"
            title="展开/折叠服务列表"
          >
            <div class="home-section-title" v-html="icon('sliders', 'icon-sm') + '服务状态'" />
            <svg class="icon icon-sm collapse-arrow" :class="{ rotated: !collapsed }">
              <use href="#i-chevron-down" />
            </svg>
          </div>
          <div class="home-section-actions">
            <span>{{ runningCount }}/{{ totalCount }} 正常</span>
            <button class="text-link" @click="currentPage = 'software'">
              管理 <svg class="icon icon-sm"><use href="#i-chevron-right" /></svg>
            </button>
          </div>
        </div>

        <!-- Service grid -->
        <div class="home-service-grid home-service-grid-scroll" :class="{ collapsed }">
          <div
            v-for="svc in dashboardServices"
            :key="svc.id"
            class="home-service-row"
            :class="{ 'not-installed': !svc.installed }"
            :data-service="svc.id"
          >
            <div class="home-service-identity">
              <div
                class="home-service-logo"
                :style="{ '--logo': getServiceMeta(svc.id).color }"
                v-html="serviceLogo({ id: svc.id, name: svc.name })"
              />
              <div class="home-service-copy">
                <div class="home-service-name">
                  {{ svc.name }}
                  <span class="home-service-port">:{{ svc.port || '—' }}</span>
                </div>
              </div>
            </div>
            <div
              class="home-service-state"
              :class="svc.installed ? stateToStatus(svc.state) : 'stopped'"
            >
              <span class="status-dot" :class="svc.installed ? stateToStatus(svc.state) : 'stopped'" />
              {{ homeStatusText(svc) }}
            </div>
            <div class="home-service-actions">
              <button
                v-if="serviceActionMode(svc).primaryAction === 'install'"
                class="home-service-action primary"
                @click="installBundled(svc.id)"
              >
                {{ serviceActionMode(svc).primaryLabel }}
              </button>
              <button
                v-else
                class="home-service-action"
                :class="svc.state === 'running' ? 'danger' : 'primary'"
                :disabled="serviceActionMode(svc).primaryDisabled || serviceStore.isServiceBusy(svc.id)"
                @click="serviceActionMode(svc).primaryAction === 'viewProblem' ? $emit('show-logs') : toggleService(svc.id)"
              >
                {{ serviceStore.isServiceBusy(svc.id) ? '处理中' : serviceActionMode(svc).primaryLabel }}
              </button>
              <button
                v-if="serviceActionMode(svc).showRestart"
                class="home-service-action"
                :disabled="serviceStore.isServiceBusy(svc.id)"
                @click="restartService(svc.id)"
              >
                重启
              </button>
              <button
                v-if="serviceActionMode(svc).showConfig"
                class="home-service-action"
                @click="openServiceConfig(svc)"
              >
                配置
              </button>
              <button
                v-if="serviceActionMode(svc).showLog"
                class="home-service-action"
                @click="$emit('show-logs')"
              >
                日志
              </button>
              <button
                v-if="!svc.installed"
                class="home-service-action"
                @click="openSoftwarePage"
              >
                软件页
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: log panel (moved from under the service list) -->
      <aside class="home-log-side">
        <div class="home-log-head">
          <div class="home-section-title" v-html="icon('file', 'icon-sm') + '日志'" />
          <button class="text-link" @click="$emit('show-logs')">
            全部 <svg class="icon icon-sm"><use href="#i-chevron-right" /></svg>
          </button>
        </div>
        <div class="home-log-list">
          <template v-if="logs.length > 0">
            <div v-for="(log, i) in logs" :key="i" class="home-log-row">
              <span class="log-dot" :class="log.type" />
              <span class="home-log-time">{{ log.time }}</span>
              <span class="home-log-text">{{ log.text }}</span>
            </div>
          </template>
          <div v-else style="color: var(--text-3)">暂无日志</div>
        </div>
      </aside>
    </section>
  </div>
</template>
