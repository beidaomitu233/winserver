use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde_json::json;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use shared::protocol::{error_codes, JsonRpcRequest, JsonRpcResponse};
use shared::types::{AppState, RuntimeType, ServiceState};
use crate::database::Database;
use crate::managers::{
    ProcessManager, SiteManager, PortManager, HostsManager, RuntimeManager,
    DownloadProgress,
};

struct CachedState {
    state: AppState,
    at: Instant,
}

/// Resolved MySQL client tools (never the server binary `mysqld.exe`).
#[derive(Debug)]
struct MysqlTools {
    mysql_exe: PathBuf,
    mysqldump_exe: PathBuf,
    port: u16,
}

fn prefer_bundled_runtime(software_id: &str, service_id: &str) -> bool {
    matches!(
        (software_id, service_id),
        ("mysql80", "mysql80") | ("redis", "redis") | ("minio", "minio")
    )
}

fn should_skip_local_service_registration(
    software_id: &str,
    service_id: &str,
    preferred_bundled_service_ids: &HashSet<String>,
) -> bool {
    preferred_bundled_service_ids.contains(service_id) && !prefer_bundled_runtime(software_id, service_id)
}

pub struct RequestHandler {
    db: Arc<Database>,
    process_manager: Arc<ProcessManager>,
    site_manager: Arc<SiteManager>,
    port_manager: Arc<PortManager>,
    hosts_manager: Arc<HostsManager>,
    runtime_manager: Arc<RuntimeManager>,
    state_cache: RwLock<Option<CachedState>>,
    download_progress: RwLock<HashMap<String, Arc<DownloadProgress>>>,
    runtime_dir: std::path::PathBuf,
}

