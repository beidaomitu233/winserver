use serde::{Deserialize, Serialize};

/// Service instance state machine.
/// Desired state and actual state are tracked separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceState {
    Installed,
    Starting,
    Running,
    Degraded,
    Stopping,
    Stopped,
    Failed,
    Unknown,
}

impl ServiceState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Starting | Self::Running | Self::Degraded)
    }
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Installed => write!(f, "installed"),
            Self::Starting => write!(f, "starting"),
            Self::Running => write!(f, "running"),
            Self::Degraded => write!(f, "degraded"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Public service info returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub id: String,
    pub name: String,
    pub service_type: String,
    pub state: ServiceState,
    pub desired_state: ServiceState,
    pub port: u16,
    pub auto: bool,
    pub pid: Option<u32>,
    pub error_message: Option<String>,
    pub config_file: Option<String>,
    pub installed: bool,
}

/// Site information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub port: u16,
    pub document_root: String,
    pub server_type: ServerType,
    pub php_runtime_id: Option<String>,
    pub ssl: bool,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerType {
    Nginx,
    Apache,
}

impl std::fmt::Display for ServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nginx => write!(f, "nginx"),
            Self::Apache => write!(f, "apache"),
        }
    }
}

/// Runtime manifest for PHP/Nginx/etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub id: String,
    pub runtime_type: RuntimeType,
    pub version: String,
    pub install_path: String,
    pub entrypoint: String,
    pub config_template: Option<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    Nginx,
    Apache,
    Php,
    Mysql,
    Redis,
}

/// Full application state returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub services: Vec<ServiceInfo>,
    pub sites: Vec<SiteInfo>,
    pub databases: Vec<DatabaseInfo>,
    pub software: Vec<SoftwareInfo>,
    pub config_files: Vec<ConfigFileInfo>,
    pub logs: Vec<String>,
    pub system_settings: SystemSettings,
    pub version: String,
}

/// Structured audit entry returned by `log.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub id: i64,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub success: bool,
    pub error_code: Option<String>,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub user: String,
    /// Database account password (UI should mask by default).
    #[serde(default)]
    pub password: String,
    pub engine: String,
    pub size: String,
    pub status: String,
    /// Owning MySQL service id (`mysql57` / `mysql80`). Used to scope lists per version.
    #[serde(default)]
    pub mysql_service_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub service_id: String,
    pub installed: bool,
    pub installable: bool,
    pub install_note: Option<String>,
    pub download_url: Option<String>,
    pub install_type: String,
    pub install_path: Option<String>,
    pub status: String,
    pub has_local: bool,
    pub has_bundled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFileInfo {
    pub id: String,
    pub label: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub autostart: bool,
    pub start_suite_on_launch: bool,
    pub php_my_admin_url: String,
    pub port: u16,
    pub data_dir: String,
    pub config_path: String,
}

/// System resource data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResource {
    pub cpu_percent: f64,
    pub cpu_count: usize,
    pub cpu_model: String,
    pub memory_percent: f64,
    pub total_memory_mb: u64,
    pub used_memory_mb: u64,
    pub disk: DiskInfo,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub percent: f64,
    pub used_gb: f64,
    pub total_gb: f64,
}

/// Port check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortCheckResult {
    pub port: u16,
    pub is_open: bool,
    pub available: bool,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub owner_type: Option<String>,
    pub owner_id: Option<String>,
}
