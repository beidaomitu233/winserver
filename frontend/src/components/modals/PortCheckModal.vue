<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { PortCheckResult } from '../../types'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const port = ref<number | string>('')
const result = ref<PortCheckResult | null>(null)
const checking = ref(false)
const errorMessage = ref('')

async function checkPort() {
  const portNum = Number(port.value)
  if (!portNum || portNum < 1 || portNum > 65535) return
  checking.value = true
  result.value = null
  errorMessage.value = ''
  try {
    result.value = await invoke<PortCheckResult>('check_port', { port: portNum })
  } catch (error) {
    errorMessage.value = typeof error === 'string' ? error : '端口检测失败'
  } finally {
    checking.value = false
  }
}

function ownerText(value: PortCheckResult) {
  if (value.owner_type === 'winserver_service' && value.owner_id) return `WinServer 服务：${value.owner_id}`
  if (value.process_name) return `外部进程：${value.process_name}`
  return '占用来源未知'
}
</script>

<template>
  <div v-if="props.visible" class="overlay show" @click.self="emit('close')">
    <div class="modal" style="max-width: 420px;">
      <div class="modal-head">
        <div class="modal-title">端口检测</div>
        <button class="btn-icon" @click="emit('close')">
          <svg class="icon"><use href="#i-close" /></svg>
        </button>
      </div>
      <div class="modal-body">
        <div class="form-group">
          <label>端口号</label>
          <div style="display: flex; gap: 8px;">
            <input
              v-model="port"
              class="input"
              type="number"
              min="1"
              max="65535"
              placeholder="输入端口号"
              style="flex: 1"
              @keyup.enter="checkPort"
            />
            <button class="btn primary" :disabled="checking" @click="checkPort">
              {{ checking ? '检测中' : '检测' }}
            </button>
          </div>
        </div>
        <div v-if="errorMessage" class="port-result">
          <div class="port-result-badge danger">
            <span class="status-dot stopped" />
            {{ errorMessage }}
          </div>
        </div>
        <div v-if="result" class="port-result">
          <template v-if="!result.available">
            <div class="port-result-badge danger">
              <span class="status-dot stopped" />
              端口 {{ result.port }} 已被占用
            </div>
            <div class="port-result-detail">{{ ownerText(result) }}</div>
            <div v-if="result.pid" class="port-result-detail">
              进程 PID: {{ result.pid }}
              <span v-if="result.process_name">({{ result.process_name }})</span>
            </div>
          </template>
          <template v-else>
            <div class="port-result-badge success">
              <span class="status-dot running" />
              端口 {{ result.port }} 可用
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.port-result {
  margin-top: 16px;
  padding: 12px;
  border-radius: 8px;
  background: var(--surface-soft);
}
.port-result-badge {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 600;
  font-size: 14px;
}
.port-result-badge.danger {
  color: var(--danger, #ef4444);
}
.port-result-badge.success {
  color: var(--success, #22a95a);
}
.port-result-detail {
  margin-top: 8px;
  font-size: 13px;
  color: var(--text-2);
}
</style>
