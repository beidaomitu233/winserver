<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { useDatabaseStore } from '../stores/useDatabaseStore'
import { useSettingsStore } from '../stores/useSettingsStore'

const databaseStore = useDatabaseStore()
const settingsStore = useSettingsStore()
const searchQuery = ref('')

// Modal states
const showCreateModal = ref(false)
const showPasswordModal = ref(false)
const showRootPasswordModal = ref(false)
const showImportModal = ref(false)
const selectedDb = ref<string>('')
const selectedUser = ref<string>('')

// Form data
const createForm = ref({ db: '', user: '', pass: '' })
const passwordForm = ref({ pass: '' })
const rootPasswordForm = ref({ currentPass: '', newPass: '', confirmPass: '' })
const importForm = ref({ dbName: '', path: '' })

const filteredDatabases = computed(() => {
  const q = searchQuery.value.toLowerCase()
  if (!q) return databaseStore.databases
  return databaseStore.databases.filter(d =>
    d.name.toLowerCase().includes(q) ||
    d.user.toLowerCase().includes(q)
  )
})

function openCreateModal() {
  createForm.value = { db: '', user: '', pass: '' }
  showCreateModal.value = true
}

async function handleCreate() {
  try {
    await databaseStore.createDatabase(createForm.value.db, createForm.value.user, createForm.value.pass)
    showCreateModal.value = false
  } catch (e) {
    console.error('Failed to create database:', e)
    alert('创建数据库失败')
  }
}

function openPasswordModal(dbName: string, user: string) {
  selectedDb.value = dbName
  selectedUser.value = user
  passwordForm.value = { pass: '' }
  showPasswordModal.value = true
}

async function handleChangePassword() {
  try {
    await databaseStore.changePassword(selectedDb.value, selectedUser.value, passwordForm.value.pass)
    showPasswordModal.value = false
  } catch (e) {
    console.error('Failed to change password:', e)
    alert('修改密码失败')
  }
}

function openRootPasswordModal() {
  rootPasswordForm.value = { currentPass: '', newPass: '', confirmPass: '' }
  showRootPasswordModal.value = true
}

async function handleRootPassword() {
  if (rootPasswordForm.value.newPass !== rootPasswordForm.value.confirmPass) {
    alert('新密码不匹配')
    return
  }
  try {
    await databaseStore.rootPassword(rootPasswordForm.value.currentPass, rootPasswordForm.value.newPass)
    showRootPasswordModal.value = false
  } catch (e) {
    console.error('Failed to change root password:', e)
    alert('修改 Root 密码失败')
  }
}

async function handleExport(dbName: string) {
  try {
    await databaseStore.exportDatabase(dbName)
    alert('导出成功')
  } catch (e) {
    console.error('Failed to export database:', e)
    alert('导出失败')
  }
}

function openImportModal(dbName: string) {
  importForm.value = { dbName, path: '' }
  showImportModal.value = true
}

async function browseSqlFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'SQL', extensions: ['sql'] }],
      title: '选择 SQL 文件',
    })
    if (selected) {
      importForm.value.path = selected
    }
  } catch {
    // user cancelled
  }
}

async function handleImport() {
  try {
    await databaseStore.importDatabase(importForm.value.dbName, importForm.value.path)
    showImportModal.value = false
    alert('导入成功')
  } catch (e) {
    console.error('Failed to import database:', e)
    alert('导入失败')
  }
}

async function handleDelete(dbName: string) {
  if (!confirm(`确定要删除数据库 "${dbName}" 吗？此操作不可恢复。`)) return
  try {
    await databaseStore.deleteDatabase(dbName)
  } catch (e) {
    console.error('Failed to delete database:', e)
    alert('删除数据库失败')
  }
}

async function handleBackupAll() {
  try {
    await databaseStore.syncDatabases()
    alert('全部备份完成')
  } catch (e) {
    console.error('Failed to backup all:', e)
    alert('备份失败')
  }
}

async function handleSync() {
  try {
    await databaseStore.syncDatabases()
  } catch (e) {
    console.error('Failed to sync:', e)
  }
}

async function openPhpMyAdmin() {
  const url = settingsStore.settings?.php_my_admin_url || 'http://localhost/phpmyadmin'
  try {
    await invoke('open_url', { url })
  } catch (error) {
    alert(typeof error === 'string' ? error : '无法打开 phpMyAdmin')
  }
}

onMounted(() => {
  databaseStore.fetchState()
  settingsStore.fetchSettings()
})
</script>

