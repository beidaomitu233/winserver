use std::collections::HashMap;
use std::sync::Arc;
use std::path::Path;
use std::io::{Read, Seek, SeekFrom};
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

        // 3. Auto-install bundled runtimes for services that have no local version and are not yet installed
        for sw in &software_list {
            if sw.installed {
                continue;
            }
            if local_services.contains_key(&sw.service_id) {
                info!("run_auto_setup: {} has local installation at {}, skipping bundled install", sw.id, local_services[&sw.service_id]);
                continue;
            }

            if self.runtime_manager.find_bundled_file(&sw.id, runtime_dir).is_none() {
                continue;
            }

            info!("run_auto_setup: auto-installing bundled runtime for {}", sw.id);
            if let Err(e) = self.runtime_manager.install_bundled_runtime(&sw.id, runtime_dir) {
                warn!("run_auto_setup: failed to install bundled {}: {}", sw.id, e);
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
            "database.create" => self.handle_database_create(&params).await,
            "database.delete" => self.handle_database_delete(&params).await,
            "database.changePassword" => self.handle_database_change_password(&params).await,
            "database.rootPassword" => self.handle_database_root_password(&params).await,
            "database.export" => self.handle_database_export(&params).await,
            "database.import" => self.handle_database_import(&params).await,
            "database.sync" => self.handle_database_sync().await,
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
            "files.list" => self.handle_files_list(&params).await,
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

    async fn handle_resource_get(&self) -> anyhow::Result<serde_json::Value> {
        let resource = self.process_manager.get_system_resource()?;
        Ok(serde_json::to_value(resource)?)
    }

    async fn handle_log_list(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let source = params["source"].as_str().unwrap_or("operation");
        let search = params["search"].as_str().unwrap_or("").trim();

        if source == "operation" {
            let mut logs = self.db.list_logs(200)?;
            if !search.is_empty() {
                logs.retain(|line| line.to_lowercase().contains(&search.to_lowercase()));
            }
            return Ok(json!({ "source": source, "logs": logs }));
        }

        let path = self.resolve_log_path(source)?;
        let logs = read_log_tail(&path, search, 256 * 1024)?;
        Ok(json!({
            "source": source,
            "path": path.to_string_lossy(),
            "logs": logs
        }))
    }

    async fn handle_log_clear(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let source = params["source"].as_str().unwrap_or("operation");
        if source == "operation" {
            self.db.clear_logs()?;
            return Ok(json!({ "message": "日志已清空" }));
        }

        let path = self.resolve_log_path(source)?;
        if !path.exists() {
            anyhow::bail!("日志文件不存在：{}", path.display());
        }
        std::fs::write(&path, "")?;
        Ok(json!({ "message": "日志已清空", "path": path.to_string_lossy() }))
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

        let runtime_type = match runtime_type_text.to_lowercase().as_str() {
            "nginx" => RuntimeType::Nginx,
            "php" => RuntimeType::Php,
            _ => anyhow::bail!("第一阶段仅支持导入 Nginx 和 PHP"),
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

        let path = self.db.get_config_file_path(file_id)?
            .ok_or_else(|| anyhow::anyhow!("Config file not found: {}", file_id))?;

        let exists = Path::new(&path).exists();
        let content = if exists {
            std::fs::read_to_string(&path)?
        } else {
            String::new()
        };

        Ok(json!({
            "id": file_id,
            "label": file_id, // Could look up label from DB if needed
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

        let path = self.db.get_config_file_path(file_id)?
            .ok_or_else(|| anyhow::anyhow!("Config file not found: {}", file_id))?;

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

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "配置已保存" }))
    }

    // ---- Structured Redis/MinIO config (visual form mode) ----

    async fn handle_redis_config_get(&self, _params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self.db.get_config_file_path("redis.conf")?
            .ok_or_else(|| anyhow::anyhow!("Redis 配置未注册"))?;
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        Ok(parse_redis_conf(&raw, &path))
    }

    async fn handle_redis_config_save(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self.db.get_config_file_path("redis.conf")?
            .ok_or_else(|| anyhow::anyhow!("Redis 配置未注册"))?;

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
        let _ = self.db.add_log("config.save", "redis", true, "redis.conf 已通过可视化配置更新");

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "Redis 配置已保存" }))
    }

    async fn handle_minio_config_get(&self, _params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self.db.get_config_file_path("minio.env")?
            .ok_or_else(|| anyhow::anyhow!("MinIO 配置未注册"))?;
        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        Ok(parse_minio_env(&raw, &path))
    }

    async fn handle_minio_config_save(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let path = self.db.get_config_file_path("minio.env")?
            .ok_or_else(|| anyhow::anyhow!("MinIO 配置未注册"))?;

        backup_config(&path);

        let user = params["root_user"].as_str().unwrap_or("minioadmin");
        let pass = params["root_password"].as_str().unwrap_or("minioadmin");
        let api_port = params["api_port"].as_u64().unwrap_or(9000) as u16;
        let console_port = params["console_port"].as_u64().unwrap_or(9001) as u16;

        let content = render_minio_env(user, pass, api_port, console_port);
        std::fs::write(&path, &content)?;

        let _ = self.db.update_service_port("minio", api_port);
        let _ = self.db.add_log("config.save", "minio", true, "minio.env 已通过可视化配置更新");

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "MinIO 配置已保存" }))
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

        // Get MySQL client path from service config
        let mysql_config = self.get_mysql_client_config()?;
        let mysql_exe = &mysql_config.exe;

        // Get root password from settings
        let root_pass = self.get_mysql_root_password()?;

        // Build and execute CREATE DATABASE and CREATE USER commands
        let sql = format!(
            "CREATE DATABASE IF NOT EXISTS {}; CREATE USER IF NOT EXISTS {}@'localhost' IDENTIFIED BY {}; GRANT ALL PRIVILEGES ON {}.* TO {}@'localhost'; FLUSH PRIVILEGES;",
            quote_mysql_ident(db_name),
            quote_mysql_string(user),
            quote_mysql_string(pass),
            quote_mysql_ident(db_name),
            quote_mysql_string(user)
        );

        let output = std::process::Command::new(mysql_exe)
            .args(["-uroot", &format!("-p{}", root_pass), "-e", &sql])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("创建数据库失败：{}", redact_secret(&stderr, &[pass, &root_pass]));
        }

        // Save to local config
        self.db.insert_database(db_name, user, pass)?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": format!("数据库 {} 已创建", db_name) }))
    }

    async fn handle_database_delete(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let db_name = params["db"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing db"))?;
        validate_mysql_identifier(db_name, "数据库名")?;

        // Remove from local config only (don't actually drop the database)
        self.db.delete_database(db_name)?;

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

        let mysql_config = self.get_mysql_client_config()?;
        let mysql_exe = &mysql_config.exe;
        let root_pass = self.get_mysql_root_password()?;

        let sql = format!(
            "ALTER USER {}@'localhost' IDENTIFIED BY {}; FLUSH PRIVILEGES;",
            quote_mysql_string(user),
            quote_mysql_string(new_pass)
        );

        let output = std::process::Command::new(mysql_exe)
            .args(["-uroot", &format!("-p{}", root_pass), "-e", &sql])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("修改密码失败：{}", redact_secret(&stderr, &[new_pass, &root_pass]));
        }

        // Update local config
        self.db.update_database_password(db_name, new_pass)?;

        Ok(json!({ "message": format!("用户 {} 密码已更新", user) }))
    }

    async fn handle_database_root_password(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let current_pass = params["currentPass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing currentPass"))?;
        let new_pass = params["pass"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pass"))?;

        let mysql_config = self.get_mysql_client_config()?;
        let mysql_exe = &mysql_config.exe;

        let sql = format!(
            "ALTER USER 'root'@'localhost' IDENTIFIED BY {}; FLUSH PRIVILEGES;",
            quote_mysql_string(new_pass)
        );

        let output = std::process::Command::new(mysql_exe)
            .args(["-uroot", &format!("-p{}", current_pass), "-e", &sql])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

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

        let mysql_config = self.get_mysql_client_config()?;
        // Use mysqldump from the same directory as mysql
        let mysql_exe_path = Path::new(&mysql_config.exe);
        let mysqldump_exe = mysql_exe_path.parent()
            .map(|p| p.join("mysqldump.exe"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "mysqldump.exe".to_string());

        let root_pass = self.get_mysql_root_password()?;

        // Create backups directory
        let settings = self.db.get_settings()?;
        let data_dir = if settings.data_dir.is_empty() {
            "data".to_string()
        } else {
            settings.data_dir
        };
        let backup_dir = Path::new(&data_dir).join("backups");
        std::fs::create_dir_all(&backup_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let backup_file = backup_dir.join(format!("{}_{}.sql", db_name, timestamp));

        let output = std::process::Command::new(&mysqldump_exe)
            .args(["-uroot", &format!("-p{}", root_pass), "--databases", db_name])
            .stdout(std::process::Stdio::from(std::fs::File::create(&backup_file)?))
            .stderr(std::process::Stdio::piped())
            .output()?;

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

        let mysql_config = self.get_mysql_client_config()?;
        let mysql_exe = &mysql_config.exe;
        let root_pass = self.get_mysql_root_password()?;

        // Read SQL file and pipe to mysql
        let sql_file = std::fs::File::open(sql_path)?;

        let output = std::process::Command::new(mysql_exe)
            .args(["-uroot", &format!("-p{}", root_pass), db_name])
            .stdin(std::process::Stdio::from(sql_file))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("导入数据库失败：{}", redact_secret(&stderr, &[&root_pass]));
        }

        Ok(json!({ "message": format!("数据库 {} 已导入", db_name) }))
    }

    async fn handle_database_sync(&self) -> anyhow::Result<serde_json::Value> {
        let mysql_config = self.get_mysql_client_config()?;
        let mysql_exe = &mysql_config.exe;
        let root_pass = self.get_mysql_root_password()?;

        // Run SHOW DATABASES
        let output = std::process::Command::new(mysql_exe)
            .args(["-uroot", &format!("-p{}", root_pass), "-e", "SHOW DATABASES"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("同步数据库失败：{}", redact_secret(&stderr, &[&root_pass]));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let system_dbs = ["Database", "information_schema", "mysql", "performance_schema", "sys"];

        for line in stdout.lines().skip(1) {
            let db_name = line.trim();
            if db_name.is_empty() || system_dbs.contains(&db_name) {
                continue;
            }
            // Add to local config if not exists
            if self.db.get_database_password(db_name)?.is_none() {
                self.db.insert_database(db_name, db_name, "")?;
            }
        }

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state, "message": "数据库已同步" }))
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

        // Check if a download_url exists; if so, delegate to download_install
        let sw = self.db.get_software(software_id)?;
        if sw.download_url.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Software {} has no download URL, please use local import", software_id);
        }

        // Delegate to download_install
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

        let local_map: serde_json::Value = found.into_iter()
            .map(|(k, v)| (k, json!(v)))
            .collect();

        Ok(json!({ "localServices": local_map }))
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

        self.db.update_service_auto(service_id, auto)?;

        let state = self.build_app_state().await?;
        Ok(json!({ "state": state }))
    }

    // ---- Helper methods ----

    fn get_mysql_client_config(&self) -> anyhow::Result<crate::database::ServiceConfig> {
        // Try mysql80 first, then mysql57
        if let Ok(config) = self.db.get_service_config_by_type("mysql80") {
            return Ok(config);
        }
        if let Ok(config) = self.db.get_service_config_by_type("mysql57") {
            return Ok(config);
        }
        if let Ok(config) = self.db.get_service_config_by_type("mysql") {
            return Ok(config);
        }
        anyhow::bail!("No MySQL service found in database")
    }

    fn get_mysql_root_password(&self) -> anyhow::Result<String> {
        Ok(self.db.get_mysql_root_password()?.unwrap_or_default())
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

    fn resolve_log_path(&self, source: &str) -> anyhow::Result<std::path::PathBuf> {
        match source {
            "nginx_error" => {
                let config = self.db.get_service_config("nginx")?;
                Ok(std::path::Path::new(config.cwd.as_deref().unwrap_or(".")).join("logs").join("error.log"))
            }
            "nginx_access" => {
                let config = self.db.get_service_config("nginx")?;
                Ok(std::path::Path::new(config.cwd.as_deref().unwrap_or(".")).join("logs").join("access.log"))
            }
            "php_error" => {
                let php = self
                    .db
                    .list_service_instances()?
                    .into_iter()
                    .find(|service| service.service_type == "php" && service.installed)
                    .ok_or_else(|| anyhow::anyhow!("没有已导入的 PHP 运行环境"))?;
                let config = self.db.get_service_config(&php.id)?;
                let cwd = std::path::Path::new(config.cwd.as_deref().unwrap_or("."));
                let candidates = [
                    cwd.join("logs").join("php_error.log"),
                    cwd.join("logs").join("php_errors.log"),
                    cwd.join("php_error.log"),
                    cwd.join("php_errors.log"),
                ];
                Ok(candidates
                    .iter()
                    .find(|path| path.exists())
                    .cloned()
                    .unwrap_or_else(|| candidates[0].clone()))
            }
            "agent" => {
                let settings = self.db.get_settings()?;
                let data_dir = if settings.data_dir.is_empty() {
                    std::path::PathBuf::from("data")
                } else {
                    std::path::PathBuf::from(settings.data_dir)
                };
                Ok(data_dir.join("logs").join("agent.log"))
            }
            _ => anyhow::bail!("未知日志来源：{}", source),
        }
    }

    async fn build_app_state(&self) -> anyhow::Result<AppState> {
        let mut services = self.db.list_service_instances()?;
        for svc in &mut services {
            if self.process_manager.is_service_running(&svc.id).await {
                svc.state = ServiceState::Running;
            } else if svc.state == ServiceState::Running {
                svc.state = ServiceState::Stopped;
            }
        }

        let sites = self.db.list_sites()?;
        let databases = self.db.list_databases()?;
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

    async fn handle_files_list(&self, params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let dir_path = params["path"].as_str().ok_or_else(|| anyhow::anyhow!("缺少 path"))?;
        let dir = std::path::Path::new(dir_path);
        if !dir.exists() || !dir.is_dir() {
            anyhow::bail!("目录不存在：{}", dir_path);
        }
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "isDir": metadata.is_dir(),
                "size": metadata.len(),
                "modifiedAt": metadata.modified().ok().map(|t| format!("{:?}", t)).unwrap_or_default()
            }));
        }
        Ok(json!({ "entries": entries }))
    }
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

