<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface FileEntry {
  name: string
  isDir: boolean
  size: number
  modifiedAt: string
}

const currentPath = ref('.')
const pathInput = ref('.')
const entries = ref<FileEntry[]>([])
const loading = ref(false)
const error = ref('')

const quickPaths = [
  { label: '当前目录', path: '.', indent: false },
  { label: 'C:\\', path: 'C:\\', indent: false },
  { label: 'D:\\', path: 'D:\\', indent: false },
  { label: 'data', path: 'data', indent: true },
  { label: 'backups', path: 'data\\backups', indent: true },
]

const sortedEntries = computed(() =>
  [...entries.value].sort((a, b) => {
    if (a.isDir !== b.isDir) return a.isDir ? -1 : 1
    return a.name.localeCompare(b.name)
  }),
)

function icon(name: string, cls = '') {
  return `<svg class="icon ${cls}"><use href="#i-${name}"></use></svg>`
}

function formatSize(size: number, isDir: boolean) {
  if (isDir) return '-'
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  if (size < 1024 * 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`
  return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`
}

function joinPath(base: string, name: string) {
  if (!base || base === '.') return name
  const trimmed = base.replace(/[\\\/]+$/, '')
  return `${trimmed}\\${name}`
}

async function loadPath(path = pathInput.value) {
  const target = path.trim()
  if (!target) {
    error.value = '请输入要浏览的目录路径'
    entries.value = []
    return
  }

  loading.value = true
  error.value = ''
  try {
    const result = await invoke<{ entries: FileEntry[] }>('files_list', { path: target })
    currentPath.value = target
    pathInput.value = target
    entries.value = result.entries || []
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    entries.value = []
  } finally {
    loading.value = false
  }
}

async function openCurrentFolder() {
  if (!currentPath.value) return
  try {
    await invoke('open_folder', { path: currentPath.value })
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
  }
}

function openEntry(entry: FileEntry) {
  if (!entry.isDir) return
  loadPath(joinPath(currentPath.value, entry.name))
}

onMounted(() => {
  loadPath('.')
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">文件</h1>
        <p class="page-desc">只读浏览本机目录，不提供删除、移动或重命名操作。</p>
      </div>
      <div class="page-actions">
        <button class="btn" :disabled="loading" @click="loadPath()">
          <svg class="icon icon-sm"><use href="#i-refresh" /></svg>
          刷新
        </button>
        <button class="btn primary" :disabled="!currentPath" @click="openCurrentFolder">
          <svg class="icon icon-sm"><use href="#i-folder" /></svg>
          打开目录
        </button>
      </div>
    </div>

    <div class="card file-layout">
      <aside class="file-sidebar">
        <div class="tree-title">快速访问</div>
        <button
          v-for="item in quickPaths"
          :key="item.path"
          class="tree-item"
          :class="{ active: currentPath === item.path, indent: item.indent }"
          type="button"
          @click="loadPath(item.path)"
        >
          <span v-html="icon(item.indent ? 'folder' : 'harddrive', 'icon-sm')" />
          <span>{{ item.label }}</span>
        </button>
      </aside>

      <section class="file-main">
        <div class="pathbar">
          <div class="crumb">
            <svg class="icon icon-sm"><use href="#i-folder" /></svg>
            <span>{{ currentPath }}</span>
          </div>
          <input
            v-model="pathInput"
            class="input"
            style="flex: 1"
            placeholder="输入目录路径"
            @keydown.enter="loadPath()"
          />
          <button class="btn" :disabled="loading" @click="loadPath()">进入</button>
        </div>

        <div class="file-grid-head">
          <span>名称</span>
          <span>类型</span>
          <span>大小</span>
          <span>修改时间</span>
          <span>操作</span>
        </div>

        <div v-if="loading" class="empty-state">正在读取目录...</div>
        <div v-else-if="error" class="empty-state">{{ error }}</div>
        <div v-else-if="sortedEntries.length === 0" class="empty-state">目录为空</div>
        <template v-else>
          <div
            v-for="entry in sortedEntries"
            :key="`${entry.name}-${entry.isDir}`"
            class="file-grid-row"
            @dblclick="openEntry(entry)"
          >
            <div class="file-name">
              <span class="file-type-icon" :class="{ folder: entry.isDir }">
                <svg class="icon icon-sm"><use :href="entry.isDir ? '#i-folder' : '#i-file'" /></svg>
              </span>
              <span>{{ entry.name }}</span>
            </div>
            <span>{{ entry.isDir ? '文件夹' : '文件' }}</span>
            <span>{{ formatSize(entry.size, entry.isDir) }}</span>
            <span>{{ entry.modifiedAt || '-' }}</span>
            <button class="btn small" :disabled="!entry.isDir" @click="openEntry(entry)">进入</button>
          </div>
        </template>
      </section>
    </div>
  </div>
</template>

<style scoped>
.tree-item {
  width: 100%;
  border: 0;
  background: transparent;
  font: inherit;
  text-align: left;
}
</style>
