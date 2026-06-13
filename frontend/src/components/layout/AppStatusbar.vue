<script setup lang="ts">
import { useServiceStore } from '../../stores/useServiceStore'

const serviceStore = useServiceStore()
</script>

<template>
  <footer class="statusbar">
    <div class="status-left">
      <span class="status-running">
        <span class="live-dot"></span>
        <span>{{ serviceStore.runningCount }} 项服务在线</span>
      </span>
    </div>
    <div class="status-right">
      <span class="status-link">检查更新</span>
    </div>
  </footer>
</template>
