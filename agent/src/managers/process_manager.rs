use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use sysinfo::{System, Disks};
use tokio::sync::Mutex;
use tracing::{info, warn};

use shared::types::{DiskInfo, ServiceState, SystemResource};

use crate::database::Database;

/// Simple shell-style word splitting: splits on whitespace while respecting
/// double-quoted segments. This avoids pulling in an extra crate.
fn shell_words(s: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn service_config_path(config_file: &Option<String>, cwd: &str, fallback_name: &str) -> Option<PathBuf> {
    config_file
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let fallback = PathBuf::from(cwd).join(fallback_name);
            if fallback.exists() {
                Some(fallback)
            } else {
                None
            }
        })
}

fn parse_port(value: &str) -> Option<u16> {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
}

fn parse_redis_port(path: &Path) -> Option<u16> {
    let raw = std::fs::read_to_string(path).ok()?;
    raw.lines().find_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        if key.eq_ignore_ascii_case("port") {
            parts.next().and_then(parse_port)
        } else {
            None
        }
    })
}

fn parse_env_file(path: &Path) -> HashMap<String, String> {
    let mut values = HashMap::new();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return values;
    };

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
        values.insert(key.to_string(), value);
    }

    values
}

/// A tracked child process entry.
#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u32,
    pub service_id: String,
    pub exe: String,
}

/// Manages spawning, stopping, and monitoring of service processes.
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    db: Arc<Database>,
    health_monitor_running: Arc<std::sync::atomic::AtomicBool>,
}

impl ProcessManager {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            db,
            health_monitor_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start a background health monitor that checks tracked PIDs every 5 seconds.
    /// When a PID dies unexpectedly, updates DB state to Failed with error message.
    /// This version is async and should be spawned via `tokio::spawn` or `tauri::async_runtime::spawn`.
    pub async fn start_health_monitor_async(&self) {
        if self.health_monitor_running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return; // Already running
        }

        let processes = self.processes.clone();
        let db = self.db.clone();
        let running_flag = self.health_monitor_running.clone();

