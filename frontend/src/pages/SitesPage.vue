<script setup lang="ts">
import { ref, computed, onMounted, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useSiteStore } from '../stores/useSiteStore'
import { useServiceStore } from '../stores/useServiceStore'
import type { SiteInfo } from '../types'

const siteStore = useSiteStore()
const serviceStore = useServiceStore()
const openConfigEditor = inject<(fileId: string, label: string, filePath: string) => void>('openConfigEditor')!
const searchQuery = ref('')
const showCreateModal = ref(false)
const isCreating = ref(false)
const createForm = ref({
  domain: '',
  port: 80,
  path: '',
  phpRuntimeId: '',
})

const filteredSites = computed(() => {
  const q = searchQuery.value.toLowerCase()
  if (!q) return siteStore.sites
  return siteStore.sites.filter(s =>
    s.name.toLowerCase().includes(q) ||
    s.domain.toLowerCase().includes(q)
  )
})

const phpServices = computed(() =>
  serviceStore.services.filter(service => service.service_type === 'php' && service.installed)
)

function statusText(status: string) {
  if (status === 'running') return '运行中'
  if (status === 'stopped') return '已停止'
  if (status === 'normal') return '正常'
  if (status === 'disabled') return '已停用'
  return status
}

async function toggleSiteEnabled(site: SiteInfo) {
  const isEnabled = site.status === 'running' || site.status === '正常'
  try {
    if (isEnabled) {
      await siteStore.disableSite(site.id)
    } else {
      await siteStore.enableSite(site.id)
    }
  } catch (e: any) {
    console.error('Toggle site enabled failed:', e)
  }
}

async function startAll() {
  // Batch start handled by service store
}

async function stopAll() {
  // Batch stop handled by service store
}

function openCreateModal() {
  createForm.value = {
    domain: '',
    port: 80,
    path: '',
    phpRuntimeId: phpServices.value[0]?.id || '',
  }
  showCreateModal.value = true
}

async function handleCreateSite() {
  isCreating.value = true
  try {
    await siteStore.createSite(
      createForm.value.domain,
      Number(createForm.value.port),
      createForm.value.path,
      'nginx',
      createForm.value.phpRuntimeId || undefined,
    )
    showCreateModal.value = false
  } finally {
    isCreating.value = false
  }
}

async function handleSwitchPhp(siteId: string, event: Event) {
  const target = event.target as HTMLSelectElement
  if (!target.value) return
  await siteStore.switchPhp(siteId, target.value)
}

async function handleDeleteSite(siteId: string, domain: string) {
  if (!confirm(`只会删除管理记录、Nginx 配置和 hosts 记录，不会删除源代码目录。\n确认删除 ${domain}？`)) {
    return
  }
  await siteStore.deleteSite(siteId)
}

async function openSiteUrl(site: SiteInfo) {
  const protocol = site.ssl ? 'https' : 'http'
  await invoke('open_url', { url: `${protocol}://${site.domain}:${site.port}` })
}

async function openSiteFolder(site: SiteInfo) {
  await invoke('open_folder', { path: site.document_root })
}

async function openSiteConfig(site: SiteInfo) {
  try {
    const result = await invoke<{ id: string; label: string; path: string; content: string }>('get_config_file', { fileId: `site:${site.id}` })
    openConfigEditor(result.id, result.label, result.path)
  } catch {
    openConfigEditor(`site:${site.id}`, `${site.domain} 配置`, '')
  }
}

