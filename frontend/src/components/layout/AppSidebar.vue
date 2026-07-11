<script setup lang="ts">
import { inject, type Ref } from 'vue'

const currentPage = inject<Ref<string>>('currentPage')!
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
      <button class="nav-item" :class="{ active: currentPage === 'logs' }" @click="currentPage = 'logs'">
        <svg class="icon"><use href="#i-file"></use></svg><span>日志</span>
      </button>
    </nav>

    <div class="sidebar-spacer"></div>

    <nav class="nav-list">
      <button class="nav-item" :class="{ active: currentPage === 'settings' }" @click="currentPage = 'settings'">
        <svg class="icon"><use href="#i-settings"></use></svg><span>设置</span>
      </button>
    </nav>
  </aside>
</template>
