<script setup lang="ts">
import { inject, type Ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const currentPage = inject<Ref<string>>('currentPage')!
const theme = inject<Ref<'light' | 'dark'>>('theme')!
const refreshAll = inject<() => Promise<void>>('refreshAll')!

const pageTitles: Record<string, [string, string]> = {
  dashboard: ['仪表盘', '首页'],
  sites: ['网站', '站点管理'],
  database: ['数据库', '数据库管理'],
  software: ['软件管理', '运行环境'],
  files: ['文件管理', '本地文件'],
  settings: ['设置', '系统设置'],
}

function toggleTheme() {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
}

async function minimizeWindow() {
  await getCurrentWindow().minimize()
}

async function maximizeWindow() {
  await getCurrentWindow().toggleMaximize()
}

async function closeWindow() {
  await getCurrentWindow().close()
}
</script>

<template>
  <header class="topbar">
    <div class="breadcrumb">
      <span>{{ pageTitles[currentPage]?.[0] || '首页' }}</span>
      <b>/</b>
      <span>{{ pageTitles[currentPage]?.[1] || '首页' }}</span>
    </div>
    <button class="search-trigger">
      <svg class="icon"><use href="#i-search"></use></svg>
      <span>搜索…</span>
      <span class="shortcut">Ctrl + K</span>
    </button>
    <div class="top-actions">
      <button class="icon-btn" title="刷新" @click="refreshAll">
        <svg class="icon"><use href="#i-refresh"></use></svg>
      </button>
      <button class="icon-btn" title="切换主题" @click="toggleTheme">
        <svg class="icon"><use :href="theme === 'dark' ? '#i-sun' : '#i-moon'"></use></svg>
      </button>
    </div>
    <div class="window-controls">
      <button class="window-btn" title="最小化" @click="minimizeWindow">
        <svg class="icon icon-sm" viewBox="0 0 12 12"><line x1="0" y1="6" x2="12" y2="6" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
      <button class="window-btn" title="最大化" @click="maximizeWindow">
        <svg class="icon icon-sm" viewBox="0 0 12 12"><rect x="1" y="1" width="10" height="10" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/></svg>
      </button>
      <button class="window-btn close" title="关闭" @click="closeWindow">
        <svg class="icon icon-sm" viewBox="0 0 12 12"><line x1="2" y1="2" x2="10" y2="10" stroke="currentColor" stroke-width="1.5"/><line x1="10" y1="2" x2="2" y2="10" stroke="currentColor" stroke-width="1.5"/></svg>
      </button>
    </div>
  </header>
</template>