onMounted(() => {
  siteStore.fetchState()
  serviceStore.fetchState()
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
        <button class="btn small" @click="startAll">
          <svg class="icon icon-sm"><use href="#i-play" /></svg>全部启动
        </button>
        <button class="btn small" @click="stopAll">
          <svg class="icon icon-sm"><use href="#i-stop" /></svg>全部停止
        </button>
        <div class="search-box">
          <svg class="icon icon-sm"><use href="#i-search" /></svg>
          <input v-model="searchQuery" class="input" placeholder="搜索站点" />
        </div>
      </div>

      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>站点名称</th>
              <th>域名</th>
              <th>端口</th>
              <th>协议</th>
              <th>PHP</th>
              <th>状态</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(site, i) in filteredSites" :key="site.id || i" :class="{ 'site-disabled': site.status === 'disabled' }">
              <td>
                <div class="table-title">{{ site.name || site.domain }}</div>
              </td>
              <td>{{ site.domain }}</td>
              <td>{{ site.port }}</td>
              <td>
                <template v-if="site.ssl">
                  <svg class="icon icon-sm"><use href="#i-lock" /></svg> HTTPS
                </template>
                <template v-else>HTTP</template>
              </td>
              <td>
                <select
                  class="select"
                  style="height: 31px; max-width: 160px"
                  :value="site.php_runtime_id || ''"
                  :disabled="phpServices.length === 0"
                  @change="handleSwitchPhp(site.id, $event)"
                >
                  <option value="">未选择</option>
                  <option v-for="service in phpServices" :key="service.id" :value="service.id">
                    {{ service.name }}
                  </option>
                </select>
              </td>
              <td>
                <div class="site-status-row">
                  <span
                    class="badge"
                    :class="site.status === 'running' || site.status === '正常' ? 'success' : site.status === 'disabled' ? 'warning' : 'warning'"
                  >
                    <span
                      class="status-dot"
                      :class="site.status === 'running' || site.status === '正常' ? 'running' : 'stopped'"
                    />
                    {{ statusText(site.status) }}
                  </span>
                  <div
                    class="switch switch-sm"
                    :class="{ on: site.status === 'running' || site.status === '正常' }"
                    @click="toggleSiteEnabled(site)"
                    title="启用/停用站点"
                  />
                </div>
              </td>
              <td>
                <div class="table-actions">
                  <button class="btn small" @click="openSiteUrl(site)">打开</button>
                  <button class="btn small" @click="openSiteFolder(site)">目录</button>
                  <button class="btn small" @click="openSiteConfig(site)">配置</button>
                  <button class="btn small danger" @click="handleDeleteSite(site.id, site.domain)">移除</button>
                </div>
              </td>
            </tr>
            <tr v-if="filteredSites.length === 0">
              <td colspan="7" style="text-align: center; color: var(--text-3); padding: 32px;">
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
      <div class="overlay" :class="{ show: showCreateModal }" @click.self="showCreateModal = false">
        <div class="modal">
          <div class="modal-head">
            <div class="modal-title">新建站点</div>
            <button class="icon-btn" @click="showCreateModal = false">
              <svg class="icon icon-sm"><use href="#i-stop" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-grid">
              <div class="form-row">
                <label class="form-label">域名</label>
                <input v-model="createForm.domain" class="input" placeholder="demo.local" />
              </div>
              <div class="form-row">
                <label class="form-label">HTTP 端口</label>
                <input v-model.number="createForm.port" class="input" type="number" min="1" max="65535" />
              </div>
              <div class="form-row full">
                <label class="form-label">网站目录</label>
                <input v-model="createForm.path" class="input" placeholder="D:\Projects\demo\public" />
                <div class="form-hint">目录必须已经存在，创建站点不会删除或覆盖源代码。</div>
              </div>
              <div class="form-row full">
                <label class="form-label">PHP 版本</label>
                <select v-model="createForm.phpRuntimeId" class="select">
                  <option value="">自动选择已导入 PHP</option>
                  <option v-for="service in phpServices" :key="service.id" :value="service.id">
                    {{ service.name }} · 端口 {{ service.port }}
                  </option>
                </select>
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" @click="showCreateModal = false">取消</button>
            <button class="btn primary" :disabled="isCreating" @click="handleCreateSite">
              {{ isCreating ? '创建中' : '创建站点' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.site-disabled {
  opacity: 0.55;
}
.site-status-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.switch-sm {
  width: 32px;
  height: 18px;
  min-width: 32px;
}
.switch-sm::after {
  width: 14px;
  height: 14px;
}
</style>
