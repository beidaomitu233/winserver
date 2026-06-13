<script setup lang="ts">
import { inject, type Ref } from 'vue'

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
  </header>
</template>
