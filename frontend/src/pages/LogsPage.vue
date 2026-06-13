<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'

const { show } = useToast()

const sources = [
  { id: 'operation', label: '操作日志' },
  { id: 'nginx_error', label: 'Nginx 错误' },
  { id: 'nginx_access', label: 'Nginx 访问' },
  { id: 'php_error', label: 'PHP 错误' },
  { id: 'agent', label: 'Agent' },
]

const activeSource = ref('operation')
const search = ref('')
const logs = ref<string[]>([])
const logPath = ref('')
const isLoading = ref(false)
const autoRefresh = ref(true)
let timer: number | null = null

function readableError(error: unknown) {
  if (typeof error === 'string') return error
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message?: unknown }).message || '操作失败')
  }
  return '操作失败'
}

async function loadLogs() {
  isLoading.value = true
  try {
    const result = await invoke<{ logs: string[]; path?: string }>('get_logs', {
      source: activeSource.value,
      search: search.value,
    })
    logs.value = result.logs || []
    logPath.value = result.path || ''
  } catch (e) {
    show('读取日志失败', readableError(e), 'error')
  } finally {
    isLoading.value = false
  }
}

async function clearCurrentLogs() {
  try {
    await invoke('clear_logs', { source: activeSource.value })
    await loadLogs()
    show('日志已清空', sources.find(source => source.id === activeSource.value)?.label || '', 'success')
  } catch (e) {
    show('清空日志失败', readableError(e), 'error')
  }
}

function selectSource(sourceId: string) {
  activeSource.value = sourceId
  loadLogs()
}

onMounted(() => {
  loadLogs()
  timer = window.setInterval(() => {
    if (autoRefresh.value) loadLogs()
  }, 3000)
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">日志</h1>
        <p v-if="logPath" class="page-desc">{{ logPath }}</p>
      </div>
      <div class="page-actions">
        <button class="btn" @click="autoRefresh = !autoRefresh">
          <svg class="icon icon-sm"><use href="#i-refresh" /></svg>
          {{ autoRefresh ? '自动刷新' : '手动刷新' }}
        </button>
        <button class="btn" @click="loadLogs">
          <svg class="icon icon-sm"><use href="#i-refresh" /></svg>刷新
        </button>
        <button class="btn danger" @click="clearCurrentLogs">
          <svg class="icon icon-sm"><use href="#i-trash" /></svg>清空
        </button>
      </div>
    </div>

    <div class="card">
      <div class="tabs">
        <button
          v-for="source in sources"
          :key="source.id"
          class="tab"
          :class="{ active: activeSource === source.id }"
          @click="selectSource(source.id)"
        >
          {{ source.label }}
        </button>
        <div class="search-box" style="margin-left: auto">
          <svg class="icon icon-sm"><use href="#i-search" /></svg>
          <input v-model="search" class="input" placeholder="搜索日志" @keyup.enter="loadLogs" />
        </div>
      </div>

      <div class="terminal-body" style="height: calc(100vh - 250px); background: transparent; color: var(--text); font-family: 'Cascadia Code', Consolas, monospace;">
        <template v-if="logs.length">
          <div v-for="(line, index) in logs" :key="index" class="terminal-line">{{ line }}</div>
        </template>
        <div v-else style="color: var(--text-3); padding: 24px;">
          {{ isLoading ? '读取中' : '暂无日志' }}
        </div>
      </div>
    </div>
  </div>
</template>
