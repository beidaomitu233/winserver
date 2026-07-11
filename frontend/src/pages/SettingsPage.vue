<script setup lang="ts">
import { ref, onMounted, inject } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useServiceStore } from '../stores/useServiceStore'
import { useSettingsStore } from '../stores/useSettingsStore'
import { useDatabaseStore } from '../stores/useDatabaseStore'
import { useToast } from '../composables/useToast'

const icon = (name: string, cls = '') => `<svg class="icon ${cls}"><use href="#i-${name}"></use></svg>`

const serviceStore = useServiceStore()
const settingsStore = useSettingsStore()
const databaseStore = useDatabaseStore()
const openConfigEditor = inject<(fileId: string, label: string, filePath: string) => void>('openConfigEditor')!
const openPortCheck = inject<() => void>('openPortCheck')!
const { show } = useToast()

const activeSection = ref('general')

// Security section state
const mysqlCurrentPass = ref('')
const mysqlNewPass = ref('')
const hostsPermissionResult = ref('')
const hostsPermissionOk = ref(false)

// Backup section state
const dbBackups = ref<Array<{ name: string; path: string; size: number; modified: number }>>([])

const menuItems: Array<[string, string, string]> = [
  ['general', '通用', 'settings'],
  ['network', '网络', 'network'],
  ['security', '安全', 'shield'],
  ['backup', '备份', 'backup'],
]

const titleMap: Record<string, string> = {
  general: '通用设置',
  network: '端口与网络',
  security: '安全设置',
  backup: '备份策略',
}

async function toggleAutostart() {
  const next = !settingsStore.settings.autostart
  await autoSave({ autostart: next })
}

async function toggleStartSuite() {
  const next = !settingsStore.settings.start_suite_on_launch
  await autoSave({ start_suite_on_launch: next })
}

async function autoSave(updates: Record<string, unknown>) {
  try {
    await settingsStore.updateSettings(updates as any)
    show('已保存', '设置已更新', 'success')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  }
}

let saveTimer: number | null = null
function debounceSave(updates: Record<string, unknown>) {
  if (saveTimer) window.clearTimeout(saveTimer)
  saveTimer = window.setTimeout(() => autoSave(updates), 400)
}

async function changeRootPassword() {
  try {
    await databaseStore.rootPassword(mysqlCurrentPass.value, mysqlNewPass.value)
    show('密码已修改', 'MySQL root 密码已更新', 'success')
    mysqlCurrentPass.value = ''
    mysqlNewPass.value = ''
  } catch (e: any) {
    show('修改失败', String(e?.message || e), 'error')
  }
}

async function checkHostsPermission() {
  try {
    const hostsPath = 'C:\\Windows\\System32\\drivers\\etc\\hosts'
    await invoke('open_file', { path: hostsPath })
    hostsPermissionResult.value = '可以访问 hosts 文件'
    hostsPermissionOk.value = true
  } catch (e: any) {
    hostsPermissionResult.value = '无法访问 hosts 文件：' + String(e?.message || e)
    hostsPermissionOk.value = false
  }
}

async function openDataDir() {
  const dir = settingsStore.settings.data_dir || 'data'
  try {
    await invoke('open_folder', { path: dir })
  } catch (e: any) {
    show('打开失败', String(e?.message || e), 'error')
  }
}

