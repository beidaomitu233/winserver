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
    label: 'å…¬å…±è¯»å†™',
    short: 'è¯»å†™',
    desc: 'åŒ¿åå¯è¯»å¯å†™ï¼ˆä¸‹è½½ / ä¸Šä¼  / åˆ é™¤ï¼‰',
    tone: 'warning',
  },
  {
    id: 'download',
    label: 'å…¬å…±è¯» Â· ç§æœ‰å†™',
    short: 'åªè¯»',
    desc: 'åŒ¿åä»…å¯åˆ—è¡¨ä¸ä¸‹è½½ï¼Œå†™å…¥éœ€é‰´æƒ',
    tone: 'primary',
  },
  {
    id: 'private',
    label: 'å…¨ç§æœ‰',
    short: 'ç§æœ‰',
    desc: 'åŒ¿åä¸å¯è®¿é—®ï¼Œè¯»å†™å‡éœ€ Access Key',
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
    show('åŠ è½½å¤±è´¥', String(e?.message || e), 'error')
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
        show('ä¿å­˜å¤±è´¥', 'Root ç”¨æˆ·åä¸èƒ½ä¸ºç©º', 'error')
        return
      }
      if (pass.length < 8) {
        show('ä¿å­˜å¤±è´¥', 'MinIO Root å¯†ç è‡³å°‘ 8 ä½', 'error')
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
    show('ä¿å­˜æˆåŠŸ', `${props.service?.name || ''} é…ç½®å·²ä¿å­˜ï¼Œé‡å¯æœåŠ¡åç”Ÿæ•ˆ`, 'success')
    emit('close')
  } catch (e: any) {
    show('ä¿å­˜å¤±è´¥', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

async function saveFile() {
  isSaving.value = true
  try {
    await invoke('save_config_file', { fileId: fileId.value, content: fileContent.value })
    fileOriginal.value = fileContent.value
    show('ä¿å­˜æˆåŠŸ', `${fileLabel.value} å·²ä¿å­˜ï¼Œé‡å¯æœåŠ¡åç”Ÿæ•ˆ`, 'success')
    emit('close')
  } catch (e: any) {
    show('ä¿å­˜å¤±è´¥', String(e?.message || e), 'error')
  } finally {
    isSaving.value = false
  }
}

function openFileEditor() {
  mode.value = 'file'
}

function backToVisual() {
  if (hasFileChanges.value && !confirm('é…ç½®æ–‡ä»¶å·²ä¿®æ”¹ä½†æœªä¿å­˜ï¼Œç¡®å®šè¿”å›ï¼Ÿ')) return
  mode.value = 'visual'
}

function handleClose() {
  if (mode.value === 'file' && hasFileChanges.value && !confirm('é…ç½®æ–‡ä»¶å·²ä¿®æ”¹ä½†æœªä¿å­˜ï¼Œç¡®å®šå…³é—­ï¼Ÿ')) return
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
    minioBucketsError.value = String(e?.message || e || 'åŠ è½½æ¡¶åˆ—è¡¨å¤±è´¥')
  } finally {
    minioBucketsLoading.value = false
  }
}

async function copyBucketInfo(bucket: { name: string; policy: string; policy_label: string; url: string }) {
  const apiUrl = minioInfo.value.api_url || minioLiveApiUrl.value
  const bucketUrl = bucket.url || `${apiUrl.replace(/\/$/, '')}/${bucket.name}`
  const text = [
    `æ¡¶åç§°ï¼š${bucket.name}`,
    `è®¿é—®åœ°å€ï¼š${bucketUrl}`,
    `æƒé™ï¼š${bucket.policy_label || policyLabel(bucket.policy)}`,
    `MinIO åœ°å€ï¼š${apiUrl}`,
    `ç”¨æˆ·åï¼š${minioForm.value.root_user}`,
    `å¯†ç ï¼š${minioForm.value.root_password}`,
  ].join('\n')

  try {
    await navigator.clipboard?.writeText(text)
    show('å¤åˆ¶æˆåŠŸ', 'æ¡¶è¿æ¥ä¿¡æ¯å·²å¤åˆ¶åˆ°å‰ªè´´æ¿', 'success')
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
    show('å¤åˆ¶æˆåŠŸ', 'æ¡¶è¿æ¥ä¿¡æ¯å·²å¤åˆ¶åˆ°å‰ªè´´æ¿', 'success')
  } catch {
    show('å¤åˆ¶å¤±è´¥', 'è¯·æ‰‹åŠ¨å¤åˆ¶×t¶‰ËkºwµçM±…ÍÌô‰Íİ¥Ñ ˆ€é±…ÍÌô‰ì½¸èÉ•‘¥Í½É´¹…ÁÁ•¹‘½¹±äôˆ±¥¬ô‰É•‘¥Í½É´¹…ÁÁ•¹‘½¹±ä€ô€…É•‘¥Í½É´¹…ÁÁ•¹‘½¹±äˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±½¹™¥œµ™¥•±µ¥¹±¥¹”ˆø4(€€€€€€€€€€€€€€ñ±…‰•°û’şwš*“š¢‡–ò<ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰Íİ¥Ñ ˆ€é±…ÍÌô‰ì½¸èÉ•‘¥Í½É´¹ÁÉ½Ñ•Ñ•‘}µ½‘”ôˆ±¥¬ô‰É•‘¥Í½É´¹ÁÉ½Ñ•Ñ•‘}µ½‘”€ô€…É•‘¥Í½É´¹ÁÉ½Ñ•Ñ•‘}µ½‘”ˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ô‰Í•ÉÙ¥•%€ôôô€µ¥¹¥¼œˆ±…ÍÌô‰µ¥¹¥¼µ½¹™¥œˆø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰µ¥¹¥¼µÑ½½±‰…Èˆø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰µ¥¹¥¼µÍÑ…ÑÕÌˆ€é±…ÍÌô‰µ¥¹¥½%¹™¼¹ÉÕ¹¹¥¹œ€ü€½¸œ€è€½™˜œˆø4(€€€€€€€€€€€€€€€€ñÍÁ…¸±…ÍÌô‰ÍÑ…ÑÕÌµ‘½Ğˆ€é±…ÍÌô‰µ¥¹¥½%¹™¼¹ÉÕ¹¹¥¹œ€ü€ÉÕ¹¹¥¹œœ€è€ÍÑ½ÁÁ•œˆ€¼ø4(€€€€€€€€€€€€€€€íìµ¥¹¥½%¹™¼¹ÉÕ¹¹¥¹œ€ü€Ÿ¢şC¢†3’â´œ€è€Ÿšr«¢şC¢†0œõô4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰µ¥¹¥¼µÑ½½±‰…Èµ±¥¹­Ìˆø4(€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸ÑåÁ”ô‰‰ÕÑÑ½¸ˆ±…ÍÌô‰±¥¹¬µ‰Ñ¸ˆ±¥¬ô‰½Á•¹5¥¹¥½UÉ°¡µ¥¹¥½1¥Ù•Á¥UÉ°¤ˆùA$ğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸ÑåÁ”ô‰‰ÕÑÑ½¸ˆ±…ÍÌô‰±¥¹¬µ‰Ñ¸ˆ±¥¬ô‰½Á•¹5¥¹¥½UÉ°¡µ¥¹¥½1¥Ù•½¹Í½±•UÉ°¤ˆûš:Ÿ–"Û–>Àğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™½É´ˆø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€€€ñ±…‰•°ùA$ƒ®¿–>Œğ½±…‰•°ø4(€€€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°¹¹Õµ‰•Èô‰µ¥¹¥½½É´¹…Á¥}Á½ÉĞˆÑåÁ”ô‰¹Õµ‰•Èˆµ¥¸ôˆÄˆµ…àôˆØÔÔÌÔˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€€€ñ±…‰•°ûš:Ÿ–"Û–>Ã®¿–>Œğ½±…‰•°ø4(€€€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°¹¹Õµ‰•Èô‰µ¥¹¥½½É´¹½¹Í½±•}Á½ÉĞˆÑåÁ”ô‰¹Õµ‰•Èˆµ¥¸ôˆÄˆµ…àôˆØÔÔÌÔˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€€€ñ±…‰•°ùI½½ĞƒR£š"Üğ½±…‰•°ø4(€€€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰µ¥¹¥½½É´¹É½½Ñ}ÕÍ•Èˆ±…ÍÌô‰¥¹ÁÕĞˆ…ÕÑ½½µÁ±•Ñ”ô‰½™˜ˆ€¼ø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€€€ñ±…‰•°ùI½½Ğƒ–¾‚ğ½±…‰•°ø4(€€€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰Á…ÍÍİ½Éµ™¥•±ˆø4(€€€€€€€€€€€€€€€€€€ñ¥¹ÁÕĞ4(€€€€€€€€€€€€€€€€€€€Øµµ½‘•°ô‰µ¥¹¥½½É´¹É½½Ñ}Á…ÍÍİ½Éˆ4(€€€€€€€€€€€€€€€€€€€€éÑåÁ”ô‰Í¡½İ5¥¹¥½A…ÍÍİ½É€ü€Ñ•áĞœ€è€Á…ÍÍİ½Éœˆ4(€€€€€€€€€€€€€€€€€€€±…ÍÌô‰¥¹ÁÕĞˆ4(€€€€€€€€€€€€€€€€€€€Á±…•¡½±‘•Èô‹¢Ï–ÂD€àƒ’ö4ˆ4(€€€€€€€€€€€€€€€€€€€…ÕÑ½½µÁ±•Ñ”ô‰¹•ÜµÁ…ÍÍİ½Éˆ4(€€€€€€€€€€€€€€€€€€¼ø4(€€€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸ÑåÁ”ô‰‰ÕÑÑ½¸ˆ±…ÍÌô‰Á…ÍÍİ½ÉµÑ½±”ˆ±¥¬ô‰Í¡½İ5¥¹¥½A…ÍÍİ½É€ô€…Í¡½İ5¥¹¥½A…ÍÍİ½Éˆø4(€€€€€€€€€€€€€€€€€€€íìÍ¡½İ5¥¹¥½A…ÍÍİ½É€ü€Ÿ¦jC¢^<œ€è€Ÿš~—r,œõô4(€€€€€€€€€€€€€€€€€€ğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€€€ñÀØµ¥˜ô‰µ¥¹¥½%¹™¼¹‘…Ñ…}‘¥Èˆ±…ÍÌô‰µ¥¹¥¼µÁ…Ñ ˆùíìµ¥¹¥½%¹™¼¹‘…Ñ…}‘¥Èõôğ½Àø4(4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰µ¥¹¥¼µ‰Õ­•ÑÌˆø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰µ¥¹¥¼µ‰Õ­•ÑÌµ¡•…ˆø4(€€€€€€€€€€€€€€€€ñÍÁ…¸±…ÍÌô‰µ¥¹¥¼µ‰Õ­•ÑÌµÑ¥Ñ±”ˆûš†Ûšv¦f@ğ½ÍÁ…¸ø4(€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸4(€€€€€€€€€€€€€€€€€ÑåÁ”ô‰‰ÕÑÑ½¸ˆ4(€€€€€€€€€€€€€€€€€±…ÍÌô‰‰Ñ¸Íµ…±°ˆ4(€€€€€€€€€€€€€€€€€€é‘¥Í…‰±•ô‰µ¥¹¥½	Õ­•ÑÍ1½…‘¥¹œˆ4(€€€€€€€€€€€€€€€€€±¥¬ô‰±½…‘5¥¹¥½	Õ­•ÑÌˆ4(€€€€€€€€€€€€€€€€ø4(€€€€€€€€€€€€€€€€€íìµ¥¹¥½	Õ­•ÑÍ1½…‘¥¹œ€ü€ŸŠ˜œ€è€Ÿ–"ßšZÀœõô4(€€€€€€€€€€€€€€€€ğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€€€€€ñ‘¥ØØµ¥˜ô‰µ¥¹¥½	Õ­•ÑÍ1½…‘¥¹œˆ±…ÍÌô‰µ¥¹¥¼µ¡¥¹Ğˆû–*ƒ¢ö÷’â·Š˜ğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ô‰µ¥¹¥½	Õ­•ÑÍÉÉ½Èˆ±…ÍÌô‰µ¥¹¥¼µ¡¥¹Ğ•ÉÉ½Èˆùíìµ¥¹¥½	Õ­•ÑÍÉÉ½Èõôğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ôˆ…µ¥¹¥½	Õ­•ÑÌ¹±•¹Ñ ˆ±…ÍÌô‰µ¥¹¥¼µ¡¥¹Ğˆûšjš^ƒš†Øğ½‘¥Øø4(€€€€€€€€€€€€€€ñ‘¥ØØµ•±Í”±…ÍÌô‰‰Õ­•Ğµ±¥ÍĞˆø4(€€€€€€€€€€€€€€€€ñ‘¥ØØµ™½Èô‰ˆ¥¸µ¥¹¥½	Õ­•ÑÌˆ€é­•äô‰ˆ¹¹…µ”ˆ±…ÍÌô‰‰Õ­•ĞµÉ½Üˆø(€€€€€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰‰Õ­•Ğµ¹…µ”ˆùíìˆ¹¹…µ”õôğ½‘¥Øø(€€€€€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰‰Õ­•Ğµ…Ñ¥½¹Ìˆø(€€€€€€€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰Á½±¥äµÍ•œˆÉ½±”ô‰É½ÕÀˆ€é…É¥„µ±…‰•°ô‰€‘íˆ¹¹…µ•ôƒšv¦fA€ˆø(€€€€€€€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸(€€€€€€€€€€€€€€€€€€€€€€€Øµ™½Èô‰½ÁĞ¥¸µ¥¹¥½A½±¥å=ÁÑ¥½¹Ìˆ(€€€€€€€€€€€€€€€€€€€€€€€€é­•äô‰½ÁĞ¹¥ˆ(€€€€€€€€€€€€€€€€€€€€€€€ÑåÁ”ô‰‰ÕÑÑ½¸ˆ(€€€€€€€€€€€€€€€€€€€€€€€±…ÍÌô‰Á½±¥äµÍ•œµ‰Ñ¸ˆ(€€€€€€€€€€€€€€€€€€€€€€€€é±…ÍÌô‰ì(€€€€€€€€€€€€€€€€€€€€€€€€€…Ñ¥Ù”è¹½Éµ…±¥é•A½±¥ä¡ˆ¹Á½±¥ä¤€ôôô½ÁĞ¹¥°(€€€€€€€€€€€€€€€€€€€€€€€€€‰ÕÍäèµ¥¹¥½A½±¥å	ÕÍä€ôôô€‘íˆ¹¹…µ•ôè‘í½ÁĞ¹¥‘õ€°(€€€€€€€€€€€€€€€€€€€€€€€€€m½ÁĞ¹Ñ½¹•tèÑÉÕ”°(€€€€€€€€€€€€€€€€€€€€€€€ôˆ(€€€€€€€€€€€€€€€€€€€€€€€€é‘¥Í…‰±•ôˆ„…µ¥¹¥½A½±¥å	ÕÍäˆ(€€€€€€€€€€€€€€€€€€€€€€€€éÑ¥Ñ±”ô‰½ÁĞ¹‘•ÍŒˆ(€€€€€€€€€€€€€€€€€€€€€€€±¥¬ô‰Í•Ñ	Õ­•ÑA½±¥ä¡ˆ¹¹…µ”°½ÁĞ¹¥¤ˆ(€€€€€€€€€€€€€€€€€€€€€€ø(€€€€€€€€€€€€€€€€€€€€€€€íìµ¥¹¥½A½±¥å	ÕÍä€ôôô€‘íˆ¹¹…µ•ôè‘í½ÁĞ¹¥‘õ€€ü€ŸŠ˜œ€è½ÁĞ¹Í¡½ÉĞõô(€€€€€€€€€€€€€€€€€€€€€€ğ½‰ÕÑÑ½¸ø(€€€€€€€€€€€€€€€€€€€€ğ½‘¥Øø(€€€€€€€€€€€€€€€€€€€€ñ‰ÕÑÑ½¸(€€€€€€€€€€€€€€€€€€€€€ÑåÁ”ô‰‰ÕÑÑ½¸ˆ(€€€€€€€€€€€€€€€€€€€€€±…ÍÌô‰‰Ñ¸Íµ…±°‰Õ­•Ğµ½Áäˆ(€€€€€€€€€€€€€€€€€€€€€Ñ¥Ñ±”ô‹–’7–"Ûš†Û’ş‡š¼ˆ(€€€€€€€€€€€€€€€€€€€€€±¥¬ô‰½Áå	Õ­•Ñ%¹™¼¡ˆ¤ˆ(€€€€€€€€€€€€€€€€€€€€ø(€€€€€€€€€€€€€€€€€€€€€€ñÍÙœ±…ÍÌô‰¥½¸¥½¸µÍ´ˆøñÕÍ”¡É•˜ôˆ¤µ½Áäˆ€¼øğ½ÍÙœû–’7–"Ø(€€€€€€€€€€€€€€€€€€€€ğ½‰ÕÑÑ½¸ø(€€€€€€€€€€€€€€€€€€ğ½‘¥Øø(€€€€€€€€€€€€€€€€ğ½‘¥Øø(€€€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ô‰Í•ÉÙ¥•%€ôôô€¹¥¹àœˆ±…ÍÌô‰½¹™¥œµ™½É´ˆø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùİ½É­•É}ÁÉ½•ÍÍ•Ìğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰¹¥¹á½É´¹İ½É­•É}ÁÉ½•ÍÍ•Ìˆ±…ÍÌô‰¥¹ÁÕĞˆÁ±…•¡½±‘•Èô‰…ÕÑ¼ˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùİ½É­•É}½¹¹•Ñ¥½¹Ìğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰¹¥¹á½É´¹İ½É­•É}½¹¹•Ñ¥½¹Ìˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ù­••Á…±¥Ù•}Ñ¥µ•½ÕĞğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰¹¥¹á½É´¹­••Á…±¥Ù•}Ñ¥µ•½ÕĞˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ù±¥•¹Ñ}µ…á}‰½‘å}Í¥é”ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰¹¥¹á½É´¹±¥•¹Ñ}µ…á}‰½‘å}Í¥é”ˆ±…ÍÌô‰¥¹ÁÕĞˆÁ±…•¡½±‘•ÈôˆÔÁ´ˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±½¹™¥œµ™¥•±µ¥¹±¥¹”ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùé¥Àƒ–:/ò¤ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰Íİ¥Ñ ˆ€é±…ÍÌô‰ì½¸è¹¥¹á½É´¹é¥Àôˆ±¥¬ô‰¹¥¹á½É´¹é¥À€ô€…¹¥¹á½É´¹é¥Àˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ô‰¥Í5åÍÅ°ˆ±…ÍÌô‰½¹™¥œµ™½É´ˆø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°û®¿–>Œğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°¹¹Õµ‰•Èô‰µåÍÅ±½É´¹Á½ÉĞˆÑåÁ”ô‰¹Õµ‰•Èˆµ¥¸ôˆÄˆµ…àôˆØÔÔÌÔˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ûšr–’Ÿ¢ş{š:—šVÀğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰µåÍÅ±½É´¹µ…á}½¹¹•Ñ¥½¹Ìˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°û–¶_²›¦nğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰µåÍÅ±½É´¹¡…É…Ñ•É}Í•Ñ}Í•ÉÙ•Èˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ù%¹¹½ƒòO–ËšÆ€ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰µåÍÅ±½É´¹¥¹¹½‘‰}‰Õ™™•É}Á½½±}Í¥é”ˆ±…ÍÌô‰¥¹ÁÕĞˆÁ±…•¡½±‘•ÈôˆÄÈá4ˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ•±Í”µ¥˜ô‰¥ÍA¡Àˆ±…ÍÌô‰½¹™¥œµ™½É´ˆø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùµ•µ½Éå}±¥µ¥Ğğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰Á¡Á½É´¹µ•µ½Éå}±¥µ¥Ğˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùµ…á}•á•ÕÑ¥½¹}Ñ¥µ”ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰Á¡Á½É´¹µ…á}•á•ÕÑ¥½¹}Ñ¥µ”ˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùÕÁ±½…‘}µ…á}™¥±•Í¥é”ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰Á¡Á½É´¹ÕÁ±½…‘}µ…á}™¥±•Í¥é”ˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ùÁ½ÍÑ}µ…á}Í¥é”ğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ¥¹ÁÕĞØµµ½‘•°ô‰Á¡Á½É´¹Á½ÍÑ}µ…á}Í¥é”ˆ±…ÍÌô‰¥¹ÁÕĞˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥•±½¹™¥œµ™¥•±µ¥¹±¥¹”ˆø4(€€€€€€€€€€€€€€ñ±…‰•°ù‘¥ÍÁ±…å}•ÉÉ½ÉÌğ½±…‰•°ø4(€€€€€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰Íİ¥Ñ ˆ€é±…ÍÌô‰ì½¸èÁ¡Á½É´¹‘¥ÍÁ±…å}•ÉÉ½ÉÌôˆ±¥¬ô‰Á¡Á½É´¹‘¥ÍÁ±…å}•ÉÉ½ÉÌ€ô€…Á¡Á½É´¹‘¥ÍÁ±…å}•ÉÉ½ÉÌˆ€¼ø4(€€€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ•±Í”±…ÍÌô‰½¹™¥œµ•µÁÑäˆø4(€€€€€€€€€€€ƒšjš^ƒ’âOR£¢†£–6W¾ò3¢¾ß’öÿR£’â/šZç3¦7ö»ò[¢úG7’ş»šRç¦7ö»šZ’îÛ4(€€€€€€€€€€ğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥ØØµ¥˜ô‰™¥±•A…Ñ ˆ±…ÍÌô‰½¹™¥œµÁ…Ñ µ¡¥¹Ğˆû¦7ö»šZ’îÛ¾òiíì™¥±•A…Ñ õôğ½‘¥Øø4(4(€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰½¹™¥œµ™¥±”µ•¹ÑÉäˆø4(€€€€€€€€€€€€ñ‰ÕÑÑ½¸ÑåÁ”ô‰‰ÕÑÑ½¸ˆ±…ÍÌô‰‰Ñ¸ˆ±¥¬ô‰½Á•¹¥±•‘¥Ñ½Èˆû¦7ö»ò[¢úDğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€ñÍÁ…¸±…ÍÌô‰¡¥¹Ğˆûš&O–ò–:–/¦7ö»šZ’îÛ¢şo¢†3¦®cêŸò[¢úDğ½ÍÁ…¸ø4(€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€ğ½Ñ•µÁ±…Ñ”ø4(4(€€€€€€€€ğ„´´¥±”•‘¥Ñ½È€´´ø4(€€€€€€€€ñÑ•µÁ±…Ñ”Øµ•±Í”ø4(€€€€€€€€€€ñ‘¥Ø±…ÍÌô‰™¥±”µÑ½½±‰…Èˆø4(€€€€€€€€€€€€ñ‰ÕÑÑ½¸ÑåÁ”ô‰‰ÕÑÑ½¸ˆ±…ÍÌô‰‰Ñ¸Íµ…±°ˆ±¥¬ô‰‰…­Q½Y¥ÍÕ…°ˆûŠ@ƒ¢şS–n{–>¿¢–2Xğ½‰ÕÑÑ½¸ø4(€€€€€€€€€€€€ñÍÁ…¸±…ÍÌô‰½¹™¥œµÁ…Ñ µ¡¥¹Ğˆùíì™¥±•A…Ñ ñğ™¥±•1…‰•°õôğ½ÍÁ…¸ø4(€€€€€€€€€€ğ½‘¥Øø4(€€€€€€€€€€ñÑ•áÑ…É•„4(€€€€€€€€€€€Øµµ½‘•°ô‰™¥±•½¹Ñ•¹Ğˆ4(€€€€€€€€€€€±…ÍÌô‰½¹™¥œµ•‘¥Ñ½ÈµÑ•áÑ…É•„ˆ4(€€€€€€€€€€€ÍÁ•±±¡•¬ô‰™…±Í”ˆ4(€€€€€€€€€€€Á±…•¡½±‘•Èô‹¦7ö»šZ’îÛ––ºäˆ4(€€€€€€€€€€¼ø4(€€€€€€€€ğ½Ñ•µÁ±…Ñ”ø4(€€€€€€ğ½‘¥Øø4(4(€€€€€€ñ‘¥Ø±…ÍÌô‰µ½‘…°µ™½½Ğˆø4(€€€€€€€€ñÍÁ…¸Øµ¥˜ô‰µ½‘”€ôôô€™¥±”œ€˜˜¡…Í¥±•¡…¹•Ìˆ±…ÍÌô‰Õ¹Í…Ù•ˆûšr'šr«’şw–¶cjšnÓšRäğ½ÍÁ…¸ø4(€€€€€€€€ñ‘¥ØÍÑå±”ô‰™±•àè€Äˆ€¼ø4(€€€€€€€€ñ‰ÕÑÑ½¸±…ÍÌô‰‰Ñ¸ˆÑåÁ”ô‰‰ÕÑÑ½¸ˆ±¥¬ô‰¡…¹‘±•±½Í”ˆû–>[šÚ ğ½‰ÕÑÑ½¸ø4(€€€€€€€€ñ‰ÕÑÑ½¸4(€€€€€€€€€Øµ¥˜ô‰µ½‘”€ôôô€Ù¥ÍÕ…°œˆ4(€€€€€€€€€±…ÍÌô‰‰Ñ¸ÁÉ¥µ…Éäˆ4(€€€€€€€€€ÑåÁ”ô‰‰ÕÑÑ½¸ˆ4(€€€€€€€€€€é‘¥Í…‰±•ô‰¥ÍM…Ù¥¹œñğ¥Í1½…‘¥¹œˆ4(€€€€€€€€€±¥¬ô‰Í…Ù•Y¥ÍÕ…°ˆ4(€€€€€€€€ø4(€€€€€€€€€íì¥ÍM…Ù¥¹œ€ü€Ÿ’şw–¶c’â·Š˜œ€è€Ÿ†»–ºhœõô4(€€€€€€€€ğ½‰ÕÑÑ½¸ø4(€€€€€€€€ñ‰ÕÑÑ½¸4(€€€€€€€€€Øµ•±Í”4(€€€€€€€€€±…ÍÌô‰‰Ñ¸ÁÉ¥µ…Éäˆ4(€€€€€€€€€ÑåÁ”ô‰‰ÕÑÑ½¸ˆ4(€€€€€€€€€€é‘¥Í…‰±•ô‰¥ÍM…Ù¥¹œñğ€…¡…Í¥±•¡…¹•Ìˆ4(€€€€€€€€€±¥¬ô‰Í…Ù•¥±”ˆ4(€€€€€€€€ø4(€€€€€€€€€íì¥ÍM…Ù¥¹œ€ü€Ÿ’şw–¶c’â·Š˜œ€è€Ÿ’şw–¶cšZ’îØœõô4(€€€€€€€€ğ½‰ÕÑÑ½¸ø4(€€€€€€ğ½‘¥Øø4(€€€€ğ½‘¥Øø4(€€ğ½‘¥Øø4(ğ½Ñ•µÁ±…Ñ”ø4(4(ñÍÑå±”Í½Á•ø4(¹Í•ÉÙ¥”µ½¹™¥œµµ½‘…°€¹µ½‘…°µ‰½‘äìµ¥¸µ¡•¥¡Ğè€ÈàÁÁàìô4(¹Í•ÉÙ¥”µ½¹™¥œµµ½‘…°¹µ½‘…°µµ¥¹¥¼ì4(€İ¥‘Ñ èµ¥¸ ÜØÁÁà°…±Œ ÄÀÁÙÜ€´€ĞÁÁà¤¤ì4)ô4(¹Í•ÉÙ¥”µ½¹™¥œµµ½‘…°¹µ½‘…°µµ¥¹¥¼€¹µ½‘…°µ‰½‘äì4(€µ…àµ¡•¥¡Ğèµ¥¸ ÜÉÙ °€ÜÈÁÁà¤ì4(€½Ù•É™±½Üè…ÕÑ¼ì4)ô4(¹±½…‘¥¹œìÁ…‘‘¥¹œè€ÈáÁàì½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ìÑ•áĞµ…±¥¸è•¹Ñ•Èìô4(¹½¹™¥œµ™½É´ì4(€‘¥ÍÁ±…äèÉ¥ì4(€É¥µÑ•µÁ±…Ñ”µ½±Õµ¹ÌèÉ•Á•…Ğ È°µ¥¹µ…à À°€Å™È¤¤ì4(€…Àè€ÄÑÁà€ÄáÁàì4)ô4(¹½¹™¥œµ™¥•±ì‘¥ÍÁ±…äè™±•àì™±•àµ‘¥É•Ñ¥½¸è½±Õµ¸ì…Àè€ÕÁàìµ¥¸µİ¥‘Ñ è€Àìô4(¹½¹™¥œµ™¥•±µ™Õ±°ìÉ¥µ½±Õµ¸è€Ä€¼€´Äìô4(¹½¹™¥œµ™¥•±±…‰•°ì™½¹ĞµÍ¥é”è€ÄÉÁàì½±½ÈèÙ…È ´µÑ•áĞ´È¤ì™½¹Ğµİ•¥¡Ğè€ØÀÀìô4(¹½¹™¥œµ™¥•±€¹¥¹ÁÕĞìİ¥‘Ñ è€ÄÀÀ”ì‰½àµÍ¥é¥¹œè‰½É‘•Èµ‰½àìô4(¹½¹™¥œµ™¥•±µ¥¹±¥¹”ì4(€™±•àµ‘¥É•Ñ¥½¸èÉ½Üì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€©ÕÍÑ¥™äµ½¹Ñ•¹ĞèÍÁ…”µ‰•Ñİ••¸ì4)ô4(¹½¹™¥œµ•µÁÑäì½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ìÁ…‘‘¥¹œè€ÄÉÁà€À€ÑÁàìô4(¹½¹™¥œµÁ…Ñ µ¡¥¹Ğì4(€µ…É¥¸µÑ½Àè€ÄÑÁàì4(€™½¹ĞµÍ¥é”è€ÄÅÁàì4(€½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ì4(€İ½Éµ‰É•…¬è‰É•…¬µ…±°ì4)ô4(¹½¹™¥œµ™¥±”µ•¹ÑÉäì4(€µ…É¥¸µÑ½Àè€ÄÙÁàì4(€Á…‘‘¥¹œµÑ½Àè€ÄÑÁàì4(€‰½É‘•ÈµÑ½Àè€ÅÁàÍ½±¥Ù…È ´µ±¥¹”¤ì4(€‘¥ÍÁ±…äè™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€…Àè€ÄÉÁàì4)ô4(¹½¹™¥œµ™¥±”µ•¹ÑÉä€¹¡¥¹Ğì™½¹ĞµÍ¥é”è€ÄÉÁàì½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ìô4(¹Á…ÍÍİ½Éµ™¥•±ì‘¥ÍÁ±…äè™±•àì…Àè€áÁàì…±¥¸µ¥Ñ•µÌè•¹Ñ•Èìô4(¹Á…ÍÍİ½Éµ™¥•±€¹¥¹ÁÕĞì™±•àè€Äìµ¥¸µİ¥‘Ñ è€Àìô4(¹Á…ÍÍİ½ÉµÑ½±”ì4(€™±•àè¹½¹”ì¡•¥¡Ğè€ÌáÁàìÁ…‘‘¥¹œè€À€ÄÉÁàì‰½É‘•ÈµÉ…‘¥ÕÌè€ÄÁÁàì4(€‰½É‘•Èè€ÅÁàÍ½±¥Ù…È ´µ±¥¹”¤ì‰…­É½Õ¹èÙ…È ´µÍÕÉ™…”µÍ½™Ğ¤ì4(€½±½ÈèÙ…È ´µÑ•áĞ´È¤ì™½¹Ğè¥¹¡•É¥Ğì™½¹ĞµÍ¥é”è€ÄÉÁàì™½¹Ğµİ•¥¡Ğè€ØÔÀìÕÉÍ½ÈèÁ½¥¹Ñ•Èì4)ô4(¹™¥±”µÑ½½±‰…Èì4(€‘¥ÍÁ±…äè™±•àì…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì…Àè€ÄÉÁàìµ…É¥¸µ‰½ÑÑ½´è€ÄÁÁàì4)ô4(¹½¹™¥œµ•‘¥Ñ½ÈµÑ•áÑ…É•„ì4(€İ¥‘Ñ è€ÄÀÀ”ì4(€µ¥¸µ¡•¥¡Ğè€ÌØÁÁàì4(€µ…àµ¡•¥¡Ğè€ÔÕÙ ì4(€™½¹Ğµ™…µ¥±äè€…Í…‘¥„½‘”œ°½¹Í½±…Ì°µ½¹½ÍÁ…”ì4(€™½¹ĞµÍ¥é”è€ÄÍÁàì4(€±¥¹”µ¡•¥¡Ğè€Ä¸Ôì4(€Á…‘‘¥¹œè€ÄÉÁàì4(€‰½É‘•Èè€ÅÁàÍ½±¥Ù…È ´µ±¥¹”¤ì4(€‰½É‘•ÈµÉ…‘¥ÕÌè€ÙÁàì4(€‰…­É½Õ¹èÙ…È ´µÍÕÉ™…”µÍ½™Ğ¤ì4(€½±½ÈèÙ…È ´µÑ•áĞ¤ì4(€É•Í¥é”èÙ•ÉÑ¥…°ì4(€‰½àµÍ¥é¥¹œè‰½É‘•Èµ‰½àì4)ô4(¹Õ¹Í…Ù•ì™½¹ĞµÍ¥é”è€ÄÉÁàì½±½ÈèÙ…È ´µİ…É¹¥¹œ¤ìô4(4(¼¨€´´´´5¥¹%<è™±…Ğµ½‘•É¸±…å½ÕĞ€¡¹¼¹•ÍÑ•‰½á•Ì¤€´´´´€¨¼4(¹µ¥¹¥¼µ½¹™¥œì4(€‘¥ÍÁ±…äè™±•àì4(€™±•àµ‘¥É•Ñ¥½¸è½±Õµ¸ì4(€…Àè€ÄÙÁàì4)ô4(¹µ¥¹¥¼µÑ½½±‰…Èì4(€‘¥ÍÁ±…äè™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€©ÕÍÑ¥™äµ½¹Ñ•¹ĞèÍÁ…”µ‰•Ñİ••¸ì4(€…Àè€ÄÉÁàì4)ô4(¹µ¥¹¥¼µÍÑ…ÑÕÌì4(€‘¥ÍÁ±…äè¥¹±¥¹”µ™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€…Àè€ÙÁàì4(€™½¹ĞµÍ¥é”è€ÄÉÁàì4(€™½¹Ğµİ•¥¡Ğè€ØÔÀì4(€½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ì4)ô4(¹µ¥¹¥¼µÍÑ…ÑÕÌ¹½¸ì½±½ÈèÙ…È ´µÍÕ•ÍÌ¤ìô4(¹µ¥¹¥¼µÑ½½±‰…Èµ±¥¹­Ìì4(€‘¥ÍÁ±…äè™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€…Àè€ÄÑÁàì4)ô4(¹±¥¹¬µ‰Ñ¸ì4(€‰½É‘•Èè€Àì4(€‰…­É½Õ¹èÑÉ…¹ÍÁ…É•¹Ğì4(€Á…‘‘¥¹œè€Àì4(€½±½ÈèÙ…È ´µÁÉ¥µ…Éä¤ì4(€™½¹Ğè¥¹¡•É¥Ğì4(€™½¹ĞµÍ¥é”è€ÄÉÁàì4(€™½¹Ğµİ•¥¡Ğè€ØÔÀì4(€ÕÉÍ½ÈèÁ½¥¹Ñ•Èì4)ô4(¹±¥¹¬µ‰Ñ¸é¡½Ù•Èì½Á…¥Ñäè€¸àÔìô4(¹µ¥¹¥¼µÁ…Ñ ì4(€µ…É¥¸è€´ÑÁà€À€Àì4(€™½¹ĞµÍ¥é”è€ÄÅÁàì4(€½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ì4(€İ½Éµ‰É•…¬è‰É•…¬µ…±°ì4(€±¥¹”µ¡•¥¡Ğè€Ä¸Ğì4)ô4(4(¹µ¥¹¥¼µ‰Õ­•ÑÌì‘¥ÍÁ±…äè™±•àì™±•àµ‘¥É•Ñ¥½¸è½±Õµ¸ì…Àè€áÁàìô4(¹µ¥¹¥¼µ‰Õ­•ÑÌµ¡•…ì4(€‘¥ÍÁ±…äè™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€©ÕÍÑ¥™äµ½¹Ñ•¹ĞèÍÁ…”µ‰•Ñİ••¸ì4(€…Àè€ÄÉÁàì4)ô4(¹µ¥¹¥¼µ‰Õ­•ÑÌµÑ¥Ñ±”ì4(€™½¹ĞµÍ¥é”è€ÄÍÁàì4(€™½¹Ğµİ•¥¡Ğè€ÜÈÀì4(€½±½ÈèÙ…È ´µÑ•áĞ¤ì4)ô4(¹µ¥¹¥¼µ¡¥¹Ğì4(€Á…‘‘¥¹œè€ÄÁÁà€Àì4(€™½¹ĞµÍ¥é”è€ÄÉÁàì4(€½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ì4)ô4(¹µ¥¹¥¼µ¡¥¹Ğ¹•ÉÉ½Èì½±½ÈèÙ…È ´µ‘…¹•È¤ìô4(4(¹‰Õ­•Ğµ±¥ÍĞì4(€‘¥ÍÁ±…äè™±•àì4(€™±•àµ‘¥É•Ñ¥½¸è½±Õµ¸ì4)ô4(¹‰Õ­•ĞµÉ½Üì4(€‘¥ÍÁ±…äè™±•àì4(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì4(€©ÕÍÑ¥™äµ½¹Ñ•¹ĞèÍÁ…”µ‰•Ñİ••¸ì4(€…Àè€ÄÉÁàì4(€µ¥¸µ¡•¥¡Ğè€ĞáÁàì4(€Á…‘‘¥¹œè€áÁà€Àì4(€‰½É‘•Èµ‰½ÑÑ½´è€ÅÁàÍ½±¥Ù…È ´µ±¥¹”¤ì4)ô4(¹‰Õ­•ĞµÉ½Üé±…ÍĞµ¡¥±ì‰½É‘•Èµ‰½ÑÑ½´è€Àìô4(¹‰Õ­•Ğµ¹…µ”ì(€™½¹Ğµİ•¥¡Ğè€ØàÀì4(€™½¹ĞµÍ¥é”è€ÄÍÁàì4(€µ¥¸µİ¥‘Ñ è€Àì4(€½Ù•É™±½Üè¡¥‘‘•¸ì4(€Ñ•áĞµ½Ù•É™±½Üè•±±¥ÁÍ¥Ìì4(€İ¡¥Ñ”µÍÁ…”è¹½İÉ…Àì4)ô(¹‰Õ­•Ğµ…Ñ¥½¹Ìì(€‘¥ÍÁ±…äè™±•àì(€…±¥¸µ¥Ñ•µÌè•¹Ñ•Èì(€…Àè€áÁàì(€™±•àè¹½¹”ì)ô(¹‰Õ­•Ğµ½Áäìİ¡¥Ñ”µÍÁ…”è¹½İÉ…Àìô((¼¨M•µ•¹Ñ•½¹ÑÉ½°™½ÈÑ¡”Ñ¡É•”Á½±¥¥•Ì€¨¼(¹Á½±¥äµÍ•œì4(€‘¥ÍÁ±…äè¥¹±¥¹”µ™±•àì4(€™±•àè¹½¹”ì4(€Á…‘‘¥¹œè€ÍÁàì4(€‰½É‘•ÈµÉ…‘¥ÕÌè€ÄÁÁàì4(€‰…­É½Õ¹èÙ…È ´µÍÕÉ™…”µÍ½™Ğ¤ì4(€…Àè€ÉÁàì4)ô4(¹Á½±¥äµÍ•œµ‰Ñ¸ì4(€¡•¥¡Ğè€ÌÁÁàì4(€µ¥¸µİ¥‘Ñ è€ÔÉÁàì4(€Á…‘‘¥¹œè€À€ÄÉÁàì4(€‰½É‘•Èè€Àì4(€‰½É‘•ÈµÉ…‘¥ÕÌè€áÁàì4(€‰…­É½Õ¹èÑÉ…¹ÍÁ…É•¹Ğì4(€½±½ÈèÙ…È ´µÑ•áĞ´Ì¤ì4(€™½¹Ğè¥¹¡•É¥Ğì4(€™½¹ĞµÍ¥é”è€ÄÉÁàì4(€™½¹Ğµİ•¥¡Ğè€ØÔÀì4(€ÕÉÍ½ÈèÁ½¥¹Ñ•Èì4(€ÑÉ…¹Í¥Ñ¥½¸è‰…­É½Õ¹€¸ÄÕÌ•…Í”°½±½È€¸ÄÕÌ•…Í”ì4)ô4(¹Á½±¥äµÍ•œµ‰Ñ¸é¡½Ù•Èé¹½Ğ é‘¥Í…‰±•¤é¹½Ğ ¹…Ñ¥Ù”¤ì4(€½±½ÈèÙ…È ´µÑ•áĞ¤ì4)ô4(¹Á½±¥äµÍ•œµ‰Ñ¸é‘¥Í…‰±•ìÕÉÍ½Èè‘•™…Õ±Ğìô4(¹Á½±¥äµÍ•œµ‰Ñ¸¹…Ñ¥Ù”ì4(€‰…­É½Õ¹èÙ…È ´µÍÕÉ™…”µÍ½±¥¤ì4(€½±½ÈèÙ…È ´µÑ•áĞ¤ì4(€‰½àµÍ¡…‘½Üè€À€ÅÁà€ÍÁàÉ‰„ ÈÀ°€ÌÌ°€ØÄ°€À¸Àà¤ì4)ô4(¹Á½±¥äµÍ•œµ‰Ñ¸¹…Ñ¥Ù”¹İ…É¹¥¹œì½±½Èè€ˆÜÜäÅ˜ìô4(¹Á½±¥äµÍ•œµ‰Ñ¸¹…Ñ¥Ù”¹ÁÉ¥µ…Éäì½±½ÈèÙ…È ´µÁÉ¥µ…Éä¤ìô4(¹Á½±¥äµÍ•œµ‰Ñ¸¹…Ñ¥Ù”¹ÍÕ•ÍÌì½±½ÈèÙ…È ´µÍÕ•ÍÌ¤ìô4(¹Á½±¥äµÍ•œµ‰Ñ¸¹‰ÕÍäì½Á…¥Ñäè€¸ØÔìô4(4)µ•‘¥„€¡µ…àµİ¥‘Ñ è€ÜÈÁÁà¤ì4(€€¹‰Õ­•ĞµÉ½Üì4(€€€™±•àµ‘¥É•Ñ¥½¸è½±Õµ¸ì4(€€€…±¥¸µ¥Ñ•µÌèÍÑÉ•Ñ ì4(€€€…Àè€áÁàì4(€€€Á…‘‘¥¹œè€ÄÉÁà€Àì4(€ô4(€€¹Á½±¥äµÍ•œìİ¥‘Ñ è€ÄÀÀ”ìô4(€€¹Á½±¥äµÍ•œµ‰Ñ¸ì™±•àè€Äìµ¥¸µİ¥‘Ñ è€Àìô4)ô4(ğ½ÍÑå±”ø4(