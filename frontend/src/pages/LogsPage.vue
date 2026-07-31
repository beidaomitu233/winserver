<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'
import type { OperationLog } from '../types'

const { show } = useToast()
const search = ref('')
const logs = ref<OperationLog[]>([])
const resultFilter = ref<'all' | 'success' | 'failed'>('all')
const isLoading = ref(false)
const isClearing = ref(false)
const autoRefresh = ref(true)
let timer: number | null = null

const actionLabels: Record<string, string> = {
  'service.start': '启动服务',
  'service.stop': '停止服务',
  'health.monitor': '服务健康检查',
  'startup.reconcile': '恢复服务状态',
  'service.toggleAuto': '更新一键启动项',
  'site.create': '创建站点',
  'site.update': '更新站点',
  'site.delete': '删除站点',
  'site.enable': '启用站点',
  'site.disable': '停用站点',
  'site.switch_php': '切换 PHP',
  'runtime.import': '导入运行环境',
  'runtime.delete': '移除运行环境',
  'software.install': '安装软件',
  'software.uninstall': '卸载软件',
  'software.installBundled': '安装内置软件',
  'software.install_bundled': '安装内置软件',
  'software.downloadInstall': '下载并安装软件',
  'software.download_install': '下载并安装软件',
  'software.detectLocal': '检测本机软件',
  'config.save': '保存配置',
  'redis.config.save': '保存 Redis 配置',
  'minio.config.save': '保存 MinIO 配置',
  'minio.policy': '设置桶权限',
  'minio.bucket.setPolicy': '设置桶权限',
  'database.create': '创建数据库',
  'database.delete': '移除数据库记录',
  'database.changePassword': '修改数据库密码',
  'database.rootPassword': '修改 Root 密码',
  'database.export': '导出数据库',
  'database.import': '导入数据库',
  'database.deleteBackup': '删除数据库备份',
  'database.pgCreate': '创建 PostgreSQL 数据库',
  'settings.update': '更新设置',
  'settings.paths': '更新路径设置',
  'hosts.sync': '写入 hosts',
  'hosts.remove': '移除 hosts',
  'port.killProcess': '结束占用进程',
}

const fieldLabels: Record<string, string> = {
  serviceId: '服务',
  siteId: '站点',
  softwareId: '软件',
  fileId: '配置',
  db: '数据库',
  user: '用户',
  bucket: '桶',
  policy: '权限',
  domain: '域名',
  port: '端口',
  path: '路径',
  pid: 'PID',
  auto: '一键启动',
  content: '配置内容',
}

const filteredLogs = computed(() => {
  if (resultFilter.value === 'all') return logs.value
  const success = resultFilter.value === 'success'
  return logs.value.filter(log => log.success === success)
})

function readableError(error: unknown) {
  if (typeof error === 'string') return error
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message?: unknown }).message || '操作失败')
  }
  return '操作失败'
}

async function loadLogs(opts?: { silent?: boolean }) {
  isLoading.value = true
  try {
    const result = await invoke<{ logs: OperationLog[] }>('get_logs', {
      source: 'operation',
      search: search.value,
    })
    logs.value = result.logs || []
  } catch (error) {
    if (!opts?.silent) show('读取操作日志失败', readableError(error), 'error')
  } finally {
    isLoading.value = false
  }
}

async function clearLogs() {
  if (!window.confirm('确认清空全部操作日志？此操作无法撤销。')) return
  isClearing.value = true
  try {
    await invoke('clear_logs', { source: 'operation' })
    logs.value = []
    show('操作日志已清空', '', 'success')
  } catch (error) {
    show('清空操作日志失败', readableError(error), 'error')
  } finally {
    isClearing.value = false
  }
}

function formatTime(value: string) {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
    hour12: false,
  }).format(date)
}

function actionLabel(action: string) {
  return actionLabels[action] || action
}

function displayValue(value: unknown) {
  if (typeof value === 'boolean') return value ? '是' : '否'
  if (value === null || value === undefined || value === '') return '—'
  if (typeof value === 'object') return JSON.stringify(value)
  return String(value)
}

function detailItems(log: OperationLog) {
  return Object.entries(log.details?.request || {}).map(([key, value]) => ({
    key,
    label: fieldLabels[key] || key,
    value: displayValue(value),
  }))
}

onMounted(() => {
  void loadLogs()
  timer = window.setInterval(() => {
    if (autoRefresh.value) void loadLogs({ silent: true })
  }, 3000)
})

onUnmounted(() => {
  if (timer) window.clearInterval(timer)
})
</script>

