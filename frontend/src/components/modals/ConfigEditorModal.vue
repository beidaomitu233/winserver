<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../../composables/useToast'

const props = defineProps<{
  visible: boolean
  fileId: string
  label: string
  filePath: string
}>()

const emit = defineEmits<{
  close: []
}>()

const { show } = useToast()
const content = ref('')
const originalContent = ref('')
const isLoading = ref(false)
const isSaving = ref(false)

watch(() => props.visible, async (val) => {
  if (val && props.fileId) {
    isLoading.value = true
    try {
      const result = await invoke<{ content: string; exists: boolean }>('get_config_file', { fileId: props.fileId })
      content.value = result.content
      originalContent.value = result.content
    } catch (e: any) {
      show('加载失败', String(e?.message || e), 'error')
    } finally {
      isLoading.value = false
    }
  }
})

const hasChanges = ref(false)
watch(content, (val) => {
  hasChanges.value = val !== originalContent.value
})

async function handleSave() {
  isSaving.value = true
  try {
    await invoke('save_config_file', { fileId: props.fileId, content: content.value })
    originalContent.value = content.value
    hasChanges.value = false
    show('保存成功', `${props.label} 已保存`, 'success')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

function handleClose() {
  if (hasChanges.value && !confirm('配置已修改但未保存，确定关闭？')) return
  emit('close')
}
</script>

<template>
  <div class="overlay" :class="{ show: visible }" @click.self="handleClose">
    <div class="modal modal-lg">
      <div class="modal-head">
        <div class="modal-title">编辑配置 · {{ label }}</div>
        <button class="icon-btn" @click="handleClose">
          <svg class="icon icon-sm"><use href="#i-stop" /></svg>
        </button>
      </div>
      <div class="modal-body">
        <div class="config-editor-path">{{ filePath }}</div>
        <div v-if="isLoading" style="padding: 20px; color: var(--text-3)">加载中...</div>
        <textarea
          v-else
          v-model="content"
          class="config-editor-textarea"
          spellcheck="false"
          placeholder="配置文件内容"
        />
      </div>
      <div class="modal-foot">
        <span v-if="hasChanges" class="config-editor-unsaved">有未保存的更改</span>
        <div style="flex: 1" />
        <button class="btn" @click="handleClose">关闭</button>
        <button class="btn primary" :disabled="isSaving || !hasChanges" @click="handleSave">
          {{ isSaving ? '保存中...' : '保存' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-editor-path {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 8px;
  word-break: break-all;
}
.config-editor-textarea {
  width: 100%;
  min-height: 420px;
  max-height: 60vh;
  font-family: 'Cascadia Code', 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-1);
  color: var(--text-1);
  resize: vertical;
  tab-size: 4;
}
.config-editor-textarea:focus {
  outline: none;
  border-color: var(--primary);
}
.config-editor-unsaved {
  font-size: 12px;
  color: var(--warning, #f59e0b);
}
</style>
