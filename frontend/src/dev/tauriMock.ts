import { mockConvertFileSrc, mockIPC, mockWindows } from '@tauri-apps/api/mocks'
import type { AppState, ServiceInfo, ServiceState, SoftwareInfo, SystemSettings } from '../types'

const services: ServiceInfo[] = [
  service('nginx', 'Nginx', 'nginx', 80, true, 'running', 'D:\\WinServer\\runtime\\nginx\\conf\\nginx.conf'),
  service('apache', 'Apache2.4', 'apache', 80, false, 'stopped', 'D:\\WinServer\\runtime\\apache\\conf\\httpd.conf'),
  service('mysql57', 'MySQL5.7', 'mysql', 3307, true, 'running', 'D:\\WinServer\\runtime\\mysql57\\my.ini'),
  service('mysql80', 'MySQL8.0', 'mysql', 3306, false, 'stopped', 'D:\\WinServer\\runtime\\mysql80\\my.ini'),
  service('php73', 'PHP7.3 CGI', 'php', 9073, true, 'running', 'D:\\WinServer\\runtime\\php73\\php.ini'),
  service('redis', 'Redis', 'redis', 6379, true, 'stopped', 'D:\\WinServer\\runtime\\redis-7.2.4\\redis.conf'),
  service('pgsql', 'PostgreSQL', 'pgsql', 5432, false, 'stopped', null),
  service('minio', 'MinIO', 'minio', 9000, true, 'stopped', 'D:\\WinServer\\runtime\\minio\\minio.env'),
]

const software: SoftwareInfo[] = services.map(svc => ({
  id: svc.id,
  name: svc.name,
  category: svc.service_type,
  service_id: svc.id,
  installed: svc.installed,
  installable: true,
  install_note: null,
  download_url: null,
  install_type: 'bundled',
  install_path: svc.installed ? `D:\\WinServer\\runtime\\${svc.id}` : null,
  status: svc.installed ? 'installed' : 'available',
  has_local: svc.id === 'redis' || svc.id === 'minio',
  has_bundled: true,
}))

const settings: SystemSettings = {
  autostart: false,
  start_suite_on_launch: false,
  php_my_admin_url: 'http://127.0.0.1/phpmyadmin',
  port: 80,
  data_dir: 'D:\\WinServer\\data',
  config_path: 'D:\\WinServer\\config',
}

const state: AppState = {
  services,
  sites: [
    {
      id: 'site-localhost',
      name: 'localhost',
      domain: 'localhost',
      port: 80,
      document_root: 'D:\\WinServer\\www\\localhost',
      server_type: 'nginx',
      php_runtime_id: 'php73',
      ssl: false,
      status: 'enabled',
    },
  ],
  databases: [
    {
      name: 'app_demo',
      user: 'app_demo',
      password: 'demo_pass',
      engine: 'mysql80',
      size: '2.4 MB',
      status: 'ok',
      mysql_service_id: 'mysql80',
    },
  ],
  software,
  config_files: [
    { id: 'nginx', label: 'Nginx 配置', path: 'D:\\WinServer\\runtime\\nginx\\conf\\nginx.conf', exists: true },
    { id: 'redis', label: 'Redis 配置', path: 'D:\\WinServer\\runtime\\redis-7.2.4\\redis.conf', exists: true },
    { id: 'minio', label: 'MinIO 环境变量', path: 'D:\\WinServer\\runtime\\minio\\minio.env', exists: true },
  ],
  logs: [
    '[2026-06-15 09:10:00] INFO 开发浏览器 mock 已启动',
    '[2026-06-15 09:10:03] INFO Nginx 服务已启动',
    '[2026-06-15 09:10:05] INFO MySQL 服务已启动',
  ],
  system_settings: settings,
}

const configText: Record<string, string> = {
  'redis.conf': '# Redis configuration\nport 6379\nbind 127.0.0.1\nprotected-mode yes\nmaxmemory 256mb\n',
  'minio.env': 'MINIO_ROOT_USER=minioadmin\nMINIO_ROOT_PASSWORD=minioadmin\nMINIO_API_PORT=9000\nMINIO_CONSOLE_PORT=9001\n',
  nginx: 'events {}\nhttp { include vhosts/*.conf; }\n',
}

function service(
  id: string,
  name: string,
  serviceType: string,
  port: number,
  installed: boolean,
  currentState: ServiceState,
  configFile: string | null,
): ServiceInfo {
  return {
    id,
    name,
    service_type: serviceType,
    state: currentState,
    desired_state: currentState,
    port,
    auto: installed && ['nginx', 'mysql57', 'php73'].includes(id),
    pid: currentState === 'running' ? 1000 + port : null,
    error_message: null,
    config_file: configFile,
    installed,
  }
}