impl RequestHandler {
    pub fn new(
        db: Arc<Database>,
        process_manager: Arc<ProcessManager>,
        site_manager: Arc<SiteManager>,
        port_manager: Arc<PortManager>,
        hosts_manager: Arc<HostsManager>,
        runtime_manager: Arc<RuntimeManager>,
        runtime_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            db,
            process_manager,
            site_manager,
            port_manager,
            hosts_manager,
            runtime_manager,
            state_cache: RwLock::new(None),
            download_progress: RwLock::new(HashMap::new()),
            runtime_dir,
        }
    }

    /// Run local-service detection and install missing bundled runtimes.
    /// Offloaded to a background task at startup so the window stays responsive.
    pub async fn run_auto_setup(&self, runtime_dir: &std::path::Path) -> anyhow::Result<()> {
        // 1. Detect local services
        let local_services = self.runtime_manager.detect_local_services();
        info!("run_auto_setup: detected {} local services", local_services.len());

        // 2. Update has_local and has_bundled flags in DB
        let software_list = self.db.list_software()?;

        for sw in &software_list {
            let has_local = local_services.contains_key(&sw.service_id);
            if has_local != sw.has_local {
                if let Err(e) = self.db.update_software_has_local(&sw.id, has_local) {
                    warn!("run_auto_setup: failed to update has_local for {}: {}", sw.id, e);
                }
            }

            let has_bundled = self.runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_some();
            if has_bundled != sw.has_bundled {
                if let Err(e) = self.db.update_software_has_bundled(&sw.id, has_bundled) {
                    warn!("run_auto_setup: failed to update has_bundled for {}: {}", sw.id, e);
                }
            }
        }

        // 3. Auto-install bundled runtimes for services that have no local version and are not yet installed.
        //    For locally-detected services, register them into the DB so they
        //    show as installed (instead of leaving installed=0 forever).
        let preferred_bundled_service_ids: HashSet<String> = software_list
            .iter()
            .filter(|sw| prefer_bundled_runtime(&sw.id, &sw.service_id))
            .filter(|sw| self.runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_some())
            .map(|sw| sw.service_id.clone())
            .collect();

        for sw in &software_list {
            let has_bundled = self.runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_some();
            if has_bundled && prefer_bundled_runtime(&sw.id, &sw.service_id) {
                info!("run_auto_setup: installing preferred bundled runtime for {}", sw.id);
                if let Err(e) = self.runtime_manager.install_bundled_runtime(&sw.id, runtime_dir) {
                    warn!("run_auto_setup: failed to install preferred bundled {}: {}", sw.id, e);
                }
                continue;
            }

            if should_skip_local_service_registration(&sw.id, &sw.service_id, &preferred_bundled_service_ids) {
                info!(
                    "run_auto_setup: skipping local registration for {} because service {} is managed by a preferred bundled runtime",
                    sw.id, sw.service_id
                );
                continue;
            }

            if sw.installed {
                if matches!(sw.service_id.as_str(), "redis" | "minio") {
                    if let Some(install_path) = sw.install_path.as_deref().filter(|path| !path.trim().is_empty()) {
                        if let Err(e) = self.runtime_manager.register_detected_service(&sw.id, &sw.service_id, install_path) {
                            warn!("run_auto_setup: failed to repair registered {}: {}", sw.id, e);
                        }
                    }
                }
                continue;
            }
            if let Some(install_path) = local_services.get(&sw.service_id) {
                info!("run_auto_setup: {} found locally at {}, registering as installed", sw.id, install_path);
                if let Err(e) = self.runtime_manager.register_detected_service(&sw.id, &sw.service_id, install_path) {
                    warn!("run_auto_setup: failed to register local {}: {}", sw.id, e);
                }
                continue;
            }

            if !has_bundled {
                continue;
            }

            info!("run_auto_setup: auto-installing bundled runtime for {}", sw.id);
            if let Err(e) = self.runtime_manager.install_bundled_runtime(&sw.id, runtime_dir) {
                warn!("run_auto_setup: failed to install bundled {}: {}", sw.id, e);
            }
        }

        // Ensure mc.exe is always co-located with any registered MinIO install.
        if let Ok(cfg) = self.db.get_service_config("minio") {
            let install = cfg
                .cwd
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    Path::new(&cfg.exe)
                        .parent()
                        .map(|p| p.to_path_buf())
                });
            if let Some(dir) = install {
                if let Err(e) = self
                    .runtime_manager
                    .ensure_minio_client(&dir, Some(runtime_dir))
                {
                    warn!(
                        "run_auto_setup: ensure mc for minio at {} failed: {}",
                        dir.display(),
                        e
                    );
                } else {
                    info!("run_auto_setup: minio client (mc) ready at {}", dir.display());
                }
            }
        }

        Ok(())
    }

    pub async fn handle(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        let method = request.method.as_str();
        let params = request.params.clone();

        info!("Handling request: {} ({})", method, id);

        let result = match method {
            "state.get" => self.handle_get_state().await,
            "state.getFast" => self.handle_get_state_fast().await,
            "service.start" => self.handle_service_start(&params).await,
            "service.stop" => self.handle_service_stop(&params).await,
            "service.restart" => self.handle_service_restart(&params).await,
            "suite.start" => self.handle_suite_start().await,
            "suite.stop" => self.handle_suite_stop().await,
            "site.create" => self.handle_site_create(&params).await,
            "site.delete" => self.handle_site_delete(&params).await,
            "site.update" => self.handle_site_update(&params).await,
            "site.list" => self.handle_site_list().await,
            "site.config" => self.handle_site_config(&params).await,
            "site.switchPhp" => self.handle_site_switch_php(&params).await,
            "site.enable" => self.handle_site_enable(&params).await,
            "site.disable" => self.handle_site_disable(&params).await,
            "port.check" => self.handle_port_check(&params).await,
            "port.killProcess" => self.handle_port_kill_process(&params).await,
            "resource.get" => self.handle_resource_get().await,
            "log.list" => self.handle_log_list(&params).await,
            "log.clear" => self.handle_log_clear(&params).await,
            "settings.get" => self.handle_settings_get().await,
            "settings.update" => self.handle_settings_update(&params).await,
            "settings.paths" => self.handle_settings_paths(&params).await,
            "runtime.list" => self.handle_runtime_list().await,
            "runtime.import" => self.handle_runtime_import(&params).await,
            "hosts.sync" => self.handle_hosts_sync(&params).await,
            "hosts.remove" => self.handle_hosts_remove(&params).await,
            "config.get" => self.handle_config_get(&params).await,
            "config.save" => self.handle_config_save(&params).await,
            "redis.config.get" => self.handle_redis_config_get(&params).await,
            "redis.config.save" => self.handle_redis_config_save(&params).await,
            "minio.config.get" => self.handle_minio_config_get(&params).await,
            "minio.config.save" => self.handle_minio_config_save(&params).await,
            "minio.buckets.list" => self.handle_minio_buckets_list(&params).await,
            "minio.bucket.setPolicy" => self.handle_minio_bucket_set_policy(&params).await,
            "database.create" => self.handle_database_create(&params).await,
            "database.delete" => self.handle_database_delete(&params).await,
            "database.changePassword" => self.handle_database_change_password(&params).await,
            "database.rootPassword" => self.handle_database_root_password(&params).await,
            "database.export" => self.handle_database_export(&params).await,
            "database.import" => self.handle_database_import(&params).await,
            "database.backups" => self.handle_database_backups().await,
            "database.deleteBackup" => self.handle_database_delete_backup(&params).await,
            "database.pgList" => self.handle_database_pg_list(&params).await,
            "database.pgCreate" => self.handle_database_pg_create(&params).await,
            "software.install" => self.handle_software_install(&params).await,
            "software.uninstall" => self.handle_software_uninstall(&params).await,
            "software.downloadInstall" => self.handle_software_download_install(&params).await,
            "software.downloadProgress" => self.handle_software_download_progress(&params).await,
            "software.installBundled" => self.handle_software_install_bundled(&params).await,
            "software.detectLocal" => self.handle_software_detect_local().await,
            "service.toggleAuto" => self.handle_service_toggle_auto(&params).await,
            _ => {
                warn!("Unknown method: {}", method);
                return JsonRpcResponse::error_with_data(
                    &id,
                    -32601,
                    format!("Method not found: {}", method),
                    json!({
                        "errorCode": "METHOD_NOT_FOUND",
                        "method": method,
                    }),
                );
            }
        };

        if let Err(audit_error) = self.record_request_audit(method, &params, &result) {
            warn!("Failed to write operation audit for {}: {}", method, audit_error);
        }

        match result {
            Ok(value) => JsonRpcResponse::success(&id, value),
            Err(e) => {
                error!("Error handling {}: {}", method, e);
                self.error_response(&id, method, &e)
            }
        }
    }

    async fn handle_get_state(&self) -> anyhow::Result<serde_json::Value> {
        let state = self.build_app_state().await?;
        Ok(serde_json::to_value(state)?)
    }

    async fn handle_service_start(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let service_id = params["serviceId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing serviceId"))?;

        self.process_manager.start_service(service_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("{} 已启动", service_id) }))
    }

    async fn handle_service_stop(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let service_id = params["serviceId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing serviceId"))?;

        self.process_manager.stop_service(service_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("{} 已停止", service_id) }))
    }

    async fn handle_service_restart(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let service_id = params["serviceId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing serviceId"))?;

        self.process_manager.stop_service(service_id).await?;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        self.process_manager.start_service(service_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("{} 已重启", service_id) }))
    }

    async fn handle_suite_start(&self) -> anyhow::Result<serde_json::Value> {
        let services = self.db.list_service_instances()?;
        let mut summary = Vec::new();
        for svc in &services {
            if !svc.auto {
                continue;
            }
            if !svc.installed {
                summary.push(json!({
                    "serviceId": svc.id,
                    "name": svc.name,
                    "status": "skipped",
                    "message": "服务未安装"
                }));
                continue;
            }
            match self.process_manager.start_service(&svc.id).await {
                Ok(_) => summary.push(json!({
                    "serviceId": svc.id,
                    "name": svc.name,
                    "status": "success",
                    "message": "已启动"
                })),
                Err(e) => {
                    warn!("Failed to start {}: {}", svc.id, e);
                    summary.push(json!({
                        "serviceId": svc.id,
                        "name": svc.name,
                        "status": "failed",
                        "message": e.to_string()
                    }));
                }
            }
        }
        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "summary": summary }))
    }

    async fn handle_suite_stop(&self) -> anyhow::Result<serde_json::Value> {
        let services = self.db.list_service_instances()?;
        let mut summary = Vec::new();
        for svc in &services {
            if !svc.auto {
                continue;
            }
            if !svc.installed {
                summary.push(json!({
                    "serviceId": svc.id,
                    "name": svc.name,
                    "status": "skipped",
                    "message": "服务未安装"
                }));
                continue;
            }
            match self.process_manager.stop_service(&svc.id).await {
                Ok(_) => summary.push(json!({
                    "serviceId": svc.id,
                    "name": svc.name,
                    "status": "success",
                    "message": "已停止"
                })),
                Err(e) => {
                    warn!("Failed to stop {}: {}", svc.id, e);
                    summary.push(json!({
                        "serviceId": svc.id,
                        "name": svc.name,
                        "status": "failed",
                        "message": e.to_string()
                    }));
                }
            }
        }
        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "summary": summary }))
    }

    async fn handle_site_create(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let domain = params["domain"].as_str().unwrap_or("").to_string();
        let port = params["port"].as_u64().unwrap_or(80) as u16;
        let path = params["path"].as_str().unwrap_or("").to_string();
        let server = params["server"].as_str().unwrap_or("nginx").to_string();
        let php_runtime_id = params["phpRuntimeId"].as_str();

        self.site_manager.create_site(&domain, port, &path, &server, php_runtime_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    async fn handle_site_delete(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing siteId"))?;

        self.site_manager.delete_site(site_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    async fn handle_site_list(&self) -> anyhow::Result<serde_json::Value> {
        let sites = self.db.list_sites()?;
        Ok(serde_json::to_value(sites)?)
    }

    async fn handle_site_switch_php(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing siteId"))?;
        let php_runtime_id = params["phpRuntimeId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing phpRuntimeId"))?;

        self.site_manager.switch_php(site_id, php_runtime_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    async fn handle_site_enable(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing siteId"))?;

        self.site_manager.enable_site(site_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    async fn handle_site_disable(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing siteId"))?;

        self.site_manager.disable_site(site_id).await?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    async fn handle_port_check(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let port = params["port"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing port"))? as u16;
        let mut result = self.port_manager.check_port(port);
        if result.is_open {
            if let Some(service) = self
                .db
                .list_service_instances()?
                .into_iter()
                .find(|service| service.port == port && service.installed)
            {
                result.owner_type = Some("winserver_service".to_string());
                result.owner_id = Some(service.id);
            }
        }
        Ok(serde_json::to_value(result)?)
    }

    async fn handle_port_kill_process(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let pid = params["pid"].as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing pid"))? as u32;
        self.port_manager.kill_process(pid)?;
        Ok(json!({ "message": format!("进程 {} 已终止", pid) }))
    }

    async fn handle_resource_get(&self) -> anyhow::Result<serde_json::Value> {
        let resource = self.process_manager.get_system_resource()?;
        Ok(serde_json::to_value(resource)?)
    }

    async fn handle_log_list(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let source = params["source"].as_str().unwrap_or("operation");
        let search = params["search"].as_str().unwrap_or("").trim();

        if source != "operation" {
            anyhow::bail!("仅支持操作日志");
        }
        let logs = self.db.list_operation_logs(500, search)?;
        Ok(json!({ "source": source, "logs": logs }))
    }

    async fn handle_log_clear(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let source = params["source"].as_str().unwrap_or("operation");
        if source != "operation" {
            anyhow::bail!("仅支持操作日志");
        }
        self.db.clear_logs()?;
        Ok(json!({ "message": "操作日志已清空" }))
    }

    async fn handle_settings_get(&self) -> anyhow::Result<serde_json::Value> {
        let settings = self.db.get_settings()?;
        Ok(serde_json::to_value(settings)?)
    }

    async fn handle_settings_update(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        self.db.update_settings(params)?;
        Ok(json!({ "message": "设置已更新" }))
    }

    async fn handle_runtime_list(&self) -> anyhow::Result<serde_json::Value> {
        let runtimes = self.runtime_manager.list_runtimes()?;
        Ok(serde_json::to_value(runtimes)?)
    }

    async fn handle_runtime_import(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let runtime_type_text = params["runtimeType"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing runtimeType"))?;
        let install_path = params["installPath"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing installPath"))?;
        let port = params["port"].as_u64().map(|value| value as u16);

        let runtime_type_text = runtime_type_text.to_lowercase();
        // MySQL is imported as a registered generic service (5.7 or 8.0), not via RuntimeType::Mysql tree.
        if runtime_type_text == "mysql" || runtime_type_text.starts_with("mysql") {
            let service_id = if runtime_type_text.contains("57") || runtime_type_text.contains("5.7") {
                "mysql57"
            } else if runtime_type_text.contains("80") || runtime_type_text.contains("8") {
                "mysql80"
            } else {
                // Prefer free slot: if 8.0 installed, map to 5.7 and vice versa; default 8.0.
                let has80 = self
                    .db
                    .get_service_config("mysql80")
                    .map(|c| c.installed)
                    .unwrap_or(false);
                if has80 { "mysql57" } else { "mysql80" }
            };
            let software_id = service_id;
            self.runtime_manager
                .import_mysql_service(software_id, service_id, install_path)?;
            let state = self.build_app_state().await?;
            return Ok(json!({
                "runtime": {
                    "id": service_id,
                    "runtime_type": "mysql",
                    "version": service_id,
                    "install_path": install_path,
                    "entrypoint": format!("{}/bin/mysqld.exe", install_path.replace('\\', "/")),
                    "config_template": format!("{}/my.ini", install_path.replace('\\', "/")),
                    "installed": true
                },
                "state": state,
                "message": format!("{} 已导入（端口 3306）", service_id)
            }));
        }

        let runtime_type = match runtime_type_text.as_str() {
            "nginx" => RuntimeType::Nginx,
            "php" => RuntimeType::Php,
            _ => anyhow::bail!("支持导入：Nginx、PHP、MySQL"),
        };

        let runtime = self.runtime_manager.import_runtime(runtime_type, install_path, port)?;
        let state = self.build_app_state().await?;
        Ok(json!({ "runtime": runtime, "state": state }))
    }

    async fn handle_hosts_sync(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let domain = params["domain"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing domain"))?;
        self.hosts_manager.sync_hosts(domain)?;
        Ok(json!({ "message": format!("hosts 已同步: {}", domain) }))
    }

    async fn handle_hosts_remove(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let domain = params["domain"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing domain"))?;
        self.hosts_manager.remove_hosts(domain)?;
        Ok(json!({ "message": format!("hosts 已移除: {}", domain) }))
    }

    // ---- Config file handlers ----

    async fn handle_config_get(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let file_id = params["fileId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing fileId"))?;

        // Site vhost configs use site:<id>
        if let Some(site_id) = file_id.strip_prefix("site:") {
            let vhost_path = self.site_manager.get_site_vhost_path(site_id)?;
            let exists = vhost_path.exists();
            let content = if exists {
                std::fs::read_to_string(&vhost_path)?
            } else {
                String::new()
            };
            return Ok(json!({
                "id": file_id,
                "label": vhost_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                "path": vhost_path.to_string_lossy(),
                "exists": exists,
                "content": content
            }));
        }

        let path = self
            .resolve_config_path(file_id)
            .ok_or_else(|| anyhow::anyhow!("配置文件未找到或服务未安装：{}", file_id))?;

        let exists = Path::new(&path).exists();
        let content = if exists {
            std::fs::read_to_string(&path)?
        } else {
            String::new()
        };

        Ok(json!({
            "id": file_id,
            "label": file_id,
            "path": path,
            "exists": exists,
            "content": content
        }))
    }

    async fn handle_config_save(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let file_id = params["fileId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing fileId"))?;
        let content = params["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;

        let path = self
            .resolve_config_path(file_id)
            .ok_or_else(|| anyhow::anyhow!("配置文件未找到或服务未安装：{}", file_id))?;

        // Create backup before saving
        if Path::new(&path).exists() {
            let backup_path = format!("{}.bak", path);
            if let Err(e) = std::fs::copy(&path, &backup_path) {
                warn!("Failed to create backup for {}: {}", path, e);
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        std::fs::write(&path, content)?;

        if file_id == "redis.conf" {
            let parsed = parse_redis_conf(content, &path);
            if let Some(port) = parsed.get("port").and_then(|value| value.as_u64()) {
                let _ = self.db.update_service_port("redis", port as u16);
            }
        } else if file_id == "minio.env" {
            let parsed = parse_minio_env(content, &path);
            if let Some(port) = parsed.get("api_port").and_then(|value| value.as_u64()) {
                let _ = self.db.update_service_port("minio", port as u16);
            }
        }

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "配置已保存" }))
    }

    // ---- Structured Redis/MinIO config (visual form mode) ----

    async fn handle_redis_config_get(&self, _params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self
            .resolve_config_path("redis.conf")
            .ok_or_else(|| anyhow::anyhow!("Redis 配置未注册，请先安装 Redis"))?;
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        Ok(parse_redis_conf(&raw, &path))
    }

    async fn handle_redis_config_save(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self
            .resolve_config_path("redis.conf")
            .ok_or_else(|| anyhow::anyhow!("Redis 配置未注册，请先安装 Redis"))?;

        // Backup before write
        backup_config(&path);

        let port = params["port"].as_u64().unwrap_or(6379) as u16;
        let bind = params["bind"].as_str().unwrap_or("127.0.0.1");
        let password = params["password"].as_str().unwrap_or("");
        let maxmemory = params["maxmemory"].as_str().unwrap_or("256mb");
        let policy = params["maxmemory_policy"].as_str().unwrap_or("allkeys-lru");
        let appendonly = params["appendonly"].as_bool().unwrap_or(false);
        let protected = params["protected_mode"].as_bool().unwrap_or(true);

        let content = render_redis_conf(port, bind, password, maxmemory, policy, appendonly, protected);
        std::fs::write(&path, &content)?;

        // Persist port back to the service instance so start checks the right port.
        let _ = self.db.update_service_port("redis", port);

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "Redis 配置已保存" }))
    }

    async fn handle_minio_config_get(&self, _params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self
            .resolve_config_path("minio.env")
            .ok_or_else(|| anyhow::anyhow!("MinIO 配置未注册，请先安装 MinIO"))?;
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        let mut info = parse_minio_env(&raw, &path);
        let ctx = self.resolve_minio_context()?;
        let running = self.process_manager.is_service_running("minio").await
            || crate::managers::ProcessManager::is_port_listening(ctx.api_port);

        if let Some(obj) = info.as_object_mut() {
            obj.insert("api_url".into(), json!(format!("http://127.0.0.1:{}", ctx.api_port)));
            obj.insert(
                "console_url".into(),
                json!(format!("http://127.0.0.1:{}", ctx.console_port)),
            );
            obj.insert("data_dir".into(), json!(ctx.data_dir.display().to_string()));
            obj.insert(
                "install_dir".into(),
                json!(ctx.install_dir.display().to_string()),
            );
            obj.insert("running".into(), json!(running));
            obj.insert("mc_available".into(), json!(ctx.mc_exe.is_some()));
            obj.insert(
                "mc_path".into(),
                json!(ctx
                    .mc_exe
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()),
            );
            obj.insert(
                "cli_alias_cmd".into(),
                json!(format!(
                    "mc alias set winserver http://127.0.0.1:{} {} {}",
                    ctx.api_port, ctx.root_user, ctx.root_password
                )),
            );
            obj.insert(
                "policy_help".into(),
                json!({
                    "public": {
                        "id": "public",
                        "label": "公共读写",
                        "desc": "匿名用户可读可写（下载 + 上传 + 删除）",
                        "mc": "mc anonymous set public ALIAS/BUCKET"
                    },
                    "download": {
                        "id": "download",
                        "label": "公共读 · 私有写",
                        "desc": "匿名用户仅可下载/列表，上传需鉴权",
                        "mc": "mc anonymous set download ALIAS/BUCKET"
                    },
                    "private": {
                        "id": "private",
                        "label": "全私有",
                        "desc": "匿名不可访问，读写均需 Access Key",
                        "mc": "mc anonymous set none ALIAS/BUCKET"
                    }
                }),
            );
        }
        Ok(info)
    }

    async fn handle_minio_config_save(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self
            .resolve_config_path("minio.env")
            .ok_or_else(|| anyhow::anyhow!("MinIO 配置未注册，请先安装 MinIO"))?;

        backup_config(&path);

        let user = params["root_user"].as_str().unwrap_or("minioadmin").trim();
        let pass = params["root_password"].as_str().unwrap_or("minioadmin");
        let api_port = params["api_port"].as_u64().unwrap_or(9000) as u16;
        let console_port = params["console_port"].as_u64().unwrap_or(9001) as u16;

        if user.is_empty() {
            anyhow::bail!("MinIO Root 用户名不能为空");
        }
        if pass.len() < 8 {
            anyhow::bail!("MinIO Root 密码至少 8 位，否则服务无法启动");
        }

        if let Some(parent) = Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = render_minio_env(user, pass, api_port, console_port);
        std::fs::write(&path, &content)?;
        let _ = self.db.upsert_config_file("minio.env", "minio.env", &path);

        let _ = self.db.update_service_port("minio", api_port);

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "MinIO 配置已保存" }))
    }

    async fn handle_minio_buckets_list(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let mut ctx = self.resolve_minio_context()?;
        apply_minio_credential_overrides(&mut ctx, params);
        let running = self.process_manager.is_service_running("minio").await
            || crate::managers::ProcessManager::is_port_listening(ctx.api_port);
        if !running {
            anyhow::bail!("MinIO 未运行，请先启动服务后再管理桶权限");
        }
        let mc = ctx.mc_exe.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "MinIO 客户端初始化失败，请确认网络可用后点击「刷新列表」（将自动内置 mc）。"
            )
        })?;

        let host_env = minio_mc_host_env(&ctx);
        let output = run_mc(mc, &host_env, &["ls", "--json", "winserver"])?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            let msg = if !err.trim().is_empty() {
                err.to_string()
            } else {
                out.to_string()
            };
            anyhow::bail!(
                "列出桶失败：{}（当前 Root 用户：{}）",
                summarize_mc_error(&msg),
                ctx.root_user
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut buckets = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if v["status"].as_str() == Some("error") {
                let msg = v["error"]["message"]
                    .as_str()
                    .or_else(|| v["error"].as_str())
                    .unwrap_or("未知错误");
                anyhow::bail!(
                    "列出桶失败：{}。请确认 Root 账号密码与正在运行的 MinIO 一致。",
                    msg
                );
            }
            let key = v["key"].as_str().unwrap_or("").trim_end_matches('/');
            if key.is_empty() {
                continue;
            }
            let policy = fetch_bucket_anonymous_policy(mc, &host_env, key).unwrap_or_else(|_| "unknown".into());
            let policy_label = minio_policy_label(&policy);
            buckets.push(json!({
                "name": key,
                "policy": policy,
                "policy_label": policy_label,
                "url": format!("http://127.0.0.1:{}/{}", ctx.api_port, key),
            }));
        }

        Ok(json!({
            "buckets": buckets,
            "count": buckets.len(),
            "api_url": format!("http://127.0.0.1:{}", ctx.api_port),
            "running": true,
            "mc_path": mc.display().to_string(),
        }))
    }

    async fn handle_minio_bucket_set_policy(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let bucket = params["bucket"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("缺少桶名称 bucket"))?;
        validate_minio_bucket_name(bucket)?;

        let policy_raw = params["policy"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("缺少权限 policy（public / download / private）"))?;
        let (mc_policy, policy_id, policy_label) = normalize_minio_policy(policy_raw)?;

        let mut ctx = self.resolve_minio_context()?;
        apply_minio_credential_overrides(&mut ctx, params);
        let running = self.process_manager.is_service_running("minio").await
            || crate::managers::ProcessManager::is_port_listening(ctx.api_port);
        if !running {
            anyhow::bail!("MinIO 未运行，请先启动服务后再修改桶权限");
        }
        let mc = ctx.mc_exe.as_ref().ok_or_else(|| {
            anyhow::anyhow!("MinIO 客户端未就绪，请点击「刷新列表」以自动内置 mc 后再试")
        })?;

        let host_env = minio_mc_host_env(&ctx);
        let target = format!("winserver/{}", bucket);
        let output = run_mc(mc, &host_env, &["anonymous", "set", mc_policy, &target])?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            let out = String::from_utf8_lossy(&output.stdout);
            let msg = if !err.trim().is_empty() {
                err.to_string()
            } else {
                out.to_string()
            };
            anyhow::bail!("设置桶权限失败：{}", summarize_mc_error(&msg));
        }

        let applied = fetch_bucket_anonymous_policy(mc, &host_env, bucket)
            .unwrap_or_else(|_| policy_id.to_string());

        Ok(json!({
            "bucket": bucket,
            "policy": applied,
            "policy_label": minio_policy_label(&applied),
            "message": format!("桶 {} 已设为「{}」", bucket, policy_label),
        }))
    }

    /// Resolve MinIO install path, credentials, ports and mc.exe location.
    fn resolve_minio_context(&self) -> anyhow::Result<MinioContext> {
        let env_path = self
            .resolve_config_path("minio.env")
            .ok_or_else(|| anyhow::anyhow!("MinIO 配置未注册，请先安装 MinIO"))?;
        let raw = std::fs::read_to_string(&env_path).unwrap_or_default();
        let parsed = parse_minio_env(&raw, &env_path);
        let root_user = parsed["root_user"]
            .as_str()
            .unwrap_or("minioadmin")
            .to_string();
        let root_password = parsed["root_password"]
            .as_str()
            .unwrap_or("minioadmin")
            .to_string();
        let api_port = parsed["api_port"].as_u64().unwrap_or(9000) as u16;
        let console_port = parsed["console_port"].as_u64().unwrap_or(9001) as u16;

        let (install_dir, data_dir) = if let Ok(cfg) = self.db.get_service_config("minio") {
            let install = if let Some(cwd) = cfg.cwd.filter(|s| !s.is_empty()) {
                PathBuf::from(heal_deps_path(&cwd).unwrap_or(cwd))
            } else {
                Path::new(&cfg.exe)
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
            };
            let data = install.join("data");
            (install, data)
        } else {
            let parent = Path::new(&env_path)
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            (parent.clone(), parent.join("data"))
        };

        // Always ensure mc.exe is present next to MinIO (copy from bundle or download).
        // Bucket policy UI depends on the client; never ask the user to install it manually.
        let mc_exe = match self
            .runtime_manager
            .ensure_minio_client(&install_dir, Some(self.runtime_dir.as_path()))
        {
            Ok(path) => Some(path),
            Err(error) => {
                warn!(
                    "ensure_minio_client failed for {}: {}",
                    install_dir.display(),
                    error
                );
                resolve_mc_exe(&install_dir, self.db.as_ref(), Some(self.runtime_dir.as_path()))
            }
        };

        let _ = env_path;
        Ok(MinioContext {
            install_dir,
            data_dir,
            root_user,
            root_password,
            api_port,
            console_port,
            mc_exe,
        })
    }

    // ---- Database handlers ----

    async fn handle_database_create(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        let user = params["user"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing user"))?;
        let pass = params["pass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pass"))?;
        validate_mysql_identifier(db_name, "数据库名")?;
        validate_mysql_identifier(user, "用户名")?;

        let tools = self.resolve_mysql_tools().map_err(|e| {
            anyhow::anyhow!("无法连接 MySQL 客户端：{}。请先在软件页安装并启动 MySQL。", e)
        })?;
        // Ensure MySQL service is running before creating databases.
        if !self.process_manager.is_service_running("mysql80").await
            && !self.process_manager.is_service_running("mysql57").await
        {
            let start_result = self.process_manager.start_service("mysql80").await;
            if start_result.is_err() {
                let alt = self.process_manager.start_service("mysql57").await;
                if let Err(err) = alt {
                    anyhow::bail!(
                        "MySQL 未运行且自动启动失败：{}。请先在首页启动 MySQL。",
                        err
                    );
                }
            }
        }
        let root_pass = self.get_mysql_root_password()?;

        // Build and execute CREATE DATABASE and CREATE USER commands.
        // Grant for both localhost and 127.0.0.1 so TCP clients work the same as socket-style apps.
        let sql = format!(
            "CREATE DATABASE IF NOT EXISTS {}; \
             CREATE USER IF NOT EXISTS {}@'localhost' IDENTIFIED BY {}; \
             CREATE USER IF NOT EXISTS {}@'127.0.0.1' IDENTIFIED BY {}; \
             GRANT ALL PRIVILEGES ON {}.* TO {}@'localhost'; \
             GRANT ALL PRIVILEGES ON {}.* TO {}@'127.0.0.1'; \
             FLUSH PRIVILEGES;",
            quote_mysql_ident(db_name),
            quote_mysql_string(user),
            quote_mysql_string(pass),
            quote_mysql_string(user),
            quote_mysql_string(pass),
            quote_mysql_ident(db_name),
            quote_mysql_string(user),
            quote_mysql_ident(db_name),
            quote_mysql_string(user),
        );

        let output = self.run_mysql_client(
            &tools,
            &root_pass,
            &["-e", &sql],
            None,
            std::process::Stdio::piped(),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("创建数据库失败：{}", redact_secret(&stderr, &[pass, &root_pass]));
        }

        // Save to local config, scoped to the active MySQL version
        let mysql_service_id = self.get_active_mysql_service_id()?;
        self.db
            .insert_database(db_name, user, pass, &mysql_service_id)?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("数据库 {} 已创建", db_name) }))
    }

    async fn handle_database_delete(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        validate_mysql_identifier(db_name, "数据库名")?;

        // Remove from local config only (don't actually drop the database)
        let mysql_service_id = self.get_active_mysql_service_id().ok();
        self.db
            .delete_database(db_name, mysql_service_id.as_deref())?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("数据库 {} 已删除", db_name) }))
    }

    async fn handle_database_change_password(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        let user = params["user"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing user"))?;
        let new_pass = params["pass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pass"))?;
        validate_mysql_identifier(db_name, "数据库名")?;
        validate_mysql_identifier(user, "用户名")?;

        let tools = self.resolve_mysql_tools()?;
        let root_pass = self.get_mysql_root_password()?;

        // Ensure both host accounts exist (older installs may only have localhost).
        let sql = format!(
            "CREATE USER IF NOT EXISTS {}@'localhost' IDENTIFIED BY {}; \
             CREATE USER IF NOT EXISTS {}@'127.0.0.1' IDENTIFIED BY {}; \
             ALTER USER {}@'localhost' IDENTIFIED BY {}; \
             ALTER USER {}@'127.0.0.1' IDENTIFIED BY {}; \
             FLUSH PRIVILEGES;",
            quote_mysql_string(user),
            quote_mysql_string(new_pass),
            quote_mysql_string(user),
            quote_mysql_string(new_pass),
            quote_mysql_string(user),
            quote_mysql_string(new_pass),
            quote_mysql_string(user),
            quote_mysql_string(new_pass),
        );

        let output = self.run_mysql_client(
            &tools,
            &root_pass,
            &["-e", &sql],
            None,
            std::process::Stdio::piped(),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("修改密码失败：{}", redact_secret(&stderr, &[new_pass, &root_pass]));
        }

        // Update local config (scoped to active MySQL version)
        let mysql_service_id = self.get_active_mysql_service_id().ok();
        self.db
            .update_database_password(db_name, new_pass, mysql_service_id.as_deref())?;

        Ok(json!({ "message": format!("用户 {} 密码已更新", user) }))
    }

    async fn handle_database_root_password(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let current_pass = params["currentPass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing currentPass"))?;
        let new_pass = params["pass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pass"))?;

        let tools = self.resolve_mysql_tools()?;

        // initialize-insecure only creates root@localhost; keep that path reliable
        // and optionally align root@127.0.0.1 for TCP clients.
        let sql = format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED BY {}; \
             CREATE USER IF NOT EXISTS 'root'@'127.0.0.1' IDENTIFIED BY {}; \
             ALTER USER 'root'@'127.0.0.1' IDENTIFIED BY {}; \
             FLUSH PRIVILEGES;",
            quote_mysql_string(new_pass),
            quote_mysql_string(new_pass),
            quote_mysql_string(new_pass),
        );

        let output = self.run_mysql_client(
            &tools,
            current_pass,
            &["-e", &sql],
            None,
            std::process::Stdio::piped(),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("修改 Root 密码失败：{}", redact_secret(&stderr, &[current_pass, new_pass]));
        }

        // Update settings with new root password
        self.db.update_settings(&json!({ "mysql_root_password": new_pass }))?;

        Ok(json!({ "message": "Root 密码已更新" }))
    }

    async fn handle_database_export(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        validate_mysql_identifier(db_name, "数据库名")?;

        let tools = self.resolve_mysql_tools()?;
        let root_pass = self.get_mysql_root_password()?;

        // Prefer caller-selected path; fall back to data/backups.
        let backup_file = if let Some(path) = params["path"].as_str().filter(|p| !p.trim().is_empty()) {
            let path = PathBuf::from(path);
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            path
        } else {
            let settings = self.db.get_settings()?;
            let data_dir = if settings.data_dir.is_empty() {
                "data".to_string()
            } else {
                settings.data_dir
            };
            let backup_dir = Path::new(&data_dir).join("backups");
            std::fs::create_dir_all(&backup_dir)?;
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            backup_dir.join(format!("{}_{}.sql", db_name, timestamp))
        };

        let output = self.run_mysqldump(
            &tools,
            &root_pass,
            &["--databases", db_name],
            std::process::Stdio::from(std::fs::File::create(&backup_file)?),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Remove failed backup file
            let _ = std::fs::remove_file(&backup_file);
            anyhow::bail!("导出数据库失败：{}", redact_secret(&stderr, &[&root_pass]));
        }

        Ok(json!({
            "message": format!("数据库 {} 已导出", db_name),
            "path": backup_file.to_string_lossy()
        }))
    }

    async fn handle_database_import(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        let sql_path = params["path"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        validate_mysql_identifier(db_name, "数据库名")?;
        let sql_path = Path::new(sql_path);
        if !sql_path.is_file() {
            anyhow::bail!("SQL 文件不存在：{}", sql_path.display());
        }

        let tools = self.resolve_mysql_tools()?;
        let root_pass = self.get_mysql_root_password()?;

        // Read SQL file and pipe to mysql
        let sql_file = std::fs::File::open(sql_path)?;

        let output = self.run_mysql_client(
            &tools,
            &root_pass,
            &[db_name],
            Some(std::process::Stdio::from(sql_file)),
            std::process::Stdio::piped(),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("导入数据库失败：{}", redact_secret(&stderr, &[&root_pass]));
        }

        Ok(json!({ "message": format!("数据库 {} 已导入", db_name) }))
    }

    async fn handle_database_backups(&self) -> anyhow::Result<serde_json::Value> {
        let backup_dir = self.database_backup_dir()?;

        let mut backups = Vec::new();
        if backup_dir.exists() {
            for entry in std::fs::read_dir(&backup_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "sql").unwrap_or(false) {
                    let metadata = entry.metadata()?;
                    let size = metadata.len();
                    let modified = metadata.modified()?
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    backups.push(json!({
                        "name": path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                        "path": path.to_string_lossy(),
                        "size": size,
                        "modified": modified
                    }));
                }
            }
        }

        Ok(json!({ "backups": backups }))
    }

    async fn handle_database_delete_backup(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let backup_path = params["path"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

        let backup_file = std::path::PathBuf::from(backup_path);
        let backup_dir = self.database_backup_dir()?;
        ensure_child_file(&backup_dir, &backup_file, "备份文件")?;
        std::fs::remove_file(backup_file)?;

        Ok(json!({ "message": "备份已删除" }))
    }

    // ---- Software handlers ----

    async fn handle_software_install(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let software_id = params["softwareId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing softwareId"))?;

        // One-click install: prefer bundled package, then download URL / built-in catalog.
        if self
            .runtime_manager
            .find_bundled_file(software_id, &self.runtime_dir)
            .is_some()
        {
            info!("handle_software_install: using bundled package for {}", software_id);
            return self.handle_software_install_bundled(params).await;
        }

        let sw = self.db.get_software(software_id)?;
        let mut url = sw.download_url.unwrap_or_default();
        if url.trim().is_empty() {
            if let Some(catalog) = crate::managers::runtime_manager::catalog_download_url(software_id)
            {
                url = catalog.to_string();
                let _ = self.db.conn(|conn| {
                    conn.execute(
                        "UPDATE software SET download_url = ?1, updated_at = ?2 WHERE id = ?3",
                        rusqlite::params![url, chrono::Utc::now().to_rfc3339(), software_id],
                    )?;
                    Ok(())
                });
            }
        }
        if url.trim().is_empty() {
            anyhow::bail!(
                "暂无 {} 的一键安装源。可在软件页使用「导入」选择本机目录。",
                software_id
            );
        }

        self.handle_software_download_install(params).await
    }

    async fn handle_software_download_install(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let software_id = params["softwareId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing softwareId"))?
            .to_string();

        info!("handle_software_download_install: start software_id={}", software_id);

        // Check if already downloading
        {
            let progress_map = self.download_progress.read().await;
            if let Some(p) = progress_map.get(&software_id) {
                let (_, _, phase) = p.get_progress();
                if phase != "done" && phase != "failed" && phase != "idle" {
                    anyhow::bail!("Software {} is already downloading", software_id);
                }
            }
        }

        // Create progress tracker
        let progress = Arc::new(DownloadProgress::new());
        {
            let mut progress_map = self.download_progress.write().await;
            progress_map.insert(software_id.clone(), progress.clone());
        }

        let runtime_manager = self.runtime_manager.clone();
        let software_id_clone = software_id.clone();

        // Run download in spawn_blocking to not block the async runtime
        info!("handle_software_download_install: spawning download task for {}", software_id);
        let result = tokio::task::spawn_blocking(move || {
            runtime_manager.download_install(&software_id_clone, &progress)
        }).await?;

        // Clean up progress tracker
        {
            let mut progress_map = self.download_progress.write().await;
            progress_map.remove(&software_id);
        }

        match result {
            Ok(runtime) => {
                info!("handle_software_download_install: SUCCESS for {}, building state", software_id);
                let state = self.build_app_state().await?;
                // Verify the service is marked installed
                if let Some(svc) = state.services.iter().find(|s| s.id == software_id) {
                    info!("handle_software_download_install: service {} installed={}", software_id, svc.installed);
                }
                Ok(json!({ "runtime": runtime, "state": state, "message": format!("{} download install complete", software_id) }))
            }
            Err(e) => {
                error!("handle_software_download_install: FAILED for {}: {}", software_id, e);
                Err(e)
            }
        }
    }

    async fn handle_software_download_progress(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let software_id = params["softwareId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing softwareId"))?;

        let progress_map = self.download_progress.read().await;
        if let Some(p) = progress_map.get(software_id) {
            let (total, downloaded, phase) = p.get_progress();
            let percent = if total > 0 { (downloaded as f64 / total as f64 * 100.0) as u32 } else { 0 };
            Ok(json!({
                "softwareId": software_id,
                "total": total,
                "downloaded": downloaded,
                "percent": percent,
                "phase": phase,
            }))
        } else {
            Ok(json!({
                "softwareId": software_id,
                "total": 0,
                "downloaded": 0,
                "percent": 0,
                "phase": "idle",
            }))
        }
    }

    async fn handle_software_install_bundled(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let software_id = params["softwareId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing softwareId"))?
            .to_string();

        info!("handle_software_install_bundled: start software_id={}", software_id);

        // Use the runtime_dir resolved during app initialization
        let runtime_dir = self.runtime_dir.clone();

        let runtime_manager = self.runtime_manager.clone();
        let software_id_clone = software_id.clone();

        let result = tokio::task::spawn_blocking(move || {
            runtime_manager.install_bundled_runtime(&software_id_clone, &runtime_dir)
        }).await?;

        match result {
            Ok(runtime) => {
                let state = self.build_app_state().await?;
                Ok(json!({ "runtime": runtime, "state": state, "message": format!("{} bundled install complete", software_id) }))
            }
            Err(e) => {
                error!("handle_software_install_bundled: FAILED for {}: {}", software_id, e);
                Err(e)
            }
        }
    }

    async fn handle_software_detect_local(&self) -> anyhow::Result<serde_json::Value> {
        let runtime_manager = self.runtime_manager.clone();
        let found = tokio::task::spawn_blocking(move || {
            runtime_manager.detect_local_services()
        }).await?;

        for sw in self.db.list_software()? {
            if let Some(path) = found.get(&sw.service_id) {
                if let Err(e) = self.runtime_manager.register_detected_service(&sw.id, &sw.service_id, path) {
                    warn!("software.detectLocal: failed to register {} from {}: {}", sw.id, path, e);
                }
            }
        }

        let local_map: serde_json::Value = found.into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect();
        let state = self.build_app_state().await?;

        Ok(json!({ "localServices": local_map, "state": state }))
    }

    async fn handle_software_uninstall(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let software_id = params["softwareId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing softwareId"))?;

        if self.process_manager.is_service_running(software_id).await {
            anyhow::bail!("运行环境正在运行,请先停止服务：{}", software_id);
        }

        let sites = self.db.list_sites()?;
        if software_id == "nginx" && !sites.is_empty() {
            anyhow::bail!("Nginx 仍被站点使用,不能删除运行环境");
        }
        if sites
            .iter()
            .any(|site| site.php_runtime_id.as_deref() == Some(software_id))
        {
            anyhow::bail!("PHP 运行环境仍被站点使用,不能删除：{}", software_id);
        }

        self.db.update_service_installed(software_id, false)?;
        self.db.update_runtimes_installed_by_service(software_id, false)?;
        self.db.update_software_installed(software_id, false).ok();
        self.db.add_log(
            "runtime.delete",
            software_id,
            true,
            "运行环境已从管理器移除,未删除本机目录",
        )?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("运行环境 {} 已删除", software_id) }))
    }

    // ---- Service handlers ----

    async fn handle_service_toggle_auto(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let service_id = params["serviceId"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing serviceId"))?;
        let auto = params["auto"].as_bool()
            .ok_or_else(|| anyhow::anyhow!("Missing auto"))?;

        if auto && service_id.starts_with("mysql") {
            for service in self.db.list_service_instances()? {
                if service.id != service_id && service.id.starts_with("mysql") && service.auto {
                    self.db.update_service_auto(&service.id, false)?;
                }
            }
        }
        self.db.update_service_auto(service_id, auto)?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    // ---- Helper methods ----

    /// Resolve the active MySQL service: prefer the one that is running, then
    /// any installed instance (8.0 or 5.7 — only one is expected to run).
    fn get_mysql_service_config(&self) -> anyhow::Result<crate::database::ServiceConfig> {
        let candidates = ["mysql80", "mysql57", "mysql"];
        let mut first_installed: Option<crate::database::ServiceConfig> = None;
        for id in candidates {
            if let Ok(config) = self.db.get_service_config(id) {
                if !config.installed {
                    continue;
                }
                if config.pid.is_some() {
                    return Ok(config);
                }
                if first_installed.is_none() {
                    first_installed = Some(config);
                }
            } else if let Ok(config) = self.db.get_service_config_by_type(id) {
                if !config.installed {
                    continue;
                }
                if config.pid.is_some() {
                    return Ok(config);
                }
                if first_installed.is_none() {
                    first_installed = Some(config);
                }
            }
        }
        first_installed.ok_or_else(|| anyhow::anyhow!("未找到 MySQL 服务，请先在软件管理中安装 MySQL"))
    }

    /// Active MySQL service id used to scope database records (mysql57 / mysql80).
    fn get_active_mysql_service_id(&self) -> anyhow::Result<String> {
        let config = self.get_mysql_service_config()?;
        if !config.id.is_empty() {
            return Ok(config.id);
        }
        // Prefer running instance, then first installed.
        for id in ["mysql80", "mysql57", "mysql"] {
            if let Ok(c) = self.db.get_service_config(id) {
                if c.installed && c.pid.is_some() {
                    return Ok(id.to_string());
                }
            }
        }
        for id in ["mysql80", "mysql57", "mysql"] {
            if let Ok(c) = self.db.get_service_config(id) {
                if c.installed {
                    return Ok(id.to_string());
                }
            }
        }
        Ok("mysql80".to_string())
    }

    /// Locate mysql.exe / mysqldump.exe next to the managed mysqld.exe.
    ///
    /// Service config stores the *server* binary (`mysqld.exe`). Client tools
    /// live in the same `bin/` directory and must be used for SQL operations.
    fn resolve_mysql_tools(&self) -> anyhow::Result<MysqlTools> {
        let config = self.get_mysql_service_config()?;
        if !config.installed {
            anyhow::bail!("MySQL 尚未安装或未导入");
        }
        resolve_mysql_tools_from_config(&config.exe, config.cwd.as_deref(), config.port)
    }

    fn get_mysql_root_password(&self) -> anyhow::Result<String> {
        Ok(self.db.get_mysql_root_password()?.unwrap_or_default())
    }

    /// Run a non-interactive `mysql` client command (no console window, no password prompt).
    fn run_mysql_client(
        &self,
        tools: &MysqlTools,
        root_pass: &str,
        extra_args: &[&str],
        stdin: Option<std::process::Stdio>,
        stdout: std::process::Stdio,
    ) -> anyhow::Result<std::process::Output> {
        let port = tools.port.to_string();
        // Always use --password=… (including empty). Bare `-p` forces an interactive prompt
        // and pops a console window — the exact UX break reported for "创建数据库".
        let password_arg = mysql_password_arg(root_pass);
        let mut cmd = crate::process_util::silent_command_path(&tools.mysql_exe);
        cmd.args([
            "-h",
            "127.0.0.1",
            "-P",
            &port,
            "-uroot",
            &password_arg,
            "--connect-timeout=8",
            "--protocol=TCP",
        ])
        .args(extra_args)
        .stdin(stdin.unwrap_or_else(std::process::Stdio::null))
        .stdout(stdout)
        .stderr(std::process::Stdio::piped());

        cmd.output().map_err(|e| {
            anyhow::anyhow!(
                "无法启动 mysql 客户端（{}）：{}",
                tools.mysql_exe.display(),
                e
            )
        })
    }

    /// Run mysqldump without a console window or interactive password prompt.
    fn run_mysqldump(
        &self,
        tools: &MysqlTools,
        root_pass: &str,
        extra_args: &[&str],
        stdout: std::process::Stdio,
    ) -> anyhow::Result<std::process::Output> {
        if !tools.mysqldump_exe.is_file() {
            anyhow::bail!(
                "找不到 mysqldump.exe：{}",
                tools.mysqldump_exe.display()
            );
        }
        let port = tools.port.to_string();
        let password_arg = mysql_password_arg(root_pass);
        let mut cmd = crate::process_util::silent_command_path(&tools.mysqldump_exe);
        cmd.args([
            "-h",
            "127.0.0.1",
            "-P",
            &port,
            "-uroot",
            &password_arg,
            "--protocol=TCP",
        ])
        .args(extra_args)
        .stdin(std::process::Stdio::null())
        .stdout(stdout)
        .stderr(std::process::Stdio::piped());

        cmd.output().map_err(|e| {
            anyhow::anyhow!(
                "无法启动 mysqldump（{}）：{}",
                tools.mysqldump_exe.display(),
                e
            )
        })
    }

    fn database_backup_dir(&self) -> anyhow::Result<std::path::PathBuf> {
        let settings = self.db.get_settings()?;
        let data_dir = if settings.data_dir.is_empty() {
            std::path::PathBuf::from("data")
        } else {
            std::path::PathBuf::from(settings.data_dir)
        };
        Ok(data_dir.join("backups"))
    }

    fn error_response(&self, id: &str, method: &str, error: &anyhow::Error) -> JsonRpcResponse {
        let message = error.to_string();
        let (code, error_code) = classify_error(&message);
        JsonRpcResponse::error_with_data(
            id,
            code,
            &message,
            json!({
                "errorCode": error_code,
                "method": method,
                "details": message,
            }),
        )
    }

    fn record_request_audit(
        &self,
        method: &str,
        params: &serde_json::Value,
        result: &anyhow::Result<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let log_success = matches!(
            method,
            "port.killProcess"
                | "settings.update"
                | "settings.paths"
                | "hosts.sync"
                | "hosts.remove"
                | "config.save"
                | "redis.config.save"
                | "minio.config.save"
                | "minio.bucket.setPolicy"
                | "database.create"
                | "database.delete"
                | "database.changePassword"
                | "database.rootPassword"
                | "database.export"
                | "database.import"
                | "database.deleteBackup"
                | "database.pgCreate"
                | "service.toggleAuto"
                | "software.detectLocal"
        );
        let log_failure = log_success
            || matches!(
                method,
                "runtime.import"
                    | "software.install"
                    | "software.uninstall"
                    | "software.downloadInstall"
                    | "software.installBundled"
            );

        if (result.is_ok() && !log_success) || (result.is_err() && !log_failure) {
            return Ok(());
        }

        let (target_type, target_id) = operation_target(method, params);
        let details = json!({ "request": sanitize_audit_value(params) });
        match result {
            Ok(value) => {
                let message = operation_success_message(method, params, value);
                self.db.add_operation_log(
                    method,
                    Some(&target_type),
                    target_id.as_deref(),
                    true,
                    None,
                    &message,
                    Some(&details),
                )
            }
            Err(error) => {
                let message = redact_audit_message(&error.to_string(), params);
                let (_, error_code) = classify_error(&message);
                self.db.add_operation_log(
                    method,
                    Some(&target_type),
                    target_id.as_deref(),
                    false,
                    Some(error_code),
                    &message,
                    Some(&details),
                )
            }
        }
    }

    /// Resolve a config file path, healing empty DB entries from installed services.
    fn resolve_config_path(&self, file_id: &str) -> Option<String> {
        if let Ok(Some(path)) = self.db.get_config_file_path(file_id) {
            if !path.trim().is_empty() && Path::new(&path).exists() {
                return Some(path);
            }
            // Path recorded but missing (e.g. moved out of deps/) — try heal below
            if !path.trim().is_empty() {
                if let Some(healed) = heal_deps_path(&path) {
                    if Path::new(&healed).exists() {
                        let _ = self.db.upsert_config_file(file_id, file_id, &healed);
                        return Some(healed);
                    }
                }
            }
        }

        // Derive from installed service instance paths
        let derived = match file_id {
            "nginx.conf" => self
                .db
                .get_service_config("nginx")
                .ok()
                .and_then(|c| {
                    c.config_file
                        .filter(|p| !p.is_empty())
                        .or_else(|| c.cwd.map(|d| format!("{}/conf/nginx.conf", d.replace('\\', "/"))))
                }),
            "mysql.ini" | "my.ini" => self
                .db
                .get_service_config("mysql80")
                .or_else(|_| self.db.get_service_config("mysql57"))
                .ok()
                .and_then(|c| {
                    c.config_file.filter(|p| !p.is_empty()).or_else(|| {
                        c.cwd
                            .map(|d| format!("{}/my.ini", d.replace('\\', "/")))
                    })
                }),
            "redis.conf" => self.db.get_service_config("redis").ok().and_then(|c| {
                c.config_file
                    .filter(|p| !p.is_empty())
                    .or_else(|| c.cwd.map(|d| format!("{}/redis.conf", d.replace('\\', "/"))))
            }),
            "minio.env" => self.db.get_service_config("minio").ok().and_then(|c| {
                c.config_file
                    .filter(|p| !p.is_empty())
                    .or_else(|| c.cwd.map(|d| format!("{}/minio.env", d.replace('\\', "/"))))
            }),
            "php.ini" => self
                .db
                .list_service_instances()
                .ok()
                .and_then(|list| {
                    list.into_iter()
                        .find(|s| {
                            s.installed
                                && (s.service_type == "php"
                                    || s.service_type.starts_with("php")
                                    || s.id.starts_with("php"))
                        })
                        .and_then(|s| {
                            self.db.get_service_config(&s.id).ok().and_then(|c| {
                                c.config_file.filter(|p| !p.is_empty()).or_else(|| {
                                    c.cwd.map(|d| format!("{}/php.ini", d.replace('\\', "/")))
                                })
                            })
                        })
                }),
            "hosts" => Some("C:/Windows/System32/drivers/etc/hosts".to_string()),
            _ => None,
        };

        if let Some(path) = derived {
            let path = heal_deps_path(&path).unwrap_or(path);
            if Path::new(&path).exists() || file_id == "hosts" {
                let _ = self.db.upsert_config_file(file_id, file_id, &path);
                return Some(path);
            }
            // Still return so UI can show the expected path
            let _ = self.db.upsert_config_file(file_id, file_id, &path);
            return Some(path);
        }
        None
    }

    async fn build_app_state(&self) -> anyhow::Result<AppState> {
        // Heal empty config file paths from installed services (best-effort).
        for id in [
            "nginx.conf",
            "mysql.ini",
            "my.ini",
            "redis.conf",
            "minio.env",
            "php.ini",
        ] {
            let _ = self.resolve_config_path(id);
        }

        let mut services = self.db.list_service_instances()?;
        for svc in &mut services {
            if self.process_manager.is_service_running(&svc.id).await {
                svc.state = ServiceState::Running;
            } else if svc.state == ServiceState::Running {
                svc.state = ServiceState::Stopped;
            }
        }

        let sites = self.db.list_sites()?;
        // Only show databases belonging to the currently active MySQL version.
        let active_mysql = self.get_active_mysql_service_id().ok();
        let databases = self.db.list_databases(active_mysql.as_deref())?;
        let software = self.db.list_software()?;
        let config_files = self.db.list_config_files()?;
        let logs = self.db.list_logs(50)?;
        let settings = self.db.get_settings()?;

        let state = AppState {
            services,
            sites,
            databases,
            software,
            config_files,
            logs,
            system_settings: settings,
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        // Update cache
        {
            let mut cache = self.state_cache.write().await;
            *cache = Some(CachedState { state: state.clone(), at: Instant::now() });
        }

        Ok(state)
    }

    // ---- New handler methods ----

    async fn handle_get_state_fast(&self) -> anyhow::Result<serde_json::Value> {
        let cache = self.state_cache.read().await;
        if let Some(cached) = cache.as_ref() {
            if cached.at.elapsed().as_secs() < 2 {
                return Ok(serde_json::to_value(&cached.state)?);
            }
        }
        drop(cache);
        let state = self.build_app_state().await?;
        Ok(serde_json::to_value(state)?)
    }

    async fn handle_site_update(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 siteId"))?;
        let domain = params["domain"].as_str().unwrap_or("");
        let port = params["port"].as_u64().and_then(|v| if v > 0 && v <= 65535 { Some(v as u16) } else { None }).ok_or_else(|| anyhow::anyhow!("缺少有效的 port"))?;
        let path = params["path"].as_str().unwrap_or("");
        let server = params["server"].as_str().unwrap_or("nginx");
        self.site_manager.update_site(site_id, domain, port, path, server).await?;
        let state = self.build_app_state().await?;
        Ok(json!({ "ok": true, "state": state }))
    }

    async fn handle_site_config(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let site_id = params["siteId"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 siteId"))?;
        let vhost_path = self.site_manager.get_site_vhost_path(site_id)?;
        let content = if vhost_path.exists() {
            std::fs::read_to_string(&vhost_path)?
        } else {
            String::new()
        };
        Ok(json!({
            "id": format!("site:{}", site_id),
            "label": vhost_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "path": vhost_path.to_string_lossy().to_string(),
            "exists": vhost_path.exists(),
            "content": content
        }))
    }

    async fn handle_settings_paths(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // Update each provided path in the settings table
        if let Some(obj) = params.as_object() {
            for (key, value) in obj {
                if let Some(path_str) = value.as_str() {
                    let path = std::path::Path::new(path_str);
                    if !path.is_absolute() {
                        anyhow::bail!("{} 必须是绝对路径", key);
                    }
                    self.db.upsert_setting(key, path_str)?;
                    // Also update the corresponding service instance if this is a service path
                    if key.ends_with("Root") {
                        let service_id = key.trim_end_matches("Root");
                        self.db.update_service_path(service_id, path_str)?;
                    }
                }
            }
        }
        let state = self.build_app_state().await?;
        Ok(json!({ "ok": true, "state": state }))
    }

    async fn handle_database_pg_list(&self, _params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let service = self.db.get_service_config("pgsql")?;
        if !service.installed {
            anyhow::bail!("PostgreSQL 未安装");
        }
        let psql_exe = service.exe.clone();
        let port = service.port;
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&psql_exe)
                .args(["-h", "127.0.0.1", "-p", &port.to_string(), "-U", "postgres", "-t", "-A", "-c", "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname"])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        }).await??;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let dbs: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        Ok(json!({ "databases": dbs }))
    }

    async fn handle_database_pg_create(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 db"))?;
        let user = params["user"].as_str().unwrap_or(db_name);
        let pass = params["pass"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 pass"))?;
        let service = self.db.get_service_config("pgsql")?;
        if !service.installed {
            anyhow::bail!("PostgreSQL 未安装");
        }
        let psql_exe = service.exe.clone();
        let port = service.port;
        let sql = format!("CREATE USER \"{}\" WITH PASSWORD '{}'; CREATE DATABASE \"{}\" OWNER \"{}\";", user, pass.replace('\'', "''"), db_name, user);
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&psql_exe)
                .args(["-h", "127.0.0.1", "-p", &port.to_string(), "-U", "postgres", "-c", &sql])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()
        }).await??;
        if !output.status.success() {
            anyhow::bail!("PostgreSQL 创建数据库失败：{}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(json!({ "ok": true, "db": db_name }))
    }

}

