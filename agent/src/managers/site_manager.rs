use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use handlebars::Handlebars;
use tracing::info;
use uuid::Uuid;

use shared::types::{ServerType, SiteInfo};

use crate::database::Database;
use super::hosts_manager::HostsManager;
use super::process_manager::ProcessManager;

/// Manages website creation, deletion, and configuration.
pub struct SiteManager {
    data_dir: PathBuf,
    _process_manager: Arc<ProcessManager>,
    hosts_manager: Arc<HostsManager>,
    db: Arc<Database>,
    handlebars: Arc<Handlebars<'static>>,
}

/// Nginx vhost template for site configuration.
const NGINX_VHOST_TEMPLATE: &str = r#"server {
    listen       {{port}};
    server_name  {{domain}};
    root   "{{document_root}}";

    location / {
        index index.php index.html;
        autoindex off;
    }

    location ~ \.php(.*)$ {
        fastcgi_pass   127.0.0.1:{{php_cgi_port}};
        fastcgi_index  index.php;
        fastcgi_split_path_info  ^((?U).+\.php)(/?.+)$;
        fastcgi_param  SCRIPT_FILENAME  $document_root$fastcgi_script_name;
        fastcgi_param  PATH_INFO        $fastcgi_path_info;
        fastcgi_param  PATH_TRANSLATED  $document_root$fastcgi_path_info;
        include        fastcgi_params;
    }
}
"#;

impl SiteManager {
    pub fn new(
        data_dir: PathBuf,
        process_manager: Arc<ProcessManager>,
        hosts_manager: Arc<HostsManager>,
        db: Arc<Database>,
    ) -> Result<Self> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("nginx_vhost", NGINX_VHOST_TEMPLATE)
            .context("Failed to register nginx vhost template")?;