function cloneState() {
  return structuredClone(state)
}

function findService(serviceId: string) {
  return state.services.find(item => item.id === serviceId)
}

function syncSoftware(serviceId: string) {
  const svc = findService(serviceId)
  const sw = state.software.find(item => item.id === serviceId)
  if (!svc || !sw) return
  sw.installed = svc.installed
  sw.install_path = svc.installed ? `D:\\WinServer\\runtime\\${svc.id}` : null
  sw.status = svc.installed ? 'installed' : 'available'
}

function setServiceState(serviceId: string, nextState: ServiceState) {
  const svc = findService(serviceId)
  if (!svc) return
  svc.state = nextState
  svc.desired_state = nextState
  svc.pid = nextState === 'running' ? 1000 + svc.port : null
}

function installService(serviceId: string) {
  const svc = findService(serviceId)
  if (!svc) return
  svc.installed = true
  if (svc.state === 'unknown') setServiceState(serviceId, 'stopped')
  syncSoftware(serviceId)
}

function objectArgs(args: unknown): Record<string, unknown> {
  if (!args || Array.isArray(args) || typeof args !== 'object') return {}
  return args as Record<string, unknown>
}

export function setupTauriDevMock() {
  mockWindows('main')
  mockConvertFileSrc('windows')
  mockIPC((cmd, args) => {
    const payload = objectArgs(args)

    if (cmd.startsWith('plugin:window|')) {
      if (cmd === 'plugin:window|is_maximized') return false
      return null
    }

    if (cmd === 'plugin:dialog|open') return null
    if (cmd === 'app_init_status') return { phase: 'ready', ready: true }
    if (cmd === 'get_state') return cloneState()
    if (cmd === 'get_settings') return { ...settings }
    if (cmd === 'get_system_resource') {
      return {
        cpu_percent: 36,
        cpu_count: 16,
        cpu_model: 'E2E Preview CPU',
        memory_percent: 58,
        total_memory_mb: 32768,
        used_memory_mb: 19005,
        disk: { percent: 72, used_gb: 312, total_gb: 512 },
        uptime_seconds: 4820,
      }
    }
    if (cmd === 'get_logs') return { logs: [...state.logs], path: 'D:\\WinServer\\logs\\operation.log' }
    if (cmd === 'clear_logs') {
      state.logs = []
      return { ok: true }
    }
    if (cmd === 'update_settings') {
      Object.assign(settings, JSON.parse(String(payload.params || '{}')))
      state.system_settings = settings
      return { ok: true }
    }
    if (cmd === 'service_start') {
      setServiceState(String(payload.serviceId), 'running')
      return { state: cloneState() }
    }
    if (cmd === 'service_stop') {
      setServiceState(String(payload.serviceId), 'stopped')
      return { state: cloneState() }
    }
    if (cmd === 'service_restart') {
      setServiceState(String(payload.serviceId), 'running')
      return { state: cloneState() }
    }
    if (cmd === 'suite_start') {
      state.services.filter(svc => svc.installed).forEach(svc => setServiceState(svc.id, 'running'))
      return {
        state: cloneState(),
        summary: state.services.filter(svc => svc.installed).map(svc => ({
          serviceId: svc.id,
          name: svc.name,
          status: 'success',
          message: 'mock started',
        })),
      }
    }
    if (cmd === 'suite_stop') {
      state.services.filter(svc => svc.installed).forEach(svc => setServiceState(svc.id, 'stopped'))
      return { state: cloneState(), summary: [] }
    }
    if (cmd === 'service_toggle_auto') {
      const svc = findService(String(payload.serviceId))
      if (svc) svc.auto = Boolean(payload.auto)
      return { state: cloneState() }
    }
    if (cmd === 'software_install_bundled' || cmd === 'software_download_install') {
      installService(String(payload.softwareId))
      return { state: cloneState(), message: 'mock installed' }
    }
    if (cmd === 'software_download_progress') return { percent: 100, phase: 'done' }
    if (cmd === 'software_uninstall') {
      const svc = findService(String(payload.softwareId))
      if (svc) {
        svc.installed = false
        setServiceState(svc.id, 'stopped')
        syncSoftware(svc.id)
      }
      return { state: cloneState() }
    }
    if (cmd === 'runtime_import') {
      const runtimeType = String(payload.runtimeType || 'nginx')
      const target = runtimeType === 'php' ? 'php73' : 'nginx'
      installService(target)
      return {
        runtime: {
          id: target,
          runtime_type: runtimeType,
          version: 'mock',
          install_path: String(payload.installPath || ''),
          entrypoint: '',
          config_template: null,
          installed: true,
        },
        state: cloneState(),
      }
    }
    if (cmd === 'db_create') {
      state.databases.push({
        name: String(payload.db || 'new_db'),
        user: String(payload.user || 'new_user'),
        engine: 'mysql',
        size: '0 KB',
        status: 'ok',
      })
      return { state: cloneState() }
    }
    if (cmd === 'db_delete') {
      state.databases = state.databases.filter(db => db.name !== payload.dbName)
      return { state: cloneState() }
    }
    if (cmd === 'db_backups') return { backups: [] }
    if (cmd === 'check_port') {
      const port = Number(payload.port || 0)
      return { port, is_open: false, available: true, pid: null, process_name: null, owner_type: null, owner_id: null }
    }
    if (cmd === 'redis_config_get') {
      return {
        path: 'D:\\WinServer\\runtime\\redis-7.2.4\\redis.conf',
        port: 6379,
        bind: '127.0.0.1',
        password: '',
        maxmemory: '256mb',
        maxmemory_policy: 'allkeys-lru',
        appendonly: false,
        protected_mode: true,
      }
    }
    if (cmd === 'minio_config_get') {
      return {
        path: 'D:\\WinServer\\runtime\\minio\\minio.env',
        root_user: 'minioadmin',
        root_password: 'minioadmin',
        api_port: 9000,
        console_port: 9001,
        api_url: 'http://127.0.0.1:9000',
        console_url: 'http://127.0.0.1:9001',
        data_dir: 'D:\\WinServer\\runtime\\minio\\data',
        install_dir: 'D:\\WinServer\\runtime\\minio',
        running: true,
        mc_available: true,
        mc_path: 'D:\\WinServer\\runtime\\minio\\mc.exe',
        cli_alias_cmd: 'mc alias set winserver http://127.0.0.1:9000 minioadmin minioadmin',
        policy_help: {
          public: { id: 'public', label: '公共读写', desc: '匿名用户可读可写', mc: 'mc anonymous set public ALIAS/BUCKET' },
          download: { id: 'download', label: '公共读 · 私有写', desc: '匿名仅可下载', mc: 'mc anonymous set download ALIAS/BUCKET' },
          private: { id: 'private', label: '全私有', desc: '匿名不可访问', mc: 'mc anonymous set none ALIAS/BUCKET' },
        },
      }
    }
    if (cmd === 'minio_buckets_list') {
      return {
        count: 3,
        running: true,
        api_url: 'http://127.0.0.1:9000',
        mc_path: 'D:\\WinServer\\runtime\\minio\\mc.exe',
        buckets: [
          { name: 'demo-public', policy: 'public', policy_label: '公共读写', url: 'http://127.0.0.1:9000/demo-public' },
          { name: 'demo-readonly', policy: 'download', policy_label: '公共读 · 私有写', url: 'http://127.0.0.1:9000/demo-readonly' },
          { name: 'demo-private', policy: 'private', policy_label: '全私有', url: 'http://127.0.0.1:9000/demo-private' },
        ],
      }
    }
    if (cmd === 'minio_bucket_set_policy') {
      const bucket = String(payload.bucket || '')
      const policy = String(payload.policy || 'private')
      const labels: Record<string, string> = {
        public: '公共读写',
        download: '公共读 · 私有写',
        private: '全私有',
      }
      return {
        bucket,
        policy,
        policy_label: labels[policy] || policy,
        message: `桶 ${bucket} 已设为「${labels[policy] || policy}」`,
      }
    }
    if (cmd === 'get_config_file') {
      const fileId = String(payload.fileId || '')
      const file = state.config_files.find(item => item.id === fileId)
      return {
        content: configText[fileId] || '# mock config\n',
        path: file?.path || '',
        exists: true,
      }
    }
    if (cmd === 'save_config_file') {
      const fileId = String(payload.fileId || '')
      configText[fileId] = String(payload.content || '')
      return { state: cloneState() }
    }
    if (cmd === 'redis_config_save' || cmd === 'minio_config_save') return { state: cloneState() }
    if (cmd === 'open_url' || cmd === 'open_file' || cmd === 'open_folder') return { ok: true }
    if (cmd.startsWith('db_')) return { state: cloneState() }
    return null
  })
}
