<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useSiteStore } from '../stores/useSiteStore'
import { useServiceStore } from '../stores/useServiceStore'
import { useSettingsStore } from '../stores/useSettingsStore'
import type { SiteInfo } from '../types'
import { useToast } from '../composables/useToast'

const siteStore = useSiteStore()
const serviceStore = useServiceStore()
const settingsStore = useSettingsStore()
const { show } = useToast()
const props = defineProps<{ createSignal?: number }>()

const searchQuery = ref('')
const showModal = ref(false)
const isSaving = ref(false)
const editingId = ref<string | null>(null)

const form = ref({
  port: 8080,
  path: '',
  siteType: 'html' as 'html' | 'php',
  phpRuntimeId: '',
  name: '',
  domain: '',
})

const filteredSites = computed(() => {
  const q = searchQuery.value.toLowerCase()
  if (!q) return siteStore.sites
  return siteStore.sites.filter(s =>
    s.name.toLowerCase().includes(q) ||
    s.domain.toLowerCase().includes(q) ||
    String(s.port).includes(q) ||
    (s.document_root || '').toLowerCase().includes(q),
  )
})

const phpServices = computed(() =>
  serviceStore.services.filter(
    s =>
      s.installed &&
      (s.service_type === 'php' || s.service_type.startsWith('php') || s.id.startsWith('php')),
  ),
)

function displayRoot(path: string) {
  if (!path) return '—'
  return path.replace(/^\\\\\?\\/, '')
}

function isEnabled(site: SiteInfo) {
  return site.status === 'running' || site.status === '正常' || site.status === 'normal'
}

async function toggleSiteEnabled(site: SiteInfo) {
  try {
    if (isEnabled(site)) await siteStore.disableSite(site.id)
    else await siteStore.enableSite(site.id)
  } catch (e) {
    console.error(e)
  }
}

function defaultWwwRoot(port: number) {
  // Prefer absolute app data/www path from settings when available.
  const dataDir = (settingsStore.settings?.data_dir || '').trim()
  const base = dataDir
    ? dataDir.replace(/[/\\]+$/, '')
    : 'data'
  const sep = base.includes('\\') || /^[A-Za-z]:/.test(base) ? '\\' : '/'
  return `${base}${sep}www${sep}site-${port}`
}

function openCreateModal() {
  editingId.value = null
  const port = 8080
  form.value = {
    port,
    path: defaultWwwRoot(port),
    siteType: 'html',
    phpRuntimeId: phpServices.value[0]?.id || '',
    name: '',
    domain: '',
  }
  showModal.value = true
}

function openEditModal(site: SiteInfo) {
  editingId.value = site.id
  form.value = {
    port: site.port,
    path: displayRoot(site.document_root),
    siteType: site.php_runtime_id ? 'php' : 'html',
    phpRuntimeId: site.php_runtime_id || phpServices.value[0]?.id || '',
    name: site.name || '',
    domain: site.domain || '',
  }
  showModal.value = true
}

watch(
  () => props.createSignal,
  (value, oldValue) => {
    if (value && value !== oldValue) openCreateModal()
  },
)

async function browseSitePath() {
  try {
    const selected = await open({ directory: true, title: '选择网站目录' })
    if (selected) form.value.path = selected
  } catch {
    // cancelled
  }
}

async function handleSaveSite() {
  isSaving.value = true
  try {
    const phpId = form.value.siteType === 'php' ? form.value.phpRuntimeId || undefined : undefined
    if (editingId.value) {
      await siteStore.updateSite(
        editingId.value,
        form.value.domain || '',
        Number(form.value.port),
        form.value.path,
        'nginx',
      )
      if (form.value.siteType === 'php' && phpId) {
        await siteStore.switchPhp(editingId.value, phpId)
      }
      show('已保存', '站点信息已更新', 'success')
    } else {
      await siteStore.createSite(
        '',
        Number(form.value.port),
        form.value.path,
        'nginx',
        phpId,
      )
    }
    showModal.value = false
  } catch (e) {
    // store already toasts
  } finally {
    isSaving.value = false
  }
}