        info!("Health monitor started");
        loop {
            if !running_flag.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            tokio::time::sleep(Duration::from_secs(5)).await;

                let tracked: Vec<ProcessEntry> = {
                    let procs = processes.lock().await;
                    procs.values().cloned().collect()
                };

                for entry in tracked {
                    let mut sys = System::new();
                    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                    let is_alive = sys.process(sysinfo::Pid::from_u32(entry.pid)).is_some();

                    if !is_alive {
                        warn!("Health monitor: PID {} ({}) died unexpectedly", entry.pid, entry.service_id);

                        {
                            let mut procs = processes.lock().await;
                            procs.remove(&entry.service_id);
                        }

                        let error_msg = format!("进程意外退出：PID {}", entry.pid);
                        let _ = db.update_service_failure(&entry.service_id, &error_msg);
                        let _ = db.add_log("health.monitor", &entry.service_id, false, &error_msg);
                    }
                }
            }
            info!("Health monitor stopped");
    }

    /// Start a service by its ID. Reads config from the database, spawns the
    /// process, and tracks it.
    pub async fn start_service(&self, service_id: &str) -> Result<()> {
        // Check if already running
        if self.is_service_running(service_id).await {
            info!("Service {} is already running", service_id);
            let current_pid = self
                .db
                .get_service_config(service_id)
                .ok()
                .and_then(|config| config.pid);
            self.db
                .update_service_state(service_id, ServiceState::Running, current_pid)?;
            return Ok(());
        }

        self.db.update_service_state(service_id, ServiceState::Starting, None)?;

        let result = self.start_service_inner(service_id).await;
        if let Err(error) = &result {
            let message = error.to_string();
            let _ = self.db.update_service_failure(service_id, &message);
            let _ = self.db.add_log("service.start", service_id, false, &message);
        }
        result
    }

    async fn start_service_inner(&self, service_id: &str) -> Result<()> {
        let config = self.db.get_service_config(service_id)?;

        if !config.installed {
            anyhow::bail!("运行环境未导入或未安装：{}", service_id);
        }

        let exe = config.exe.clone();
        let mut args: Vec<String> = config
            .args
            .as_deref()
            .map(|s| shell_words(s))
            .unwrap_or_default();
        let mut port = config.port;
        let cwd = config.cwd.clone().unwrap_or_else(|| {
            PathBuf::from(&exe)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        });
        let mut env_extra = config.env.unwrap_or_default();

        // Verify executable exists
        if !Path::new(&exe).exists() {
            anyhow::bail!("可执行文件不存在：{}", exe);
        }

        if service_id == "redis" {
            if let Some(conf_path) = service_config_path(&config.config_file, &cwd, "redis.conf") {
                if let Some(config_port) = parse_redis_port(&conf_path) {
                    port = config_port;
                    let _ = self.db.update_service_port(service_id, port);
                }
                args = vec![conf_path.to_string_lossy().to_string()];
            }
        } else if service_id == "minio" {
            let env_path = service_config_path(&config.config_file, &cwd, "minio.env");
            let minio_env = env_path
                .as_deref()
                .map(parse_env_file)
                .unwrap_or_default();
            for (key, value) in &minio_env {
                env_extra.insert(key.clone(), value.clone());
            }
            if let Some(path) = env_path {
                env_extra.insert("MINIO_CONFIG_ENV_FILE".to_string(), path.to_string_lossy().to_string());
            }
            let api_port = minio_env
                .get("MINIO_API_PORT")
                .and_then(|value| parse_port(value))
                .unwrap_or(port);
            let console_port = minio_env
                .get("MINIO_CONSOLE_PORT")
                .and_then(|value| parse_port(value))
                .unwrap_or(9001);
            port = api_port;
            let _ = self.db.update_service_port(service_id, port);
            let data_dir = PathBuf::from(&cwd).join("data");
            let _ = std::fs::create_dir_all(&data_dir);
            args = vec![
                "server".to_string(),
                data_dir.to_string_lossy().to_string(),
                "--address".to_string(),
                format!(":{}", api_port),
                "--console-address".to_string(),
                format!(":{}", console_port),
            ];
        }

        if port > 0 && Self::is_port_listening(port) {
            anyhow::bail!("端口 {} 已被占用，无法启动 {}", port, service_id);
        }

        // Auto-start PHP-CGI when Nginx or Apache starts
        if service_id == "nginx" || service_id == "apache" {
            self.ensure_php_cgi_running().await;
        }

        // Spawn the process
        info!("Starting service {}: {} {:?}", service_id, exe, args);

        let mut cmd = std::process::Command::new(&exe);
        cmd.args(&args)
            .current_dir(&cwd)
            .envs(&env_extra)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        // On Windows, use CREATE_NO_WINDOW + DETACHED_PROCESS + CREATE_NEW_PROCESS_GROUP
        // to prevent console windows from appearing on screen.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            const DETACHED_PROCESS: u32 = 0x00000008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        }

        let child = cmd
            .spawn()
            .context(format!("进程启动失败：{} {:?}", exe, args))?;

        let pid = child.id();
        info!("Service {} started with PID {}", service_id, pid);

        // Track the process
        {
            let mut processes = self.processes.lock().await;
            processes.insert(
                service_id.to_string(),
                ProcessEntry {
                    pid,
                    service_id: service_id.to_string(),
                    exe: exe.clone(),
                },
            );
        }

        let ready = self
            .wait_for_service_ready(pid, port, Duration::from_secs(8))
            .await;
        if let Err(error) = ready {
            let _ = self.kill_process_tree(pid).await;
            let mut processes = self.processes.lock().await;
            processes.remove(service_id);
            anyhow::bail!("{}", error);
        }

        // Update database state
        self.db
            .update_service_state(service_id, ServiceState::Running, Some(pid))?;
        self.db.add_log(
            "service.start",
            service_id,
            true,
            &format!("服务已启动，PID {}，端口 {}", pid, port),
        )?;

        Ok(())
    }

    /// Stop a service by its ID. Tries graceful stop first for nginx,
    /// then falls back to taskkill /F /T /PID.
    pub async fn stop_service(&self, service_id: &str) -> Result<()> {
        self.db.update_service_state(service_id, ServiceState::Stopping, None)?;

        let result = self.stop_service_inner(service_id).await;
        if let Err(error) = &result {
            let message = error.to_string();
            let _ = self.db.update_service_failure(service_id, &message);
            let _ = self.db.add_log("service.stop", service_id, false, &message);
        }
        result
    }

    async fn stop_service_inner(&self, service_id: &str) -> Result<()> {
        let entry = {
            let processes = self.processes.lock().await;
            processes.get(service_id).cloned()
        };

        let pid: Option<u32> = match entry {
            Some(e) => Some(e.pid),
            None => {
                // Try to find the PID from the database
                if let Ok(config) = self.db.get_service_config(service_id) {
                    config.pid
                } else {
                    None
                }
            }
        };

        if let Some(pid) = pid {
            // Graceful stop for nginx
            if service_id == "nginx" {
                if let Err(e) = self.graceful_nginx_quit().await {
                    warn!("Nginx graceful quit failed, falling back to taskkill: {}", e);
                } else {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                }
            }

            if self.is_pid_alive(pid) {
                self.kill_process_tree(pid).await?;
            }

            if !self.wait_for_process_exit(pid, Duration::from_secs(6)).await {
                anyhow::bail!("服务 {} 停止失败：PID {} 仍在运行", service_id, pid);
            }
        } else {
            let config = self.db.get_service_config(service_id)?;
            if config.port > 0 && Self::is_port_listening(config.port) {
                anyhow::bail!(
                    "无法停止 {}：没有托管 PID，但端口 {} 仍被占用。为避免误杀同名进程，请先导入并启动该服务实例。",
                    service_id,
                    config.port
                );
            }
            info!("Service {} has no tracked PID and no listening port", service_id);
        }

        // Remove from tracked processes
        {
            let mut processes = self.processes.lock().await;
            processes.remove(service_id);
        }

        // Update database state
        self.db
            .update_service_state(service_id, ServiceState::Stopped, None)?;
        self.db
            .add_log("service.stop", service_id, true, "服务已停止")?;

        // Auto-stop PHP-CGI if neither Nginx nor Apache are running
        if service_id == "nginx" || service_id == "apache" {
            self.maybe_stop_php_cgi().await;
        }

        info!("Service {} stopped", service_id);
        Ok(())
    }

    /// Check if a service is still running by verifying its PID is alive.
    pub async fn is_service_running(&self, service_id: &str) -> bool {
        let entry = {
            let processes = self.processes.lock().await;
            processes.get(service_id).cloned()
        };

        let pid: Option<u32> = match entry {
            Some(e) => Some(e.pid),
            None => {
                // Check database for last known PID
                if let Ok(config) = self.db.get_service_config(service_id) {
                    config.pid
                } else {
                    None
                }
            }
        };

        match pid {
            Some(pid) => self.is_pid_alive(pid),
            None => false,
        }
    }

    /// Get system resource information using the sysinfo crate.
    pub fn get_system_resource(&self) -> Result<SystemResource> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_percent = sys.global_cpu_usage() as f64;
        let cpu_count = sys.cpus().len();
        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_default();

        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_percent = if total_memory > 0 {
            (used_memory as f64 / total_memory as f64) * 100.0
        } else {
            0.0
        };

        // Get disk info for the system drive
        let disks = Disks::new_with_refreshed_list();
        let disk_info = disks
            .first()
            .map(|d| {
                let total = d.total_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let available = d.available_space() as f64 / (1024.0 * 1024.0 * 1024.0);
                let used = total - available;
                let percent = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
                DiskInfo {
                    percent,
                    used_gb: used,
                    total_gb: total,
                }
            })
            .unwrap_or(DiskInfo {
                percent: 0.0,
                used_gb: 0.0,
                total_gb: 0.0,
            });

        let uptime_seconds = System::uptime();

        Ok(SystemResource {
            cpu_percent,
            cpu_count,
            cpu_model,
            memory_percent,
            total_memory_mb: total_memory / (1024 * 1024),
            used_memory_mb: used_memory / (1024 * 1024),
            disk: disk_info,
            uptime_seconds,
        })
    }

    // ---- Private helpers ----

    /// Check if a PID is still alive using sysinfo.
    fn is_pid_alive(&self, pid: u32) -> bool {
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        sys.process(sysinfo::Pid::from_u32(pid)).is_some()
    }

    /// Send nginx -s quit for graceful shutdown.
    async fn graceful_nginx_quit(&self) -> Result<()> {
        let config = self.db.get_service_config("nginx")?;

        let exe = &config.exe;
        let cwd = config.cwd.clone().unwrap_or_else(|| {
            PathBuf::from(exe)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        });

        let output = std::process::Command::new(exe)
            .args(["-p", &cwd, "-s", "quit"])
            .current_dir(&cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to run nginx -s quit")?;

        if !output.status.success() {
            anyhow::bail!(
                "nginx -s quit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Kill a process and its entire process tree using taskkill.
    async fn kill_process_tree(&self, pid: u32) -> Result<()> {
        info!("Killing process tree for PID {}", pid);

        let output = std::process::Command::new("taskkill.exe")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .context("Failed to run taskkill")?;

        // taskkill returns non-zero if the process doesn't exist, which is fine
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // "not found" type errors are acceptable
            if !stderr.contains("not found") && !stderr.contains("ERROR_") {
                warn!("taskkill output: {}", stderr);
            }
        }

        Ok(())
    }

    /// Ensure PHP-CGI is running when Nginx or Apache starts.
    fn ensure_php_cgi_running<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            // Find PHP-CGI service IDs (they start with "php")
            let php_service_ids = {
                match self.db.list_service_instances() {
                    Ok(services) => services
                        .into_iter()
                        .filter(|s| s.service_type == "php" || s.id.starts_with("php"))
                        .filter(|s| s.installed)
                        .map(|s| s.id)
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        warn!("Failed to list services for PHP auto-start: {}", e);
                        vec![]
                    }
                }
            };

            for php_id in php_service_ids {
                if !self.is_service_running(&php_id).await {
                    info!("Auto-starting PHP-CGI service: {}", php_id);
                    if let Err(e) = self.start_service(&php_id).await {
                        warn!("PHP-CGI auto-start failed for {}: {}", php_id, e);
                    }
                }
            }
        })
    }

    /// Stop PHP-CGI if neither Nginx nor Apache are running.
    fn maybe_stop_php_cgi<'a>(&'a self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let nginx_running = self.is_service_running("nginx").await;
            let apache_running = self.is_service_running("apache").await;

            if !nginx_running && !apache_running {
                // Find PHP-CGI services and stop them
                let php_service_ids = {
                    match self.db.list_service_instances() {
                        Ok(services) => services
                            .into_iter()
                            .filter(|s| s.service_type == "php" || s.id.starts_with("php"))
                            .map(|s| s.id)
                            .collect::<Vec<_>>(),
                        Err(e) => {
                            warn!("Failed to list services for PHP auto-stop: {}", e);
                            vec![]
                        }
                    }
                };

                for php_id in php_service_ids {
                    if self.is_service_running(&php_id).await {
                        info!("Auto-stopping PHP-CGI service: {}", php_id);
                        if let Err(e) = self.stop_service(&php_id).await {
                            warn!("PHP-CGI auto-stop failed for {}: {}", php_id, e);
                        }
                    }
                }
            }
        })
    }

    async fn wait_for_service_ready(&self, pid: u32, port: u16, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        let mut saw_pid = false;
        while Instant::now() < deadline {
            if !self.is_pid_alive(pid) {
                anyhow::bail!("进程启动后立即退出：PID {}", pid);
            }

            saw_pid = true;
            if port == 0 || Self::is_port_listening(port) {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(150)).await;
        }

        if saw_pid {
            anyhow::bail!("健康检查失败：端口 {} 未监听", port);
        }

        anyhow::bail!("健康检查失败：PID {} 未运行", pid);
    }

    async fn wait_for_process_exit(&self, pid: u32, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if !self.is_pid_alive(pid) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        !self.is_pid_alive(pid)
    }

    fn is_port_listening(port: u16) -> bool {
        if port == 0 {
            return false;
        }
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use shared::types::ServiceInfo;
    use std::fs;
    use std::net::TcpListener;
    use uuid::Uuid;

    fn make_db() -> Arc<Database> {
        let dir = std::env::temp_dir().join(format!("winserver-agent-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let db = Database::new(&dir.join("winserver.db")).expect("open db");
        db.run_migrations().expect("migrate db");
        Arc::new(db)
    }

    fn free_port() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind free port");
        listener.local_addr().expect("local addr").port()
    }

    fn node_exe() -> Option<String> {
        let available = std::process::Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())?;

        if !available.status.success() {
            return None;
        }

        #[cfg(windows)]
        {
            let output = std::process::Command::new("where")
                .arg("node")
                .output()
                .ok()?;
            let first = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())?
                .to_string();
            Some(first)
        }

        #[cfg(not(windows))]
        {
            let output = std::process::Command::new("which")
                .arg("node")
                .output()
                .ok()?;
            let first = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())?
                .to_string();
            Some(first)
        }
    }

    fn write_node_listener(port: u16) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("winserver-node-listener-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create listener dir");
        let script = dir.join("listener.js");
        fs::write(
            &script,
            format!(
                "const net=require('net');const server=net.createServer(s=>s.end('ok'));server.listen({},'127.0.0.1');setInterval(()=>{{}},1000);",
                port
            ),
        )
        .expect("write listener script");
        (dir, script)
    }

    fn insert_test_service(
        db: &Database,
        id: &str,
        exe: &str,
        args: &str,
        cwd: &std::path::Path,
        port: u16,
    ) {
        let now = chrono::Utc::now().to_rfc3339();
        db.conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO service_instances
                 (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed, desired_state, actual_state, created_at, updated_at)
                 VALUES (?1, ?2, 'test', 'node.exe', ?3, ?4, ?5, ?6, '', 0, 1, 'installed', 'stopped', ?7, ?8)",
                params![
                    id,
                    id,
                    port as i32,
                    exe,
                    args,
                    cwd.to_string_lossy().to_string(),
                    &now,
                    &now,
                ],
            )?;
            Ok(())
        })
        .expect("insert test service");
    }

    fn service_state(db: &Database, id: &str) -> ServiceInfo {
        db.list_service_instances()
            .expect("list services")
            .into_iter()
            .find(|svc| svc.id == id)
            .expect("test service")
    }

    #[test]
    fn parse_redis_port_uses_uncommented_port() {
        let dir = std::env::temp_dir().join(format!("winserver-redis-conf-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create redis conf dir");
        let conf = dir.join("redis.conf");
        fs::write(
            &conf,
            "# port 6379\n\nbind 127.0.0.1\nport 6388\nprotected-mode yes\n",
        )
        .expect("write redis conf");

        assert_eq!(parse_redis_port(&conf), Some(6388));
    }

    #[test]
    fn parse_env_file_strips_quotes_and_ignores_comments() {
        let dir = std::env::temp_dir().join(format!("winserver-minio-env-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create minio env dir");
        let env = dir.join("minio.env");
        fs::write(
            &env,
            "# comment\nMINIO_ROOT_USER=\"admin\"\nMINIO_ROOT_PASSWORD='secret'\nMINIO_API_PORT=9010\n",
        )
        .expect("write minio env");

        let parsed = parse_env_file(&env);
        assert_eq!(parsed.get("MINIO_ROOT_USER").map(String::as_str), Some("admin"));
        assert_eq!(parsed.get("MINIO_ROOT_PASSWORD").map(String::as_str), Some("secret"));
        assert_eq!(parsed.get("MINIO_API_PORT").map(String::as_str), Some("9010"));
    }

    #[test]
    fn service_config_path_prefers_db_path_then_existing_fallback() {
        let dir = std::env::temp_dir().join(format!("winserver-config-path-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create service dir");
        let fallback = dir.join("redis.conf");
        fs::write(&fallback, "port 6379\n").expect("write fallback");
        let explicit = dir.join("custom.conf");

        assert_eq!(
            service_config_path(&Some(explicit.to_string_lossy().to_string()), &dir.to_string_lossy(), "redis.conf"),
            Some(explicit)
        );
        assert_eq!(
            service_config_path(&None, &dir.to_string_lossy(), "redis.conf"),
            Some(fallback)
        );
    }

    #[tokio::test]
    async fn start_and_stop_service_verifies_pid_and_port() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping service start test because node is unavailable");
            return;
        };

        let db = make_db();
        let port = free_port();
        let (dir, script) = write_node_listener(port);
        let args = format!("\"{}\"", script.to_string_lossy());
        insert_test_service(&db, "test-listener", &node, &args, &dir, port);

        let manager = ProcessManager::new(db.clone());
        manager
            .start_service("test-listener")
            .await
            .expect("start service");

        let state = service_state(&db, "test-listener");
        assert_eq!(state.state, ServiceState::Running);
        assert!(state.pid.is_some(), "service should store a real PID");
        assert!(ProcessManager::is_port_listening(port), "service port should listen");

        manager
            .stop_service("test-listener")
            .await
            .expect("stop service");

        let state = service_state(&db, "test-listener");
        assert_eq!(state.state, ServiceState::Stopped);
        assert!(state.pid.is_none(), "stopped service should clear PID");
        assert!(
            !ProcessManager::is_port_listening(port),
            "service port should be released"
        );
    }

    #[tokio::test]
    async fn start_service_reports_port_conflict_before_spawning() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping port conflict test because node is unavailable");
            return;
        };

        let db = make_db();
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind occupied port");
        let port = listener.local_addr().expect("listener addr").port();
        let (dir, script) = write_node_listener(port);
        let args = format!("\"{}\"", script.to_string_lossy());
        insert_test_service(&db, "test-conflict", &node, &args, &dir, port);

        let manager = ProcessManager::new(db.clone());
        let error = manager
            .start_service("test-conflict")
            .await
            .expect_err("start should fail");

        assert!(error.to_string().contains("端口"));
        let state = service_state(&db, "test-conflict");
        assert_eq!(state.state, ServiceState::Failed);
        assert!(
            state
                .error_message
                .as_deref()
                .unwrap_or_default()
                .contains("端口")
        );
        drop(listener);
    }
}