fn operation_target(method: &str, params: &serde_json::Value) -> (String, Option<String>) {
    let field = |name: &str| {
        params
            .get(name)
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
    };

    if let Some(id) = field("serviceId") {
        return ("service".to_string(), Some(id));
    }
    if let Some(id) = field("siteId") {
        return ("site".to_string(), Some(id));
    }
    if let Some(id) = field("softwareId") {
        return ("runtime".to_string(), Some(id));
    }
    if let Some(id) = field("fileId") {
        return ("config".to_string(), Some(id));
    }
    if let Some(id) = field("db") {
        return ("database".to_string(), Some(id));
    }
    if let Some(id) = field("bucket") {
        return ("bucket".to_string(), Some(id));
    }
    if let Some(id) = field("domain") {
        return ("hosts".to_string(), Some(id));
    }
    if let Some(path) = field("path") {
        let name = Path::new(&path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or(path);
        return ("file".to_string(), Some(name));
    }
    if let Some(pid) = params.get("pid").and_then(|value| value.as_u64()) {
        return ("process".to_string(), Some(pid.to_string()));
    }

    let target_type = method.split('.').next().unwrap_or("system");
    (target_type.to_string(), Some("system".to_string()))
}

fn operation_success_message(
    method: &str,
    params: &serde_json::Value,
    result: &serde_json::Value,
) -> String {
    if method == "service.toggleAuto" {
        return if params.get("auto").and_then(|value| value.as_bool()).unwrap_or(false) {
            "已加入一键启动".to_string()
        } else {
            "已从一键启动移除".to_string()
        };
    }
    result
        .get("message")
        .and_then(|value| value.as_str())
        .unwrap_or("操作已完成")
        .to_string()
}

fn sanitize_audit_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(values) => {
            let sanitized = values
                .iter()
                .map(|(key, value)| {
                    let normalized = key.to_lowercase().replace(['_', '-'], "");
                    let value = if is_sensitive_audit_key(&normalized) {
                        json!("[REDACTED]")
                    } else if normalized == "content" {
                        json!(format!(
                            "[{} characters]",
                            value.as_str().map(|text| text.chars().count()).unwrap_or(0)
                        ))
                    } else {
                        sanitize_audit_value(value)
                    };
                    (key.clone(), value)
                })
                .collect();
            serde_json::Value::Object(sanitized)
        }
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values.iter().take(20).map(sanitize_audit_value).collect(),
        ),
        serde_json::Value::String(value) => {
            let truncated = value.chars().take(500).collect::<String>();
            serde_json::Value::String(truncated)
        }
        _ => value.clone(),
    }
}