async function handleSwitchPhp(siteId: string, event: Event) {
  const target = event.target as HTMLSelectElement
  if (!target.value) return
  await siteStore.switchPhp(siteId, target.value)
}

async function handleDeleteSite(siteId: string, label: string) {
  if (!confirm(`确认删除站点 ${label}？不会删除源代码目录。`)) return
  await siteStore.deleteSite(siteId)
}

async function openSiteUrl(site: SiteInfo) {
  await invoke('open_url', { url: `http://127.0.0.1:${site.port}` })
}

async function openSiteFolder(site: SiteInfo) {
  const path = displayRoot(site.document_root)
  if (!path || path === '—') {
    show('无法打开', '站点运行目录无效', 'warning')
    return
  }
  try {
    await invoke('open_folder', { path })
  } catch (e: any) {
    show('打开失败', typeof e === 'string' ? e : e?.message || String(e), 'error')
  }
}

// Keep default path in sync when port changes during create.
watch(
  () => form.value.port,
  (port) => {
    if (editingId.value) return
    // Only auto-update if path is empty or still a generated www/site-* path
    if (!form.value.path || /[/\\]www[/\\]site-\d+$/i.test(form.value.path)) {
      form.value.path = defaultWwwRoot(Number(port) || 8080)
    }
  },
)