<template>
  <div class="page-enter operation-page">
    <div class="page-header">
      <div>
        <h1 class="page-title">操作日志</h1>
        <p class="page-desc">{{ logs.length }} 条记录</p>
      </div>
      <div class="page-actions">
        <label class="auto-refresh">
          <input v-model="autoRefresh" type="checkbox" />
          <span>自动刷新</span>
        </label>
        <button class="btn-icon" type="button" title="刷新" :disabled="isLoading" @click="loadLogs()">
          <svg class="icon"><use href="#i-refresh" /></svg>
        </button>
        <button class="btn-icon danger" type="button" title="清空操作日志" :disabled="isClearing || logs.length === 0" @click="clearLogs">
          <svg class="icon"><use href="#i-trash" /></svg>
        </button>
      </div>
    </div>

    <div class="log-toolbar">
      <div class="search-box">
        <svg class="icon icon-sm"><use href="#i-search" /></svg>
        <input v-model="search" class="input" placeholder="搜索操作、目标或错误" @keyup.enter="loadLogs()" />
      </div>
      <div class="result-filter" role="group" aria-label="结果筛选">
        <button type="button" :class="{ active: resultFilter === 'all' }" @click="resultFilter = 'all'">全部</button>
        <button type="button" :class="{ active: resultFilter === 'success' }" @click="resultFilter = 'success'">成功</button>
        <button type="button" :class="{ active: resultFilter === 'failed' }" @click="resultFilter = 'failed'">失败</button>
      </div>
    </div>

    <div class="operation-list" :aria-busy="isLoading">
      <article v-for="log in filteredLogs" :key="log.id" class="operation-row">
        <div class="operation-state" :class="log.success ? 'success' : 'failed'">
          <svg class="icon"><use :href="log.success ? '#i-check' : '#i-close'" /></svg>
        </div>
        <div class="operation-main">
          <div class="operation-title-line">
            <strong>{{ actionLabel(log.action) }}</strong>
            <code>{{ log.action }}</code>
            <span v-if="log.target_id" class="operation-target">{{ log.target_id }}</span>
          </div>
          <p>{{ log.message || (log.success ? '操作成功' : '操作失败') }}</p>
          <div v-if="detailItems(log).length" class="operation-details">
            <span v-for="item in detailItems(log)" :key="item.key">
              <b>{{ item.label }}</b>{{ item.value }}
            </span>
          </div>
        </div>
        <div class="operation-meta">
          <time :datetime="log.created_at">{{ formatTime(log.created_at) }}</time>
          <span v-if="log.error_code" class="error-code">{{ log.error_code }}</span>
          <span v-else :class="log.success ? 'result-success' : 'result-failed'">{{ log.success ? '成功' : '失败' }}</span>
        </div>
      </article>

      <div v-if="!isLoading && filteredLogs.length === 0" class="log-empty">
        {{ search ? '未找到匹配记录' : '暂无操作记录' }}
      </div>
      <div v-if="isLoading && logs.length === 0" class="log-empty">读取中</div>
    </div>
  </div>
</template>

<style scoped>
.operation-page { height: 100%; min-height: 0; display: flex; flex-direction: column; }
.auto-refresh { display: flex; align-items: center; gap: 7px; color: var(--text-2); font-size: 12px; cursor: pointer; }
.auto-refresh input { width: 15px; height: 15px; margin: 0; accent-color: var(--primary); }
.log-toolbar {
  display: flex; align-items: center; justify-content: space-between; gap: 12px;
  padding: 10px 0; border-bottom: 1px solid var(--line);
}
.log-toolbar .search-box { width: min(420px, 60%); }
.result-filter { display: flex; align-items: center; padding: 2px; border: 1px solid var(--line); border-radius: 6px; }
.result-filter button {
  min-width: 54px; height: 28px; padding: 0 10px; border: 0; border-radius: 4px;
  background: transparent; color: var(--text-3); font: inherit; font-size: 12px; cursor: pointer;
}
.result-filter button.active { background: var(--surface-hover); color: var(--text); font-weight: 700; }
.operation-list { flex: 1; min-height: 0; overflow: auto; }
.operation-row {
  display: grid; grid-template-columns: 32px minmax(0, 1fr) 128px; gap: 12px;
  padding: 14px 4px; border-bottom: 1px solid var(--line);
}
.operation-state {
  width: 28px; height: 28px; display: grid; place-items: center; border-radius: 6px;
  background: var(--success-soft); color: var(--success);
}
.operation-state.failed { background: var(--danger-soft); color: var(--danger); }
.operation-state .icon { width: 15px; height: 15px; }
.operation-main { min-width: 0; }
.operation-title-line { display: flex; align-items: center; gap: 8px; min-width: 0; }
.operation-title-line strong { font-size: 13px; }
.operation-title-line code { color: var(--text-3); font-size: 11px; }
.operation-target {
  min-width: 0; max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--primary); font-size: 12px;
}
.operation-main p { margin: 5px 0 0; color: var(--text-2); font-size: 12px; line-height: 1.45; word-break: break-word; }
.operation-details { display: flex; flex-wrap: wrap; gap: 5px 14px; margin-top: 7px; }
.operation-details span { color: var(--text-3); font-size: 11px; word-break: break-all; }
.operation-details b { margin-right: 4px; color: var(--text-2); font-weight: 650; }
.operation-meta { display: flex; flex-direction: column; align-items: flex-end; gap: 6px; font-size: 11px; }
.operation-meta time { color: var(--text-3); font-variant-numeric: tabular-nums; }
.result-success { color: var(--success); }
.result-failed, .error-code { color: var(--danger); }
.error-code { max-width: 128px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: monospace; }
.log-empty { padding: 48px 16px; text-align: center; color: var(--text-3); font-size: 13px; }
@media (max-width: 760px) {
  .log-toolbar { align-items: stretch; flex-direction: column; }
  .log-toolbar .search-box { width: 100%; }
  .result-filter { align-self: flex-start; }
  .operation-row { grid-template-columns: 28px minmax(0, 1fr); }
  .operation-meta { grid-column: 2; align-items: flex-start; flex-direction: row; }
  .operation-title-line { align-items: flex-start; flex-wrap: wrap; }
}
</style>
