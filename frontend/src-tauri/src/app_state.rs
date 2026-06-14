use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use winserver_agent::database::Database;
use winserver_agent::managers::{
    ProcessManager, SiteManager, PortManager, HostsManager, RuntimeManager,
};
use winserver_agent::server::RequestHandler;
use std::path::PathBuf;
use tracing::{info, warn};

pub struct App {
    pub handler: Arc<RequestHandler>,
    pub process_manager: Arc<ProcessManager>,
    pub init_state: Arc<InitState>,
}

/// Tracks the progress of background service auto-setup so the frontend can
/// render an initialization splash instead of freezing the window.
pub struct InitState {
    phase: Mutex<String>,
    ready: AtomicBool,
}

impl InitState {
    pub fn new() -> Self {
        Self {
            phase: Mutex::new("pending".to_string()),
            ready: AtomicBool::new(false),
        }
    }

    pub async fn phase(&self) -> String {
        self.phase.lock().await.clone()
    }

    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    async fn set_phase(&self, phase: &str) {
        let mut guard = self.phase.lock().await;
        *guard = phase.to_string();
    }

    fn set_ready(&self) {
        self.ready.store(true, Ordering::SeqCst);
    }
}

impl Default for InitState {
    fn default() -> Self {
        Self::new()
    }
}

/// Synchronous, fast initialization: opens the database, builds managers and the
/// request handler. This must return quickly so the Tauri window can render and
/// become interactive. The heavier auto-setup work (local service detection +
/// bundled runtime install) runs later via `run_auto_setup_async`.
pub fn init_app(app_dir: PathBuf, resource_dir: PathBuf) -> App {
    let data_dir = resolve_data_dir(&app_dir);

    let db = Arc::new(Database::new(&data_dir.join("winserver.db")).expect("Failed to open database"));
    db.run_migrations().expect("Failed to run migrations");

    // Migrate data from legacy config.json if it exists and hasn't been migrated yet
    let config_json_path = data_dir.join("config.json");
    db.import_from_config_json(&config_json_path).expect("Failed to import config.json");

    let process_manager = Arc::new(ProcessManager::new(db.clone()));
    let port_manager = Arc::new(PortManager::new());
    let hosts_manager = Arc::new(HostsManager::new());
    let site_manager = Arc::new(SiteManager::new(
        data_dir.clone(),
        process_manager.clone(),
        hosts_manager.clone(),
        db.clone(),
    ).expect("Failed to create SiteManager"));
    let runtime_manager = Arc::new(RuntimeManager::new(data_dir.clone(), db.clone()));

    let runtime_dir = resolve_runtime_dir(&app_dir, &resource_dir);

    let handler = Arc::new(RequestHandler::new(
        db,
        process_manager.clone(),
        site_manager,
        port_manager,
        hosts_manager,
        runtime_manager,
        runtime_dir.clone(),
    ));

    App {
        handler,
        process_manager,
        init_state: Arc::new(InitState::new()),
    }
}

/// Run the (potentially slow) local-service detection and bundled-runtime install
/// on a background task. Updates `init_state` so the frontend splash can reflect
/// progress and know when setup is complete.
pub fn run_auto_setup_async(app: &App, app_dir: PathBuf, resource_dir: PathBuf) {
    let init_state = app.init_state.clone();
    let handler = app.handler.clone();

    tauri::async_runtime::spawn(async move {
        let runtime_dir = resolve_runtime_dir(&app_dir, &resource_dir);

        init_state.set_phase("detecting").await;
        info!("run_auto_setup_async: starting local service detection");

        let result = handler.run_auto_setup(&runtime_dir).await;

        match &result {
            Ok(()) => info!("run_auto_setup_async: completed successfully"),
            Err(e) => warn!("run_auto_setup_async: completed with errors: {}", e),
        }

        init_state.set_phase("ready").await;
        init_state.set_ready();
    });
}

/// Resolve a writable data directory: prefer app_dir/data, fall back to LOCALAPPDATA.
fn resolve_data_dir(app_dir: &std::path::Path) -> PathBuf {
    let preferred = app_dir.join("data");
    if std::fs::create_dir_all(&preferred).is_ok() {
        let test_file = preferred.join(".write_test");
        if std::fs::write(&test_file, b"test").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            return preferred;
        }
    }

    info!("init_app: app_dir/data is not writable, falling back to LOCALAPPDATA");
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| preferred.clone());
    let fallback = local_app_data.join("WinServer").join("data");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Resolve the directory containing bundled runtime resources.
fn resolve_runtime_dir(app_dir: &std::path::Path, resource_dir: &std::path::Path) -> PathBuf {
    if resource_dir.join("nginx-1.26.3.zip").exists() || resource_dir.join("minio.exe").exists() {
        info!("init_app: using resource_dir for bundled runtimes: {}", resource_dir.display());
        resource_dir.to_path_buf()
    } else if app_dir.join("runtime").join("nginx-1.26.3.zip").exists() || app_dir.join("runtime").join("minio.exe").exists() {
        let rd = app_dir.join("runtime");
        info!("init_app: using app_dir/runtime for bundled runtimes: {}", rd.display());
        rd
    } else {
        // During dev mode, check the project runtime/ directory
        let dev_runtime = std::path::PathBuf::from("../../runtime");
        if dev_runtime.join("nginx-1.26.3.zip").exists() {
            info!("init_app: using dev runtime directory: {}", dev_runtime.display());
            dev_runtime
        } else {
            info!("init_app: no bundled runtime directory found");
            resource_dir.to_path_buf()
        }
    }
}