fn read_log_tail(path: &Path, search: &str, max_bytes: u64) -> anyhow::Result<Vec<String>> {
    if !path.exists() {
        return Ok(vec![format!("日志文件不存在：{}", path.display())]);
    }

    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;
    if start > 0 {
        if let Some(index) = content.find('\n') {
            content = content[index + 1..].to_string();
        }
    }

    let search_lower = search.to_lowercase();
    let mut lines = content
        .lines()
        .filter(|line| search_lower.is_empty() || line.to_lowercase().contains(&search_lower))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    if lines.len() > 500 {
        lines = lines.split_off(lines.len() - 500);
    }

    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::{
        classify_error, ensure_child_file, quote_mysql_ident, quote_mysql_string, read_log_tail,
        redact_secret, validate_mysql_identifier,
    };
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn read_log_tail_filters_and_limits_large_files() {
        let path = std::env::temp_dir().join(format!("winserver-log-{}.log", Uuid::new_v4()));
        let mut content = String::new();
        for index in 0..1200 {
            let kind = if index % 100 == 0 { "ERROR" } else { "INFO" };
            content.push_str(&format!("{} line {}\n", kind, index));
        }
        fs::write(&path, content).expect("write log");

        let filtered = read_log_tail(&path, "ERROR", 128 * 1024).expect("read log");
        assert!(filtered.iter().all(|line| line.contains("ERROR")));
        assert!(filtered.len() <= 500);

        let missing = read_log_tail(&path.with_extension("missing"), "", 128).expect("missing log");
        assert!(missing[0].contains("日志文件不存在"));
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
    fn error_helpers_classify_and_redact_sensitive_text() {
        let (_, code) = classify_error("80 端口已被占用");
        assert_eq!(code, "PORT_OCCUPIED");
        let message = redact_secret("using password abc123 failed", &["abc123"]);
        assert_eq!(message, "using password *** failed");
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
}
