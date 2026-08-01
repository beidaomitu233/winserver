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

/** visual = default form; file = raw config editor (opened via bottom button) */
const mode = ref<'visual' | 'file'>('visual')
const showRedisPassword = ref(false)
const showMinioPassword = ref(false)
const isLoading = ref(false)
const isSaving = ref(false)

const fileLabel = ref('')
const filePath = ref('')
const fileId = ref('')
const fileContent = ref('')
const fileOriginal = ref('')
const hasFileChanges = computed(() => fileContent.value !== fileOriginal.value)

const serviceId = computed(() => props.service?.id || '')
const isMysql = computed(() => serviceId.value.startsWith('mysql'))
const isPhp = computed(() => serviceId.value.startsWith('php'))

// ---- forms ----
const redisForm = ref({
  port: 6379,
  bind: '127.0.0.1',
  password: '',
  maxmemory: '256mb',
  maxmemory_policy: 'allkeys-lru',
  appendonly: false,
  protected_mode: true,
})
const minioForm = ref({
  root_user: 'minioadmin',
  root_password: 'minioadmin',
  api_port: 9000,
  console_port: 9001,
})
const minioInfo = ref({
  api_url: '',
  console_url: '',
  data_dir: '',
  install_dir: '',
  running: false,
  mc_available: false,
  mc_path: '',
  cli_alias_cmd: '',
  path: '',
})
const minioBuckets = ref<Array<{
  name: string
  policy: string
  policy_label: string
  url: string
}>>([])
const minioBucketsLoading = ref(false)
const minioBucketsError = ref('')
const minioPolicyBusy = ref('')

const minioPolicyOptions = [
  {
    id: 'public',
    label: '公共读写',
    short: '读写',
    desc: '匿名可读可写（下载 / 上传 / 删除）',
    tone: 'warning',
  },
  {
    id: 'download',
    label: '公共读 · 私有写',
    short: '只读',
    desc: '匿名仅可列表与下载，写入需鉴权',
    tone: 'primary',
  },
  {
    id: 'private',
    label: '全私有',
    short: '私有',
    desc: '匿名不可访问，读写均需 Access Key',
    tone: 'success',
  },
] as const
const nginxForm = ref({
  worker_processes: 'auto',
  worker_connections: '1024',
  keepalive_timeout: '65',
  client_max_body_size: '50m',
  gzip: true,
})
const mysqlForm = ref({
  port: 3306,
  max_connections: '200',
  character_set_server: 'utf8mb4',
  innodb_buffer_pool_size: '128M',
})
const phpForm = ref({
  memory_limit: '256M',
  max_execution_time: '300',
  upload_max_filesize: '64M',
  post_max_size: '64M',
  display_errors: true,
})

const policies = [
  'noeviction', 'allkeys-lru', 'volatile-lru', 'allkeys-random',
  'volatile-random', 'volatile-ttl', 'allkeys-lfu', 'volatile-lfu',
]

function resolveFileId(id: string): string {
  if (id === 'redis') return 'redis.conf'
  if (id === 'minio') return 'minio.env'
  if (id === 'nginx') return 'nginx.conf'
  if (id.startsWith('mysql')) return 'mysql.ini'
  if (id.startsWith('php')) return 'php.ini'
  return id
}