fn is_sensitive_audit_key(normalized: &str) -> bool {
    normalized.ends_with("pass")
        || normalized == "key"
        || normalized == "authorization"
        || normalized == "cookie"
        || normalized == "session"
        || normalized.contains("password")
        || normalized.contains("secret")
        || normalized.contains("token")
        || normalized.contains("credential")
        || normalized.ends_with("apikey")
        || normalized.ends_with("accesskey")
        || normalized.ends_with("privatekey")
}

fn collect_audit_secrets(value: &serde_json::Value, secrets: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(values) => {
            for (key, value) in values {
                let normalized = key.to_lowercase().replace(['_', '-'], "");
                if is_sensitive_audit_key(&normalized) || normalized == "content" {
                    if let Some(secret) = value.as_str().filter(|secret| !secret.is_empty()) {
                        secrets.push(secret.to_string());
                    }
                } else {
                    collect_audit_secrets(value, secrets);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_audit_secrets(value, secrets);
            }
        }
        _ => {}
    }
}

fn redact_audit_message(message: &str, params: &serde_json::Value) -> String {
    let mut secrets = Vec::new();
    collect_audit_secrets(params, &mut secrets);
    let secret_refs = secrets.iter().map(String::as_str).collect::<Vec<_>>();
    redact_secret(message, &secret_refs)
}

