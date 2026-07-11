<script setup lang="ts">
import { computed, inject, onMounted, type Ref } from 'vue'
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
    nginx: `<svg viewBox="0 0 48 48"><path d="M24 3.8 41.2 13.7v20.1L24 44.2 6.8 34.3V13.7Z" fill="currentColor"/><path d="M15.3 33V15l17.4 18V15" fill="none" stroke="#fff" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    mysql57: `<svg viewBox="0 0 48 48"><path d="M6.5 30.5c5.6-8.3 13-12.7 21.4-12.7 6.3 0 10.8 2.3 13.6 6.7-4.4-1.9-7.8-2.4-10.1-1.7 3.8 1.3 6.2 4.5 7 9.5-5.1-4-9.2-5.3-12.5-3.9-2.4 1-4.6 3.3-6.6 7-3.4-3.8-7.7-5.4-12.8-4.9Z" fill="currentColor" opacity=".95"/></svg>`,
    mysql80: `<svg viewBox="0 0 48 48"><path d="M6.5 30.5c5.6-8.3 13-12.7 21.4-12.7 6.3 0 10.8 2.3 13.6 6.7-4.4-1.9-7.8-2.4-10.1-1.7 3.8 1.3 6.2 4.5 7 9.5-5.1-4-9.2-5.3-12.5-3.9-2.4 1-4.6 3.3-6.6 7-3.4-3.8-7.7-5.4-12.8-4.9Z" fill="currentColor" opacity=".95"/></svg>`,
    php73: `<svg viewBox="0 0 48 48"><ellipse cx="24" cy="24" rx="21" ry="12.8" fill="currentColor"/><text x="24" y="28.2" text-anchor="middle" class="brand-word" font-size="12.2" fill="#fff">PHP</text></svg>`,
    redis: `<svg viewBox="0 0 48 48"><path d="m24 7 18 8-18 8L6 15Z" fill="currentColor"/><path d="m6 22 18 8 18-8v7l-18 8-18-8Z" fill="currentColor" opacity=".86"/></svg>`,
    minio: `<svg viewBox="0 0 48 48"><rect x="8" y="8" width="32" height="32" rx="6" fill="currentColor" opacity=".18"/><path d="M16 16h16v16H16z" fill="currentColor" opacity=".8"/></svg>`,
  }
  return `<span class="service-brand" title="${name}">${logos[id] || `<svg viewBox="0 0 48 48"><circle cx="24" cy="24" r="18" fill="currentColor" opacity=".14"/><text x="24" y="28" text-anchor="middle" font-size="13" fill="currentColor">${String(name).slice(0, 2)}</text></svg>`}</span>`
}

const serviceStore = useServiceStore()
const siteStore = useSiteStore()
const databaseStore = useDatabaseStore()
const settingsStore = useSettingsStore()
const currentPage = inject<Ref<string>>('currentPage')!
const resources = inject<Ref<SystemResource>>('systemResource')!
const openServiceConfigModal = inject<(svc: { id: string; name: string; config_file: string | null }) => void>('openServiceConfigModal')!

const primaryOrder = ['nginx', 'mysql80', 'mysql57', 'php73', 'redis', 'minio']

const dashboardServices = computed(() => {
  const primaryTypes = new Set(primaryOrder)
  const list = serviceStore.services.filter(
    s => primaryTypes.has(s.id) || primaryTypes.has(s.service_type),
  )
  return [...list].sort((a, b) => {
    const ai = primaryOrder.indexOf(a.id)
    const bi = primaryOrder.indexOf(b.id)
    return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi)
  })
})

const runningCount = computed(() => dashboardServices.value.filter(s => s.state === 'running').length)
const installedCount = computed(() => dashboardServices.value.filter(s => s.installed).length)
const siteCount = computed(() => siteStore.sites.length)
const dbCount = computed(() => databaseStore.databases.length)
const installedApps = computed(() => dashboardServices.value.filter(s => s.installed))

const activeMysql = computed(() => {
  const mysqls = dashboardServices.value.filter(s => s.id.startsWith('mysql') && s.installed)
  return mysqls.find(s => s.state === 'running') || mysqls[0] || null
})

function formatUptime(seconds: number) {
  const total = Math.max(0, Number(seconds || 0))
  const days = Math.floor(total / 86400)
  const hours = Math.floor((total % 86400) / 3600)
  const minutes = Math.floor((total % 3600) / 60)
  if (days > 0) return `${days} 天 ${hours} 小时`
  if (hours > 0) return `${hours} 小时 ${minutes} 分`
  return `${minutes} 分`
}

const uptimeText = computed(() => formatUptime(resources.value.uptime_seconds))

function clampPct(n: number) {
  return Math.max(0, Math.min(100, Math.round(Number(n) || 0)))
}

function fmt2(n: number) {
  const v = Number(n)
  if (!Number.isFinite(v)) return '0.00'
  return v.toFixed(2)
}

/** Approximate load = cores * cpu% (Windows has no loadavg). */
const loadValue = computed(() => {
  const cores = Math.max(1, Number(resources.value.cpu_count) || 1)
  return (cores * (Number(resources.value.cpu_percent) || 0)) / 100
})

const loadPercent = computed(() => {
  const cores = Math.max(1, Number(resources.value.cpu_count) || 1)
  return clampPct((loadValue.value / cores) * 100)
})

const rings = computed(() => [
  {
    key: 'cpu',
    label: 'CPU',
    pct: clampPct(resources.value.cpu_percent),
    value: `${clampPct(resources.value.cpu_percent)}%`,
    unit: `${resources.value.cpu_count || '—'} 核`,
    color: 'var(--primary)',
  },
  {
    key: 'mem',
    label: '内存',
    pct: clampPct(resources.value.memory_percent),
    value: `${clampPct(resources.value.memory_percent)}%`,
    unit: `${fmt2((resources.value.used_memory_mb || 0) / 1024)} / ${fmt2((resources.value.total_memory_mb || 0) / 1024)} GB`,
    color: 'var(--purple, #7c5cfc)',
  },
  {
    key: 'load',
    label: '负载',
    pct: loadPercent.value,
    value: fmt2(loadValue.value),
    unit: `约 ${resources.value.cpu_count || 1} 核容量`,
    color: 'var(--warning, #f5a524)',
  },
  {
    key: 'disk',
    label: '存储',
    pct: clampPct(resources.value.disk?.percent),
    value: `${clampPct(resources.value.disk?.percent)}%`,
    unit: `${fmt2(resources.value.disk?.used_gb || 0)} / ${fmt2(resources.value.disk?.total_gb || 0)} GB`,
    color: 'var(--success)',
  },
])

// Ring geometry is in viewBox units (0–100). SVG CSS size scales independently.
const RING_R = 38
const RING_C = 2 * Math.PI * RING_R

function ringOffset(pct: number) {
  const p = clampPct(pct) / 100
  return RING_C * (1 - p)
}

async function startAll() {
  try {
    await serviceStore.startAll()
  } catch (e) {
    console.error(e)
  }
}

async function toggleService(id: string) {
  const svc = serviceStore.services.find(s => s.id === id)
  if (!svc) return
  try {
    if (svc.state === 'running') await serviceStore.stopService(id)
    else await serviceStore.startService(id)
  } catch (e) {
    console.error(e)
  }
}

function openServiceConfig(svc: { id: string; name: string; config_file: string | null }) {
  // All core services use the visual config modal (file editor is optional inside).
  openServiceConfigModal?.(svc)
}

function homeStatusText(svc: { installed: boolean; state: string }) {
  if (!svc.installed) return '未安装'
  return statusText(svc.state as any)
}

onMounted(async () => {
  await Promise.all([
    serviceStore.fetchState(),
    siteStore.fetchState(),
    databaseStore.fetchState(),
    settingsStore.fetchSettings(),
  ])
})
</script>

<template>
  <div class="page-enter panel-home">
    <!-- 概览 -->
    <section class="panel-overview">
      <div class="panel-overview-head">
        <div>
          <div class="panel-eyebrow">概览</div>
          <h1 class="panel-title">WinServer 控制台</h1>
        </div>
        <div class="panel-overview-actions">
          <span class="panel-online">
            <span class="status-dot running" />
            {{ runningCount }} 在线 / {{ installedCount }} 已装
          </span>
          <button class="btn primary" :disabled="serviceStore.suiteLoading" @click="startAll">
            <svg class="icon icon-sm"><use href="#i-play" /></svg>
            {{ serviceStore.suiteLoading ? '启动中' : '一键启动' }}
          </button>
        </div>
      </div>
      <div class="panel-kpi-grid">
        <button class="panel-kpi" type="button" @click="currentPage = 'software'">
          <div class="panel-kpi-label">服务</div>
          <div class="panel-kpi-value">{{ runningCount }}<small>/{{ dashboardServices.length }}</small></div>
        </button>
        <button class="panel-kpi" type="button" @click="currentPage = 'sites'">
          <div class="panel-kpi-label">站点</div>
          <div class="panel-kpi-value">{{ siteCount }}</div>
        </button>
        <button class="panel-kpi" type="button" @click="currentPage = 'database'">
          <div class="panel-kpi-label">数据库</div>
          <div class="panel-kpi-value">{{ dbCount }}</div>
          <div class="panel-kpi-hint">{{ activeMysql?.name || '未安装 MySQL' }}</div>
        </button>
        <div class="panel-kpi">
          <div class="panel-kpi-label">运行时间</div>
          <div class="panel-kpi-value panel-kpi-value-sm">{{ uptimeText }}</div>
        </div>
      </div>
    </section>

    <section class="panel-mid">
      <!-- 监控：四个环形图 -->
      <div class="panel-card panel-monitor">
        <div class="panel-card-head">
          <div class="panel-card-title" v-html="icon('monitor', 'icon-sm') + '监控'" />
        </div>
        <div class="panel-rings">
          <div v-for="ring in rings" :key="ring.key" class="panel-ring">
            <div class="panel-ring-chart">
              <svg viewBox="0 0 100 100">
                <circle class="ring-bg" cx="50" cy="50" :r="RING_R" />
                <circle
                  class="ring-fg"
                  cx="50"
                  cy="50"
                  :r="RING_R"
                  :stroke="ring.color"
                  :stroke-dasharray="RING_C"
                  :stroke-dashoffset="ringOffset(ring.pct)"
                />
              </svg>
              <div class="panel-ring-center">
                <strong>{{ ring.value }}</strong>
              </div>
            </div>
            <div class="panel-ring-label">{{ ring.label }}</div>
            <div class="panel-ring-unit">{{ ring.unit }}</div>
          </div>
        </div>
      </div>

      <!-- 系统信息 -->
      <div class="panel-card panel-sysinfo">
        <div class="panel-card-head">
          <div class="panel-card-title" v-html="icon('server', 'icon-sm') + '系统信息'" />
        </div>
        <div class="panel-sys-list">
          <div class="panel-sys-row"><span>面板</span><strong>WinServer 0.2</strong></div>
          <div class="panel-sys-row"><span>开机时长</span><strong>{{ uptimeText }}</strong></div>
          <div class="panel-sys-row"><span>CPU</span><strong class="truncate">{{ resources.cpu_model || '—' }}</strong></div>
          <div class="panel-sys-row">
            <span>内存</span>
            <strong>{{ fmt2((resources.total_memory_mb || 0) / 1024) }} GB</strong>
          </div>
          <div class="panel-sys-row">
            <span>存储</span>
            <strong>{{ fmt2(resources.disk?.used_gb || 0) }} / {{ fmt2(resources.disk?.total_gb || 0) }} GB</strong>
          </div>
          <div class="panel-sys-row">
            <span>活动 MySQL</span>
            <strong>{{ activeMysql ? activeMysql.name : '未安装' }}</strong>
          </div>
        </div>
      </div>
    </section>

    <!-- 应用 -->
    <section class="panel-card panel-apps">
      <div class="panel-card-head">
        <div class="panel-card-title" v-html="icon('box', 'icon-sm') + '应用'" />
        <button class="text-link" type="button" @click="currentPage = 'software'">全部软件</button>
      </div>
      <div class="panel-app-grid">
        <div
          v-for="svc in installedApps"
          :key="'app-' + svc.id"
          class="panel-app-card"
          :class="{ running: svc.state === 'running' }"
        >
          <div
            class="panel-app-logo"
            :style="{ '--logo': getServiceMeta(svc.id).color }"
            v-html="serviceLogo({ id: svc.id, name: svc.name })"
          />
          <div class="panel-app-name">{{ svc.name }}</div>
          <div class="panel-app-state">
            <span class="status-dot" :class="stateToStatus(svc.state)" />
            {{ homeStatusText(svc) }}
            <template v-if="svc.port"> · {{ svc.port }}</template>
          </div>
          <div class="panel-app-actions">
            <button
              class="btn small"
              :class="svc.state === 'running' ? '' : 'primary'"
              :disabled="serviceStore.isServiceBusy(svc.id)"
              @click="toggleService(svc.id)"
            >
              {{ serviceStore.isServiceBusy(svc.id) ? '…' : svc.state === 'running' ? '停止' : '启动' }}
            </button>
            <button class="btn small ghost" @click="openServiceConfig(svc)">配置</button>
          </div>
        </div>

        <button
          v-if="installedApps.length === 0"
          class="panel-app-empty"
          type="button"
          @click="currentPage = 'software'"
        >
          <svg class="icon"><use href="#i-plus" /></svg>
          <span>安装运行环境</span>
        </button>
      </div>
    </section>
  </div>
</template>

<style scoped>
/*
  Layout rules:
  - Overview + monitor/sysinfo keep stable size (never compressed by apps).
  - Apps can wrap to multiple rows; page scrolls via .content.
  - Rings scale with viewport (vmin/vw), not a low fixed max.
*/
.panel-home {
  display: flex;
  flex-direction: column;
  gap: 12px;
  box-sizing: border-box;
  min-height: 100%;
  height: auto;
  overflow: visible;
  padding-bottom: 8px;
}

.panel-overview {
  flex: 0 0 auto;
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-solid);
  padding: 12px 16px 12px;
}
.panel-overview-head {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
  align-items: center;
}
.panel-eyebrow {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: .08em;
  text-transform: uppercase;
  color: var(--text-3);
  margin-bottom: 2px;
}
.panel-title { margin: 0; font-size: 17px; font-weight: 780; }
.panel-overview-actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.panel-online {
  display: inline-flex; align-items: center; gap: 6px;
  height: 32px; padding: 0 12px; border-radius: 999px;
  border: 1px solid var(--line); background: var(--surface-soft);
  font-size: 12px; font-weight: 650; color: var(--text-2);
}
.panel-kpi-grid { display: grid; grid-template-columns: repeat(4, minmax(0,1fr)); gap: 10px; }
.panel-kpi {
  text-align: left; border: 1px solid var(--line); border-radius: 10px;
  background: var(--surface-soft); padding: 10px 14px; font: inherit; color: inherit; cursor: pointer;
}
button.panel-kpi:hover { border-color: color-mix(in srgb, var(--primary) 40%, var(--line)); }
div.panel-kpi { cursor: default; }
.panel-kpi-label { font-size: 12px; color: var(--text-3); font-weight: 650; }
.panel-kpi-value { margin-top: 4px; font-size: 24px; font-weight: 800; line-height: 1.1; font-variant-numeric: tabular-nums; }
.panel-kpi-value small { font-size: 13px; color: var(--text-3); font-weight: 650; }
.panel-kpi-value-sm { font-size: 16px; padding-top: 4px; }
.panel-kpi-hint { margin-top: 3px; font-size: 11px; color: var(--text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* Fixed-size mid band — never flex-shrink when apps overflow */
.panel-mid {
  flex: 0 0 auto;
  display: grid;
  grid-template-columns: minmax(0, 1.65fr) minmax(280px, 1fr);
  gap: 12px;
  align-items: stretch;
  min-height: 220px;
  height: clamp(220px, 28vh, 320px);
}

.panel-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  background: var(--surface-solid);
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  height: 100%;
}

.panel-monitor,
.panel-sysinfo {
  min-height: 0;
}

.panel-card-head {
  flex: 0 0 auto;
  min-height: 40px;
  padding: 0 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--line);
}
.panel-card-title { display: flex; align-items: center; gap: 6px; font-size: 13px; font-weight: 740; }
.panel-card-title :deep(.icon) { color: var(--primary); }
.text-link { border: 0; background: transparent; color: var(--primary); font: inherit; font-size: 12px; font-weight: 650; cursor: pointer; }

/* Rings scale with monitor card size */
.panel-rings {
  flex: 1 1 auto;
  min-height: 0;
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  padding: clamp(10px, 1.6vh, 18px) clamp(10px, 1.2vw, 16px);
  align-content: center;
  justify-items: center;
}
.panel-ring {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  height: 100%;
  min-width: 0;
}
.panel-ring-chart {
  position: relative;
  /* Grow on maximize: track both viewport and available row height */
  width: clamp(76px, min(11vw, 18vh), 132px);
  height: clamp(76px, min(11vw, 18vh), 132px);
  flex: 0 0 auto;
}
.panel-ring-chart svg {
  display: block;
  width: 100%;
  height: 100%;
  transform: rotate(-90deg);
  overflow: visible;
}
.ring-bg {
  fill: none;
  stroke: var(--line);
  stroke-width: 7.5;
}
.ring-fg {
  fill: none;
  stroke-width: 7.5;
  stroke-linecap: round;
  transition: stroke-dashoffset .45s ease;
}
.panel-ring-center {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  pointer-events: none;
}
.panel-ring-center strong {
  font-size: clamp(12px, 1.3vw, 16px);
  font-weight: 800;
  font-variant-numeric: tabular-nums;
  line-height: 1;
}
.panel-ring-label { font-size: clamp(11px, 1vw, 13px); font-weight: 740; }
.panel-ring-unit {
  font-size: clamp(9px, 0.85vw, 11px);
  color: var(--text-3);
  text-align: center;
  line-height: 1.3;
  max-width: 100%;
  padding: 0 4px;
  word-break: break-word;
}

.panel-sys-list {
  flex: 1 1 auto;
  min-height: 0;
  padding: 6px 14px 10px;
  display: flex;
  flex-direction: column;
  justify-content: space-evenly;
}
.panel-sys-row {
  display: flex; justify-content: space-between; gap: 12px;
  min-height: 30px; align-items: center; font-size: 12px;
  border-bottom: 1px solid var(--line);
}
.panel-sys-row:last-child { border-bottom: 0; }
.panel-sys-row span { color: var(--text-3); flex: none; }
.panel-sys-row strong { font-weight: 650; text-align: right; min-width: 0; }
.panel-sys-row .truncate { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 62%; }

/* Apps grow freely; extra rows scroll the page, never squeeze mid/overview */
.panel-apps {
  flex: 0 0 auto;
  display: flex;
  flex-direction: column;
}
.panel-app-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  padding: 12px 14px 14px;
  align-content: start;
}
.panel-app-card {
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 12px;
  background: var(--surface-soft);
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 132px;
}
.panel-app-card.running { border-color: color-mix(in srgb, var(--success) 35%, var(--line)); }
.panel-app-logo {
  width: 40px; height: 40px; border-radius: 11px; display: grid; place-items: center;
  color: var(--logo, var(--primary));
  background: color-mix(in srgb, var(--logo, var(--primary)) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--logo, var(--primary)) 18%, transparent);
  flex: 0 0 auto;
}
.panel-app-logo :deep(svg) { width: 28px; height: 28px; }
.panel-app-name { font-weight: 740; font-size: 13px; }
.panel-app-state { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-3); }
.panel-app-actions {
  display: flex;
  gap: 6px;
  margin-top: auto;
  width: 100%;
  padding-top: 4px;
}
.panel-app-actions .btn.small { height: 32px; padding: 0 12px; font-size: 12px; }
.btn.ghost { background: transparent; color: var(--text-3); }
.panel-app-empty {
  border: 1px dashed var(--line-strong); border-radius: 12px; background: transparent;
  color: var(--text-3); font: inherit; cursor: pointer; min-height: 120px;
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 8px;
  grid-column: 1 / -1;
}

@media (max-width: 1180px) {
  .panel-mid {
    grid-template-columns: 1fr;
    height: auto;
    min-height: 200px;
  }
  .panel-monitor { min-height: 200px; height: auto; }
  .panel-sysinfo { height: auto; }
  .panel-app-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .panel-kpi-grid { grid-template-columns: repeat(2, minmax(0,1fr)); }
}

@media (max-width: 720px) {
  .panel-rings { grid-template-columns: repeat(2, minmax(0,1fr)); }
  .panel-app-grid { grid-template-columns: 1fr; }
  .panel-mid { height: auto; }
}
</style>