async function exportSettings() {
  try {
    const settings = await invoke('get_settings')
    const blob = new Blob([JSON.stringify(settings, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'winserver-settings.json'
    a.click()
    URL.revokeObjectURL(url)
    show('导出成功', '设置已导出', 'success')
  } catch (e: any) {
    show('导出失败', String(e?.message || e), 'error')
  }
}

async function importSettings() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    try {
      const text = await file.text()
      const settings = JSON.parse(text)
      await invoke('update_settings', { params: JSON.stringify(settings) })
      await settingsStore.fetchSettings()
      show('导入成功', '设置已恢复', 'success')
    } catch (e: any) {
      show('导入失败', String(e?.message || e), 'error')
    }
  }
  input.click()
}

async function loadBackups() {
  try {
    const result = await invoke<{ backups: Array<{ name: string; path: string; size: number; modified: number }> }>('db_backups')
    dbBackups.value = result.backups || []
  } catch {
    dbBackups.value = []
  }
}

async function deleteBackup(path: string) {
  if (!confirm('确认删除此备份？')) return
  try {
    await invoke('db_delete_backup', { path })
    show('已删除', '备份已删除', 'success')
    await loadBackups()
  } catch (e: any) {
    show('删除失败', String(e?.message || e), 'error')
  }
}

function formatBackupSize(bytes: number) {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB'
}

function formatBackupTime(ms: number) {
  if (!ms) return '—'
  return new Date(ms).toLocaleString()
}

onMounted(() => {
  serviceStore.fetchState()
  settingsStore.fetchSettings()
  loadBackups()
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">设置</h1>
      </div>
      <div class="page-actions">
        <span style="font-size: 12px; color: var(--text-3)">修改后自动保存</span>
      </div>
    </div>

    <div class="settings-layout">
      <!-- Sidebar menu -->
      <div class="card settings-menu">
        <div
          v-for="item in menuItems"
          :key="item[0]"
          class="settings-menu-item"
          :class="{ active: activeSection === item[0] }"
          @click="activeSection = item[0]"
          v-html="icon(item[2], 'icon-sm') + item[1]"
        />
      </div>

      <!-- Settings panel -->
      <div class="card">
        <!-- General -->
        <template v-if="activeSection === 'general'">
          <div class="card-head">
            <div class="card-title">{{ titleMap.general }}</div>
            <span class="badge primary">本机配置</span>
          </div>
          <div class="settings-section">
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">开机启动</div>
                  <div class="setting-desc">登录 Windows 后自动启动 WinServer 云栈</div>
                </div>
                <div
                  class="switch"
                  :class="{ on: settingsStore.settings.autostart }"
                  @click="toggleAutostart"
                />
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">启动时自动启动核心服务</div>
                  <div class="setting-desc">启动 WinServer 时自动运行 Nginx、MySQL 与 PHP</div>
                </div>
                <div
                  class="switch"
                  :class="{ on: settingsStore.settings.start_suite_on_launch }"
                  @click="toggleStartSuite"
                />
              </div>
            </div>
          </div>
        </template>

        <!-- Network -->
        <template v-else-if="activeSection === 'network'">
          <div class="card-head">
            <div class="card-title">{{ titleMap.network }}</div>
            <span class="badge primary">本机配置</span>
          </div>
          <div class="settings-section">
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">默认 HTTP 端口</div>
                  <div class="setting-desc">新建站点时的默认端口</div>
                </div>
                <input
                  class="input"
                  type="number"
                  min="1"
                  max="65535"
                  :value="settingsStore.settings.port || 80"
                  style="width: 100px"
                  @input="debounceSave({ port: Number(($event.target as HTMLInputElement).value) })"
                />
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">端口检测</div>
                  <div class="setting-desc">检测端口是否被占用</div>
                </div>
                <button class="btn small" @click="openPortCheck">
                  <svg class="icon icon-sm"><use href="#i-network" /></svg>检测端口
                </button>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-name" style="margin-bottom: 8px">配置文件</div>
              <div v-if="serviceStore.configFiles.length === 0" style="color: var(--text-3); font-size: 13px">
                暂无配置文件，请先导入运行环境
              </div>
              <div v-for="file in serviceStore.configFiles" :key="file.id" class="setting-row config-file-row">
                <div>
                  <div class="setting-name">{{ file.label }}</div>
                  <div class="setting-desc">{{ file.path }}</div>
                </div>
                <div class="config-file-actions">
                  <span v-if="!file.exists" class="badge warning">不存在</span>
                  <button class="btn small" @click="openConfigEditor(file.id, file.label, file.path)">编辑</button>
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- Security -->
        <template v-else-if="activeSection === 'security'">
          <div class="card-head">
            <div class="card-title">{{ titleMap.security }}</div>
            <span class="badge primary">本机配置</span>
          </div>
          <div class="settings-section">
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">MySQL Root 密码</div>
                  <div class="setting-desc">修改 MySQL root 用户密码</div>
                </div>
                <div style="display: flex; gap: 8px; align-items: center">
                  <input
                    class="input"
                    type="password"
                    placeholder="当前密码"
                    style="width: 120px"
                    v-model="mysqlCurrentPass"
                  />
                  <input
                    class="input"
                    type="password"
                    placeholder="新密码"
                    style="width: 120px"
                    v-model="mysqlNewPass"
                  />
                  <button class="btn small" :disabled="!mysqlCurrentPass || !mysqlNewPass" @click="changeRootPassword">修改</button>
                </div>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">Hosts 写入权限</div>
                  <div class="setting-desc">检查是否有权限修改系统 hosts 文件</div>
                </div>
                <button class="btn small" @click="checkHostsPermission">检查权限</button>
              </div>
              <div v-if="hostsPermissionResult" style="margin-top: 4px; font-size: 13px" :style="{ color: hostsPermissionOk ? 'var(--success, #22a95a)' : 'var(--danger, #ef5a49)' }">
                {{ hostsPermissionResult }}
              </div>
            </div>
          </div>
        </template>

        <!-- Backup -->
        <template v-else-if="activeSection === 'backup'">
          <div class="card-head">
            <div class="card-title">{{ titleMap.backup }}</div>
            <span class="badge primary">本机配置</span>
          </div>
          <div class="settings-section">
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">数据目录</div>
                  <div class="setting-desc">{{ settingsStore.settings.data_dir || '默认位置' }}</div>
                </div>
                <button class="btn small" @click="openDataDir">打开目录</button>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">导出设置</div>
                  <div class="setting-desc">将当前配置导出为 JSON 文件</div>
                </div>
                <button class="btn small" @click="exportSettings">导出</button>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-row">
                <div>
                  <div class="setting-name">导入设置</div>
                  <div class="setting-desc">从 JSON 文件恢复配置</div>
                </div>
                <button class="btn small" @click="importSettings">导入</button>
              </div>
            </div>
            <div class="setting-group">
              <div class="setting-name" style="margin-bottom: 8px">数据库备份</div>
              <div v-if="dbBackups.length === 0" style="color: var(--text-3); font-size: 13px">
                暂无备份
              </div>
              <div v-for="backup in dbBackups" :key="backup.path" class="setting-row config-file-row">
                <div>
                  <div class="setting-name">{{ backup.name }}</div>
                  <div class="setting-desc">{{ formatBackupSize(backup.size) }} · {{ formatBackupTime(backup.modified) }}</div>
                </div>
                <button class="btn small danger" @click="deleteBackup(backup.path)">删除</button>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-file-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid var(--line);
}
.config-file-row:last-child {
  border-bottom: none;
}
.config-file-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
</style>