        Ok(Self {
            data_dir,
            _process_manager: process_manager,
            hosts_manager,
            db,
            handlebars: Arc::new(handlebars),
        })
    }

    /// Create a new site with the given parameters.
    pub async fn create_site(
        &self,
        domain: &str,
        port: u16,
        path: &str,
        server_type: &str,
        php_runtime_id: Option<&str>,
    ) -> Result<()> {
        let site_id = Uuid::new_v4().to_string();
        let domain = validate_domain(domain)?;
        if port == 0 {
            anyhow::bail!("HTTP 端口必须在 1-65535 范围内");
        }

        // Parse server type
        let server = match server_type.to_lowercase().as_str() {
            "nginx" => ServerType::Nginx,
            _ => anyhow::bail!("第一阶段仅支持 Nginx 站点"),
        };

        // Determine document root
        let document_root = if path.is_empty() {
            let www_dir = self.data_dir.join("www").join(&domain);
            www_dir.to_string_lossy().to_string()
        } else {
            path.to_string()
        };

        let doc_root_path = Path::new(&document_root);
        if !doc_root_path.exists() {
            anyhow::bail!("网站目录不存在：{}", document_root);
        }
        if !doc_root_path.is_dir() {
            anyhow::bail!("网站目录不是文件夹：{}", document_root);
        }

        if self
            .db
            .list_sites()?
            .iter()
            .any(|site| site.domain.eq_ignore_ascii_case(&domain) && site.port == port)
        {
            anyhow::bail!("站点已存在：{}:{}", domain, port);
        }

        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;
        if !nginx_config.installed {
            anyhow::bail!("请先导入 Nginx 运行环境");
        }

        let php_service_id = self.resolve_php_service(php_runtime_id)?;
        if !self._process_manager.is_service_running(&php_service_id).await {
            self._process_manager
                .start_service(&php_service_id)
                .await
                .with_context(|| format!("PHP FastCGI 启动失败：{}", php_service_id))?;
        }
        let php_config = self.db.get_service_config(&php_service_id)?;

        let rollback = self.write_nginx_vhost(&nginx_config, &domain, port, &document_root, php_config.port)?;
        let mut hosts_synced = false;

        let result = (|| -> Result<()> {
            self.validate_nginx_config(&nginx_config)?;
            self.hosts_manager.sync_hosts(&domain)?;
            hosts_synced = true;
            self.reload_nginx(&nginx_config)?;
            self.health_check_site(&domain, port)?;

            self.db.insert_site(&SiteInfo {
                id: site_id.clone(),
                name: domain.clone(),
                domain: domain.clone(),
                port,
                document_root: document_root.clone(),
                server_type: server,
                php_runtime_id: Some(php_service_id.clone()),
                ssl: false,
                status: "running".to_string(),
            })?;

            Ok(())
        })();

        if let Err(error) = result {
            rollback.restore()?;
            if hosts_synced {
                let _ = self.hosts_manager.remove_hosts(&domain);
            }
            let message = error.to_string();
            let _ = self.db.add_log("site.create", &domain, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log(
            "site.create",
            &domain,
            true,
            &format!("站点已创建，端口 {}，PHP {}", port, php_service_id),
        )?;

        info!(
            "Site created: {} (port={}, server={})",
            domain, port, server_type
        );

        Ok(())
    }

    /// Delete a site by its ID.
    pub async fn delete_site(&self, site_id: &str) -> Result<()> {
        // Get site info from database
        let site = self.db.get_site(site_id)?;
        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;

        let rollbacks = self.remove_vhost_configs(&nginx_config, &site)?;
        let mut hosts_removed = false;

        let result = (|| -> Result<()> {
            self.hosts_manager.remove_hosts(&site.domain)?;
            hosts_removed = true;
            self.validate_nginx_config(&nginx_config)?;
            self.reload_nginx(&nginx_config)?;
            self.db.delete_site(site_id)?;
            Ok(())
        })();

        if let Err(error) = result {
            for rollback in &rollbacks {
                let _ = rollback.restore();
            }
            if hosts_removed {
                let _ = self.hosts_manager.sync_hosts(&site.domain);
            }
            let message = error.to_string();
            let _ = self.db.add_log("site.delete", site_id, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log(
            "site.delete",
            site_id,
            true,
            &format!("站点 {} 已删除，源代码目录保留", site.domain),
        )?;
        info!("Site deleted: {} ({})", site.domain, site_id);
        Ok(())
    }

    /// Enable a site: write vhost config, sync hosts, validate, reload, health check.
    pub async fn enable_site(&self, site_id: &str) -> Result<()> {
        let site = self.db.get_site(site_id)?;
        if site.status == "running" || site.status == "normal" {
            return Ok(());
        }

        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;

        let php_service_id = site.php_runtime_id.clone().unwrap_or_default();
        let php_cgi_port = if !php_service_id.is_empty() {
            if !self._process_manager.is_service_running(&php_service_id).await {
                self._process_manager
                    .start_service(&php_service_id)
                    .await
                    .with_context(|| format!("PHP FastCGI 启动失败：{}", php_service_id))?;
            }
            self.db.get_service_config(&php_service_id).map(|c| c.port).unwrap_or(9073)
        } else {
            9073
        };

        let rollback = self.write_nginx_vhost(&nginx_config, &site.domain, site.port, &site.document_root, php_cgi_port)?;
        let mut hosts_synced = false;

        let result = (|| -> Result<()> {
            self.validate_nginx_config(&nginx_config)?;
            self.hosts_manager.sync_hosts(&site.domain)?;
            hosts_synced = true;
            self.reload_nginx(&nginx_config)?;
            self.health_check_site(&site.domain, site.port)?;
            self.db.update_site_status(site_id, "running")?;
            Ok(())
        })();

        if let Err(error) = result {
            rollback.restore()?;
            if hosts_synced {
                let _ = self.hosts_manager.remove_hosts(&site.domain);
            }
            let message = error.to_string();
            let _ = self.db.add_log("site.enable", site_id, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log("site.enable", site_id, true, &format!("站点 {} 已启用", site.domain))?;
        info!("Site enabled: {} ({})", site.domain, site_id);
        Ok(())
    }

    /// Disable a site: remove vhost config, remove hosts, validate, reload.
    pub async fn disable_site(&self, site_id: &str) -> Result<()> {
        let site = self.db.get_site(site_id)?;
        if site.status == "disabled" || site.status == "stopped" {
            return Ok(());
        }

        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;

        let rollbacks = self.remove_vhost_configs(&nginx_config, &site)?;
        let mut hosts_removed = false;

        let result = (|| -> Result<()> {
            self.hosts_manager.remove_hosts(&site.domain)?;
            hosts_removed = true;
            self.validate_nginx_config(&nginx_config)?;
            self.reload_nginx(&nginx_config)?;
            self.db.update_site_status(site_id, "disabled")?;
            Ok(())
        })();

        if let Err(error) = result {
            for rollback in &rollbacks {
                let _ = rollback.restore();
            }
            if hosts_removed {
                let _ = self.hosts_manager.sync_hosts(&site.domain);
            }
            let message = error.to_string();
            let _ = self.db.add_log("site.disable", site_id, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log("site.disable", site_id, true, &format!("站点 {} 已停用", site.domain))?;
        info!("Site disabled: {} ({})", site.domain, site_id);
        Ok(())
    }

    pub async fn switch_php(&self, site_id: &str, php_runtime_id: &str) -> Result<()> {
        let site = self.db.get_site(site_id)?;
        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;
        if !nginx_config.installed {
            anyhow::bail!("请先导入 Nginx 运行环境");
        }

        let target_php = self.resolve_php_service(Some(php_runtime_id))?;
        if site.php_runtime_id.as_deref() == Some(target_php.as_str()) {
            return Ok(());
        }

        if !self._process_manager.is_service_running(&target_php).await {
            self._process_manager
                .start_service(&target_php)
                .await
                .with_context(|| format!("PHP FastCGI 启动失败：{}", target_php))?;
        }
        let php_config = self.db.get_service_config(&target_php)?;

        let rollback = self.write_nginx_vhost(
            &nginx_config,
            &site.domain,
            site.port,
            &site.document_root,
            php_config.port,
        )?;

        let result = (|| -> Result<()> {
            self.validate_nginx_config(&nginx_config)?;
            self.reload_nginx(&nginx_config)?;
            self.health_check_site(&site.domain, site.port)?;
            self.db
                .update_site_php_runtime(site_id, &target_php, "running")?;
            Ok(())
        })();

        if let Err(error) = result {
            rollback.restore()?;
            let _ = self.validate_nginx_config(&nginx_config);
            let _ = self.reload_nginx(&nginx_config);
            let message = error.to_string();
            let _ = self.db.add_log("site.switch_php", site_id, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log(
            "site.switch_php",
            site_id,
            true,
            &format!("站点 {} 已切换到 {}", site.domain, target_php),
        )?;
        Ok(())
    }

    /// Update an existing site's domain, port, and path.
    pub async fn update_site(
        &self,
        site_id: &str,
        domain: &str,
        port: u16,
        path: &str,
        server_type: &str,
    ) -> Result<()> {
        let site = self.db.get_site(site_id)?;        let nginx_config = self
            .db
            .get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;

        let new_domain = validate_domain(domain)?;
        if port == 0 {
            anyhow::bail!("HTTP 端口必须在 1-65535 范围内");
        }

        let document_root = if path.is_empty() {
            site.document_root.clone()
        } else {
            path.to_string()
        };

        let doc_root_path = Path::new(&document_root);
        if !doc_root_path.exists() || !doc_root_path.is_dir() {
            anyhow::bail!("网站目录不存在或不是文件夹：{}", document_root);
        }

        // Check for domain+port conflict (excluding current site)
        let all_sites = self.db.list_sites()?;
        if all_sites.iter().any(|s| s.id != site_id && s.domain.eq_ignore_ascii_case(&new_domain) && s.port == port) {
            anyhow::bail!("站点已存在：{}:{}", new_domain, port);
        }

        let _server = match server_type.to_lowercase().as_str() {
            "nginx" => ServerType::Nginx,
            _ => anyhow::bail!("第一阶段仅支持 Nginx 站点"),
        };

        // Resolve PHP service for the updated site
        let php_service_id = site.php_runtime_id.clone().unwrap_or_default();
        let php_config = if !php_service_id.is_empty() {
            self.db.get_service_config(&php_service_id).ok()
        } else {
            None
        };
        let php_cgi_port = php_config.map(|c| c.port).unwrap_or(9073);

        // Remove old vhost configs and write new ones
        let old_rollbacks = self.remove_vhost_configs(&nginx_config, &site)?;
        let new_rollback = self.write_nginx_vhost(&nginx_config, &new_domain, port, &document_root, php_cgi_port)?;
        let mut hosts_synced = false;
        let domain_changed = site.domain != new_domain;

        let result = (|| -> Result<()> {
            // If domain changed, update hosts
            if domain_changed {
                self.hosts_manager.remove_hosts(&site.domain)?;
                self.hosts_manager.sync_hosts(&new_domain)?;
                hosts_synced = true;
            }
            self.validate_nginx_config(&nginx_config)?;
            self.reload_nginx(&nginx_config)?;
            self.health_check_site(&new_domain, port)?;

            // Update the database record
            self.db.update_site(site_id, &new_domain, port, &document_root, "nginx", "running")?;
            Ok(())
        })();

        if let Err(error) = result {
            new_rollback.restore()?;
            for rollback in &old_rollbacks {
                let _ = rollback.restore();
            }
            if hosts_synced {
                let _ = self.hosts_manager.remove_hosts(&new_domain);
                let _ = self.hosts_manager.sync_hosts(&site.domain);
            }
            let message = error.to_string();
            let _ = self.db.add_log("site.update", site_id, false, &message);
            anyhow::bail!(message);
        }

        self.db.add_log(
            "site.update",
            site_id,
            true,
            &format!("站点已更新为 {}:{}，目录 {}", new_domain, port, document_root),
        )?;

        info!("Site updated: {} (port={})", new_domain, port);
        Ok(())
    }

    /// Get the vhost config file path for a site.
    pub fn get_site_vhost_path(&self, site_id: &str) -> Result<PathBuf> {
        let site = self.db.get_site(site_id)?;
        let nginx_config = self.db.get_service_config("nginx")
            .context("请先导入 Nginx 运行环境")?;
        let nginx_root = nginx_config.cwd.as_deref().unwrap_or_default();
        let safe_name = site.domain.replace(['.', '*', ':'], "_");
        let config_name = format!("{}_{}.conf", safe_name, site.port);
        Ok(Path::new(nginx_root).join("conf").join("vhosts").join(&config_name))
    }

    // ---- Private helpers ----

    /// Generate and write an Nginx vhost configuration file.
    fn write_nginx_vhost(
        &self,
        nginx_config: &crate::database::ServiceConfig,
        domain: &str,
        port: u16,
        document_root: &str,
        php_cgi_port: u16,
    ) -> Result<FileRollback> {
        let nginx_root = nginx_config.cwd.as_deref().unwrap_or_default();
        if nginx_root.is_empty() {
            anyhow::bail!("Nginx 安装目录未配置");
        }
        let nginx_conf_dir = Path::new(nginx_root).join("conf").join("vhosts");
        fs::create_dir_all(&nginx_conf_dir)
            .with_context(|| format!("无法创建 Nginx vhosts 目录：{}", nginx_conf_dir.display()))?;

        self.write_vhost_to_dir(&nginx_conf_dir, domain, port, document_root, php_cgi_port)
    }

    /// Write vhost config to a specific directory.
    fn write_vhost_to_dir(
        &self,
        vhost_dir: &Path,
        domain: &str,
        port: u16,
        document_root: &str,
        php_cgi_port: u16,
    ) -> Result<FileRollback> {
        let safe_name = domain.replace(['.', '*', ':'], "_");
        let config_name = format!("{}_{}.conf", safe_name, port);
        let config_path = vhost_dir.join(&config_name);

        // Render the template
        let template_data = serde_json::json!({
            "domain": domain,
            "port": port,
            "document_root": document_root.replace('\\', "/"),
            "php_cgi_port": php_cgi_port,
        });

        let config_content = self
            .handlebars
            .render("nginx_vhost", &template_data)
            .context("Failed to render nginx vhost template")?;

        let rollback = FileRollback::capture(config_path.clone())?;

        fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write vhost config: {:?}", config_path))?;

        info!("Wrote nginx vhost config: {:?}", config_path);
        Ok(rollback)
    }

    /// Remove vhost configuration files for a site.
    fn remove_vhost_configs(
        &self,
        nginx_config: &crate::database::ServiceConfig,
        site: &SiteInfo,
    ) -> Result<Vec<FileRollback>> {
        let safe_name = site.domain.replace(['.', '*', ':'], "_");
        let config_name = format!("{}_{}.conf", safe_name, site.port);

        let nginx_root = nginx_config.cwd.as_deref().unwrap_or_default();
        if nginx_root.is_empty() {
            anyhow::bail!("Nginx 安装目录未配置");
        }
        let vhost_dirs = [Path::new(nginx_root).join("conf").join("vhosts")];

        let mut rollbacks = Vec::new();
        for vhost_dir in &vhost_dirs {
            let config_path = vhost_dir.join(&config_name);
            if config_path.exists() {
                let rollback = FileRollback::capture(config_path.clone())?;
                fs::remove_file(&config_path)
                    .with_context(|| format!("Failed to remove vhost config: {:?}", config_path))?;
                rollbacks.push(rollback);
                info!("Removed vhost config: {:?}", config_path);
            }
        }

        Ok(rollbacks)
    }

    fn resolve_php_service(&self, requested: Option<&str>) -> Result<String> {
        let services = self.db.list_service_instances()?;
        if let Some(id) = requested.filter(|value| !value.trim().is_empty()) {
            let service = services
                .iter()
                .find(|service| service.id == id)
                .ok_or_else(|| anyhow::anyhow!("PHP 运行环境不存在：{}", id))?;
            if service.service_type != "php" || !service.installed {
                anyhow::bail!("PHP 运行环境不可用：{}", id);
            }
            return Ok(service.id.clone());
        }

        services
            .into_iter()
            .find(|service| service.service_type == "php" && service.installed)
            .map(|service| service.id)
            .ok_or_else(|| anyhow::anyhow!("没有可用 PHP 运行环境，请先导入 PHP"))
    }

    fn validate_nginx_config(&self, nginx_config: &crate::database::ServiceConfig) -> Result<()> {
        let exe = Path::new(&nginx_config.exe);
        if !exe.is_file() {
            anyhow::bail!("Nginx 可执行文件不存在：{}", nginx_config.exe);
        }

        let cwd = nginx_config.cwd.as_deref().unwrap_or(".");
        let output = std::process::Command::new(exe)
            .arg("-t")
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Nginx 配置检查命令执行失败")?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Nginx 配置检查失败：{} {}", stdout.trim(), stderr.trim());
        }

        Ok(())
    }

    fn reload_nginx(&self, nginx_config: &crate::database::ServiceConfig) -> Result<()> {
        let cwd = nginx_config.cwd.as_deref().unwrap_or(".");
        let output = std::process::Command::new(&nginx_config.exe)
            .args(["-s", "reload"])
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Nginx reload 命令执行失败")?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Nginx reload 失败：{} {}", stdout.trim(), stderr.trim());
        }

        Ok(())
    }

    fn health_check_site(&self, domain: &str, port: u16) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
            .with_context(|| format!("站点健康检查失败：127.0.0.1:{} 无法连接", port))?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(2))).ok();

        let request = format!(
            "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            domain
        );
        stream.write_all(request.as_bytes())?;

        let mut response = String::new();
        stream.read_to_string(&mut response)?;
        if response.starts_with("HTTP/") {
            return Ok(());
        }

        anyhow::bail!("站点健康检查失败：未收到 HTTP 响应");
    }
}

struct FileRollback {
    path: PathBuf,
    previous: Option<Vec<u8>>,
}

impl FileRollback {
    fn capture(path: PathBuf) -> Result<Self> {
        let previous = if path.exists() {
            Some(fs::read(&path)?)
        } else {
            None
        };
        Ok(Self { path, previous })
    }

    fn restore(&self) -> Result<()> {
        match &self.previous {
            Some(bytes) => fs::write(&self.path, bytes)?,
            None => {
                if self.path.exists() {
                    fs::remove_file(&self.path)?;
                }
            }
        }
        Ok(())
    }
}

fn validate_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().to_lowercase();
    if domain.is_empty() {
        anyhow::bail!("域名不能为空");
    }
    if domain.contains("://") || domain.contains('/') || domain.contains('\\') {
        anyhow::bail!("域名格式不正确，不能包含协议或路径");
    }
    if domain.len() > 253 {
        anyhow::bail!("域名过长");
    }
    let labels = domain.split('.').collect::<Vec<_>>();
    if labels.iter().any(|label| label.is_empty()) {
        anyhow::bail!("域名格式不正确");
    }
    for label in labels {
        if label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            anyhow::bail!("域名格式不正确：{}", domain);
        }
    }
    Ok(domain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::TcpListener;
    use std::sync::Arc;
    use std::thread;
    use uuid::Uuid;

    fn make_db() -> Arc<Database> {
        let dir = std::env::temp_dir().join(format!("winserver-site-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let db = Database::new(&dir.join("winserver.db")).expect("open db");
        db.run_migrations().expect("migrate db");
        Arc::new(db)
    }

    fn make_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{}-{}", prefix, Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create dir");
        dir
    }

    fn free_port() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind free port");
        listener.local_addr().expect("local addr").port()
    }

    fn node_exe() -> Option<String> {
        std::process::Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())?;

        let output = std::process::Command::new("where")
            .arg("node")
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .find(|line| line.to_ascii_lowercase().ends_with(".exe"))
            .map(str::to_string)
    }

    fn write_node_listener(dir: &Path, port: u16) -> PathBuf {
        let script = dir.join(format!("php-listener-{}.js", port));
        fs::write(
            &script,
            format!(
                "const net=require('net');const server=net.createServer(s=>s.end('ok'));server.listen({},'127.0.0.1');setInterval(()=>{{}},1000);",
                port
            ),
        )
        .expect("write node listener");
        script
    }

    fn write_fake_nginx(root: &Path, success: bool) -> PathBuf {
        fs::create_dir_all(root.join("conf").join("vhosts")).expect("create nginx dirs");
        fs::write(root.join("conf").join("nginx.conf"), "events {}\nhttp {}\n")
            .expect("write nginx conf");

        let script = root.join("nginx.cmd");
        let body = if success {
            "@echo fake nginx %*\r\nexit /b 0\r\n"
        } else {
            "@echo invalid nginx config 1>&2\r\nexit /b 1\r\n"
        };
        fs::write(&script, body).expect("write fake nginx");
        script
    }

    fn spawn_health_server() -> (u16, thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind health server");
        let port = listener.local_addr().expect("health addr").port();
        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0; 512];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                );
            }
        });
        (port, handle)
    }

    fn configure_nginx(db: &Database, root: &Path, exe: &Path, port: u16) {
        db.upsert_service_runtime(
            "nginx",
            "Nginx Test",
            "nginx",
            "nginx.exe",
            port,
            &exe.to_string_lossy(),
            "",
            &root.to_string_lossy(),
            Some(&root.join("conf").join("nginx.conf").to_string_lossy()),
        )
        .expect("upsert nginx");
    }

    fn configure_php_service(db: &Database, id: &str, node: &str, dir: &Path, port: u16) {
        let script = write_node_listener(dir, port);
        db.upsert_service_runtime(
            id,
            "PHP Test CGI",
            "php",
            "php-cgi.exe",
            port,
            node,
            &format!("\"{}\"", script.to_string_lossy()),
            &dir.to_string_lossy(),
            Some(&dir.join("php.ini").to_string_lossy()),
        )
        .expect("upsert php");
    }

    fn configure_php(db: &Database, node: &str, dir: &Path, port: u16) {
        configure_php_service(db, "php-test", node, dir, port);
    }

    #[tokio::test]
    async fn create_site_writes_config_hosts_and_database() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping site create test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_dir("winserver-site-data");
        let nginx_root = make_dir("winserver-nginx");
        let nginx_exe = write_fake_nginx(&nginx_root, true);
        let php_dir = make_dir("winserver-php");
        let php_port = free_port();
        let (site_port, health_handle) = spawn_health_server();
        let hosts_path = data_dir.join("hosts");
        fs::write(&hosts_path, "127.0.0.1 localhost\r\n").expect("write hosts");
        let document_root = data_dir.join("www").join("demo");
        fs::create_dir_all(&document_root).expect("create doc root");
        fs::write(document_root.join("index.php"), "<?php echo 'ok';").expect("write index");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        configure_php(&db, &node, &php_dir, php_port);

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(
            data_dir.clone(),
            process_manager.clone(),
            hosts_manager,
            db.clone(),
        )
        .expect("site manager");

        site_manager
            .create_site(
                "demo.local",
                site_port,
                &document_root.to_string_lossy(),
                "nginx",
                Some("php-test"),
            )
            .await
            .expect("create site");

        let _ = health_handle.join();
        let _ = process_manager.stop_service("php-test").await;

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("demo_local_{}.conf", site_port));
        let vhost_content = fs::read_to_string(&vhost).expect("read vhost");
        assert!(vhost_content.contains("server_name  demo.local"));
        assert!(vhost_content.contains(&format!("127.0.0.1:{}", php_port)));

        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(hosts.contains("127.0.0.1 demo.local # winserver demo.local"));

        let sites = db.list_sites().expect("list sites");
        let site = sites.iter().find(|site| site.domain == "demo.local").expect("site saved");
        assert_eq!(site.port, site_port);
        assert_eq!(site.php_runtime_id.as_deref(), Some("php-test"));
        assert_eq!(site.status, "running");
    }

    #[tokio::test]
    async fn create_site_rolls_back_config_and_hosts_on_nginx_error() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping site rollback test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_dir("winserver-site-rollback-data");
        let nginx_root = make_dir("winserver-nginx-bad");
        let nginx_exe = write_fake_nginx(&nginx_root, false);
        let php_dir = make_dir("winserver-php-bad");
        let php_port = free_port();
        let site_port = free_port();
        let hosts_path = data_dir.join("hosts");
        fs::write(&hosts_path, "127.0.0.1 localhost\r\n").expect("write hosts");
        let document_root = data_dir.join("www").join("broken");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        configure_php(&db, &node, &php_dir, php_port);

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(
            data_dir,
            process_manager.clone(),
            hosts_manager,
            db.clone(),
        )
        .expect("site manager");

        let error = site_manager
            .create_site(
                "broken.local",
                site_port,
                &document_root.to_string_lossy(),
                "nginx",
                Some("php-test"),
            )
            .await
            .expect_err("nginx validation should fail");

        let _ = process_manager.stop_service("php-test").await;

        assert!(error.to_string().contains("Nginx 配置检查失败"));
        let vhost = nginx_root.join("conf").join("vhosts").join(format!("broken_local_{}.conf", site_port));
        assert!(!vhost.exists(), "failed create should remove vhost");
        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(!hosts.contains("broken.local"), "failed create should rollback hosts");
        assert!(
            !db.list_sites()
                .expect("list sites")
                .iter()
                .any(|site| site.domain == "broken.local")
        );
    }

    #[tokio::test]
    async fn switch_php_updates_vhost_and_database_after_health_check() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping php switch test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_dir("winserver-switch-data");
        let nginx_root = make_dir("winserver-switch-nginx");
        let nginx_exe = write_fake_nginx(&nginx_root, true);
        let php_dir = make_dir("winserver-switch-php");
        let php_port = free_port();
        let (site_port, health_handle) = spawn_health_server();
        let document_root = data_dir.join("www").join("switch");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        configure_php_service(&db, "php-new", &node, &php_dir, php_port);

        db.insert_site(&SiteInfo {
            id: "site-switch".to_string(),
            name: "switch.local".to_string(),
            domain: "switch.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: Some("php-old".to_string()),
            ssl: false,
            status: "running".to_string(),
        })
        .expect("insert site");

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("switch_local_{}.conf", site_port));
        fs::write(&vhost, "fastcgi_pass 127.0.0.1:9001;\n").expect("write old vhost");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(data_dir.join("hosts")));
        let site_manager = SiteManager::new(data_dir, process_manager.clone(), hosts_manager, db.clone())
            .expect("site manager");

        site_manager
            .switch_php("site-switch", "php-new")
            .await
            .expect("switch php");

        let _ = health_handle.join();
        let _ = process_manager.stop_service("php-new").await;

        let vhost_content = fs::read_to_string(&vhost).expect("read vhost");
        assert!(vhost_content.contains(&format!("127.0.0.1:{}", php_port)));
        let site = db.get_site("site-switch").expect("get site");
        assert_eq!(site.php_runtime_id.as_deref(), Some("php-new"));
    }

    #[tokio::test]
    async fn switch_php_rolls_back_vhost_and_keeps_original_database_value() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping php switch rollback test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_dir("winserver-switch-rollback-data");
        let nginx_root = make_dir("winserver-switch-nginx-bad");
        let nginx_exe = write_fake_nginx(&nginx_root, false);
        let php_dir = make_dir("winserver-switch-php-bad");
        let php_port = free_port();
        let site_port = free_port();
        let document_root = data_dir.join("www").join("switch-bad");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        configure_php_service(&db, "php-new", &node, &php_dir, php_port);

        db.insert_site(&SiteInfo {
            id: "site-switch-bad".to_string(),
            name: "switch-bad.local".to_string(),
            domain: "switch-bad.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: Some("php-old".to_string()),
            ssl: false,
            status: "running".to_string(),
        })
        .expect("insert site");

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("switch-bad_local_{}.conf", site_port));
        let original = "fastcgi_pass 127.0.0.1:9001;\n";
        fs::write(&vhost, original).expect("write old vhost");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(data_dir.join("hosts")));
        let site_manager = SiteManager::new(data_dir, process_manager.clone(), hosts_manager, db.clone())
            .expect("site manager");

        let error = site_manager
            .switch_php("site-switch-bad", "php-new")
            .await
            .expect_err("bad nginx should fail");

        let _ = process_manager.stop_service("php-new").await;

        assert!(error.to_string().contains("Nginx 配置检查失败"));
        assert_eq!(fs::read_to_string(&vhost).expect("read vhost"), original);
        let site = db.get_site("site-switch-bad").expect("get site");
        assert_eq!(site.php_runtime_id.as_deref(), Some("php-old"));
    }

    #[tokio::test]
    async fn delete_site_removes_managed_config_hosts_and_keeps_sources() {
        let db = make_db();
        let data_dir = make_dir("winserver-delete-data");
        let nginx_root = make_dir("winserver-delete-nginx");
        let nginx_exe = write_fake_nginx(&nginx_root, true);
        let site_port = free_port();
        let hosts_path = data_dir.join("hosts");
        fs::write(
            &hosts_path,
            "127.0.0.1 localhost\r\n127.0.0.1 delete.local # winserver delete.local\r\n127.0.0.1 other.local # winserver other.local\r\n",
        )
        .expect("write hosts");
        let document_root = data_dir.join("www").join("delete");
        fs::create_dir_all(&document_root).expect("create doc root");
        fs::write(document_root.join("index.php"), "<?php echo 'keep';").expect("write source");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        db.insert_site(&SiteInfo {
            id: "site-delete".to_string(),
            name: "delete.local".to_string(),
            domain: "delete.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: Some("php-old".to_string()),
            ssl: false,
            status: "running".to_string(),
        })
        .expect("insert site");

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("delete_local_{}.conf", site_port));
        let other_vhost = nginx_root.join("conf").join("vhosts").join("other_local_80.conf");
        fs::write(&vhost, "server_name delete.local;\n").expect("write vhost");
        fs::write(&other_vhost, "server_name other.local;\n").expect("write other vhost");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(data_dir, process_manager, hosts_manager, db.clone())
            .expect("site manager");

        site_manager.delete_site("site-delete").await.expect("delete site");

        assert!(!vhost.exists(), "managed vhost should be removed");
        assert!(other_vhost.exists(), "other site vhost should stay");
        assert!(document_root.join("index.php").exists(), "source files should stay");
        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(!hosts.contains("delete.local # winserver delete.local"));
        assert!(hosts.contains("other.local # winserver other.local"));
        assert!(db.get_site("site-delete").is_err(), "database site record should be removed");
    }

    #[tokio::test]
    async fn delete_site_rolls_back_when_nginx_reload_validation_fails() {
        let db = make_db();
        let data_dir = make_dir("winserver-delete-rollback-data");
        let nginx_root = make_dir("winserver-delete-nginx-bad");
        let nginx_exe = write_fake_nginx(&nginx_root, false);
        let site_port = free_port();
        let hosts_path = data_dir.join("hosts");
        fs::write(
            &hosts_path,
            "127.0.0.1 localhost\r\n127.0.0.1 keep.local # winserver keep.local\r\n",
        )
        .expect("write hosts");
        let document_root = data_dir.join("www").join("keep");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        db.insert_site(&SiteInfo {
            id: "site-keep".to_string(),
            name: "keep.local".to_string(),
            domain: "keep.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: Some("php-old".to_string()),
            ssl: false,
            status: "running".to_string(),
        })
        .expect("insert site");

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("keep_local_{}.conf", site_port));
        let original = "server_name keep.local;\n";
        fs::write(&vhost, original).expect("write vhost");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(data_dir, process_manager, hosts_manager, db.clone())
            .expect("site manager");

        let error = site_manager
            .delete_site("site-keep")
            .await
            .expect_err("delete should rollback");

        assert!(error.to_string().contains("Nginx 配置检查失败"));
        assert_eq!(fs::read_to_string(&vhost).expect("read vhost"), original);
        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(hosts.contains("keep.local # winserver keep.local"));
        assert!(db.get_site("site-keep").is_ok(), "database site record should remain");
    }

    #[tokio::test]
    async fn enable_site_writes_config_hosts_and_validates() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping enable test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_dir("winserver-enable-data");
        let nginx_root = make_dir("winserver-enable-nginx");
        let nginx_exe = write_fake_nginx(&nginx_root, true);
        let php_dir = make_dir("winserver-enable-php");
        let php_port = free_port();
        let (site_port, health_handle) = spawn_health_server();
        let hosts_path = data_dir.join("hosts");
        fs::write(&hosts_path, "127.0.0.1 localhost\r\n").expect("write hosts");
        let document_root = data_dir.join("www").join("enable");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);
        configure_php(&db, &node, &php_dir, php_port);

        db.insert_site(&SiteInfo {
            id: "site-enable".to_string(),
            name: "enable.local".to_string(),
            domain: "enable.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: Some("php-test".to_string()),
            ssl: false,
            status: "disabled".to_string(),
        })
        .expect("insert site");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(
            data_dir.clone(),
            process_manager.clone(),
            hosts_manager,
            db.clone(),
        )
        .expect("site manager");

        site_manager.enable_site("site-enable").await.expect("enable site");

        let _ = health_handle.join();
        let _ = process_manager.stop_service("php-test").await;

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("enable_local_{}.conf", site_port));
        assert!(vhost.exists(), "vhost config should be written");

        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(hosts.contains("enable.local # winserver enable.local"));

        let site = db.get_site("site-enable").expect("get site");
        assert_eq!(site.status, "running");
    }

    #[tokio::test]
    async fn disable_site_removes_config_hosts_and_validates() {
        let db = make_db();
        let data_dir = make_dir("winserver-disable-data");
        let nginx_root = make_dir("winserver-disable-nginx");
        let nginx_exe = write_fake_nginx(&nginx_root, true);
        let site_port = free_port();
        let hosts_path = data_dir.join("hosts");
        fs::write(&hosts_path, "127.0.0.1 localhost\r\n127.0.0.1 disable.local # winserver disable.local\r\n").expect("write hosts");
        let document_root = data_dir.join("www").join("disable");
        fs::create_dir_all(&document_root).expect("create doc root");

        configure_nginx(&db, &nginx_root, &nginx_exe, site_port);

        let vhost = nginx_root.join("conf").join("vhosts").join(format!("disable_local_{}.conf", site_port));
        fs::write(&vhost, "server_name disable.local;\n").expect("write vhost");

        db.insert_site(&SiteInfo {
            id: "site-disable".to_string(),
            name: "disable.local".to_string(),
            domain: "disable.local".to_string(),
            port: site_port,
            document_root: document_root.to_string_lossy().to_string(),
            server_type: ServerType::Nginx,
            php_runtime_id: None,
            ssl: false,
            status: "running".to_string(),
        })
        .expect("insert site");

        let process_manager = Arc::new(ProcessManager::new(db.clone()));
        let hosts_manager = Arc::new(HostsManager::with_path(hosts_path.clone()));
        let site_manager = SiteManager::new(data_dir, process_manager, hosts_manager, db.clone())
            .expect("site manager");

        site_manager.disable_site("site-disable").await.expect("disable site");

        assert!(!vhost.exists(), "vhost config should be removed");

        let hosts = fs::read_to_string(&hosts_path).expect("read hosts");
        assert!(!hosts.contains("disable.local # winserver disable.local"));

        let site = db.get_site("site-disable").expect("get site");
        assert_eq!(site.status, "disabled");
    }
}