fn classify_error(message: &str) -> (i32, &'static str) {
    let lower = message.to_lowercase();
    if lower.contains("hosts") && (message.contains("权限") || lower.contains("permission")) {
        return (error_codes::HOSTS_PERMISSION_DENIED, "HOSTS_PERMISSION_DENIED");
    }
    if lower.contains("port") || message.contains("端口") || message.contains("占用") {
        return (error_codes::PORT_OCCUPIED, "PORT_OCCUPIED");
    }
    if lower.contains("config") || message.contains("配置") {
        return (error_codes::CONFIG_INVALID, "CONFIG_INVALID");
    }
    if lower.contains("not found") || message.contains("不存在") || message.contains("缺少") {
        return (error_codes::BINARY_NOT_FOUND, "NOT_FOUND");
    }
    if message.contains("健康") || lower.contains("health") {
        return (error_codes::HEALTH_CHECK_FAILED, "HEALTH_CHECK_FAILED");
    }
    if message.contains("已退出") || lower.contains("process") {
        return (error_codes::PROCESS_EXITED, "PROCESS_EXITED");
    }
    (-32000, "INTERNAL_ERROR")
}

fn ensure_child_file(
    parent_dir: &std::path::Path,
    candidate_file: &std::path::Path,
    label: &str,
) -> anyhow::Result<()> {
    let parent = parent_dir
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("备份目录不存在：{}", parent_dir.display()))?;
    let candidate = candidate_file
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("{}不存在：{}", label, candidate_file.display()))?;

    if !candidate.starts_with(&parent) {
        anyhow::bail!("{}不在备份目录内：{}", label, candidate.display());
    }
    if !candidate.is_file() {
        anyhow::bail!("{}不是普通文件：{}", label, candidate.display());
    }
    Ok(())
}

