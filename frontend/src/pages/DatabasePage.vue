<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useDatabaseStore } from '../stores/useDatabaseStore'
import { useToast } from '../composables/useToast'

const databaseStore = useDatabaseStore()
const { show } = useToast()
const searchQuery = ref('')

// Modal states
const showCreateModal = ref(false)
const showPasswordModal = ref(false)
const showRootPasswordModal = ref(false)
const showImportModal = ref(false)
const showDeleteModal = ref(false)
const selectedDb = ref<string>('')
const selectedUser = ref<string>('')
const deleteConfirmName = ref('')

// Form data
const createForm = ref({ db: '', user: '', pass: '' })
const passwordForm = ref({ pass: '' })
const rootPasswordForm = ref({ currentPass: '', newPass: '', confirmPass: '' })
const importForm = ref({ dbName: '', path: '' })

// Loading / reveal state
const isImporting = ref(false)
const isExporting = ref(false)
const isDeleting = ref(false)
const revealedPasswords = ref<Record<string, boolean>>({})

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
  const db = createForm.value.db.trim()
  const user = createForm.value.user.trim()
  const pass = createForm.value.pass
  if (!db || !user) {
    alert('请填写数据库名和用户名')
    return
  }
  try {
    await databaseStore.createDatabase(db, user, pass)
    showCreateModal.value = false
  } catch (e: any) {
    console.error('Failed to create database:', e)
    const msg = typeof e === 'string' ? e : e?.message || String(e)
    alert(msg && msg !== '创建数据库失败' ? msg : `创建数据库失败：${msg || '请先启动 MySQL 并确认 Root 密码'}`)
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
    // Update local revealed map after change
    if (revealedPasswords.value[selectedDb.value]) {
      // keep revealed; store will refresh list with new password
    }
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
  if (isExporting.value) return
  try {
    const selected = await save({
      title: '导出数据库',
      defaultPath: `${dbName}.sql`,
      filters: [{ name: 'SQL', extensions: ['sql'] }],
    })
    if (!selected) return
    isExporting.value = true
    await databaseStore.exportDatabase(dbName, selected)
    alert(`导出成功\n${selected}`)
  } catch (e) {
    console.error('Failed to export database:', e)
    alert('导出失败')
  } finally {
    isExporting.value = false
  }
}

function openImportModal(dbName: string) {
  importForm.value = { dbName, path: '' }
  isImporting.value = false
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
  if (!importForm.value.path.trim()) {
    alert('请选择 SQL 文件')
    return
  }
  if (isImporting.value) return
  isImporting.value = true
  try {
    await databaseStore.importDatabase(importForm.value.dbName, importForm.value.path)
    showImportModal.value = false
    alert('导入成功')
  } catch (e) {
    console.error('Failed to import database:', e)
    const msg = typeof e === 'string' ? e : (e as any)?.message || String(e)
    alert(`导入失败：${msg || '未知错误'}`)
  } finally {
    isImporting.value = false
  }
}

function openDeleteModal(dbName: string) {
  selectedDb.value = dbName
  deleteConfirmName.value = ''
  showDeleteModal.value = true
}

async function handleDeleteConfirm() {
  const dbName = selectedDb.value
  if (deleteConfirmName.value.trim() !== dbName) {
    alert(`请输入数据库名「${dbName}」以确认删除`)
    return
  }
  if (isDeleting.value) return
  isDeleting.value = true
  try {
    await databaseStore.deleteDatabase(dbName)
    showDeleteModal.value = false
  } catch (e) {
    console.error('Failed to delete database:', e)
    alert('删除数据库失败')
  } finally {
    isDeleting.value = false
  }
}

function togglePassword(dbName: string) {
  revealedPasswords.value[dbName] = !revealedPasswords.value[dbName]
}

function passwordDisplay(db: { name: string; password?: string }) {
  const pass = db.password ?? ''
  if (!pass) return '（空）'
  if (revealedPasswords.value[db.name]) return pass
  return '••••••••'
}

