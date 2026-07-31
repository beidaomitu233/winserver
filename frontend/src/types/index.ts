export interface ServiceInfo {
  id: string
  name: string
  service_type: string
  state: ServiceState
  desired_state: ServiceState
  port: number
  auto: boolean
  pid: number | null
  error_message: string | null
  config_file: string | null
  installed: boolean
}

export type ServiceState = 'installed' | 'starting' | 'running' | 'degraded' | 'stopping' | 'stopped' | 'failed' | 'unknown'

export interface SiteInfo {
  id: string
  name: string
  domain: string
  port: number
  document_root: string
  server_type: string
  php_runtime_id: string | null
  ssl: boolean
  status: string
}

export interface DatabaseInfo {
  name: string
  user: string
  password?: string
  engine: string
  size: string
  status: string
  mysql_service_id?: string
}

export interface SoftwareInfo {
  id: string
  name: string
  category: string
  service_id: string
  installed: boolean
  installable: boolean
  install_note: string | null
  download_url: string | null
  install_type: string
  install_path: string | null
  status: string
  has_local: boolean
  has_bundled: boolean
}

export interface RuntimeManifest {
  id: string
  runtime_type: 'nginx' | 'apache' | 'php' | 'mysql' | 'redis'
  version: string
  install_path: string
  entrypoint: string
  config_template: string | null
  installed: boolean
}

export interface ConfigFileInfo {
  id: string
  label: string
  path: string
  exists: boolean
}

export interface SystemSettings {
  autostart: boolean
  start_suite_on_launch: boolean
  php_my_admin_url: string
  port: number
  data_dir: string
  config_path: string
}

export interface SystemResource {
  cpu_percent: number
  cpu_count: number
  cpu_model: string
  memory_percent: number
  total_memory_mb: number
  used_memory_mb: number
  disk: { percent: number; used_gb: number; total_gb: number }
  uptime_seconds: number
}

export interface AppState {
  services: ServiceInfo[]
  sites: SiteInfo[]
  databases: DatabaseInfo[]
  software: SoftwareInfo[]
  config_files: ConfigFileInfo[]
  logs: string[]
  system_settings: SystemSettings
}

export interface OperationLog {
  id: number
  action: string
  target_type: string | null
  target_id: string | null
  success: boolean
  error_code: string | null
  message: string
  details: { request?: Record<string, unknown> } | null
  created_at: string
}

export interface PortCheckResult {
  port: number
  is_open: boolean
  available: boolean
  pid: number | null
  process_name: string | null
  owner_type: string | null
  owner_id: string | null
}

// Service type/color mapping
export const serviceTypeMap: Record<string, { type: string; color: string }> = {
  apache: { type: 'Web 服务', color: '#ef5a49' },
  nginx: { type: 'Web 服务', color: '#22a95a' },
  mysql57: { type: '数据库', color: '#1384b5' },
  mysql80: { type: '数据库', color: '#1384b5' },
  mysql: { type: '数据库', color: '#1384b5' },
  pgsql: { type: '数据库', color: '#336791' },
  php73: { type: '运行环境', color: '#6772e5' },
  php: { type: '运行环境', color: '#6772e5' },
  redis: { type: '缓存服务', color: '#d8342a' },
  minio: { type: '对象存储', color: '#c72c48' },
}

export function getServiceMeta(id: string) {
  return serviceTypeMap[id] || { type: '服务', color: '#64748b' }
}

export function statusText(state: ServiceState): string {
  const map: Record<ServiceState, string> = {
    installed: '已安装',
    starting: '启动中',
    running: '运行中',
    degraded: '异常',
    stopping: '停止中',
    stopped: '已停止',
    failed: '失败',
    unknown: '未知',
  }
  return map[state] || state
}

export function stateToStatus(state: ServiceState): 'running' | 'stopped' | 'warning' {
  if (state === 'running' || state === 'starting') return 'running'
  if (state === 'failed' || state === 'degraded') return 'warning'
  return 'stopped'
}
