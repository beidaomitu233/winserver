<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const props = defineProps<{
  visible: boolean
  service: { id: string; name: string; config_file: string | null } | null
}>()

const emit = defineEmits<{ close: [] }>()

const { show } = useToast()
const activeTab = ref<'visual' | 'file'>('visual')

// ---- Visual config state (Redis) ----
const redisForm = ref({
  port: 6379,
  bind: '127.0.0.1',
  password: '',
  maxmemory: '256mb',
  maxmemory_policy: 'allkeys-lru',
  appendonly: false,
  protected_mode: true,
})

// ---- Visual config state (MinIO) ----
const minioForm = ref({
  root_user: 'minioadmin',
  root_password: 'minioadmin',
  api_port: 9000,
  console_port: 9001,
})

// ---- File editor state ----
const fileLabel = ref('')
const filePath = ref('')
const fileContent = ref('')
const fileOriginal = ref('')
const isLoading = ref(false)
const isSaving = ref(false)
const hasFileChanges = computed(() => fileContent.value !== fileOriginal.value)

const serviceId = computed(() => props.service?.id || '')

async function loadConfig() {
  if (!props.service) return
  const id = props.service.id
  activeTab.value = 'visual'

  if (id === 'redis') {
    fileLabel.value = 'redis.conf'
    try {
      const cfg = await invoke<Record<string, any>>('redis_config_get')
      redisForm.value = {
        port: Number(cfg.port) || 6379,
        bind: String(cfg.bind || '127.0.0.1'),
        password: String(cfg.password || ''),
        maxmemory: String(cfg.maxmemory || '256mb'),
        maxmemory_policy: String(cfg.maxmemory_policy || 'allkeys-lru'),
        appendonly: Boolean(cfg.appendonly),
        protected_mode: cfg.protected_mode !== false,
      }
      filePath.value = String(cfg.path || '')
      await loadFileText('redis.conf')
    } catch (e: any) {
      show('加载失败', String(e?.message || e), 'error')
    }
  } else if (id === 'minio') {
    fileLabel.value = 'minio.env'
    try {
      const cfg = await invoke<Record<string, any>>('minio_config_get')
      minioForm.value = {
        root_user: String(cfg.root_user || 'minioadmin'),
        root_password: String(cfg.root_password || 'minioadmin'),
        api_port: Number(cfg.api_port) || 9000,
        console_port: Number(cfg.console_port) || 9001,
      }
      filePath.value = String(cfg.path || '')
      await loadFileText('minio.env')
    } catch (e: any) {
      show('加载失败', String(e?.message || e), 'error')
    }
  }
}

async function loadFileText(fileId: string) {
  isLoading.value = true
  try {
    const result = await invoke<{ content: string; path: string }>('get_config_file', { fileId })
    fileContent.value = result.content
    fileOriginal.value = result.content
    if (result.path) filePath.value = result.path
  } catch (e: any) {
    fileContent.value = ''
    fileOriginal.value = ''
  } finally {
    isLoading.value = false
  }
}

watch(() => props.visible, (val) => {
  if (val) loadConfig()
})

