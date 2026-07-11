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

    let port_manager = Arc::new(PortManager::new());
    let process_manager = Arc::new(ProcessManager::new(db.clone(), port_manager.clone()));
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
    let process_manager = app.process_manager.clone();

    tauri::async_runtime::spawn(async move {
        let runtime_dir = resolve_runtime_dir(&app_dir, &resource_dir);

        // Step 1: Reconcile process tracking
        init_state.set_phase("reconciling").await;
        info!("run_auto_setup_async: reconciling process tracking");
        if let Err(e) = process_manager.reconcile_process_tracking().await {
            warn!("run_auto_setup_async: reconcile failed: {}", e);
        }

        // Step 2: Detect local services and install bundled runtimes
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
/// Always returns an absolute path so service logs/configs never resolve under
/// accidental relative locations like `target/debug/deps/data`.
fn resolve_data_dir(app_dir: &std::path::Path) -> PathBuf {
    let preferred = app_dir.join("data");
    if std::fs::create_dir_all(&preferred).is_ok() {
        let test_file = preferred.join(".write_test");
        if std::fs::write(&test_file, b"test").is_ok() {
            let _ = std::fs::remove_file(&test_file);
            return absolutize(&preferred);
        }
    }

    info!("init_app: app_dir/data is not writable, falling back to LOCALAPPDATA");
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| preferred.clone());
    let fallback = local_app_data.join("WinServer").join("data");
    let _ = std::fs::create_dir_all(&fallback);
    absolutize(&fallback)
}

fn absolutize(path: &std::path::Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// Resolve the directory containing bundled runtime resources.
fn resolve_runtime_dir(app_dir: &std::path::Path, resource_dir: &std::path::Path) -> PathBuf {
    if has_runtime_marker(resource_dir) {
        info!("init_app: using resource_dir for bundled runtimes: {}", resource_dir.display());
        resource_dir.to_path_buf()
    } else if has_runtime_marker(&app_dir.join("runtime")) {
        let rd = app_dir.join("runtime");
        info!("init_app: using app_dir/runtime for bundled runtimes: {}", rd.display());
        rd
    } else if let Some(packaged_runtime_dir) = find_packaged_runtime_dir(app_dir, resource_dir) {
        info!(
            "init_app: using packaged runtime directory: {}",
            packaged_runtime_dir.display()
        );
        packaged_runtime_dir
    } else {
        // Try several dev-mode fallbacks relative to the binary
        let candidates = [
            std::path::PathBuf::from("../../runtime"),           // from target/debug/
            std::path::PathBuf::from("../../../runtime"),        // from target/debug/ (deeper)
            std::path::PathBuf::from("runtime"),                  // from project root
        ];
        for candidate in &candidates {
            if has_runtime_marker(candidate) {
                info!("init_app: using dev runtime directory: {}", candidate.display());
                return candidate.clone();
            }
        }
        // Try the parent chain: walk up looking for a runtime/ directory
        let mut current = app_dir.to_path_buf();
        for _ in 0..6 {
            let candidate = current.join("runtime");
            if has_runtime_marker(&candidate) {
                info!("init_app: found runtime directory by walking up: {}", candidate.display());
                return candidate;
            }
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }
        info!("init_app: no bundled runtime directory found, falling back to resource_dir");
        resource_dir.to_path_buf()
    }
}

fn find_packaged_runtime_dir(
    app_dir: &std::path::Path,
    resource_dir: &std::path::Path,
) -> Option<PathBuf> {
    let candidates = [
        resource_dir.join("runtime"),
        resource_dir.join("_up_").join("runtime"),
        resource_dir.join("_up_").join("_up_").join("runtime"),
        app_dir.join("runtime"),
        app_dir.join("_up_").join("runtime"),
        app_dir.join("_up_").join("_up_").join("runtime"),
    ];
    for candidate in candidates {
        if has_runtime_marker(&candidate) {
            return Some(candidate);
        }
    }

    for base in [resource_dir, app_dir] {
        if let Some(found) = find_runtime_dir_shallow(base, 4) {
            return Some(found);
        }
    }

    None
}

fn find_runtime_dir_shallow(base: &std::path::Path, max_depth: usize) -> Option<PathBuf> {
    if max_depth == 0 || !base.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(base).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("runtime")
            && has_runtime_marker(&path)
        {
            return Some(path);
        }
        if let Some(found) = find_runtime_dir_shallow(&path, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

fn has_runtime_marker(dir: &std::path::Path) -> bool {
    dir.join("nginx-1.26.3.zip").exists()
        || dir.join("minio.exe").exists()
        || dir.join("minio").join("minio.exe").exists()
        || dir.join("redis-7.2.4").join("redis-server.exe").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("winserver-app-state-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn resolves_nsis_packaged_runtime_under_up_up() {
        let app_dir = temp_dir();
        let resource_dir = app_dir.clone();
        let runtime_dir = app_dir.join("_up_").join("_up_").join("runtime");
        fs::create_dir_all(runtime_dir.join("redis-7.2.4")).expect("create runtime dir");
        fs::write(runtime_dir.join("redis-7.2.4").join("redis-server.exe"), "")
            .expect("write runtime marker");

        let resolved = resolve_runtime_dir(&app_dir, &resource_dir);

        assert_eq!(resolved, runtime_dir);
    }
}