/// Build non-interactive MySQL auth args. Always use `--password=` (even when empty)
/// so the client never falls into an interactive password prompt.
fn mysql_password_arg(password: &str) -> String {
    format!("--password={}", password)
}

/// Rewrite accidental `target/.../deps/data` paths to `target/.../data` when that
/// sibling directory exists (unit tests often install under deps/).
fn heal_deps_path(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    if let Some(idx) = normalized.find("/deps/data/") {
        let healed = format!(
            "{}{}",
            &normalized[..idx],
            &normalized[idx + "/deps".len()..]
        );
        let healed_native = healed.replace('/', "\\");
        if Path::new(&healed).exists() || Path::new(&healed_native).exists() {
            return Some(if path.contains('\\') {
                healed_native
            } else {
                healed
            });
        }
    }
    if let Some(idx) = normalized.find("/deps/data") {
        if idx + "/deps/data".len() == normalized.len() {
            let healed = format!("{}/data", &normalized[..idx]);
            let healed_native = healed.replace('/', "\\");
            if Path::new(&healed).exists() || Path::new(&healed_native).exists() {
                return Some(if path.contains('\\') {
                    healed_native
                } else {
                    healed
                });
            }
        }
    }
    None
}

/// Resolve mysql/mysqldump client tools from a service server path.
fn resolve_mysql_tools_from_config(
    server_exe: &str,
    cwd: Option<&str>,
    port: u16,
) -> anyhow::Result<MysqlTools> {
    let server_exe_path = Path::new(server_exe);
    let bin_dir = if server_exe_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.eq_ignore_ascii_case("mysqld.exe") || n.eq_ignore_ascii_case("mysql.exe"))
        .unwrap_or(false)
    {
        server_exe_path
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or_else(|| anyhow::anyhow!("无法解析 MySQL bin 目录：{}", server_exe))?
    } else if let Some(cwd) = cwd {
        Path::new(cwd).join("bin")
    } else {
        anyhow::bail!("无法定位 MySQL bin 目录");
    };

    let mysql_exe = bin_dir.join("mysql.exe");
    let mysqldump_exe = bin_dir.join("mysqldump.exe");
    if !mysql_exe.is_file() {
        anyhow::bail!(
            "找不到 MySQL 客户端 mysql.exe：{}（请确认安装目录完整）",
            mysql_exe.display()
        );
    }

    let port = if port == 0 { 3306 } else { port };
    Ok(MysqlTools {
        mysql_exe,
        mysqldump_exe,
        port,
    })
}

