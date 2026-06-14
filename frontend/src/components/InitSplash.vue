<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  phase: string
  ready: boolean
}>()

const message = computed(() => {
  switch (props.phase) {
    case 'detecting':
      return '正在检测本机已安装的服务…'
    case 'installing':
      return '正在导入运行环境…'
    case 'ready':
      return '准备就绪'
    default:
      return '正在初始化…'
  }
})

const progress = computed(() => {
  switch (props.phase) {
    case 'ready':
      return 100
    case 'detecting':
      return 55
    case 'installing':
      return 80
    default:
      return 25
  }
})
</script>

<template>
  <div class="init-splash">
    <div class="init-splash-card">
      <div class="init-splash-logo">
        <svg viewBox="0 0 48 48" aria-hidden="true">
          <path d="M24 3.8 41.2 13.7v20.1L24 44.2 6.8 34.3V13.7Z" fill="var(--primary)" opacity=".18" />
          <path d="M24 3.8 41.2 13.7v20.1L24 44.2 6.8 34.3V13.7Z" fill="none" stroke="var(--primary)" stroke-width="2" />
          <path d="M15.3 33V15l17.4 18V15" fill="none" stroke="var(--primary)" stroke-width="3.4" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </div>
      <div class="init-splash-title">WinServer</div>
      <div class="init-splash-sub">{{ message }}</div>
      <div class="init-splash-track">
        <div class="init-splash-fill" :style="{ width: progress + '%' }" />
      </div>
      <div class="init-splash-hint">首次启动可能需要几秒钟，请稍候</div>
    </div>
  </div>
</template>

<style scoped>
.init-splash {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg);
  z-index: 3000;
}
.init-splash-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 36px 48px;
  border-radius: var(--radius-lg);
  background: var(--surface-solid);
  box-shadow: var(--shadow);
  border: 1px solid var(--line);
  min-width: 320px;
}
.init-splash-logo svg {
  width: 52px;
  height: 52px;
}
.init-splash-title {
  font-size: 20px;
  font-weight: 760;
  color: var(--text);
  letter-spacing: -.3px;
}
.init-splash-sub {
  font-size: 13px;
  color: var(--text-2);
}
.init-splash-track {
  width: 100%;
  height: 6px;
  border-radius: 99px;
  background: var(--surface-soft);
  overflow: hidden;
  margin-top: 6px;
}
.init-splash-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--primary);
  transition: width .35s ease;
}
.init-splash-hint {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 2px;
}
</style>