<template>
  <div class="page-enter">
    <div class="page-header">
      <div>
        <h1 class="page-title">数据库</h1>
      </div>
      <div class="page-actions">
        <button class="btn" @click="openRootPasswordModal">
          <svg class="icon icon-sm"><use href="#i-key" /></svg>Root 密码
        </button>
        <button class="btn primary" @click="openCreateModal">
          <svg class="icon icon-sm"><use href="#i-plus" /></svg>新建
        </button>
      </div>
    </div>

    <div class="card">
      <div class="toolbar">
        <button class="btn small" @click="openPhpMyAdmin">
          <svg class="icon icon-sm"><use href="#i-external" /></svg>phpMyAdmin
        </button>
        <button class="btn small" @click="handleBackupAll">
          <svg class="icon icon-sm"><use href="#i-backup" /></svg>全部备份
        </button>
        <button class="btn small" @click="handleSync">
          <svg class="icon icon-sm"><use href="#i-refresh" /></svg>同步
        </button>
        <div class="search-box">
          <svg class="icon icon-sm"><use href="#i-search" /></svg>
          <input v-model="searchQuery" class="input" placeholder="搜索数据库" />
        </div>
      </div>

      <div class="data-table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th>数据库名</th>
              <th>用户名</th>
              <th>大小</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="db in filteredDatabases" :key="db.name">
              <td>
                <div class="table-title">{{ db.name }}</div>
              </td>
              <td>{{ db.user }}</td>
              <td>{{ db.size }}</td>
              <td>
                <div class="table-actions">
                  <button class="btn small" @click="openPhpMyAdmin">phpMyAdmin</button>
                  <button class="btn small" @click="openPasswordModal(db.name, db.user)">改密</button>
                  <button class="btn small" @click="handleExport(db.name)">导出</button>
                  <button class="btn small" @click="openImportModal(db.name)">导入</button>
                  <button class="btn small danger" @click="handleDelete(db.name)">移除</button>
                </div>
              </td>
            </tr>
            <tr v-if="filteredDatabases.length === 0">
              <td colspan="4" style="text-align: center; color: var(--text-3); padding: 32px;">
                暂无数据库
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="table-footer">
        <span>共 {{ databaseStore.databases.length }} 个数据库</span>
      </div>
    </div>

    <Teleport to="body">
      <!-- Create Database Modal -->
      <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
        <div class="modal">
          <div class="modal-header">
            <h3>新建数据库</h3>
            <button class="btn-icon" @click="showCreateModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label>数据库名</label>
              <input v-model="createForm.db" class="input" placeholder="请输入数据库名" />
            </div>
            <div class="form-group">
              <label>用户名</label>
              <input v-model="createForm.user" class="input" placeholder="请输入用户名" />
            </div>
            <div class="form-group">
              <label>密码</label>
              <input v-model="createForm.pass" type="password" class="input" placeholder="请输入密码" />
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn" @click="showCreateModal = false">取消</button>
            <button class="btn primary" @click="handleCreate">创建</button>
          </div>
        </div>
      </div>

      <!-- Change Password Modal -->
      <div v-if="showPasswordModal" class="modal-overlay" @click.self="showPasswordModal = false">
        <div class="modal">
          <div class="modal-header">
            <h3>修改密码 - {{ selectedDb }}</h3>
            <button class="btn-icon" @click="showPasswordModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label>用户: {{ selectedUser }}</label>
            </div>
            <div class="form-group">
              <label>新密码</label>
              <input v-model="passwordForm.pass" type="password" class="input" placeholder="请输入新密码" />
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn" @click="showPasswordModal = false">取消</button>
            <button class="btn primary" @click="handleChangePassword">确认</button>
          </div>
        </div>
      </div>

      <!-- Root Password Modal -->
      <div v-if="showRootPasswordModal" class="modal-overlay" @click.self="showRootPasswordModal = false">
        <div class="modal">
          <div class="modal-header">
            <h3>修改 Root 密码</h3>
            <button class="btn-icon" @click="showRootPasswordModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label>当前密码</label>
              <input v-model="rootPasswordForm.currentPass" type="password" class="input" placeholder="请输入当前密码" />
            </div>
            <div class="form-group">
              <label>新密码</label>
              <input v-model="rootPasswordForm.newPass" type="password" class="input" placeholder="请输入新密码" />
            </div>
            <div class="form-group">
              <label>确认新密码</label>
              <input v-model="rootPasswordForm.confirmPass" type="password" class="input" placeholder="请再次输入新密码" />
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn" @click="showRootPasswordModal = false">取消</button>
            <button class="btn primary" @click="handleRootPassword">确认</button>
          </div>
        </div>
      </div>

      <!-- Import Modal -->
      <div v-if="showImportModal" class="modal-overlay" @click.self="showImportModal = false">
        <div class="modal">
          <div class="modal-header">
            <h3>导入 SQL - {{ importForm.dbName }}</h3>
            <button class="btn-icon" @click="showImportModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="form-group">
              <label>SQL 文件路径</label>
              <div style="display: flex; gap: 8px;">
                <input v-model="importForm.path" class="input" style="flex: 1" placeholder="请输入 SQL 文件路径" />
                <button class="btn" @click="browseSqlFile" type="button">浏览</button>
              </div>
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn" @click="showImportModal = false">取消</button>
            <button class="btn primary" @click="handleImport">导入</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
}

.modal {
  background: var(--surface-solid);
  border: 1px solid var(--line);
  border-radius: 12px;
  width: 100%;
  max-width: 400px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.3);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--line);
}

.modal-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}

.modal-body {
  padding: 20px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid var(--line);
}

.form-group {
  margin-bottom: 16px;
}

.form-group:last-child {
  margin-bottom: 0;
}

.form-group label {
  display: block;
  margin-bottom: 6px;
  font-size: 13px;
  color: var(--text-2);
}

.btn-icon {
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  color: var(--text-3);
  border-radius: 6px;
}

.btn-icon:hover {
  background: var(--surface-soft);
  color: var(--text);
}
</style>