fn validate_mysql_identifier(value: &str, label: &str) -> anyhow::Result<()> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if !valid {
        anyhow::bail!("{}只能包含 1-64 位字母、数字和下划线", label);
    }
    Ok(())
}

fn quote_mysql_ident(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn quote_mysql_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''"))
}

fn redact_secret(message: &str, secrets: &[&str]) -> String {
    let mut redacted = message.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            redacted = redacted.replace(secret, "***");
        }
    }
    redacted
}

/// Create a timestamped backup of a config file before overwriting it.
fn backup_config(path: &str) {
    if !Path::new(path).exists() {
        return;
    }
    let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("{}.bak.{}", path, stamp);
    if let Err(e) = std::fs::copy(path, &backup_path) {
        warn!("Failed to create backup for {}: {}", path, e);
    }
}

/// Parse a redis.conf into structured fields for the visual config form.
fn parse_redis_conf(raw: &str, path: &str) -> serde_json::Value {
    let mut port = 6379u16;
    let mut bind = "127.0.0.1".to_string();
    let mut password = String::new();
    let mut maxmemory = "256mb".to_string();
    let mut maxmemory_policy = "allkeys-lru".to_string();
    let mut appendonly = false;
    let mut protected_mode = true;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("").trim();
        match key {
            "port" => port = val.parse().unwrap_or(port),
            "bind" => bind = val.to_string(),
            "requirepass" => password = val.to_string(),
            "maxmemory" => maxmemory = val.to_string(),
            "maxmemory-policy" => maxmemory_policy = val.to_string(),
            "appendonly" => appendonly = val == "yes",
            "protected-mode" => protected_mode = val == "yes",
            _ => {}
        }
    }

    json!({
        "path": path,
        "port": port,
        "bind": bind,
        "password": password,
        "maxmemory": maxmemory,
        "maxmemory_policy": maxmemory_policy,
        "appendonly": appendonly,
        "protected_mode": protected_mode,
    })
}

/// Render structured Redis fields back to a redis.conf file body.
fn render_redis_conf(
    port: u16,
    bind: &str,
    password: &str,
    maxmemory: &str,
    policy: &str,
    appendonly: bool,
    protected: bool,
) -> String {
    let pass_line = if password.is_empty() {
        "# requirepass disabled".to_string()
    } else {
        format!("requirepass {}", password)
    };
    format!(
        r#"# Redis configuration (managed by WinServer)
# Editable via WinServer visual config or this file directly.

port {}
bind {}
protected-mode {}
{}
maxmemory {}
maxmemory-policy {}
appendonly {}
save ""
rdbchecksum no
"#,
        port,
        bind,
        if protected { "yes" } else { "no" },
        pass_line,
        maxmemory,
        policy,
        if appendonly { "yes" } else { "no" },
    )
}

/// Runtime context for talking to a local MinIO instance via `mc`.
#[derive(Debug, Clone)]
struct MinioContext {
    install_dir: PathBuf,
    data_dir: PathBuf,
    root_user: String,
    root_password: String,
    api_port: u16,
    console_port: u16,
    mc_exe: Option<PathBuf>,
}

/// Allow UI form values to override saved env for live bucket ops (before save).
fn apply_minio_credential_overrides(ctx: &mut MinioContext, params: &serde_json::Value) {
    if let Some(user) = params["root_user"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        ctx.root_user = user.to_string();
    }
    if let Some(pass) = params["root_password"].as_str() {
        // empty string is invalid for MinIO root; only apply non-empty
        if !pass.is_empty() {
            ctx.root_password = pass.to_string();
        }
    }
    if let Some(port) = params["api_port"].as_u64().or_else(|| {
        params["api_port"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
    }) {
        if port > 0 && port <= 65535 {
            ctx.api_port = port as u16;
        }
    }
}

/// Parse a minio.env into structured fields for the visual config form.
fn parse_minio_env(raw: &str, path: &str) -> serde_json::Value {
    let mut user = "minioadmin".to_string();
    let mut password = "minioadmin".to_string();
    let mut api_port = 9000u16;
    let mut console_port = 9001u16;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = trimmed.split_once('=') {
            let val = val.trim().trim_matches('"');
            match key.trim() {
                "MINIO_ROOT_USER" => user = val.to_string(),
                "MINIO_ROOT_PASSWORD" => password = val.to_string(),
                "MINIO_API_PORT" => api_port = val.parse().unwrap_or(api_port),
                "MINIO_CONSOLE_PORT" => console_port = val.parse().unwrap_or(console_port),
                _ => {}
            }
        }
    }

    json!({
        "path": path,
        "root_user": user,
        "root_password": password,
        "api_port": api_port,
        "console_port": console_port,
    })
}

/// Render structured MinIO fields back to a minio.env file body.
fn render_minio_env(user: &str, password: &str, api_port: u16, console_port: u16) -> String {
    format!(
        r#"# MinIO configuration (managed by WinServer)
# Editable via WinServer visual config or this file directly.

MINIO_ROOT_USER={}
MINIO_ROOT_PASSWORD={}
MINIO_API_PORT={}
MINIO_CONSOLE_PORT={}
"#,
        user, password, api_port, console_port
    )
}

/// Locate `mc.exe` next to MinIO, from software registry, runtime bundle, or fallbacks.
fn resolve_mc_exe(
    install_dir: &Path,
    db: &crate::database::Database,
    runtime_dir: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates = vec![
        install_dir.join("mc.exe"),
        install_dir.join("bin").join("mc.exe"),
    ];
    if let Some(rd) = runtime_dir {
        candidates.push(rd.join("minio").join("mc.exe"));
        candidates.push(rd.join("mc.exe"));
    }
    candidates.push(PathBuf::from("runtime/minio/mc.exe"));

    for c in &candidates {
        if c.is_file() {
            if let Ok(meta) = std::fs::metadata(c) {
                if meta.len() > 0 {
                    return Some(c.clone());
                }
            }
        }
    }

    if let Ok(sw) = db.get_software("mc") {
        if let Some(path) = sw.install_path.filter(|s| !s.is_empty()) {
            let p = PathBuf::from(heal_deps_path(&path).unwrap_or(path));
            if p.is_file() {
                return Some(p);
            }
            let as_dir = p.join("mc.exe");
            if as_dir.is_file() {
                return Some(as_dir);
            }
        }
    }

    None
}

fn minio_mc_host_env(ctx: &MinioContext) -> String {
    // MC_HOST_<alias>=http://user:pass@host:port — avoids writing alias to disk.
    format!(
        "http://{}:{}@127.0.0.1:{}",
        urlencoding_user(&ctx.root_user),
        urlencoding_user(&ctx.root_password),
        ctx.api_port
    )
}

/// Minimal URL-encoding for user/password segments in MC_HOST URLs.
fn urlencoding_user(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn run_mc(mc: &Path, host_env: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let mut cmd = crate::process_util::silent_command_path(mc);
    cmd.env("MC_HOST_winserver", host_env)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd.output()
        .map_err(|e| anyhow::anyhow!("无法启动 mc（{}）：{}", mc.display(), e))
}

fn fetch_bucket_anonymous_policy(
    mc: &Path,
    host_env: &str,
    bucket: &str,
) -> anyhow::Result<String> {
    let target = format!("winserver/{}", bucket);
    let output = run_mc(mc, host_env, &["anonymous", "get", "--json", &target])?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if !stdout.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(p) = v["permission"].as_str() {
                return Ok(normalize_policy_id(p));
            }
        }
    }
    if !output.status.success() {
        anyhow::bail!("查询桶权限失败");
    }
    Ok("private".into())
}

fn normalize_policy_id(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "public" | "readwrite" | "read-write" | "rw" | "public-read-write" => "public".into(),
        "download" | "readonly" | "read-only" | "read" | "public-read" => "download".into(),
        "upload" | "write" | "writeonly" => "upload".into(),
        "none" | "private" | "deny" => "private".into(),
        "" => "private".into(),
        other => other.to_string(),
    }
}

