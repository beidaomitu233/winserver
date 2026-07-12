use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

use shared::types::{
    ConfigFileInfo, DatabaseInfo, RuntimeManifest, RuntimeType, ServiceInfo, ServiceState,
    SiteInfo, SoftwareInfo, SystemSettings,
};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &Path) -> anyhow::Result<Self> {
        // Create parent dir if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Open connection
        let conn = Connection::open(path)?;

        // Set pragmas
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn run_migrations(&self) -> anyhow::Result<()> {
        self.conn(|conn| {
            super::schema::run_migrations(conn)?;
            Ok(())
        })
    }

    pub fn conn<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&Connection) -> anyhow::Result<T>,
    {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("DB lock: {}", e))?;
        f(&conn)
    }

    pub fn list_service_instances(&self) -> anyhow::Result<Vec<ServiceInfo>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, service_type, desired_state, actual_state, port, auto, pid, error_message, config_file, installed
                 FROM service_instances
                 ORDER BY created_at DESC"
            )?;

            let rows = stmt.query_map([], |row| {
                let actual_state_str: String = row.get(4)?;
                let desired_state_str: String = row.get(3)?;

                Ok(ServiceInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    service_type: row.get(2)?,
                    desired_state: parse_service_state(&desired_state_str),
                    state: parse_service_state(&actual_state_str),
                    port: row.get::<_, i32>(5)? as u16,
                    auto: row.get::<_, i32>(6)? != 0,
                    pid: row.get::<_, Option<i32>>(7)?.map(|p| p as u32),
                    error_message: row.get(8)?,
                    config_file: row.get(9)?,
                    installed: row.get::<_, i32>(10)? != 0,
                })
            })?;

            let mut services = Vec::new();
            for row in rows {
                services.push(row?);
            }
            Ok(services)
        })
    }

    pub fn list_sites(&self) -> anyhow::Result<Vec<SiteInfo>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, COALESCE(NULLIF(name, ''), domain) AS display_name, domain, port, document_root, server_type, php_runtime_id, ssl, status
                 FROM sites
                 ORDER BY created_at DESC"
            )?;

            let rows = stmt.query_map([], |row| {
                let server_type_str: String = row.get(5)?;
                let server_type = match server_type_str.to_lowercase().as_str() {
                    "nginx" => shared::types::ServerType::Nginx,
                    "apache" => shared::types::ServerType::Apache,
                    _ => shared::types::ServerType::Nginx,
                };

                Ok(SiteInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    domain: row.get(2)?,
                    port: row.get::<_, i32>(3)? as u16,
                    document_root: row.get(4)?,
                    server_type,
                    php_runtime_id: row.get(6)?,
                    ssl: row.get::<_, i32>(7)? != 0,
                    status: row.get(8)?,
                })
            })?;

            let mut sites = Vec::new();
            for row in rows {
                sites.push(row?);
            }
            Ok(sites)
        })
    }

    pub fn list_logs(&self, limit: usize) -> anyhow::Result<Vec<String>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT action, target_id, success, error_code, message, created_at
                 FROM operation_logs
                 ORDER BY created_at DESC
                 LIMIT ?"
            )?;

            let rows = stmt.query_map([limit as i32], |row| {
                let action: String = row.get(0)?;
                let target_id: String = row.get(1)?;
                let success: bool = row.get::<_, i32>(2)? != 0;
                let error_code: Option<String> = row.get(3)?;
                let message: String = row.get(4)?;
                let created_at: String = row.get(5)?;

                let status = if success { "OK" } else { "FAIL" };
                let error_part = error_code
                    .map(|c| format!(" [{}]", c))
                    .unwrap_or_default();

                Ok(format!("[{}] {} {} on {}: {}{}", created_at, status, action, target_id, message, error_part))
            })?;

            let mut logs = Vec::new();
            for row in rows {
                logs.push(row?);
            }
            Ok(logs)
        })
    }

    pub fn clear_logs(&self) -> anyhow::Result<()> {
        self.conn(|conn| {
            conn.execute("DELETE FROM operation_logs", [])?;
            Ok(())
        })
    }

    pub fn get_settings(&self) -> anyhow::Result<SystemSettings> {
        self.conn(|conn| {
            let get_setting = |key: &str| -> anyhow::Result<String> {
                let result: Option<String> = conn
                    .query_row("SELECT value FROM settings WHERE key = ?", [key], |row| row.get(0))
                    .optional()?;
                Ok(result.unwrap_or_default())
            };

            let get_bool = |key: &str, default: bool| -> anyhow::Result<bool> {
                let val = get_setting(key)?;
                Ok(if val.is_empty() { default } else { val == "true" || val == "1" })
            };

            let get_u16 = |key: &str, default: u16| -> anyhow::Result<u16> {
                let val = get_setting(key)?;
                Ok(val.parse().unwrap_or(default))
            };

            Ok(SystemSettings {
                autostart: get_bool("autostart", false)?,
                start_suite_on_launch: get_bool("start_suite_on_launch", false)?,
                php_my_admin_url: get_setting("php_my_admin_url")?,
                port: get_u16("port", 8080)?,
                data_dir: get_setting("data_dir")?,
                config_path: get_setting("config_path")?,
            })
        })
    }

    pub fn update_settings(&self, params: &serde_json::Value) -> anyhow::Result<()> {
        self.conn(|conn| {
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    let value_str = match value {
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::String(s) => s.clone(),
                        _ => value.to_string(),
                    };

                    conn.execute(
                        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                        [key, &value_str],
                    )?;
                }
            }
            Ok(())
        })
    }

    pub fn upsert_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                [key, value],
            )?;
            Ok(())
        })
    }

    pub fn update_service_path(&self, service_id: &str, install_dir: &str) -> anyhow::Result<()> {
        // Update the service's cwd and exe based on the new install directory
        let dir = install_dir.replace('\\', "/");
        tracing::info!("update_service_path: service_id={}, dir={}", service_id, dir);
        let service = self.get_service_config(service_id).ok();
        if let Some(_svc) = service {
            let exe = match service_id {
                "nginx" => format!("{}/nginx.exe", dir),
                "apache" => format!("{}/bin/httpd.exe", dir),
                "mysql57" | "mysql80" => format!("{}/bin/mysqld.exe", dir),
                "php73" => format!("{}/php-cgi.exe", dir),
                "redis" => format!("{}/redis-server.exe", dir),
                "minio" => format!("{}/minio.exe", dir),
                "pgsql" => format!("{}/bin/pg_ctl.exe", dir),
                _ => {
                    tracing::warn!("update_service_path: unknown service_id={}, skipping", service_id);
                    return Ok(());
                }
            };
            let config_file = match service_id {
                "nginx" => format!("{}/conf/nginx.conf", dir),
                "apache" => format!("{}/conf/httpd.conf", dir),
                "mysql57" | "mysql80" => format!("{}/my.ini", dir),
                "php73" => format!("{}/php.ini", dir),
                "redis" => format!("{}/redis.conf", dir),
                "minio" => format!("{}/minio.env", dir),
                "pgsql" => format!("{}/data/postgresql.conf", dir),
                _ => return Ok(()),
            };
            let installed = std::path::Path::new(&exe).exists();
            tracing::info!("update_service_path: exe={}, exists={}, installed={}", exe, std::path::Path::new(&exe).exists(), installed);
            self.conn(|conn| {
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE service_instances SET exe = ?1, cwd = ?2, config_file = ?3, installed = ?4, updated_at = ?5 WHERE id = ?6",
                    rusqlite::params![exe, dir, config_file, installed as i32, &now, service_id],
                )?;
                Ok(())
            })?;
        } else {
            tracing::warn!("update_service_path: service_id={} not found in DB, cannot update path", service_id);
        }
        Ok(())
    }

    pub fn insert_service_instance(&self, svc: &ServiceInfo) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR REPLACE INTO service_instances
                 (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed, desired_state, actual_state, pid, error_message, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                rusqlite::params![
                    svc.id,
                    svc.name,
                    svc.service_type,
                    "", // process_name
                    svc.port as i32,
                    "", // exe
                    "", // args
                    "", // cwd
                    svc.config_file,
                    svc.auto as i32,
                    svc.installed as i32,
                    svc.desired_state.to_string(),
                    svc.state.to_string(),
                    svc.pid.map(|p| p as i32),
                    svc.error_message,
                    &now,
                    &now,
                ],
            )?;
            Ok(())
        })
    }

    pub fn insert_site(&self, site: &SiteInfo) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR REPLACE INTO sites
                 (id, name, domain, port, document_root, server_type, php_runtime_id, ssl, status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    site.id,
                    if site.name.trim().is_empty() { &site.domain } else { &site.name },
                    site.domain,
                    site.port as i32,
                    site.document_root,
                    site.server_type.to_string(),
                    site.php_runtime_id,
                    site.ssl as i32,
                    site.status,
                    &now,
                    &now,
                ],
            )?;
            Ok(())
        })
    }

    pub fn delete_site(&self, id: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            conn.execute("DELETE FROM sites WHERE id = ?", [id])?;
            Ok(())
        })
    }

    pub fn update_site_php_runtime(&self, id: &str, php_runtime_id: &str, status: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE sites SET php_runtime_id = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
                rusqlite::params![php_runtime_id, status, &now, id],
            )?;
            Ok(())
        })
    }

    pub fn update_site_status(&self, id: &str, status: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE sites SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status, &now, id],
            )?;
            Ok(())
        })
    }

    pub fn update_site(&self, id: &str, domain: &str, port: u16, document_root: &str, server_type: &str, status: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE sites SET name = ?1, domain = ?2, port = ?3, document_root = ?4, server_type = ?5, status = ?6, updated_at = ?7 WHERE id = ?8",
                rusqlite::params![domain, domain, port, document_root, server_type, status, &now, id],
            )?;
            Ok(())
        })
    }

    pub fn add_log(&self, action: &str, target_id: &str, success: bool, message: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO operation_logs (action, target_id, success, error_code, message, created_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
                rusqlite::params![
                    action,
                    target_id,
                    success as i32,
                    message,
                    &now,
                ],
            )?;
            Ok(())
        })
    }

    /// Get service config (exe, args, cwd, config_file, env, process_name, pid) by service ID.
    pub fn get_service_config(&self, service_id: &str) -> anyhow::Result<ServiceConfig> {
        self.conn(|conn| {
            let result = conn.query_row(
                "SELECT id, exe, args, cwd, config_file, process_name, pid, port, installed FROM service_instances WHERE id = ?",
                [service_id],
                |row| {
                    Ok(ServiceConfig {
                        id: row.get(0)?,
                        exe: row.get(1)?,
                        args: row.get::<_, Option<String>>(2)?,
                        cwd: row.get::<_, Option<String>>(3)?,
                        config_file: row.get::<_, Option<String>>(4)?,
                        env: None,
                        process_name: row.get(5)?,
                        pid: row.get::<_, Option<i32>>(6)?.map(|p| p as u32),
                        port: row.get::<_, i32>(7)? as u16,
                        installed: row.get::<_, i32>(8)? != 0,
                    })
                },
            ).optional()?;

            match result {
                Some(config) => Ok(config),
                None => anyhow::bail!("Service not found: {}", service_id),
            }
        })
    }

    /// Update the state and optionally the PID of a service instance.
    pub fn update_service_state(
        &self,
        service_id: &str,
        state: ServiceState,
        pid: Option<u32>,
    ) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(pid) = pid {
                conn.execute(
                    "UPDATE service_instances SET actual_state = ?1, pid = ?2, error_message = NULL, updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![state.to_string(), pid as i32, &now, service_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE service_instances SET actual_state = ?1, pid = NULL, error_message = NULL, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![state.to_string(), &now, service_id],
                )?;
            }
            Ok(())
        })
    }

    /// Update a service's listening port (e.g. after editing redis/minio config).
    pub fn update_service_port(&self, service_id: &str, port: u16) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE service_instances SET port = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![port as i32, &now, service_id],
            )?;
            Ok(())
        })
    }

    /// Mark a service as failed and keep a user-facing error message.
    pub fn update_service_failure(
        &self,
        service_id: &str,
        error_message: &str,
    ) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE service_instances SET actual_state = 'failed', pid = NULL, error_message = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![error_message, &now, service_id],
            )?;
            Ok(())
        })
    }

    /// Get a single site by its ID.
    pub fn get_site(&self, site_id: &str) -> anyhow::Result<SiteInfo> {
        self.conn(|conn| {
            let result = conn.query_row(
                "SELECT id, COALESCE(NULLIF(name, ''), domain) AS display_name, domain, port, document_root, server_type, php_runtime_id, ssl, status
                 FROM sites WHERE id = ?",
                [site_id],
                |row| {
                    let server_type_str: String = row.get(5)?;
                    let server_type = match server_type_str.to_lowercase().as_str() {
                        "nginx" => shared::types::ServerType::Nginx,
                        "apache" => shared::types::ServerType::Apache,
                        _ => shared::types::ServerType::Nginx,
                    };

                    Ok(SiteInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        domain: row.get(2)?,
                        port: row.get::<_, i32>(3)? as u16,
                        document_root: row.get(4)?,
                        server_type,
                        php_runtime_id: row.get(6)?,
                        ssl: row.get::<_, i32>(7)? != 0,
                        status: row.get(8)?,
                    })
                },
            ).optional()?;

            match result {
                Some(site) => Ok(site),
                None => anyhow::bail!("Site not found: {}", site_id),
            }
        })
    }

    // ---- Config files ----

    /// List all config files.
    pub fn list_config_files(&self) -> anyhow::Result<Vec<ConfigFileInfo>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, label, path FROM config_files ORDER BY created_at ASC"
            )?;

            let rows = stmt.query_map([], |row| {
                let path: String = row.get(2)?;
                let exists = Path::new(&path).exists();
                Ok(ConfigFileInfo {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    path,
                    exists,
                })
            })?;

            let mut files = Vec::new();
            for row in rows {
                files.push(row?);
            }
            Ok(files)
        })
    }

    /// Get the file path for a config file by ID.
    pub fn get_config_file_path(&self, id: &str) -> anyhow::Result<Option<String>> {
        self.conn(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT path FROM config_files WHERE id = ?",
                    [id],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(result)
        })
    }

    // ---- Databases ----

    /// Insert a new database record scoped to a MySQL service instance.
    pub fn insert_database(
        &self,
        name: &str,
        user: &str,
        password: &str,
        mysql_service_id: &str,
    ) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            let engine = if mysql_service_id.is_empty() {
                "mysql".to_string()
            } else {
                mysql_service_id.to_string()
            };
            conn.execute(
                "INSERT OR REPLACE INTO databases
                    (name, user, password, engine, size, status, mysql_service_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, '', 'active', ?5, ?6, ?7)",
                rusqlite::params![name, user, password, engine, mysql_service_id, &now, &now],
            )?;
            Ok(())
        })
    }

    /// Delete a database record by name (optionally scoped to a MySQL service).
    pub fn delete_database(&self, name: &str, mysql_service_id: Option<&str>) -> anyhow::Result<()> {
        self.conn(|conn| {
            if let Some(service_id) = mysql_service_id {
                conn.execute(
                    "DELETE FROM databases WHERE name = ?1 AND mysql_service_id = ?2",
                    rusqlite::params![name, service_id],
                )?;
            } else {
                conn.execute("DELETE FROM databases WHERE name = ?", [name])?;
            }
            Ok(())
        })
    }

    /// List database records, optionally filtered by active MySQL service id.
    pub fn list_databases(&self, mysql_service_id: Option<&str>) -> anyhow::Result<Vec<DatabaseInfo>> {
        self.conn(|conn| {
            let mut databases = Vec::new();
            if let Some(service_id) = mysql_service_id {
                let mut stmt = conn.prepare(
                    "SELECT name, user, password, engine, size, status, mysql_service_id
                     FROM databases
                     WHERE mysql_service_id = ?1
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map([service_id], |row| {
                    Ok(DatabaseInfo {
                        name: row.get(0)?,
                        user: row.get(1)?,
                        password: row.get(2)?,
                        engine: row.get(3)?,
                        size: row.get(4)?,
                        status: row.get(5)?,
                        mysql_service_id: row.get(6)?,
                    })
                })?;
                for row in rows {
                    databases.push(row?);
                }
            } else {
                let mut stmt = conn.prepare(
                    "SELECT name, user, password, engine, size, status, mysql_service_id
                     FROM databases
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok(DatabaseInfo {
                        name: row.get(0)?,
                        user: row.get(1)?,
                        password: row.get(2)?,
                        engine: row.get(3)?,
                        size: row.get(4)?,
                        status: row.get(5)?,
                        mysql_service_id: row.get(6)?,
                    })
                })?;
                for row in rows {
                    databases.push(row?);
                }
            }
            Ok(databases)
        })
    }

    /// Get database password by name (optionally scoped to a MySQL service).
    pub fn get_database_password(
        &self,
        name: &str,
        mysql_service_id: Option<&str>,
    ) -> anyhow::Result<Option<String>> {
        self.conn(|conn| {
            let result: Option<String> = if let Some(service_id) = mysql_service_id {
                conn.query_row(
                    "SELECT password FROM databases WHERE name = ?1 AND mysql_service_id = ?2",
                    rusqlite::params![name, service_id],
                    |row| row.get(0),
                )
                .optional()?
            } else {
                conn.query_row(
                    "SELECT password FROM databases WHERE name = ? LIMIT 1",
                    [name],
                    |row| row.get(0),
                )
                .optional()?
            };
            Ok(result)
        })
    }

    /// Update database user password.
    pub fn update_database_password(
        &self,
        name: &str,
        new_pass: &str,
        mysql_service_id: Option<&str>,
    ) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(service_id) = mysql_service_id {
                conn.execute(
                    "UPDATE databases SET password = ?1, updated_at = ?2
                     WHERE name = ?3 AND mysql_service_id = ?4",
                    rusqlite::params![new_pass, &now, name, service_id],
                )?;
            } else {
                conn.execute(
                    "UPDATE databases SET password = ?1, updated_at = ?2 WHERE name = ?3",
                    rusqlite::params![new_pass, &now, name],
                )?;
            }
            Ok(())
        })
    }

    // ---- Service config by type ----

    /// Get service config by service_type (e.g. "mysql80", "mysql57").
    /// Looks for a service whose service_type matches. Note: `exe` is the
    /// server binary (mysqld.exe); callers that need the SQL client must resolve
    /// `bin/mysql.exe` beside it.
    pub fn get_service_config_by_type(&self, service_type: &str) -> anyhow::Result<ServiceConfig> {
        self.conn(|conn| {
            let result = conn.query_row(
                "SELECT id, exe, args, cwd, config_file, process_name, pid, port, installed FROM service_instances WHERE service_type = ? LIMIT 1",
                [service_type],
                |row| {
                    Ok(ServiceConfig {
                        id: row.get(0)?,
                        exe: row.get(1)?,
                        args: row.get::<_, Option<String>>(2)?,
                        cwd: row.get::<_, Option<String>>(3)?,
                        config_file: row.get::<_, Option<String>>(4)?,
                        env: None,
                        process_name: row.get(5)?,
                        pid: row.get::<_, Option<i32>>(6)?.map(|p| p as u32),
                        port: row.get::<_, i32>(7)? as u16,
                        installed: row.get::<_, i32>(8)? != 0,
                    })
                },
            ).optional()?;

            match result {
                Some(config) => Ok(config),
                None => anyhow::bail!("Service not found for type: {}", service_type),
            }
        })
    }

    // ---- Service auto toggle ----

    /// Update the auto flag for a service instance.
    pub fn update_service_auto(&self, id: &str, auto: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE service_instances SET auto = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![auto as i32, &now, id],
            )?;
            Ok(())
        })
    }

    /// Upsert a concrete service runtime configuration created by local import.
    pub fn upsert_service_runtime(&self, runtime: ServiceRuntimeRegistration<'_>) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO service_instances
                 (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed, desired_state, actual_state, pid, error_message, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, 1, 'installed', 'stopped', NULL, NULL, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    service_type = excluded.service_type,
                    process_name = excluded.process_name,
                    port = excluded.port,
                    exe = excluded.exe,
                    args = excluded.args,
                    cwd = excluded.cwd,
                    config_file = excluded.config_file,
                    installed = 1,
                    actual_state = CASE
                        WHEN service_instances.actual_state = 'running' THEN service_instances.actual_state
                        ELSE 'stopped'
                    END,
                    error_message = NULL,
                    updated_at = excluded.updated_at",
                rusqlite::params![
                    runtime.id,
                    runtime.name,
                    runtime.service_type,
                    runtime.process_name,
                    runtime.port as i32,
                    runtime.exe,
                    runtime.args,
                    runtime.cwd,
                    runtime.config_file,
                    &now,
                    &now,
                ],
            )?;
            Ok(())
        })
    }

    pub fn upsert_config_file(&self, id: &str, label: &str, path: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO config_files (id, label, path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET label = excluded.label, path = excluded.path, updated_at = excluded.updated_at",
                rusqlite::params![id, label, path, &now, &now],
            )?;
            Ok(())
        })
    }

    pub fn upsert_runtime(&self, manifest: &RuntimeManifest) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO runtimes
                 (id, runtime_type, version, install_path, entrypoint, config_template, installed, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    runtime_type = excluded.runtime_type,
                    version = excluded.version,
                    install_path = excluded.install_path,
                    entrypoint = excluded.entrypoint,
                    config_template = excluded.config_template,
                    installed = excluded.installed",
                rusqlite::params![
                    manifest.id,
                    runtime_type_to_string(manifest.runtime_type),
                    manifest.version,
                    manifest.install_path,
                    manifest.entrypoint,
                    manifest.config_template,
                    manifest.installed as i32,
                    &now,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_runtimes(&self) -> anyhow::Result<Vec<RuntimeManifest>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, runtime_type, version, install_path, entrypoint, config_template, installed
                 FROM runtimes
                 ORDER BY runtime_type ASC, version ASC",
            )?;
            let rows = stmt.query_map([], |row| {
                let runtime_type: String = row.get(1)?;
                Ok(RuntimeManifest {
                    id: row.get(0)?,
                    runtime_type: parse_runtime_type(&runtime_type),
                    version: row.get(2)?,
                    install_path: row.get(3)?,
                    entrypoint: row.get(4)?,
                    config_template: row.get(5)?,
                    installed: row.get::<_, i32>(6)? != 0,
                })
            })?;

            let mut runtimes = Vec::new();
            for row in rows {
                runtimes.push(row?);
            }
            Ok(runtimes)
        })
    }

    // ---- Software ----

    /// Get a single software record by ID.
    pub fn get_software(&self, id: &str) -> anyhow::Result<SoftwareInfo> {
        self.conn(|conn| {
            let result = conn.query_row(
                "SELECT id, name, category, service_id, installed, installable, install_note, download_url, install_type, install_path, status, COALESCE(has_local,0), COALESCE(has_bundled,0)
                 FROM software WHERE id = ?",
                [id],
                |row| {
                    Ok(SoftwareInfo {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        category: row.get(2)?,
                        service_id: row.get(3)?,
                        installed: row.get::<_, i32>(4)? != 0,
                        installable: row.get::<_, i32>(5)? != 0,
                        install_note: row.get(6)?,
                        download_url: row.get(7)?,
                        install_type: row.get::<_, Option<String>>(8)?.unwrap_or_else(|| "local".to_string()),
                        install_path: row.get(9)?,
                        status: row.get::<_, Option<String>>(10)?.unwrap_or_else(|| "available".to_string()),
                        has_local: row.get::<_, i32>(11)? != 0,
                        has_bundled: row.get::<_, i32>(12)? != 0,
                    })
                },
            ).optional()?;

            match result {
                Some(software) => Ok(software),
                None => anyhow::bail!("Software not found: {}", id),
            }
        })
    }

    /// List all software records.
    pub fn list_software(&self) -> anyhow::Result<Vec<SoftwareInfo>> {
        self.conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, category, service_id, installed, installable, install_note, download_url, install_type, install_path, status, COALESCE(has_local,0), COALESCE(has_bundled,0)
                 FROM software ORDER BY created_at ASC"
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(SoftwareInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    service_id: row.get(3)?,
                    installed: row.get::<_, i32>(4)? != 0,
                    installable: row.get::<_, i32>(5)? != 0,
                    install_note: row.get(6)?,
                    download_url: row.get(7)?,
                    install_type: row.get::<_, Option<String>>(8)?.unwrap_or_else(|| "local".to_string()),
                    install_path: row.get(9)?,
                    status: row.get::<_, Option<String>>(10)?.unwrap_or_else(|| "available".to_string()),
                    has_local: row.get::<_, i32>(11)? != 0,
                    has_bundled: row.get::<_, i32>(12)? != 0,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
    }

    /// Update software installed status.
    pub fn update_software_installed(&self, id: &str, installed: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET installed = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![installed as i32, &now, id],
            )?;
            Ok(())
        })
    }

    /// Update software status (available/downloading/installing/installed/failed).
    pub fn update_software_status(&self, id: &str, status: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET status = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![status, &now, id],
            )?;
            Ok(())
        })
    }

    /// Update software install info after a download install completes.
    pub fn update_software_download_installed(&self, id: &str, install_path: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET installed = 1, install_type = 'download', install_path = ?1, status = 'installed', updated_at = ?2 WHERE id = ?3",
                rusqlite::params![install_path, &now, id],
            )?;
            Ok(())
        })
    }

    /// Update software install info after a bundled install completes.
    pub fn update_software_bundled_installed(&self, id: &str, install_path: &str) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET installed = 1, install_type = 'bundled', install_path = ?1, status = 'installed', updated_at = ?2 WHERE id = ?3",
                rusqlite::params![install_path, &now, id],
            )?;
            Ok(())
        })
    }

    /// Update software has_local flag.
    pub fn update_software_has_local(&self, id: &str, has_local: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET has_local = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![has_local as i32, &now, id],
            )?;
            Ok(())
        })
    }

    /// Update software has_bundled flag.
    pub fn update_software_has_bundled(&self, id: &str, has_bundled: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE software SET has_bundled = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![has_bundled as i32, &now, id],
            )?;
            Ok(())
        })
    }

    pub fn update_service_installed(&self, id: &str, installed: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE service_instances
                 SET installed = ?1,
                     actual_state = CASE WHEN ?1 = 0 THEN 'stopped' ELSE actual_state END,
                     pid = CASE WHEN ?1 = 0 THEN NULL ELSE pid END,
                     updated_at = ?2
                 WHERE id = ?3",
                rusqlite::params![installed as i32, &now, id],
            )?;
            Ok(())
        })
    }

    pub fn update_runtimes_installed_by_service(&self, service_id: &str, installed: bool) -> anyhow::Result<()> {
        self.conn(|conn| {
            if service_id == "nginx" {
                conn.execute(
                    "UPDATE runtimes SET installed = ?1 WHERE runtime_type = 'nginx'",
                    rusqlite::params![installed as i32],
                )?;
            } else {
                conn.execute(
                    "UPDATE runtimes SET installed = ?1 WHERE id = ?2",
                    rusqlite::params![installed as i32, service_id],
                )?;
            }
            Ok(())
        })
    }

    /// Get software download_url and install_dir by ID.
    pub fn get_software_install_info(&self, id: &str) -> anyhow::Result<(Option<String>, Option<String>)> {
        self.conn(|conn| {
            let result = conn.query_row(
                "SELECT download_url, install_dir FROM software WHERE id = ?",
                [id],
                |row| {
                    let url: Option<String> = row.get(0)?;
                    let dir: Option<String> = row.get(1)?;
                    Ok((url, dir))
                },
            ).optional()?;

            match result {
                Some(pair) => Ok(pair),
                None => anyhow::bail!("Software not found: {}", id),
            }
        })
    }

    /// Import data from the legacy config.json file into the SQLite database.
    /// This is called once during migration; a marker setting prevents re-import.
    pub fn import_from_config_json(&self, config_path: &Path) -> anyhow::Result<()> {
        // Check if we already migrated
        let already_migrated: bool = self.conn(|conn| {
            let val: Option<String> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'config_json_migrated'",
                    [],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(val.unwrap_or_default() == "true")
        })?;

        if already_migrated {
            return Ok(());
        }

        if !config_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(config_path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;

        let now = chrono::Utc::now().to_rfc3339();

        self.conn(|conn| {
            // --- Import services ---
            if let Some(services) = config.get("services").and_then(|v| v.as_array()) {
                for svc in services {
                    let id = svc["id"].as_str().unwrap_or("");
                    if id.is_empty() {
                        continue;
                    }

                    let name = svc["name"].as_str().unwrap_or(id);
                    let service_type = id; // use id as service_type
                    let process_name = svc["processName"].as_str().unwrap_or("");
                    let port = svc["port"].as_i64().unwrap_or(0) as i32;
                    let exe = svc["exe"].as_str().unwrap_or("");
                    let args = if let Some(arr) = svc["args"].as_array() {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(" ")
                    } else {
                        svc["args"].as_str().unwrap_or("").to_string()
                    };
                    let cwd = svc["cwd"].as_str().unwrap_or("");
                    let config_file = svc["configFile"].as_str().unwrap_or("");
                    let auto = svc["auto"].as_bool().unwrap_or(false) as i32;

                    let _env_json = svc.get("env").map(|e| e.to_string());

                    // Check if service already exists (seed_default_data may have inserted it)
                    let exists: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM service_instances WHERE id = ?",
                            [id],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|c| c > 0)?;

                    if exists {
                        // Update with the richer data from config.json
                        conn.execute(
                            "UPDATE service_instances SET name=?1, service_type=?2, process_name=?3, port=?4, exe=?5, args=?6, cwd=?7, config_file=?8, auto=?9, updated_at=?10 WHERE id=?11",
                            rusqlite::params![name, service_type, process_name, port, exe, &args, cwd, config_file, auto, &now, id],
                        )?;
                    } else {
                        conn.execute(
                            "INSERT INTO service_instances
                             (id, name, service_type, process_name, port, exe, args, cwd, config_file, auto, installed, desired_state, actual_state, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 'installed', 'unknown', ?11, ?12)",
                            rusqlite::params![id, name, service_type, process_name, port, exe, &args, cwd, config_file, auto, &now, &now],
                        )?;
                    }
                }
            }

            // --- Import sites ---
            if let Some(sites) = config.get("sites").and_then(|v| v.as_array()) {
                for site in sites {
                    let domain = site["domain"].as_str().unwrap_or("");
                    if domain.is_empty() {
                        continue;
                    }
                    let port = site["port"].as_str()
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(80);
                    let document_root = site["path"].as_str().unwrap_or("");

                    // Skip if site already exists
                    let exists: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM sites WHERE domain = ? AND port = ?",
                            rusqlite::params![domain, port],
                            |row| row.get::<_, i64>(0),
                        )
                        .map(|c| c > 0)?;

                    if !exists {
                        conn.execute(
                            "INSERT INTO sites (id, name, domain, port, document_root, server_type, php_runtime_id, ssl, status, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, 'nginx', '', 0, 'active', ?6, ?7)",
                            rusqlite::params![domain, domain, domain, port, document_root, &now, &now],
                        )?;
                    }
                }
            }

            // --- Import databases ---
            if let Some(databases) = config.get("databases").and_then(|v| v.as_array()) {
                for db in databases {
                    let name = db["db"].as_str().unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let user = db["user"].as_str().unwrap_or("root");
                    let pass = db["pass"].as_str().unwrap_or("");

                    conn.execute(
                        "INSERT OR IGNORE INTO databases
                            (name, user, password, engine, size, status, mysql_service_id, created_at, updated_at)
                         VALUES (?1, ?2, ?3, 'mysql80', '', 'active', 'mysql80', ?4, ?5)",
                        rusqlite::params![name, user, pass, &now, &now],
                    )?;
                }
            }

            // --- Import settings ---
            if let Some(sys_settings) = config.get("systemSettings") {
                if let Some(v) = sys_settings.get("autostart") {
                    let val = v.as_bool().unwrap_or(false).to_string();
                    conn.execute(
                        "INSERT OR REPLACE INTO settings (key, value) VALUES ('autostart', ?1)",
                        rusqlite::params![val],
                    )?;
                }
                if let Some(v) = sys_settings.get("startSuiteOnLaunch") {
                    let val = v.as_bool().unwrap_or(false).to_string();
                    conn.execute(
                        "INSERT OR REPLACE INTO settings (key, value) VALUES ('start_suite_on_launch', ?1)",
                        rusqlite::params![val],
                    )?;
                }
                if let Some(v) = sys_settings.get("phpMyAdminUrl") {
                    let val = v.as_str().unwrap_or("");
                    conn.execute(
                        "INSERT OR REPLACE INTO settings (key, value) VALUES ('php_my_admin_url', ?1)",
                        rusqlite::params![val],
                    )?;
                }
            }

            // Import top-level port
            if let Some(v) = config.get("port") {
                let val = v.as_i64().unwrap_or(18113).to_string();
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES ('port', ?1)",
                    rusqlite::params![val],
                )?;
            }

            // Import mysqlRootPassword
            if let Some(v) = config.get("mysqlRootPassword") {
                let val = v.as_str().unwrap_or("");
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES ('mysql_root_password', ?1)",
                    rusqlite::params![val],
                )?;
            }

            // --- Import config files ---
            if let Some(config_files) = config.get("configFiles").and_then(|v| v.as_array()) {
                for cf in config_files {
                    let id = cf["id"].as_str().unwrap_or("");
                    let label = cf["label"].as_str().unwrap_or("");
                    let path = cf["path"].as_str().unwrap_or("");

                    if id.is_empty() {
                        continue;
                    }

                    conn.execute(
                        "INSERT OR REPLACE INTO config_files (id, label, path, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![id, label, path, &now, &now],
                    )?;
                }
            }

            // --- Import software ---
            if let Some(software) = config.get("software").and_then(|v| v.as_array()) {
                for sw in software {
                    let id = sw["id"].as_str().unwrap_or("");
                    if id.is_empty() {
                        continue;
                    }
                    let name = sw["name"].as_str().unwrap_or("");
                    let category = sw["category"].as_str().unwrap_or("");
                    let service_id = sw["serviceId"].as_str().unwrap_or("");
                    let download_url = sw["downloadUrl"].as_str().unwrap_or("");
                    let install_dir = sw["installDir"].as_str().unwrap_or("");
                    let install_note = sw["installNote"].as_str().unwrap_or("");

                    conn.execute(
                        "INSERT OR REPLACE INTO software (id, name, category, service_id, download_url, install_dir, installed, installable, install_note, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 1, ?7, ?8, ?9)",
                        rusqlite::params![id, name, category, service_id, download_url, install_dir, install_note, &now, &now],
                    )?;
                }
            }

            // Mark migration as done
            conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES ('config_json_migrated', 'true')",
                [],
            )?;

            Ok(())
        })
    }

    /// Get MySQL root password from settings.
    pub fn get_mysql_root_password(&self) -> anyhow::Result<Option<String>> {
        self.conn(|conn| {
            let result: Option<String> = conn
                .query_row("SELECT value FROM settings WHERE key = 'mysql_root_password'", [], |row| row.get(0))
                .optional()?;
            Ok(result)
        })
    }
}