async function copyDatabaseInfo(db: { name: string; user: string; password?: string }) {
  const pass = db.password ?? ''
  const text = `数据库名：${db.name}\n用户名：${db.user}\n密码：${pass}`
  try {
    await navigator.clipboard.writeText(text)
    show('复制成功', '数据库信息已复制到剪贴板', 'success')
  } catch {
    // Fallback for non-secure context or Tauri webview
    const textarea = document.createElement('textarea')
    textarea.value = text
    textarea.style.position = 'fixed'
    textarea.style.opacity = '0'
    document.body.appendChild(textarea)
    textarea.select()
    try {
      const copied = document.execCommand('copy')
      if (!copied) throw new Error('copy command was rejected')
      show('复制成功', '数据库信息已复制到剪贴板', 'success')
    } catch {
      show('复制失败', '请手动复制', 'error')
    }
    document.body.removeChild(textarea)
  }
}

onMounted(() => {
  databaseStore.fetchState()
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
              <th>密码</th>
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
              <td>
                <button
                  type="button"
                  class="password-cell"
                  :title="revealedPasswords[db.name] ? '点击隐藏' : '点击查看'"
                  @click="togglePassword(db.name)"
                >
                  <span class="password-text">{{ passwordDisplay(db) }}</span>
                  <span class="password-toggle">{{ revealedPasswords[db.name] ? '隐藏' : '查看' }}</span>
                </button>
              </td>
              <td>{{ db.size || '—' }}</td>
              <td>
                <div class="table-actions">
                  <button class="btn small" type="button" title="复制数据库信息" @click="copyDatabaseInfo(db)">
                    <svg class="icon icon-sm"><use href="#i-copy" /></svg>复制
                  </button>
                  <button class="btn small" @click="openPasswordModal(db.name, db.user)">改密</button>
                  <button class="btn small" :disabled="isExporting" @click="handleExport(db.name)">
                    {{ isExporting ? '导出中…' : '导出' }}
                  </button>
                  <button class="btn small" @click="openImportModal(db.name)">导入</button>
                  <button class="btn small danger" @click="openDeleteModal(db.name)">移除</button>
                </div>
              </td>
            </tr>
            <tr v-if="filteredDatabases.length === 0">
              <td colspan="5" style="text-align: center; color: var(--text-3); padding: 32px;">
                暂无数据库（请确认已启动对应版本的 MySQL，并使用「新建」创建）
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
      <div v-if="showCreateModal" class="overlay show" @click.self="showCreateModal = false">
        <div class="modal modal-compact">
          <div class="modal-head">
            <div class="modal-title">新建数据库</div>
            <button class="btn-icon" type="button" @click="showCreateModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="db-form">
              <div class="db-field">
                <label class="db-label" for="create-db-name">数据库名</label>
                <input
                  id="create-db-name"
                  v-model="createForm.db"
                  class="input db-input"
                  placeholder="例如 app_db"
                  autocomplete="off"
                  spellcheck="false"
                />
              </div>
              <div class="db-field-row">
                <div class="db-field">
                  <label class="db-label" for="create-db-user">用户名</label>
                  <input
                    id="create-db-user"
                    v-model="createForm.user"
                    class="input db-input"
                    placeholder="数据库用户"
                    autocomplete="off"
                    spellcheck="false"
                  />
                </div>
                <div class="db-field">
                  <label class="db-label" for="create-db-pass">密码</label>
                  <input
                    id="create-db-pass"
                    v-model="createForm.pass"
                    type="password"
                    class="input db-input"
                    placeholder="登录密码"
                    autocomplete="new-password"
                  />
                </div>
              </div>
              <p class="db-hint">创建后将自动授予该用户对本库的全部权限（localhost / 127.0.0.1）。</p>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" @click="showCreateModal = false">取消</button>
            <button class="btn primary" type="button" @click="handleCreate">创建</button>
          </div>
        </div>
      </div>

      <!-- Change Password Modal -->
      <div v-if="showPasswordModal" class="overlay show" @click.self="showPasswordModal = false">
        <div class="modal modal-compact">
          <div class="modal-head">
            <div class="modal-title">修改密码</div>
            <button class="btn-icon" type="button" @click="showPasswordModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="db-form">
              <div class="db-meta">
                <span>数据库 <strong>{{ selectedDb }}</strong></span>
                <span>用户 <strong>{{ selectedUser }}</strong></span>
              </div>
              <div class="db-field">
                <label class="db-label" for="change-db-pass">新密码</label>
                <input
                  id="change-db-pass"
                  v-model="passwordForm.pass"
                  type="password"
                  class="input db-input"
                  placeholder="请输入新密码"
                  autocomplete="new-password"
                />
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" @click="showPasswordModal = false">取消</button>
            <button class="btn primary" type="button" @click="handleChangePassword">确认</button>
          </div>
        </div>
      </div>

      <!-- Root Password Modal -->
      <div v-if="showRootPasswordModal" class="overlay show" @click.self="showRootPasswordModal = false">
        <div class="modal modal-compact">
          <div class="modal-head">
            <div class="modal-title">修改 Root 密码</div>
            <button class="btn-icon" type="button" @click="showRootPasswordModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="db-form">
              <div class="db-field">
                <label class="db-label" for="root-current-pass">当前密码</label>
                <input
                  id="root-current-pass"
                  v-model="rootPasswordForm.currentPass"
                  type="password"
                  class="input db-input"
                  placeholder="空密码可留空"
                  autocomplete="current-password"
                />
              </div>
              <div class="db-field-row">
                <div class="db-field">
                  <label class="db-label" for="root-new-pass">新密码</label>
                  <input
                    id="root-new-pass"
                    v-model="rootPasswordForm.newPass"
                    type="password"
                    class="input db-input"
                    placeholder="新密码"
                    autocomplete="new-password"
                  />
                </div>
                <div class="db-field">
                  <label class="db-label" for="root-confirm-pass">确认新密码</label>
                  <input
                    id="root-confirm-pass"
                    v-model="rootPasswordForm.confirmPass"
                    type="password"
                    class="input db-input"
                    placeholder="再输入一次"
                    autocomplete="new-password"
                  />
                </div>
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" @click="showRootPasswordModal = false">取消</button>
            <button class="btn primary" type="button" @click="handleRootPassword">确认</button>
          </div>
        </div>
      </div>

      <!-- Import Modal -->
      <div
        v-if="showImportModal"
        class="overlay show"
        @click.self="!isImporting && (showImportModal = false)"
      >
        <div class="modal modal-compact modal-import">
          <div class="modal-head">
            <div class="modal-title">导入 SQL</div>
            <button
              class="btn-icon"
              type="button"
              :disabled="isImporting"
              @click="showImportModal = false"
            >
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="db-form">
              <div class="db-meta">
                <span>目标库 <strong>{{ importForm.dbName }}</strong></span>
              </div>
              <div class="db-field">
                <label class="db-label" for="import-sql-path">SQL 文件</label>
                <div class="db-path-row">
                  <input
                    id="import-sql-path"
                    v-model="importForm.path"
                    class="input db-input"
                    placeholder="选择或粘贴 .sql 路径"
                    :disabled="isImporting"
                  />
                  <button class="btn" type="button" :disabled="isImporting" @click="browseSqlFile">浏览</button>
                </div>
              </div>
              <div v-if="isImporting" class="import-progress">
                <div class="import-spinner" />
                <div class="import-progress-text">
                  <strong>正在执行 SQL…</strong>
                  <span>大文件可能需要较长时间，请勿关闭窗口</span>
                </div>
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" :disabled="isImporting" @click="showImportModal = false">取消</button>
            <button class="btn primary" type="button" :disabled="isImporting" @click="handleImport">
              {{ isImporting ? '导入中…' : '导入' }}
            </button>
          </div>
        </div>
      </div>

      <!-- Delete Confirm Modal (二次确认) -->
      <div v-if="showDeleteModal" class="overlay show" @click.self="!isDeleting && (showDeleteModal = false)">
        <div class="modal modal-compact">
          <div class="modal-head">
            <div class="modal-title">移除数据库</div>
            <button class="btn-icon" type="button" :disabled="isDeleting" @click="showDeleteModal = false">
              <svg class="icon"><use href="#i-close" /></svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="db-form">
              <div class="delete-warning">
                <strong>危险操作</strong>
                <p>
                  将从管理列表中移除数据库 <strong>{{ selectedDb }}</strong>。
                  此操作不可撤销，请确认后继续。
                </p>
              </div>
              <div class="db-field">
                <label class="db-label" for="delete-confirm-name">
                  请输入数据库名 <code>{{ selectedDb }}</code> 以确认
                </label>
                <input
                  id="delete-confirm-name"
                  v-model="deleteConfirmName"
                  class="input db-input"
                  placeholder="输入完整数据库名"
                  autocomplete="off"
                  spellcheck="false"
                  :disabled="isDeleting"
                  @keyup.enter="handleDeleteConfirm"
                />
              </div>
            </div>
          </div>
          <div class="modal-foot">
            <button class="btn" type="button" :disabled="isDeleting" @click="showDeleteModal = false">取消</button>
            <button
              class="btn danger"
              type="button"
              :disabled="isDeleting || deleteConfirmName.trim() !== selectedDb"
              @click="handleDeleteConfirm"
            >
              {{ isDeleting ? '移除中…' : '确认移除' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.page-subtitle {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-3);
}

/* Compact form dialog: avoid global 560px modal + short default inputs */
.modal-compact {
  width: min(420px, calc(100vw - 40px));
  max-width: 420px;
}

.modal-import {
  width: min(480px, calc(100vw - 40px));
  max-width: 480px;
}

.modal-compact :deep(.modal-head),
.modal-compact.modal .modal-head {
  min-height: 52px;
  padding: 0 16px 0 18px;
}

.modal-compact :deep(.modal-title),
.modal-compact.modal .modal-title {
  font-size: 15px;
}

.modal-compact :deep(.modal-body),
.modal-compact.modal .modal-body {
  padding: 16px 18px;
}

.modal-compact :deep(.modal-foot),
.modal-compact.modal .modal-foot {
  min-height: 52px;
  padding: 10px 16px;
}

.db-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.db-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  flex: 1;
}

.db-field-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.db-label {
  font-size: 12px;
  font-weight: 650;
  color: var(--text-2);
  letter-spacing: 0.01em;
}

.db-label code {
  font-size: 12px;
  color: var(--danger, #e5484d);
  background: color-mix(in srgb, var(--danger, #e5484d) 10%, transparent);
  padding: 1px 6px;
  border-radius: 4px;
}

.db-input {
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
}

.db-hint {
  margin: 0;
  font-size: 11px;
  line-height: 1.45;
  color: var(--text-3);
}

.db-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--surface-soft);
  border: 1px solid var(--line);
  font-size: 12px;
  color: var(--text-3);
}

.db-meta strong {
  color: var(--text);
  font-weight: 680;
  margin-left: 4px;
}

.db-path-row {
  display: flex;
  gap: 8px;
  align-items: center;
  min-width: 0;
}

.db-path-row .db-input {
  flex: 1;
}

.db-path-row .btn {
  flex: none;
  white-space: nowrap;
}

.password-cell {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  max-width: 180px;
  padding: 4px 8px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font: inherit;
  text-align: left;
}

.password-cell:hover {
  background: var(--surface-soft);
  border-color: var(--line);
}

.password-text {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.password-toggle {
  flex: none;
  font-size: 11px;
  color: var(--primary, #3b82f6);
  font-weight: 600;
}

.import-progress {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--primary, #3b82f6) 8%, var(--surface-soft));
  border: 1px solid color-mix(in srgb, var(--primary, #3b82f6) 22%, var(--line));
}

.import-spinner {
  width: 18px;
  height: 18px;
  margin-top: 2px;
  border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--primary, #3b82f6) 25%, transparent);
  border-top-color: var(--primary, #3b82f6);
  animation: db-spin 0.75s linear infinite;
  flex: none;
}

.import-progress-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.4;
}

.import-progress-text strong {
  color: var(--text);
  font-weight: 680;
}

.delete-warning {
  padding: 12px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--danger, #e5484d) 8%, var(--surface-soft));
  border: 1px solid color-mix(in srgb, var(--danger, #e5484d) 25%, var(--line));
  font-size: 12px;
  color: var(--text-2);
  line-height: 1.5;
}

.delete-warning strong {
  display: block;
  color: var(--danger, #e5484d);
  margin-bottom: 4px;
  font-size: 13px;
}

.delete-warning p {
  margin: 0;
}

.delete-warning p strong {
  display: inline;
  color: var(--text);
  font-size: inherit;
  margin: 0;
}

.btn.danger {
  background: color-mix(in srgb, var(--danger, #e5484d) 12%, transparent);
  color: var(--danger, #e5484d);
  border-color: color-mix(in srgb, var(--danger, #e5484d) 30%, var(--line));
}

.btn.danger:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-icon {
  background: none;
  border: none;
  padding: 4px;
  cursor: pointer;
  color: var(--text-3);
  border-radius: 6px;
  display: grid;
  place-items: center;
}

.btn-icon:hover:not(:disabled) {
  background: var(--surface-soft);
  color: var(--text);
}

.btn-icon:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

@keyframes db-spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 480px) {
  .db-field-row {
    grid-template-columns: 1fr;
  }
}
</style>
