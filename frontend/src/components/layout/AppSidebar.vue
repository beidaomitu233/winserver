<script setup lang="ts">
import { inject, computed, type Ref } from 'vue'
import { useServiceStore } from '../../stores/useServiceStore'
import type { SystemResource } from '../../types'

const currentPage = inject<Ref<string>>('currentPage')!
const systemResource = inject<Ref<SystemResource>>('systemResource')!
const serviceStore = useServiceStore()

const runningCount = computed(() => serviceStore.runningCount)
const cpuPercent = computed(() => Math.max(0, Math.min(100, Math.round(systemResource.value.cpu_percent))))
const memoryPercent = computed(() => Math.max(0, Math.min(100, Math.round(systemResource.value.memory_percent))))
const diskPercent = computed(() => Math.max(0, Math.min(100, Math.round(systemResource.value.disk?.percent ?? 0))))
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-mark">WS</div>
      <div><div class="brand-name">云栈</div></div>
    </div>

    <nav class="nav-list" id="navList">
      <button class="nav-item" :class="{ active: currentPage === 'dashboard' }" @click="currentPage = 'dashboard'">
        <svg class="icon"><use href="#i-home"></use></svg><span>首页</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'sites' }" @click="currentPage = 'sites'">
        <svg class="icon"><use href="#i-globe"></use></svg><span>网站</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'database' }" @click="currentPage = 'database'">
        <svg class="icon"><use href="#i-database"></use></svg><span>数据库</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'software' }" @click="currentPage = 'software'">
        <svg class="icon"><use href="#i-grid"></use></svg><span>软件</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'files' }" @click="currentPage = 'files'">
        <svg class="icon"><use href="#i-folder"></use></svg><span>文件</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'logs' }" @click="currentPage = 'logs'">
        <svg class="icon"><use href="#i-file"></use></svg><span>日志</span>
      </button>
    </nav>

    <div class="sidebar-spacer"></div>
    <nav class="nav-list">
      <button class="nav-item" data-action="open-terminal">
        <svg class="icon"><use href="#i-terminal"></use></svg><span>终端</span>
      </button>
      <button class="nav-item" :class="{ active: currentPage === 'settings' }" @click="currentPage = 'settings'">
        <svg class="icon"><use href="#i-settings"></use></svg><span>设置</span>
      </button>
    </nav>

    <div class="sidebar-spacer"></div>
    <div class="mini-monitor">
      <div class="mini-monitor-head">
        <div class="mini-monitor-title"><span class="live-dot"></span><span>运行中</span></div>
        <span style="color:var(--text-3);font-size:11px">{{ runningCount }} 项</span>
      </div>
      <div class="mini-row"><span>CPU</span><div class="bar"><i :style="{ width: cpuPercent + '%' }"></i></div><span>{{ cpuPercent }}%</span></div>
      <div class="mini-row"><span>内存</span><div class="bar green"><i :style="{ width: memoryPercent + '%' }"></i></div><span>{{ memoryPercent }}%</span></div>
      <div class="mini-row"><span>磁盘</span><div class="bar purple"><i :style="{ width: diskPercent + '%' }"></i></div><span>{{ diskPercent }}%</span></div>
    </div>
  </aside>
</template>