/// Service configuration retrieved from the database.
pub struct ServiceRuntimeRegistration<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub service_type: &'a str,
    pub process_name: &'a str,
    pub port: u16,
    pub exe: &'a str,
    pub args: &'a str,
    pub cwd: &'a str,
    pub config_file: Option<&'a str>,
}

pub struct ServiceConfig {
    pub id: String,
    pub exe: String,
    pub args: Option<String>,
    pub cwd: Option<String>,
    pub config_file: Option<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
    pub port: u16,
    pub installed: bool,
}

fn parse_service_state(s: &str) -> ServiceState {
    match s.to_lowercase().as_str() {
        "installed" => ServiceState::Installed,
        "starting" => ServiceState::Starting,
        "running" => ServiceState::Running,
        "degraded" => ServiceState::Degraded,
        "stopping" => ServiceState::Stopping,
        "stopped" => ServiceState::Stopped,
        "failed" => ServiceState::Failed,
        _ => ServiceState::Unknown,
    }
}

fn parse_runtime_type(s: &str) -> RuntimeType {
    match s.to_lowercase().as_str() {
        "nginx" => RuntimeType::Nginx,
        "apache" => RuntimeType::Apache,
        "php" => RuntimeType::Php,
        "mysql" => RuntimeType::Mysql,
        "redis" => RuntimeType::Redis,
        _ => RuntimeType::Php,
    }
}

fn runtime_type_to_string(runtime_type: RuntimeType) -> &'static str {
    match runtime_type {
        RuntimeType::Nginx => "nginx",
        RuntimeType::Apache => "apache",
        RuntimeType::Php => "php",
        RuntimeType::Mysql => "mysql",
        RuntimeType::Redis => "redis",
    }
}