onMounted(async () => {
  await Promise.all([
    siteStore.fetchState(),
    serviceStore.fetchState(),
    settingsStore.fetchSettings(),
  ])
  if (props.createSignal) openCreateModal()
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">网站</h1>
      </div>
      <div class="page-actions">
        <button class="btn primary" @click="openCreateModal">
          <svg class="icon icon-sm"><use href="#i-plus" /></svg>新建站点
        </button>
      </div>
    </div>

    <div class="card">
      <div class="toolbar">
        <div class="search-box">
          <svg class="icon icon-sm"><use href="#i-search" /></svg>
          <input v-model="searchQuery" class="input" placeholder="搜索端口 / 目录" />
        </div>
      </div>

      <div class="data-table-wrap">
        <table class="data-table sites-table">
          <thead>
            <tr>
              <th style="width: 110px">端口</th>
              <th style="width: 72px">类型</th>
              <th>运行目录</th>
              <th style="width: 150px">PHP</th>
              <th style="width: 72px">启用</th>
              <th style="width: 240px">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="site in filteredSites"
              :key="site.id"
              :class="{ 'site-disabled': !isEnabled(site) }"
            >
              <td>
                <div class="table-title">:{{ site.port }}</div>
                <div class="subline">127.0.0.1</div>
              </td>
              <td>
                <span class="type-pill">{{ site.php_runtime_id ? 'PHP' : 'HTML' }}</span>
              </td>
              <td>
                <div class="path-cell" :title="displayRoot(site.document_root)">
                  {{ displayRoot(site.document_root) }}
                </div>
              </td>
              <td>
                <select
                  v-if="site.php_runtime_id"
                  class="select php-select"
                  :value="site.php_runtime_id || ''"
                  :disabled="phpServices.length === 0"
                  @change="handleSwitchPhp(site.id, $event)"
                >
                  <option v-for="service in phpServices" :key="service.id" :value="service.id">
                    {{ service.name }}
                  </option>
                </select>
                <span v-else class="subline">—</span>
              </td>
              <td>
                <div
                  class="switch switch-sm"
                  :class="{ on: isEnabled(site) }"
                  title="启用 / 停用"
                  @click="toggleSiteEnabled(site)"
                />
              </td>
              <td>
                <div class="table-actions">
                  <button class="btn small" @click="openSiteUrl(site)">打开</button>
                  <button class="btn small" @click="openEditModal(site)">管理</button>
                  <button class="btn small" @click="openSiteFolder(site)">目录</button>
                  <button class="btn small danger" @click="handleDeleteSite(site.id, `:${site.port}`)">移除</button>
                </div>
              </td>
            </tr>
            <tr v-if="filteredSites.length === 0">
              <td colspan="6" style="text-align: center; color: var(--text-3); padding: 32px">
                暂无站点
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="table-footer">
        <span>共 {{ siteStore.sites.length }} 个站点</span>
      </div>
    </div>

    <Teleport to="body">
      <div class="overlay" :class="{ show: showModal }" @click.self="showModal = false">
        <div class="modal modal-site">
          <div class="modal-head">
            <div class="modal-title">{{ editingId ? '管理站点' : '新建站点' }}</div>
            <button class="icon-btn" type="button" @click="showModal = false">
              <svg class="icon icon-sm"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="site-form">
              <div class="form-row">
                <label class="form-label">HTTP 端口</label>
                <input v-model.number="form.port" class="input" type="number" min="1" max="65535" />
                <div class="form-hint">访问地址：http://127.0.0.1:{{ form.port || '—' }}</div>
              </div>
              <div class="form-row">
                <label class="form-label">站点类型</label>
                <div class="type-toggle">
                  <button
                    type="button"
                    class="type-btn"
                    :class="{ active: form.siteType === 'html' }"
                    @click="form.siteType = 'html'"
                  >HTML 静态</button>
                  <button
                    type="button"
                    class="type-btn"
                    :class="{ active: form.siteType === 'php' }"
                    @click="form.siteType = 'php'"
                  >PHP</button>
                </div>
              </div>
              <div v-if="form.siteType === 'php'" class="form-row">
                <label class="form-label">PHP 版本</label>
                <select v-model="form.phpRuntimeId" class="select">
                  <option value="">自动选择</option>
                  <option v-for="service in phpServices" :key="service.id" :value="service.id">
                    {{ service.name }} · {{ service.port }}
                  </option>
                </select>
                <div v-if="phpServices.length === 0" class="form-hint warn">
                  尚未安装 PHP，请先到软件页一键安装。
                </div>
              </div>
              <div class="form-row">
                <label class="form-label">网站目录</label>
                <div class="path-row">
                  <input
                    v-model="form.path"
                    class="input"
                    placeholder="默认：应用 data/www/site-端口"
                  />
                  <button class="btn" type="button" @click="browseSitePath">浏览</button>
                </div>
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" @click="showModal = false">取消</button>
            <button
              class="btn primary"
              type="button"
              :disabled="isSaving || (form.siteType === 'php' && phpServices.length === 0 && !editingId)"
              @click="handleSaveSite"
            >
              {{ isSaving ? '保存中' : editingId ? '保存' : '创建' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.site-disabled { opacity: 0.55; }
.subline { font-size: 11px; color: var(--text-3); margin-top: 2px; }
.path-cell {
  max-width: 360px;
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.type-pill {
  display: inline-flex;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 700;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  color: var(--text-2);
}
.php-select { height: 32px; max-width: 140px; }

/* Fix switch thumb alignment (global uses ::before, not ::after) */
.switch-sm {
  width: 36px;
  height: 20px;
  min-width: 36px;
  padding: 2px;
  box-sizing: border-box;
  flex: none;
}
.switch-sm::before {
  width: 16px !important;
  height: 16px !important;
}
.switch-sm.on::before {
  transform: translateX(16px) !important;
}

.modal-site { width: min(440px, calc(100vw - 40px)); }
.site-form { display: flex; flex-direction: column; gap: 14px; }
.site-form .form-row { display: flex; flex-direction: column; gap: 6px; }
.site-form .input,
.site-form .select { width: 100%; box-sizing: border-box; }
.type-toggle { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.type-btn {
  height: 38px; border-radius: 10px; border: 1px solid var(--line);
  background: var(--surface-solid); color: var(--text-2); font: inherit; font-weight: 650; cursor: pointer;
}
.type-btn.active {
  border-color: var(--primary); color: var(--primary); background: var(--primary-soft);
}
.path-row { display: flex; gap: 8px; }
.path-row .input { flex: 1; min-width: 0; }
.form-hint { font-size: 11px; color: var(--text-3); }
.form-hint.warn { color: var(--warning); }
.form-label { font-size: 12px; font-weight: 650; color: var(--text-2); }
</style>