function parseIniLike(content: string, key: string, fallback = ''): string {
  const re = new RegExp(`^\\s*${key}\\s*=\\s*(.+?)\\s*$`, 'im')
  const m = content.match(re)
  return m ? m[1].replace(/^["']|["']$/g, '').trim() : fallback
}

function parseNginxDir(content: string, key: string, fallback = ''): string {
  const re = new RegExp(`\\b${key}\\s+([^;\\n]+);`, 'i')
  const m = content.match(re)
  return m ? m[1].trim() : fallback
}

function parsePhpIni(content: string, key: string, fallback = ''): string {
  const re = new RegExp(`^\\s*${key}\\s*=\\s*(.+?)\\s*$`, 'im')
  const m = content.match(re)
  return m ? m[1].replace(/;.*$/, '').trim() : fallback
}

function upsertIniLine(content: string, key: string, value: string): string {
  const re = new RegExp(`^(\\s*${key}\\s*=\\s*).*$`, 'im')
  if (re.test(content)) return content.replace(re, `$1${value}`)
  return content.trimEnd() + `\n${key}=${value}\n`
}

function upsertNginxDir(content: string, key: string, value: string): string {
  const re = new RegExp(`(\\b${key}\\s+)[^;\\n]+;`, 'i')
  if (re.test(content)) return content.replace(re, `$1${value};`)
  // insert into events/http if possible
  if (key === 'worker_connections' && /events\s*\{/i.test(content)) {
    return content.replace(/events\s*\{/i, `events {\n    ${key} ${value};`)
  }
  if (/http\s*\{/i.test(content)) {
    return content.replace(/http\s*\{/i, `http {\n    ${key} ${value};`)
  }
  return content.trimEnd() + `\n${key} ${value};\n`
}

function upsertPhpIni(content: string, key: string, value: string): string {
  const re = new RegExp(`^(\\s*${key}\\s*=\\s*).*$`, 'im')
  if (re.test(content)) return content.replace(re, `$1${value}`)
  return content.trimEnd() + `\n${key} = ${value}\n`
}

async function loadConfig() {
  if (!props.service) return
  const id = props.service.id
  mode.value = 'visual'
  showRedisPassword.value = false
  showMinioPassword.value = false
  fileId.value = resolveFileId(id)
  fileLabel.value = fileId.value
  isLoading.value = true

  try {
    if (id === 'redis') {
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
      await loadFileText(fileId.value)
    } else if (id === 'minio') {
      const cfg = await invoke<Record<string, any>>('minio_config_get')
      minioForm.value = {
        root_user: String(cfg.root_user || 'minioadmin'),
        root_password: String(cfg.root_password || 'minioadmin'),
        api_port: Number(cfg.api_port) || 9000,
        console_port: Number(cfg.console_port) || 9001,
      }
      minioInfo.value = {
        api_url: String(cfg.api_url || `http://127.0.0.1:${cfg.api_port || 9000}`),
        console_url: String(cfg.console_url || `http://127.0.0.1:${cfg.console_port || 9001}`),
        data_dir: String(cfg.data_dir || ''),
        install_dir: String(cfg.install_dir || ''),
        running: Boolean(cfg.running),
        mc_available: Boolean(cfg.mc_available),
        mc_path: String(cfg.mc_path || ''),
        cli_alias_cmd: String(cfg.cli_alias_cmd || ''),
        path: String(cfg.path || ''),
      }
      filePath.value = String(cfg.path || '')
      await loadFileText(fileId.value)
      await loadMinioBuckets()
    } else {
      await loadFileText(fileId.value)
      const c = fileContent.value
      if (id === 'nginx') {
        nginxForm.value = {
          worker_processes: parseNginxDir(c, 'worker_processes', 'auto') || parseIniLike(c, 'worker_processes', 'auto'),
          worker_connections: parseNginxDir(c, 'worker_connections', '1024'),
          keepalive_timeout: parseNginxDir(c, 'keepalive_timeout', '65'),
          client_max_body_size: parseNginxDir(c, 'client_max_body_size', '50m'),
          gzip: /gzip\s+on\s*;/i.test(c),
        }
      } else if (id.startsWith('mysql')) {
        mysqlForm.value = {
          port: Number(parseIniLike(c, 'port', '3306')) || 3306,
          max_connections: parseIniLike(c, 'max_connections', '200'),
          character_set_server: parseIniLike(c, 'character-set-server', 'utf8mb4') || parseIniLike(c, 'character_set_server', 'utf8mb4'),
          innodb_buffer_pool_size: parseIniLike(c, 'innodb_buffer_pool_size', '128M'),
        }
      } else if (id.startsWith('php')) {
        phpForm.value = {
          memory_limit: parsePhpIni(c, 'memory_limit', '256M'),
          max_execution_time: parsePhpIni(c, 'max_execution_time', '300'),
          upload_max_filesize: parsePhpIni(c, 'upload_max_filesize', '64M'),
          post_max_size: parsePhpIni(c, 'post_max_size', '64M'),
          display_errors: !/^\\s*display_errors\\s*=\\s*Off/im.test(c),
        }
      }
    }
  } catch (e: any) {
    show('加载失败', String(e?.message || e), 'error')
  } finally {
    isLoading.value = false
  }
}

async function loadFileText(id: string) {
  try {
    const result = await invoke<{ content: string; path: string }>('get_config_file', { fileId: id })
    fileContent.value = result.content || ''
    fileOriginal.value = fileContent.value
    if (result.path) filePath.value = result.path
  } catch {
    // fallback to service config_file path via empty content
    fileContent.value = ''
    fileOriginal.value = ''
    if (props.service?.config_file) filePath.value = props.service.config_file
  }
}

watch(() => props.visible, (val) => {
  if (val) loadConfig()
})

async function saveVisual() {
  isSaving.value = true
  try {
    const id = serviceId.value
    if (id === 'redis') {
      await invoke('redis_config_save', { params: JSON.stringify(redisForm.value) })
    } else if (id === 'minio') {
      const user = minioForm.value.root_user.trim()
      const pass = minioForm.value.root_password
      if (!user) {
        show('保存失败', 'Root 用户名不能为空', 'error')
        return
      }
      if (pass.length < 8) {
        show('保存失败', 'MinIO Root 密码至少 8 位', 'error')
        return
      }
      await invoke('minio_config_save', {
        params: JSON.stringify({
          root_user: user,
          root_password: pass,
          api_port: minioForm.value.api_port,
          console_port: minioForm.value.console_port,
        }),
      })
    } else {
      // patch config file from visual fields
      let content = fileContent.value || fileOriginal.value
      if (id === 'nginx') {
        let c = content
        c = upsertNginxDir(c, 'worker_processes', nginxForm.value.worker_processes)
        c = upsertNginxDir(c, 'worker_connections', nginxForm.value.worker_connections)
        c = upsertNginxDir(c, 'keepalive_timeout', nginxForm.value.keepalive_timeout)
        c = upsertNginxDir(c, 'client_max_body_size', nginxForm.value.client_max_body_size)
        if (/gzip\s+(on|off)\s*;/i.test(c)) {
          c = c.replace(/gzip\s+(on|off)\s*;/i, `gzip ${nginxForm.value.gzip ? 'on' : 'off'};`)
        } else {
          c = upsertNginxDir(c, 'gzip', nginxForm.value.gzip ? 'on' : 'off')
        }
        content = c
      } else if (id.startsWith('mysql')) {
        let c = content
        c = upsertIniLine(c, 'port', String(mysqlForm.value.port))
        c = upsertIniLine(c, 'max_connections', mysqlForm.value.max_connections)
        c = upsertIniLine(c, 'character-set-server', mysqlForm.value.character_set_server)
        c = upsertIniLine(c, 'innodb_buffer_pool_size', mysqlForm.value.innodb_buffer_pool_size)
        content = c
      } else if (id.startsWith('php')) {
        let c = content
        c = upsertPhpIni(c, 'memory_limit', phpForm.value.memory_limit)
        c = upsertPhpIni(c, 'max_execution_time', phpForm.value.max_execution_time)
        c = upsertPhpIni(c, 'upload_max_filesize', phpForm.value.upload_max_filesize)
        c = upsertPhpIni(c, 'post_max_size', phpForm.value.post_max_size)
        c = upsertPhpIni(c, 'display_errors', phpForm.value.display_errors ? 'On' : 'Off')
        content = c
      }
      await invoke('save_config_file', { fileId: fileId.value, content })
      fileContent.value = content
      fileOriginal.value = content
    }
    show('保存成功', `${props.service?.name || ''} 配置已保存，重启服务后生效`, 'success')
    emit('close')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

async function saveFile() {
  isSaving.value = true
  try {
    await invoke('save_config_file', { fileId: fileId.value, content: fileContent.value })
    fileOriginal.value = fileContent.value
    show('保存成功', `${fileLabel.value} 已保存，重启服务后生效`, 'success')
    emit('close')
  } catch (e: any) {
    show('保存失败', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

function openFileEditor() {
  mode.value = 'file'
}

function backToVisual() {
  if (hasFileChanges.value && !confirm('配置文件已修改但未保存，确定返回？')) return
  mode.value = 'visual'
}

function handleClose() {
  if (mode.value === 'file' && hasFileChanges.value && !confirm('配置文件已修改但未保存，确定关闭？')) return
  emit('close')
}

function minioLiveCreds() {
  return {
    root_user: minioForm.value.root_user.trim(),
    root_password: minioForm.value.root_password,
    api_port: minioForm.value.api_port,
  }
}

async function loadMinioBuckets() {
  minioBucketsLoading.value = true
  minioBucketsError.value = ''
  try {
    const res = await invoke<{
      buckets?: Array<{ name: string; policy: string; policy_label: string; url: string }>
    }>('minio_buckets_list', { params: JSON.stringify(minioLiveCreds()) })
    minioBuckets.value = Array.isArray(res?.buckets) ? res.buckets : []
  } catch (e: any) {
    minioBuckets.value = []
    minioBucketsError.value = String(e?.message || e || '加载桶列表失败')
  } finally {
    minioBucketsLoading.value = false
  }
}

async function copyBucketInfo(bucket: { name: string; policy: string; policy_label: string; url: string }) {
  const apiUrl = minioInfo.value.api_url || minioLiveApiUrl.value
  const bucketUrl = bucket.url || `${apiUrl.replace(/\/$/, '')}/${bucket.name}`
  const text = [
    `桶名称：${bucket.name}`,
    `访问地址：${bucketUrl}`,
    `权限：${bucket.policy_label || policyLabel(bucket.policy)}`,
    `MinIO 地址：${apiUrl}`,
    `用户名：${minioForm.value.root_user}`,
    `密码：${minioForm.value.root_password}`,
  ].join('\n')

  try {
    await navigator.clipboard?.writeText(text)
    show('复制成功', '桶连接信息已复制到剪贴板', 'success')
    return
  } catch {
    // Fallback for non-secure context or Tauri webview.
  }

  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.style.position = 'fixed'
  textarea.style.opacity = '0'
  document.body.appendChild(textarea)
  try {
    textarea.select()
    if (!document.execCommand('copy')) throw new Error('copy command was rejected')
    show('复制成功', '桶连接信息已复制到剪贴板', 'success')
  } catch {
    show('复制失败', '请手动复制', 'error')
  } finally {
    document.body.removeChild(textarea)
  }
}

async function setBucketPolicy(bucket: string, policy: string) {
  if (!bucket || minioPolicyBusy.value) return
  minioPolicyBusy.value = `${bucket}:${policy}`
  try {
    const creds = minioLiveCreds()
    const res = await invoke<{ message?: string; policy?: string; policy_label?: string }>(
      'minio_bucket_set_policy',
      {
        bucket,
        policy,
        rootUser: creds.root_user,
        rootPassword: creds.root_password,
        apiPort: creds.api_port,
      },
    )
    const idx = minioBuckets.value.findIndex(b => b.name === bucket)
    if (idx >= 0) {
      minioBuckets.value[idx] = {
        ...minioBuckets.value[idx],
        policy: String(res?.policy || policy),
        policy_label: String(res?.policy_label || policyLabel(policy)),
      }
    }
    show('权限已更新', res?.message || `桶 ${bucket} 权限已修改`, 'success')
  } catch (e: any) {
    show('设置失败', String(e?.message || e), 'error')
  } finally {
    minioPolicyBusy.value = ''
  }
}

function normalizePolicy(policy: string) {
  const p = (policy || '').toLowerCase()
  if (p === 'public' || p === 'readwrite' || p === 'rw') return 'public'
  if (p === 'download' || p === 'readonly' || p === 'read') return 'download'
  if (p === 'private' || p === 'none') return 'private'
  if (p === 'upload') return 'upload'
  return p
}

function policyLabel(policy: string) {
  const p = normalizePolicy(policy)
  if (p === 'public') return '公共读写'
  if (p === 'download') return '公共读 · 私有写'
  if (p === 'private') return '全私有'
  if (p === 'upload') return '公共写 · 私有读'
  return policy || '未知'
}

async function openMinioUrl(url: string) {
  if (!url) return
  try {
    await invoke('open_url', { url })
  } catch {
    window.open(url, '_blank')
  }
}

const minioLiveApiUrl = computed(() =>
  `http://127.0.0.1:${minioForm.value.api_port || 9000}`,
)
const minioLiveConsoleUrl = computed(() =>
  `http://127.0.0.1:${minioForm.value.console_port || 9001}`,
)
</script>

<template>
  <div class="overlay" :class="{ show: visible }" @click.self="handleClose">
    <div class="modal modal-lg service-config-modal" :class="{ 'modal-minio': serviceId === 'minio' }">
      <div class="modal-head">
        <div class="modal-title">配置 · {{ service?.name || '' }}</div>
        <button class="icon-btn" type="button" @click="handleClose">
          <svg class="icon icon-sm"><use href="#i-close" /></svg>
        </button>
      </div>

      <div class="modal-body">
        <div v-if="isLoading" class="loading">加载中…</div>

        <!-- Visual (default) -->
        <template v-else-if="mode === 'visual'">
          <div v-if="serviceId === 'redis'" class="config-form">
            <div class="config-field">
              <label>端口</label>
              <input v-model.number="redisForm.port" type="number" min="1" max="65535" class="input" />
            </div>
            <div class="config-field">
              <label>绑定地址</label>
              <input v-model="redisForm.bind" class="input" />
            </div>
            <div class="config-field config-field-full">
              <label>密码</label>
              <div class="password-field">
                <input
                  v-model="redisForm.password"
                  :type="showRedisPassword ? 'text' : 'password'"
                  class="input"
                  placeholder="留空则不启用"
                  autocomplete="off"
                />
                <button type="button" class="password-toggle" @click="showRedisPassword = !showRedisPassword">
                  {{ showRedisPassword ? '隐藏' : '查看' }}
                </button>
              </div>
            </div>
            <div class="config-field">
              <label>最大内存</label>
              <input v-model="redisForm.maxmemory" class="input" placeholder="256mb" />
            </div>
            <div class="config-field">
              <label>淘汰策略</label>
              <select v-model="redisForm.maxmemory_policy" class="input">
                <option v-for="p in policies" :key="p" :value="p">{{ p }}</option>
              </select>
            </div>
            <div class="config-field config-field-inline">
              <label>AOF 持久化</label>
              <div class="switch" :class="{ on: redisForm.appendonly }" @click="redisForm.appendonly = !redisForm.appendonly" />
            </div>
            <div class="config-field config-field-inline">
              <label>保护模式</label>
              <div class="switch" :class="{ on: redisForm.protected_mode }" @click="redisForm.protected_mode = !redisForm.protected_mode" />
            </div>
          </div>

          <div v-else-if="serviceId === 'minio'" class="minio-config">
            <div class="minio-toolbar">
              <div class="minio-status" :class="minioInfo.running ? 'on' : 'off'">
                <span class="status-dot" :class="minioInfo.running ? 'running' : 'stopped'" />
                {{ minioInfo.running ? '运行中' : '未运行' }}
              </div>
              <div class="minio-toolbar-links">
                <button type="button" class="link-btn" @click="openMinioUrl(minioLiveApiUrl)">API</button>
                <button type="button" class="link-btn" @click="openMinioUrl(minioLiveConsoleUrl)">控制台</button>
              </div>
            </div>

            <div class="config-form">
              <div class="config-field">
                <label>API 端口</label>
                <input v-model.number="minioForm.api_port" type="number" min="1" max="65535" class="input" />
              </div>
              <div class="config-field">
                <label>控制台端口</label>
                <input v-model.number="minioForm.console_port" type="number" min="1" max="65535" class="input" />
              </div>
              <div class="config-field">
                <label>Root 用户</label>
                <input v-model="minioForm.root_user" class="input" autocomplete="off" />
              </div>
              <div class="config-field">
                <label>Root 密码</label>
                <div class="password-field">
                  <input
                    v-model="minioForm.root_password"
                    :type="showMinioPassword ? 'text' : 'password'"
                    class="input"
                    placeholder="至少 8 位"
                    autocomplete="new-password"
                  />
                  <button type="button" class="password-toggle" @click="showMinioPassword = !showMinioPassword">
                    {{ showMinioPassword ? '隐藏' : '查看' }}
                  </button>
                </div>
              </div>
            </div>

            <p v-if="minioInfo.data_dir" class="minio-path">{{ minioInfo.data_dir }}</p>

            <div class="minio-buckets">
              <div class="minio-buckets-head">
                <span class="minio-buckets-title">桶权限</span>
                <button
                  type="button"
                  class="btn small"
                  :disabled="minioBucketsLoading"
                  @click="loadMinioBuckets"
                >
                  {{ minioBucketsLoading ? '…' : '刷新' }}
                </button>
              </div>

              <div v-if="minioBucketsLoading" class="minio-hint">加载中…</div>
              <div v-else-if="minioBucketsError" class="minio-hint error">{{ minioBucketsError }}</div>
              <div v-else-if="!minioBuckets.length" class="minio-hint">暂无桶</div>
              <div v-else class="bucket-list">
                <div v-for="b in minioBuckets" :key="b.name" class="bucket-row">
                  <div class="bucket-name">{{ b.name }}</div>
                  <div class="bucket-actions">
                    <div class="policy-seg" role="group" :aria-label="`${b.name} 权限`">
                      <button
                        v-for="opt in minioPolicyOptions"
                        :key="opt.id"
                        type="button"
                        class="policy-seg-btn"
                        :class="{
                          active: normalizePolicy(b.policy) === opt.id,
                          busy: minioPolicyBusy === `${b.name}:${opt.id}`,
                          [opt.tone]: true,
                        }"
                        :disabled="!!minioPolicyBusy"
                        :title="opt.desc"
                        @click="setBucketPolicy(b.name, opt.id)"
                      >
                        {{ minioPolicyBusy === `${b.name}:${opt.id}` ? '…' : opt.short }}
                      </button>
                    </div>
                    <button
                      type="button"
                      class="btn small bucket-copy"
                      title="复制桶信息"
                      @click="copyBucketInfo(b)"
                    >
                      <svg class="icon icon-sm"><use href="#i-copy" /></svg>复制
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div v-else-if="serviceId === 'nginx'" class="config-form">
            <div class="config-field">
              <label>worker_processes</label>
              <input v-model="nginxForm.worker_processes" class="input" placeholder="auto" />
            </div>
            <div class="config-field">
              <label>worker_connections</label>
              <input v-model="nginxForm.worker_connections" class="input" />
            </div>
            <div class="config-field">
              <label>keepalive_timeout</label>
              <input v-model="nginxForm.keepalive_timeout" class="input" />
            </div>
            <div class="config-field">
              <label>client_max_body_size</label>
              <input v-model="nginxForm.client_max_body_size" class="input" placeholder="50m" />
            </div>
            <div class="config-field config-field-inline">
              <label>gzip 压缩</label>
              <div class="switch" :class="{ on: nginxForm.gzip }" @click="nginxForm.gzip = !nginxForm.gzip" />
            </div>
          </div>

          <div v-else-if="isMysql" class="config-form">
            <div class="config-field">
              <label>端口</label>
              <input v-model.number="mysqlForm.port" type="number" min="1" max="65535" class="input" />
            </div>
            <div class="config-field">
              <label>最大连接数</label>
              <input v-model="mysqlForm.max_connections" class="input" />
            </div>
            <div class="config-field">
              <label>字符集</label>
              <input v-model="mysqlForm.character_set_server" class="input" />
            </div>
            <div class="config-field">
              <label>InnoDB 缓冲池</label>
              <input v-model="mysqlForm.innodb_buffer_pool_size" class="input" placeholder="128M" />
            </div>
          </div>

          <div v-else-if="isPhp" class="config-form">
            <div class="config-field">
              <label>memory_limit</label>
              <input v-model="phpForm.memory_limit" class="input" />
            </div>
            <div class="config-field">
              <label>max_execution_time</label>
              <input v-model="phpForm.max_execution_time" class="input" />
            </div>
            <div class="config-field">
              <label>upload_max_filesize</label>
              <input v-model="phpForm.upload_max_filesize" class="input" />
            </div>
            <div class="config-field">
              <label>post_max_size</label>
              <input v-model="phpForm.post_max_size" class="input" />
            </div>
            <div class="config-field config-field-inline">
              <label>display_errors</label>
              <div class="switch" :class="{ on: phpForm.display_errors }" @click="phpForm.display_errors = !phpForm.display_errors" />
            </div>
          </div>

          <div v-else class="config-empty">
            暂无专用表单，请使用下方「配置编辑」修改配置文件。
          </div>

          <div v-if="filePath" class="config-path-hint">配置文件：{{ filePath }}</div>

          <div class="config-file-entry">
            <button type="button" class="btn" @click="openFileEditor">配置编辑</button>
            <span class="hint">打开原始配置文件进行高级编辑</span>
          </div>
        </template>

        <!-- File editor -->
        <template v-else>
          <div class="file-toolbar">
            <button type="button" class="btn small" @click="backToVisual">← 返回可视化</button>
            <span class="config-path-hint">{{ filePath || fileLabel }}</span>
          </div>
          <textarea
            v-model="fileContent"
            class="config-editor-textarea"
            spellcheck="false"
            placeholder="配置文件内容"
          />
        </template>
      </div>

      <div class="modal-foot">
        <span v-if="mode === 'file' && hasFileChanges" class="unsaved">有未保存的更改</span>
        <div style="flex: 1" />
        <button class="btn" type="button" @click="handleClose">取消</button>
        <button
          v-if="mode === 'visual'"
          class="btn primary"
          type="button"
          :disabled="isSaving || isLoading"
          @click="saveVisual"
        >
          {{ isSaving ? '保存中…' : '确定' }}
        </button>
        <button
          v-else
          class="btn primary"
          type="button"
          :disabled="isSaving || !hasFileChanges"
          @click="saveFile"
        >
          {{ isSaving ? '保存中…' : '保存文件' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.service-config-modal .modal-body { min-height: 280px; }
.service-config-modal.modal-minio {
  width: min(760px, calc(100vw - 40px));
}
.service-config-modal.modal-minio .modal-body {
  max-height: min(72vh, 720px);
  overflow: auto;
}
.loading { padding: 28px; color: var(--text-3); text-align: center; }
.config-form {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px 18px;
}
.config-field { display: flex; flex-direction: column; gap: 5px; min-width: 0; }
.config-field-full { grid-column: 1 / -1; }
.config-field label { font-size: 12px; color: var(--text-2); font-weight: 600; }
.config-field .input { width: 100%; box-sizing: border-box; }
.config-field-inline {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}
.config-empty { color: var(--text-3); padding: 12px 0 4px; }
.config-path-hint {
  margin-top: 14px;
  font-size: 11px;
  color: var(--text-3);
  word-break: break-all;
}
.config-file-entry {
  margin-top: 16px;
  padding-top: 14px;
  border-top: 1px solid var(--line);
  display: flex;
  align-items: center;
  gap: 12px;
}
.config-file-entry .hint { font-size: 12px; color: var(--text-3); }
.password-field { display: flex; gap: 8px; align-items: center; }
.password-field .input { flex: 1; min-width: 0; }
.password-toggle {
  flex: none; height: 38px; padding: 0 12px; border-radius: 10px;
  border: 1px solid var(--line); background: var(--surface-soft);
  color: var(--text-2); font: inherit; font-size: 12px; font-weight: 650; cursor: pointer;
}
.file-toolbar {
  display: flex; align-items: center; gap: 12px; margin-bottom: 10px;
}
.config-editor-textarea {
  width: 100%;
  min-height: 360px;
  max-height: 55vh;
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 13px;
  line-height: 1.5;
  padding: 12px;
  border: 1px solid var(--line);
  border-radius: 6px;
  background: var(--surface-soft);
  color: var(--text);
  resize: vertical;
  box-sizing: border-box;
}
.unsaved { font-size: 12px; color: var(--warning); }

/* ---- MinIO: flat modern layout (no nested boxes) ---- */
.minio-config {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.minio-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.minio-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 650;
  color: var(--text-3);
}
.minio-status.on { color: var(--success); }
.minio-toolbar-links {
  display: flex;
  align-items: center;
  gap: 14px;
}
.link-btn {
  border: 0;
  background: transparent;
  padding: 0;
  color: var(--primary);
  font: inherit;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
}
.link-btn:hover { opacity: .85; }
.minio-path {
  margin: -4px 0 0;
  font-size: 11px;
  color: var(--text-3);
  word-break: break-all;
  line-height: 1.4;
}

.minio-buckets { display: flex; flex-direction: column; gap: 8px; }
.minio-buckets-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.minio-buckets-title {
  font-size: 13px;
  font-weight: 720;
  color: var(--text);
}
.minio-hint {
  padding: 10px 0;
  font-size: 12px;
  color: var(--text-3);
}
.minio-hint.error { color: var(--danger); }

.bucket-list {
  display: flex;
  flex-direction: column;
}
.bucket-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 48px;
  padding: 8px 0;
  border-bottom: 1px solid var(--line);
}
.bucket-row:last-child { border-bottom: 0; }
.bucket-name {
  font-weight: 680;
  font-size: 13px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bucket-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}
.bucket-copy { white-space: nowrap; }

/* Segmented control for the three policies */
.policy-seg {
  display: inline-flex;
  flex: none;
  padding: 3px;
  border-radius: 10px;
  background: var(--surface-soft);
  gap: 2px;
}
.policy-seg-btn {
  height: 30px;
  min-width: 52px;
  padding: 0 12px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--text-3);
  font: inherit;
  font-size: 12px;
  font-weight: 650;
  cursor: pointer;
  transition: background .15s ease, color .15s ease;
}
.policy-seg-btn:hover:not(:disabled):not(.active) {
  color: var(--text);
}
.policy-seg-btn:disabled { cursor: default; }
.policy-seg-btn.active {
  background: var(--surface-solid);
  color: var(--text);
  box-shadow: 0 1px 3px rgba(20, 33, 61, 0.08);
}
.policy-seg-btn.active.warning { color: #b7791f; }
.policy-seg-btn.active.primary { color: var(--primary); }
.policy-seg-btn.active.success { color: var(--success); }
.policy-seg-btn.busy { opacity: .65; }

@media (max-width: 720px) {
  .bucket-row {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
    padding: 12px 0;
  }
  .policy-seg { width: 100%; }
  .policy-seg-btn { flex: 1; min-width: 0; }
}
</style>
