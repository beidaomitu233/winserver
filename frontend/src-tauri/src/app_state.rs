use std::sync::Arc;
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
}

/// Initialize the app with the given directories.
/// `app_dir` = directory containing the exe (and possibly runtime/ resources)
/// `resource_dir` = Tauri's resource directory (where bundled resources are placed)
pub fn init_app(app_dir: PathBuf, resource_dir: PathBuf) -> App {
    // Use a writable data directory: prefer app_dir/data, fall back to LOCALAPPDATA/WinServer
    let data_dir = {
        let preferred = app_dir.join("data");
        if std::fs::create_dir_all(&preferred).is_ok() {
            // Test write access
            let test_file = preferred.join(".write_test");
            if std::fs::write(&test_file, b"test").is_ok() {
                let _ = std::fs::remove_file(&test_file);
                preferred
            } else {
                info!("init_app: app_dir/data is not writable, falling back to LOCALAPPDATA");
                let local_app_data = std::env::var("LOCALAPPDATA")
                    .map(|p| PathBuf::from(p))
                    .unwrap_or_else(|_| preferred.clone());
                let fallback = local_app_data.join("WinServer").join("data");
                let _ = std::fs::create_dir_all(&fallback);
                fallback
            }
        } else {
            info!("init_app: cannot create app_dir/data, falling back to LOCALAPPDATA");
            let local_app_data = std::env::var("LOCALAPPDATA")
                .map(|p| PathBuf::from(p))
                .unwrap_or_else(|_| preferred.clone());
            let fallback = local_app_data.join("WinServer").join("data");
            let _ = std::fs::create_dir_all(&fallback);
            fallback
        }
    };

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

    // Detect local services and install bundled runtimes on first launch
    // Try resource_dir first (Tauri bundled), then fall back to app_dir/runtime/
    let runtime_dir = if resource_dir.join("nginx-1.26.3.zip").exists() || resource_dir.join("minio.exe").exists() {
        info!("init_app: using resource_dir for bundled runtimes: {}", resource_dir.display());
        resource_dir
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
            resource_dir
        }
    };

    auto_setup_services(&db, &runtime_manager, &runtime_dir);

    let handler = Arc::new(RequestHandler::new(
        db,
        process_manager.clone(),
        site_manager,
        port_manager,
        hosts_manager,
        runtime_manager,
        runtime_dir.clone(),
    ));

    App { handler, process_manager }
}

/// Auto-detect local services and install bundled runtimes for missing ones.
fn auto_setup_services(db: &Arc<Database>, runtime_manager: &Arc<RuntimeManager>, runtime_dir: &std::path::Path) {
    // 1. Detect local services
    let local_services = runtime_manager.detect_local_services();
    info!("auto_setup_services: detected {} local services", local_services.len());

    // 2. Update has_local and has_bundled flags in DB
    let software_list = match db.list_software() {
        Ok(list) => list,
        Err(e) => {
            warn!("auto_setup_services: failed to list software: {}", e);
            return;
        }
    };

    for sw in &software_list {
        let has_local = local_services.contains_key(&sw.service_id);
        if has_local != sw.has_local {
            if let Err(e) = db.update_software_has_local(&sw.id, has_local) {
                warn!("auto_setup_services: failed to update has_local for {}: {}", sw.id, e);
            }
        }

        let has_bundled = runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_some();
        if has_bundled != sw.has_bundled {
            if let Err(e) = db.update_software_has_bundled(&sw.id, has_bundled) {
                warn!("auto_setup_services: failed to update has_bundled for {}: {}", sw.id, e);
            }
        }
    }

    // 3. Auto-install bundled runtimes for services that have no local version and are not yet installed
    for sw in &software_list {
        if sw.installed {
            continue;
        }
        if local_services.contains_key(&sw.service_id) {
            info!("auto_setup_services: {} has local installation at {}, skipping bundled install", sw.id, local_services[&sw.service_id]);
            continue;
        }

        if runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_none() {
            continue;
        }

        info!("auto_setup_services: auto-installing bundled runtime for {}", sw.id);
        match runtime_manager.install_bundled_runtime(&sw.id, runtime_dir) {
            Ok(manifest) => {
                info!("auto_setup_services: successfully installed bundled {} (version={})", sw.id, manifest.version);
            }
            Err(e) => {
                warn!("auto_setup_services: failed to install bundled {}: {}", sw.id, e);
            }
        }
    }
}