fn normalize_minio_policy(raw: &str) -> anyhow::Result<(&'static str, &'static str, &'static str)> {
    // returns (mc_set_arg, policy_id, label)
    match normalize_policy_id(raw).as_str() {
        "public" => Ok(("public", "public", "公共读写")),
        "download" => Ok(("download", "download", "公共读 · 私有写")),
        "private" => Ok(("none", "private", "全私有")),
        other => anyhow::bail!(
            "不支持的权限「{}」，请使用 public（公共读写）、download（公共读私有写）或 private（全私有）",
            other
        ),
    }
}

fn minio_policy_label(policy: &str) -> &'static str {
    match normalize_policy_id(policy).as_str() {
        "public" => "公共读写",
        "download" => "公共读 · 私有写",
        "upload" => "公共写 · 私有读",
        "private" => "全私有",
        _ => "自定义",
    }
}

fn validate_minio_bucket_name(name: &str) -> anyhow::Result<()> {
    if name.is_empty() || name.len() > 63 {
        anyhow::bail!("桶名称长度无效");
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("桶名称不合法");
    }
    let ok = name
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_'));
    if !ok {
        anyhow::bail!("桶名称包含非法字符");
    }
    Ok(())
}

fn summarize_mc_error(msg: &str) -> String {
    let lower = msg.to_lowercase();
    if lower.contains("invalidaccesskeyid") || lower.contains("access key") {
        return "Access Key 无效：请在上方把 Root 用户/密码改成当前 MinIO 实际账号，然后点「刷新列表」（无需先去别处安装客户端）".into();
    }
    if lower.contains("signaturedoesnotmatch") || lower.contains("password") {
        return "密码不匹配：请修正上方 Root 密码后点「刷新列表」".into();
    }
    if lower.contains("connection refused") || lower.contains("connectex") {
        return "无法连接 MinIO API，请确认服务已启动".into();
    }
    // Prefer first JSON error message if present
    if let Some(start) = msg.find('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg[start..]) {
            if let Some(m) = v
                .pointer("/error/cause/message")
                .or_else(|| v.pointer("/error/message"))
                .or_else(|| v.get("error"))
                .and_then(|x| x.as_str())
            {
                return m.to_string();
            }
        }
    }
    let one_line = msg.lines().next().unwrap_or(msg).trim();
    if one_line.len() > 240 {
        format!("{}…", &one_line[..240])
    } else {
        one_line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_error, ensure_child_file, minio_policy_label, mysql_password_arg,
        normalize_minio_policy, normalize_policy_id, parse_minio_env, parse_redis_conf,
        prefer_bundled_runtime, quote_mysql_ident, quote_mysql_string, redact_secret,
        redact_audit_message, render_minio_env, render_redis_conf,
        resolve_mysql_tools_from_config, sanitize_audit_value, should_skip_local_service_registration,
        validate_minio_bucket_name,
        validate_mysql_identifier,
    };
    use std::collections::HashSet;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn core_services_prefer_packaged_bundles_over_external_dirs() {
        assert!(prefer_bundled_runtime("mysql80", "mysql80"));
        assert!(prefer_bundled_runtime("redis", "redis"));
        assert!(prefer_bundled_runtime("minio", "minio"));
        assert!(!prefer_bundled_runtime("mysql57", "mysql57"));
        assert!(!prefer_bundled_runtime("nginx", "nginx"));
    }

    #[test]
    fn auxiliary_software_does_not_override_preferred_bundled_service() {
        let preferred = HashSet::from(["minio".to_string()]);

        assert!(should_skip_local_service_registration("mc", "minio", &preferred));
        assert!(!should_skip_local_service_registration("minio", "minio", &preferred));
        assert!(!should_skip_local_service_registration("redis", "redis", &preferred));
    }

    #[test]
    fn mysql_identifier_validation_blocks_sql_fragments() {
        assert!(validate_mysql_identifier("app_db_01", "数据库名").is_ok());
        assert!(validate_mysql_identifier("bad-name", "数据库名").is_err());
        assert!(validate_mysql_identifier("db`;DROP", "数据库名").is_err());
        assert_eq!(quote_mysql_ident("app_db"), "`app_db`");
        assert_eq!(quote_mysql_string("pa's\\word"), "'pa''s\\\\word'");
    }

    #[test]
    fn mysql_password_arg_never_uses_bare_p_flag() {
        // Bare `-p` would force an interactive password prompt and a console window.
        assert_eq!(mysql_password_arg(""), "--password=");
        assert_eq!(mysql_password_arg("secret"), "--password=secret");
        assert!(!mysql_password_arg("").starts_with("-p") || mysql_password_arg("").contains('='));
    }

    #[test]
    fn resolve_mysql_tools_uses_client_next_to_mysqld() {
        let root = std::env::temp_dir().join(format!("winserver-mysql-tools-{}", Uuid::new_v4()));
        let bin = root.join("bin");
        fs::create_dir_all(&bin).expect("mkdir bin");
        let mysqld = bin.join("mysqld.exe");
        let mysql = bin.join("mysql.exe");
        let dump = bin.join("mysqldump.exe");
        fs::write(&mysqld, b"").expect("touch mysqld");
        fs::write(&mysql, b"").expect("touch mysql");
        fs::write(&dump, b"").expect("touch dump");

        let tools = resolve_mysql_tools_from_config(
            &mysqld.to_string_lossy(),
            Some(&root.to_string_lossy()),
            3306,
        )
        .expect("resolve tools");
        assert_eq!(tools.mysql_exe, mysql);
        assert_eq!(tools.mysqldump_exe, dump);
        assert_eq!(tools.port, 3306);

        // Missing client must fail with a clear message (not fall back to mysqld.exe).
        let _ = fs::remove_file(&mysql);
        let err = resolve_mysql_tools_from_config(&mysqld.to_string_lossy(), None, 0)
            .expect_err("mysql.exe required");
        assert!(err.to_string().contains("mysql.exe"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn error_helpers_classify_and_redact_sensitive_text() {
        let (_, code) = classify_error("80 端口已被占用");
        assert_eq!(code, "PORT_OCCUPIED");
        let message = redact_secret("using password abc123 failed", &["abc123"]);
        assert_eq!(message, "using password *** failed");
    }

    #[test]
    fn audit_details_redact_secrets_and_summarize_config_content() {
        let sanitized = sanitize_audit_value(&serde_json::json!({
            "db": "app_db",
            "pass": "database-secret",
            "currentPass": "current-secret",
            "root_password": "root-secret",
            "api_key": "api-secret",
            "authorization": "Bearer session-secret",
            "content": "port 6379\nrequirepass secret",
        }));

        assert_eq!(sanitized["db"], "app_db");
        assert_eq!(sanitized["pass"], "[REDACTED]");
        assert_eq!(sanitized["currentPass"], "[REDACTED]");
        assert_eq!(sanitized["root_password"], "[REDACTED]");
        assert_eq!(sanitized["api_key"], "[REDACTED]");
        assert_eq!(sanitized["authorization"], "[REDACTED]");
        assert_eq!(sanitized["content"], "[28 characters]");
        let raw = sanitized.to_string();
        assert!(!raw.contains("database-secret"));
        assert!(!raw.contains("current-secret"));
        assert!(!raw.contains("root-secret"));
        assert!(!raw.contains("session-secret"));
        assert!(!raw.contains("requirepass"));
    }

    #[test]
    fn audit_error_messages_redact_secrets_from_request() {
        let params = serde_json::json!({
            "password": "database-secret",
            "nested": { "access_key": "access-secret" },
        });
        let message = redact_audit_message(
            "database-secret failed while using access-secret",
            &params,
        );

        assert_eq!(message, "*** failed while using ***");
    }

    #[test]
    fn ensure_child_file_rejects_path_escape() {
        let base = std::env::temp_dir().join(format!("winserver-backups-{}", Uuid::new_v4()));
        let backups = base.join("backups");
        let outside = base.join("outside.sql");
        let inside = backups.join("inside.sql");
        fs::create_dir_all(&backups).expect("mkdir backups");
        fs::write(&inside, "-- ok").expect("inside");
        fs::write(&outside, "-- no").expect("outside");

        assert!(ensure_child_file(&backups, &inside, "备份文件").is_ok());
        assert!(ensure_child_file(&backups, &outside, "备份文件").is_err());
    }

    #[test]
    fn redis_visual_config_parse_and_render_roundtrip() {
        let raw = "port 6380\nbind 127.0.0.1\nrequirepass secret\nappendonly yes\nprotected-mode no\nmaxmemory 512mb\nmaxmemory-policy noeviction\n";
        let parsed = parse_redis_conf(raw, "redis.conf");

        assert_eq!(parsed["port"], 6380);
        assert_eq!(parsed["password"], "secret");
        assert_eq!(parsed["appendonly"], true);
        assert_eq!(parsed["protected_mode"], false);

        let rendered = render_redis_conf(6381, "0.0.0.0", "", "256mb", "allkeys-lru", false, true);
        assert!(rendered.contains("port 6381"));
        assert!(rendered.contains("bind 0.0.0.0"));
        assert!(rendered.contains("# requirepass disabled"));
        assert!(rendered.contains("protected-mode yes"));
    }

    #[test]
    fn minio_visual_config_parse_and_render_roundtrip() {
        let raw = "MINIO_ROOT_USER=\"root\"\nMINIO_ROOT_PASSWORD=\"pass\"\nMINIO_API_PORT=9010\nMINIO_CONSOLE_PORT=9011\n";
        let parsed = parse_minio_env(raw, "minio.env");

        assert_eq!(parsed["root_user"], "root");
        assert_eq!(parsed["root_password"], "pass");
        assert_eq!(parsed["api_port"], 9010);
        assert_eq!(parsed["console_port"], 9011);

        let rendered = render_minio_env("admin", "secret", 9000, 9001);
        assert!(rendered.contains("MINIO_ROOT_USER=admin"));
        assert!(rendered.contains("MINIO_ROOT_PASSWORD=secret"));
        assert!(rendered.contains("MINIO_API_PORT=9000"));
        assert!(rendered.contains("MINIO_CONSOLE_PORT=9001"));
    }

    #[test]
    fn minio_policy_normalization_maps_to_mc_commands() {
        assert_eq!(normalize_policy_id("public"), "public");
        assert_eq!(normalize_policy_id("download"), "download");
        assert_eq!(normalize_policy_id("none"), "private");
        assert_eq!(normalize_policy_id("private"), "private");

        let (mc, id, label) = normalize_minio_policy("public").unwrap();
        assert_eq!(mc, "public");
        assert_eq!(id, "public");
        assert_eq!(label, "公共读写");

        let (mc, id, label) = normalize_minio_policy("download").unwrap();
        assert_eq!(mc, "download");
        assert_eq!(id, "download");
        assert_eq!(label, "公共读 · 私有写");

        let (mc, id, label) = normalize_minio_policy("private").unwrap();
        assert_eq!(mc, "none");
        assert_eq!(id, "private");
        assert_eq!(label, "全私有");

        assert_eq!(minio_policy_label("download"), "公共读 · 私有写");
        assert!(validate_minio_bucket_name("my-bucket_01").is_ok());
        assert!(validate_minio_bucket_name("../etc").is_err());
        assert!(normalize_minio_policy("invalid-policy").is_err());
    }
}