async function saveVisual() {
  isSaving.value = true
  try {
    if (serviceId.value === 'redis') {
      const params = JSON.stringify(redisForm.value)
      await invoke('redis_config_save', { params })
    } else if (serviceId.value === 'minio') {
      const params = JSON.stringify(minioForm.value)
      await invoke('minio_config_save', { params })
    }
    show('保存成功', `${props.service?.name || ''} 配置已保存，重启后生效`, 'success')
    // Refresh the file view so it reflects the rendered config.
    await loadFileText(serviceId.value === 'redis' ? 'redis.conf' : 'minio.env')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

async function saveFile() {
  if (!serviceId.value) return
  isSaving.value = true
  try {
    const fileId = serviceId.value === 'redis' ? 'redis.conf' : 'minio.env'
    await invoke('save_config_file', { fileId, content: fileContent.value })
    fileOriginal.value = fileContent.value
    show('保存成功', `${fileLabel.value} 已保存，重启后生效`, 'success')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

function handleClose() {
  if (activeTab.value === 'file' && hasFileChanges.value && !confirm('配置文件已修改但未保存，确定关闭？')) return
  emit('close')
}

const policies = [
  'noeviction',
  'allkeys-lru',
  'volatile-lru',
  'allkeys-random',
  'volatile-random',
  'volatile-ttl',
  'allkeys-lfu',
  'volatile-lfu',
]
</script>

<template>
  <div class="overlay" :class="{ show: visible }" @click.self="handleClose">
    <div class="modal modal-lg service-config-modal">
      <div class="modal-head">
        <div class="modal-title">配置 · {{ service?.name || '' }}</div>
        <div class="config-tabs">
          <button class="config-tab" :class="{ active: activeTab === 'visual' }" @click="activeTab = 'visual'">可视化配置</button>
          <button class="config-tab" :class="{ active: activeTab === 'file' }" @click="activeTab = 'file'">直接编辑文件</button>
        </div>
        <button class="icon-btn" @click="handleClose">
          <svg class="icon icon-sm"><use href="#i-stop" /></svg>
        </button>
      </div>

      <div class="modal-body">
        <!-- Visual config -->
        <div v-if="activeTab === 'visual'">
          <!-- Redis form -->
          <div v-if="serviceId === 'redis'" class="config-form">
            <div class="config-field">
              <label>端口</label>
              <input v-model.number="redisForm.port" type="number" min="1" max="65535" class="input" />
            </div>
            <div class="config-field">
              <label>绑定地址 (bind)</label>
              <input v-model="redisForm.bind" class="input" />
            </div>
            <div class="config-field">
              <label>密码 (requirepass)</label>
              <input v-model="redisForm.password" type="password" class="input" placeholder="留空则不启用密码" />
            </div>
            <div class="config-field">
              <label>最大内存 (maxmemory)</label>
              <input v-model="redisForm.maxmemory" class="input" placeholder="如 256mb" />
            </div>
            <div class="config-field">
              <label>淘汰策略 (maxmemory-policy)</label>
              <select v-model="redisForm.maxmemory_policy" class="input">
                <option v-for="p in policies" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>
            <div class="config-field config-field-inline">
              <label>AOF 持久化 (appendonly)</label>
              <div class="switch" :class="{ on: redisForm.appendonly }" @click="redisForm.appendonly = !redisForm.appendonly" />
            </div>
            <div class="config-field config-field-inline">
              <label>保护模式 (protected-mode)</label>
              <div class="switch" :class="{ on: redisForm.protected_mode }" @click="redisForm.protected_mode = !redisForm.protected_mode" />
            </div>
          </div>

          <!-- MinIO form -->
          <div v-else-if="serviceId === 'minio'" class="config-form">
            <div class="config-field">
              <label>API 端口</label>
              <input v-model.number="minioForm.api_port" type="number" min="1" max="65535" class="input" />
            </div>
            <div class="config-field">
              <label>控制台端口</label>
              <input v-model.number="minioForm.console_port" type="number" min="1" max="65535" class="input" />
            </div>
            <div class="config-field">
              <label>Root 用户名</label>
              <input v-model="minioForm.root_user" class="input" />
            </div>
            <div class="config-field">
              <label>Root 密码</label>
              <input v-model="minioForm.root_password" type="password" class="input" />
            </div>
          </div>

          <div v-else class="config-empty">该服务暂不支持可视化配置，请使用「直接编辑文件」。</div>
        </div>

        <!-- File editor -->
        <div v-else class="config-editor">
          <div class="config-editor-path">{{ filePath }}</div>
          <div v-if="isLoading" style="padding: 20px; color: var(--text-3)">加载中...</div>
          <textarea
            v-else
            v-model="fileContent"
            class="config-editor-textarea"
            spellcheck="false"
            placeholder="配置文件内容"
          />
        </div>
      </div>

      <div class="modal-foot">
        <span v-if="activeTab === 'file' && hasFileChanges" class="config-editor-unsaved">有未保存的更改</span>
        <div style="flex: 1" />
        <button class="btn" @click="handleClose">关闭</button>
        <button
          v-if="activeTab === 'visual'"
          class="btn primary"
          :disabled="isSaving"
          @click="saveVisual"
        >
          {{ isSaving ? '保存中...' : '保存' }}
        </button>
        <button
          v-else
          class="btn primary"
          :disabled="isSaving || !hasFileChanges"
          @click="saveFile"
        >
          {{ isSaving ? '保存中...' : '保存' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.service-config-modal .config-tabs {
  display: flex;
  gap: 4px;
  margin: 0 auto;
}
.config-tab {
  border: 1px solid var(--line);
  background: transparent;
  color: var(--text-2);
  padding: 5px 12px;
  border-radius: 8px;
  font: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: .14s ease;
}
.config-tab:hover { color: var(--primary); border-color: var(--primary); }
.config-tab.active { background: var(--primary); color: #fff; border-color: var(--primary); }

.config-form {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px 18px;
}
.config-field { display: flex; flex-direction: column; gap: 5px; }
.config-field label { font-size: 12px; color: var(--text-2); font-weight: 600; }
.config-field-inline {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}
.config-empty { color: var(--text-3); padding: 20px 0; text-align: center; }

.config-editor-path {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 8px;
  word-break: break-all;
}
.config-editor-textarea {
  width: 100%;
  min-height: 360px;
  max-height: 55vh;
  font-family: 'Cascadia Code', 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface-soft);
  color: var(--text);
  resize: vertical;
  tab-size: 4;
}
.config-editor-textarea:focus { outline: none; border-color: var(--primary); }
.config-editor-unsaved { font-size: 12px; color: var(--warning); }
</style>
