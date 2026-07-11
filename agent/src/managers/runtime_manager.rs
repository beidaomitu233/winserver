use std::fs;
use std::io::{Read as IoRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};
use tracing::{info, warn, error};
use zip::ZipArchive;

use shared::types::{RuntimeManifest, RuntimeType};
use crate::database::Database;

/// Scans the data directory for installed runtime versions (PHP, Nginx, etc.).
pub struct RuntimeManager {
    data_dir: PathBuf,
    db: Arc<Database>,
}

impl RuntimeManager {
    pub fn new(data_dir: PathBuf, db: Arc<Database>) -> Self {
        Self { data_dir, db }
    }

    fn update_bundled_companion_software(&self, software_id: &str, target_dir: &Path, install_path: &str) {
        if software_id == "minio" && target_dir.join("mc.exe").exists() {
            if let Err(error) = self.db.update_software_bundled_installed("mc", install_path) {
                warn!("failed to mark bundled MinIO client as installed: {}", error);
            }
        }
    }

    /// List all installed runtimes by scanning the runtimes directory.
    pub fn list_runtimes(&self) -> Result<Vec<RuntimeManifest>> {
        let imported = self.db.list_runtimes()?;
        if !imported.is_empty() {
            return Ok(imported);
        }

        let runtimes_dir = self.data_dir.join("runtimes");

        if !runtimes_dir.exists() {
            info!("Runtimes directory not found: {:?}", runtimes_dir);
            return Ok(vec![]);
        }

        let mut manifests = Vec::new();

        // Scan for PHP installations
        if let Ok(entries) = fs::read_dir(&runtimes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let dir_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let lower_name = dir_name.to_lowercase();

                if lower_name.starts_with("php") {
                    if let Some(manifest) = self.scan_php_runtime(&path, &dir_name) {
                        manifests.push(manifest);
                    }
                } else if lower_name.starts_with("nginx") {
                    if let Some(manifest) = self.scan_nginx_runtime(&path, &dir_name) {
                        manifests.push(manifest);
                    }
                } else if lower_name.starts_with("apache") || lower_name.contains("httpd") {
                    if let Some(manifest) = self.scan_apache_runtime(&path, &dir_name) {
                        manifests.push(manifest);
                    }
                } else if lower_name.starts_with("mysql") {
                    if let Some(manifest) = self.scan_mysql_runtime(&path, &dir_name) {
                        manifests.push(manifest);
                    }
                } else if lower_name.starts_with("redis") {
                    if let Some(manifest) = self.scan_redis_runtime(&path, &dir_name) {
                        manifests.push(manifest);
                    }
                }
            }
        }

        info!("Found {} runtime(s)", manifests.len());
        Ok(manifests)
    }

    /// Import a local Nginx or PHP runtime directory and make it available as a service.
    pub fn import_runtime(
        &self,
        runtime_type: RuntimeType,
        install_path: &str,
        port: Option<u16>,
    ) -> Result<RuntimeManifest> {
        let root = PathBuf::from(install_path);
        if !root.is_dir() {
            anyhow::bail!("运行环境目录不存在：{}", install_path);
        }

        let manifest = match runtime_type {
            RuntimeType::Nginx => self.import_nginx(&root, port.unwrap_or(80))?,
            RuntimeType::Php => self.import_php(&root, port)?,
            _ => anyhow::bail!("第一阶段仅支持导入 Nginx 和 PHP"),
        };

        self.db.upsert_runtime(&manifest)?;
        info!("Imported runtime: {} {}", manifest.id, manifest.version);
        Ok(manifest)
    }

    /// Install a bundled runtime from the runtime/ directory.
    /// runtime_dir is the directory containing bundled zip/exe files (e.g. {app_dir}/runtime/).
    pub fn install_bundled_runtime(
        &self,
        software_id: &str,
        runtime_dir: &Path,
    ) -> Result<RuntimeManifest> {
        let sw = self.db.get_software(software_id)?;
        let service_id = &sw.service_id;
        let target_dir = self.data_dir.join("server").join(service_id);

        info!("install_bundled_runtime: software_id={}, service_id={}, runtime_dir={}", software_id, service_id, runtime_dir.display());

        // If the target already exists AND the service is already registered as
        // installed, this is a no-op re-run (e.g. app relaunch). Skip all DB
        // writes and logging so we don't spam operation_logs on every startup.
        if target_dir.exists() {
            let already_installed = self.db.get_software(software_id).map(|sw| sw.installed).unwrap_or(false);
            if already_installed {
                info!("install_bundled_runtime: target_dir exists and software already installed, skipping (no-op)");
                normalize_service_root(&target_dir, service_id)?;
                let install_path_str = target_dir.to_string_lossy().to_string();
                match service_id.as_str() {
                    "nginx" => {
                        self.import_runtime(RuntimeType::Nginx, &install_path_str, None)?;
                    }
                    "php" | "php73" => {
                        self.import_runtime(RuntimeType::Php, &install_path_str, None)?;
                    }
                    s if s.starts_with("php") => {
                        self.import_runtime(RuntimeType::Php, &install_path_str, None)?;
                    }
                    _ => {
                        if let Err(error) = self.register_generic_service(software_id, service_id, &install_path_str) {
                            self.db.update_service_installed(service_id, false).ok();
                            self.db.update_software_installed(software_id, false).ok();
                            self.db.update_software_status(software_id, "failed").ok();
                            return Err(error.context("已安装服务文件校验失败"));
                        }
                        self.update_bundled_companion_software(software_id, &target_dir, &install_path_str);
                    }
                }
                return Ok(RuntimeManifest {
                    id: software_id.to_string(),
                    runtime_type: RuntimeType::Nginx,
                    version: "unknown".to_string(),
                    install_path: install_path_str,
                    entrypoint: String::new(),
                    config_template: None,
                    installed: true,
                });
            }
        }

        if target_dir.exists() {
            info!("install_bundled_runtime: target_dir already exists, skipping extraction");
        } else {
            fs::create_dir_all(&target_dir)?;

            // Find the bundled file for this software
            let bundled_file = self.find_bundled_file(software_id, runtime_dir);
            match bundled_file {
                Some(path) => {
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    info!("install_bundled_runtime: found bundled entry: {}", file_name);

                    if path.is_dir() {
                        // Bundled directory layout (e.g. runtime/redis-7.2.4/) —
                        // copy its contents into the target dir verbatim.
                        copy_dir_contents(&path, &target_dir)?;
                        info!("install_bundled_runtime: copied dir contents to {}", target_dir.display());
                    } else if file_name.to_lowercase().ends_with(".exe") {
                        let dest = target_dir.join(&file_name);
                        fs::copy(&path, &dest).context("failed to copy bundled exe")?;
                        info!("install_bundled_runtime: copied exe to {}", dest.display());
                    } else {
                        // Zip file — extract
                        self.extract_zip_to(&path, &target_dir)?;
                    }
                }
                None => {
                    info!("install_bundled_runtime: no bundled file found for {}, skipping", software_id);
                    return Ok(RuntimeManifest {
                        id: software_id.to_string(),
                        runtime_type: RuntimeType::Nginx,
                        version: "unknown".to_string(),
                        install_path: String::new(),
                        entrypoint: String::new(),
                        config_template: None,
                        installed: false,
                    });
                }
            }
        }

        normalize_service_root(&target_dir, service_id)?;

        // Register the service
        let install_path_str = target_dir.to_string_lossy().to_string();
        let runtime_type = match service_id.as_str() {
            "nginx" => RuntimeType::Nginx,
            "php" | "php73" => RuntimeType::Php,
            s if s.starts_with("php") => RuntimeType::Php,
            _ => {
                // Generic service
                self.register_generic_service(software_id, service_id, &install_path_str)?;

                // MySQL special: initialize data directory if needed
                if service_id.starts_with("mysql") {
                    self.initialize_mysql_if_needed(service_id, &target_dir)?;
                }

                self.db.update_software_bundled_installed(software_id, &install_path_str)?;
                self.update_bundled_companion_software(software_id, &target_dir, &install_path_str);
                self.db.add_log("software.install_bundled", software_id, true, &format!("bundled install {} complete", software_id))?;

                return Ok(RuntimeManifest {
                    id: software_id.to_string(),
                    runtime_type: RuntimeType::Nginx,
                    version: "unknown".to_string(),
                    install_path: install_path_str,
                    entrypoint: String::new(),
                    config_template: None,
                    installed: true,
                });
            }
        };

        let manifest = self.import_runtime(runtime_type, &install_path_str, None)?;
        self.db.update_software_bundled_installed(software_id, &install_path_str)?;
        self.db.add_log("software.install_bundled", software_id, true, &format!("bundled install {} complete", software_id))?;

        Ok(manifest)
    }

    /// Find the bundled file for a software ID in the runtime directory.
    /// Prefers an unpacked directory (e.g. `redis-7.2.4/`) over a zip archive,
    /// so pre-assembled runtime folders win over zips when both exist.
    pub fn find_bundled_file(&self, software_id: &str, runtime_dir: &Path) -> Option<PathBuf> {
        if !runtime_dir.exists() {
            return None;
        }
        let prefixes = match software_id {
            "nginx" => vec!["nginx"],
            "mysql80" => vec!["mysql-8.0", "mysql-8"],
            "mysql57" => vec!["mysql-5.7", "mysql-57"],
            "redis" => vec!["redis"],
            "minio" => vec!["minio"],
            "php73" => vec!["php-7.3", "php73"],
            _ => vec![software_id],
        };

        let mut dir_match: Option<PathBuf> = None;
        let mut file_match: Option<PathBuf> = None;
        if let Ok(entries) = fs::read_dir(runtime_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                for prefix in &prefixes {
                    if name.starts_with(prefix) {
                        if entry.path().is_dir() {
                            dir_match.get_or_insert(entry.path());
                        } else {
                            file_match.get_or_insert(entry.path());
                        }
                        break;
                    }
                }
            }
        }
        dir_match.or(file_match)
    }

    /// Extract a zip file to a target directory, stripping the top-level dir if present.
    fn extract_zip_to(&self, zip_path: &Path, target_dir: &Path) -> Result<()> {
        let zip_file = fs::File::open(zip_path).context("failed to open bundled zip")?;
        let mut archive = ZipArchive::new(zip_file).context("failed to read bundled zip")?;
        info!("extract_zip_to: zip has {} entries", archive.len());

        let entry_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| normalize_zip_entry_name(e.name())))
            .collect();
        let top_dir = self.detect_top_dir_from_names(&entry_names);
        info!("extract_zip_to: top_dir={:?}", top_dir);

        let zip_file2 = fs::File::open(zip_path).context("failed to reopen bundled zip")?;
        let mut archive2 = ZipArchive::new(zip_file2).context("failed to reopen bundled zip")?;

        let mut file_count = 0u32;
        for i in 0..archive2.len() {
            let mut entry = archive2.by_index(i)?;
            let entry_path = normalize_zip_entry_name(entry.name());
            let relative_path = if let Some(ref prefix) = top_dir {
                match entry_path.strip_prefix(prefix.as_str()) {
                    Some(rest) => rest.to_string(),
                    None => continue,
                }
            } else {
                entry_path
            };

            if relative_path.is_empty() || relative_path == "/" {
                continue;
            }

            let out_path = target_dir.join(&relative_path);
            if relative_path.starts_with('/') || relative_path.contains("..") {
                continue;
            }

            if entry.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
                file_count += 1;
            }
        }
        info!("extract_zip_to: extracted {} files", file_count);
        Ok(())
    }

    /// Initialize MySQL data directory if it doesn't exist.
    fn initialize_mysql_if_needed(&self, service_id: &str, install_dir: &Path) -> Result<()> {
        let my_ini = install_dir.join("my.ini");
        let ini_content = render_managed_mysql_ini(service_id, install_dir);
        fs::write(&my_ini, &ini_content)?;
        info!("initialize_mysql_if_needed: wrote managed my.ini at {}", my_ini.display());

        let data_dir = install_dir.join("data");
        if data_dir.exists() {
            info!("initialize_mysql_if_needed: data dir already exists for {}", service_id);
            return Ok(());
        }

        let mysqld_exe = install_dir.join("bin").join("mysqld.exe");
        if !mysqld_exe.exists() {
            warn!("initialize_mysql_if_needed: mysqld.exe not found at {}", mysqld_exe.display());
            return Ok(());
        }

        info!("initialize_mysql_if_needed: initializing MySQL data directory for {}", service_id);

        // Run mysqld --initialize-insecure
        let defaults_file = format!("--defaults-file={}", my_ini.to_string_lossy());
        let output = silent_command_path(&mysqld_exe)
            .arg(defaults_file)
            .args(["--initialize-insecure", "--console"])
            .current_dir(install_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();

        match output {
            Ok(out) => {
                if out.status.success() && data_dir.exists() {
                    info!("initialize_mysql_if_needed: MySQL data directory initialized successfully");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    anyhow::bail!(
                        "MySQL 初始化失败：{}{}",
                        stderr.trim(),
                        if stdout.trim().is_empty() { String::new() } else { format!(" {}", stdout.trim()) }
                    );
                }
            }
            Err(e) => {
                Err(e).context("failed to run mysqld --initialize")
            }
        }
    }

    /// Detect local services already installed on this machine.
    /// Returns a map of service_id -> install_path for found services.
    ///
    /// Detection order (first match wins):
    ///   1. The app's own managed dir `data_dir/server/{service_id}` — where
    ///      bundled/runtime installs land. This lets services survive a DB reset
    ///      or a fresh launch against an existing data directory.
    ///   2. Well-known system install paths.
    ///   3. System PATH via `where <marker>` (minio included).
    pub fn detect_local_services(&self) -> std::collections::HashMap<String, String> {
        let mut found = std::collections::HashMap::new();

        // (service_id, well-known system dirs, marker file relative to the dir)
        let checks: &[(&str, &[&str], &str)] = &[
            ("nginx", &["C:/nginx", "D:/nginx", "C:/Program Files/nginx"], "nginx.exe"),
            ("mysql80", &["C:/Program Files/MySQL/MySQL Server 8.0", "D:/MySQL8", "D:/mysql80"], "bin/mysqld.exe"),
            ("mysql57", &["C:/Program Files/MySQL/MySQL Server 5.7", "D:/MySQL5", "D:/mysql57"], "bin/mysqld.exe"),
            ("redis", &["C:/Redis", "D:/Redis", "C:/Program Files/Redis"], "redis-server.exe"),
            ("minio", &["C:/minio", "D:/minio", "C:/Program Files/minio"], "minio.exe"),
            ("pgsql", &["C:/Program Files/PostgreSQL", "D:/PostgreSQL", "D:/pgsql"], "bin/pg_ctl.exe"),
            ("apache", &["C:/Apache24", "D:/Apache24", "C:/Program Files/Apache Software Foundation/Apache2.4"], "bin/httpd.exe"),
            ("php73", &["C:/php", "D:/php", "C:/xampp/php"], "php-cgi.exe"),
        ];

        for (service_id, dirs, marker) in checks {
            // 1. App-managed directory first (survives DB resets, portable installs).
            let managed_dir = self.data_dir.join("server").join(service_id);
            let _ = normalize_service_root(&managed_dir, service_id);
            let managed_marker = managed_dir.join(marker);
            if managed_marker.exists() {
                let p = managed_dir.to_string_lossy().replace('\\', "/");
                info!("detect_local_services: found {} in app-managed dir {}", service_id, p);
                found.insert(service_id.to_string(), p);
                continue;
            }

            // 2. Well-known system directories.
            for dir in *dirs {
                let marker_path = PathBuf::from(dir).join(marker);
                if marker_path.exists() {
                    info!("detect_local_services: found {} at {}", service_id, dir);
                    found.insert(service_id.to_string(), dir.to_string());
                    break;
                }
            }

            // 3. System PATH. Minio ships a single exe, so it is most commonly on
            //    PATH; always probe PATH regardless of whether system dirs matched.
            if !found.contains_key(*service_id) {
                if let Ok(output) = silent_command("where").arg(marker).output() {
                    if output.status.success() {
                        let path_str = String::from_utf8_lossy(&output.stdout);
                        if let Some(first_line) = path_str.lines().next() {
                            if let Some(parent) = Path::new(first_line).parent() {
                                info!("detect_local_services: found {} in PATH at {}", service_id, parent.display());
                                found.insert(service_id.to_string(), parent.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        found
    }

    /// Register a service detected on disk into the DB (marks `installed = 1`
    /// and fills exe/args/config_file). Used by run_auto_setup when a local
    /// installation is found, so the UI no longer shows "未安装".
    pub fn register_detected_service(&self, software_id: &str, service_id: &str, install_path: &str) -> Result<()> {
        info!("register_detected_service: software_id={} service_id={} path={}", software_id, service_id, install_path);
        match service_id {
            "nginx" => {
                self.import_runtime(RuntimeType::Nginx, install_path, Some(80))?;
            }
            "php73" | "php" => {
                self.import_runtime(RuntimeType::Php, install_path, None)?;
            }
            _ => {
                self.register_generic_service(software_id, service_id, install_path)?;
            }
        }
        // Mark the software row installed too, so the Software page agrees.
        self.db.update_software_bundled_installed(software_id, install_path)?;
        Ok(())
    }

    /// Download and install software from its download_url.
    /// Progress is tracked via the shared atomic counters.
    pub fn download_install(
        &self,
        software_id: &str,
        progress: &DownloadProgress,
    ) -> Result<RuntimeManifest> {
        info!("download_install: start software_id={}", software_id);
        let sw = self.db.get_software(software_id)?;
        let download_url = sw.download_url.as_deref().unwrap_or("");
        if download_url.is_empty() {
            anyhow::bail!("software {} has no download_url", software_id);
        }

        info!("download_install: url={}, service_id={}, category={}", download_url, sw.service_id, sw.category);

        // Set status to downloading
        self.db.update_software_status(software_id, "downloading")?;
        progress.set_phase("downloading");

        // Target directory: data_dir/server/{service_id}/
        let target_dir = self.data_dir.join("server").join(&sw.service_id);
        info!("download_install: target_dir={}", target_dir.display());
        if target_dir.exists() {
            info!("download_install: removing existing target dir");
            let _ = fs::remove_dir_all(&target_dir);
        }
        fs::create_dir_all(&target_dir)?;

        // Download to temp file
        let temp_dir = self.data_dir.join("tmp");
        fs::create_dir_all(&temp_dir)?;
        let temp_file = temp_dir.join(format!("{}-download", software_id));
        info!("download_install: temp_file={}", temp_file.display());

        let result = self.do_download_and_install(
            software_id,
            download_url,
            &sw.service_id,
            &target_dir,
            &temp_file,
            progress,
        );

        if let Err(e) = &result {
            error!("download_install: FAILED for {}: {}", software_id, e);
            self.db.update_software_status(software_id, "failed")?;
            // Cleanup
            let _ = fs::remove_dir_all(&target_dir);
            let _ = fs::remove_file(&temp_file);
        } else {
            info!("download_install: SUCCESS for {}", software_id);
        }

        // Cleanup temp file
        let _ = fs::remove_file(&temp_file);

        result
    }

    fn do_download_and_install(
        &self,
        software_id: &str,
        download_url: &str,
        service_id: &str,
        target_dir: &Path,
        temp_file: &Path,
        progress: &DownloadProgress,
    ) -> Result<RuntimeManifest> {
        // Download
        info!("do_download_and_install: requesting {}", download_url);
        let mut response = reqwest::blocking::Client::new()
            .get(download_url)
            .send()
            .context("download request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("download returned HTTP {}", status);
        }
        let total_size: u64 = response.content_length().unwrap_or(0);
        info!("do_download_and_install: HTTP {}, content_length={}", status, total_size);
        progress.set_total(total_size);

        let mut file = fs::File::create(temp_file).context("failed to create temp file")?;
        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = response.read(&mut buffer)?;
            if bytes_read == 0 { break; }
            file.write_all(&buffer[..bytes_read])?;
            downloaded += bytes_read as u64;
            progress.set_downloaded(downloaded);
        }
        drop(file);

        info!("do_download_and_install: downloaded {} bytes to {}", downloaded, temp_file.display());

        // Check if it's a single exe or a zip
        self.db.update_software_status(software_id, "installing")?;
        progress.set_phase("installing");

        let is_exe = download_url.to_lowercase().ends_with(".exe");
        info!("do_download_and_install: is_exe={}, extracting to {}", is_exe, target_dir.display());

        if is_exe {
            // Single exe — just move it to target dir
            let exe_name = Path::new(download_url)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let dest = target_dir.join(&exe_name);
            info!("do_download_and_install: copying exe to {}", dest.display());
            fs::copy(temp_file, &dest).context("failed to copy executable")?;
            info!("do_download_and_install: exe copied successfully");
        } else {
            // Zip — extract
            let zip_file = fs::File::open(temp_file).context("failed to open downloaded file")?;
            let mut archive = ZipArchive::new(zip_file).context("failed to open zip archive")?;
            info!("do_download_and_install: zip opened, {} entries", archive.len());

            // First pass: collect entry names and detect top-level directory
            let entry_names: Vec<String> = (0..archive.len())
                .filter_map(|i| archive.by_index(i).ok().map(|e| normalize_zip_entry_name(e.name())))
                .collect();
            let top_dir = self.detect_top_dir_from_names(&entry_names);
            info!("do_download_and_install: top_dir={:?}", top_dir);

            // Second pass: extract using a fresh archive handle
            let zip_file2 = fs::File::open(temp_file).context("failed to reopen downloaded file")?;
            let mut archive2 = ZipArchive::new(zip_file2).context("failed to reopen zip archive")?;

            let mut file_count = 0u32;
            for i in 0..archive2.len() {
                let mut entry = archive2.by_index(i)?;
                let entry_path = normalize_zip_entry_name(entry.name());
                let relative_path = if let Some(ref prefix) = top_dir {
                    match entry_path.strip_prefix(prefix.as_str()) {
                        Some(rest) => rest.to_string(),
                        None => continue,
                    }
                } else {
                    entry_path
                };

                if relative_path.is_empty() || relative_path == "/" {
                    continue;
                }

                let out_path = target_dir.join(&relative_path);

                // Sanitize: skip absolute paths or path traversal
                if relative_path.starts_with('/') || relative_path.contains("..") {
                    warn!("do_download_and_install: skipping suspicious path: {}", relative_path);
                    continue;
                }

                if entry.is_dir() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let mut out_file = fs::File::create(&out_path)?;
                    std::io::copy(&mut entry, &mut out_file)?;
                    file_count += 1;
                }
            }
            info!("do_download_and_install: extracted {} files", file_count);
        }

        normalize_service_root(target_dir, service_id)?;

        // Determine runtime type and run import logic
        let install_path_str = target_dir.to_string_lossy().to_string();
        let runtime_type = match service_id {
            "nginx" => RuntimeType::Nginx,
            "php" | "php73" => RuntimeType::Php,
            s if s.starts_with("php") => RuntimeType::Php,
            _ => {
                // For non-nginx/php, just register the service path
                info!("do_download_and_install: registering generic service software_id={}, service_id={}", software_id, service_id);
                self.register_generic_service(software_id, service_id, &install_path_str)?;
                self.db.update_software_download_installed(software_id, &install_path_str)?;
                self.db.add_log("software.download_install", software_id, true, &format!("online install {} complete", software_id))?;
                info!("do_download_and_install: generic service install complete for {}", software_id);

                return Ok(RuntimeManifest {
                    id: software_id.to_string(),
                    runtime_type: RuntimeType::Nginx,
                    version: "unknown".to_string(),
                    install_path: install_path_str,
                    entrypoint: String::new(),
                    config_template: None,
                    installed: true,
                });
            }
        };

        info!("do_download_and_install: importing runtime type={:?} from {}", runtime_type, install_path_str);
        let manifest = self.import_runtime(runtime_type, &install_path_str, None)?;
        self.db.update_software_download_installed(software_id, &install_path_str)?;
        self.db.add_log("software.download_install", software_id, true, &format!("online install {} complete", software_id))?;
        progress.set_phase("done");
        info!("do_download_and_install: runtime import complete for {}", software_id);

        Ok(manifest)
    }

    /// Detect if zip entries share a single top-level directory prefix.
    fn detect_top_dir_from_names(&self, names: &[String]) -> Option<String> {
        let mut top_dirs = std::collections::HashSet::new();
        for name in names {
            if let Some(slash_pos) = name.find('/') {
                top_dirs.insert(name[..slash_pos + 1].to_string());
            }
        }
        if top_dirs.len() == 1 {
            top_dirs.into_iter().next()
        } else {
            None
        }
    }

    /// Register a generic service (non-nginx/php) by updating its install path.
    fn register_generic_service(&self, software_id: &str, service_id: &str, install_path: &str) -> Result<()> {
        info!("Registering generic service: software_id={}, service_id={}, path={}", software_id, service_id, install_path);

        let install_dir = PathBuf::from(install_path);
        let dir_str = install_path.replace('\\', "/");

        // Determine exe, args, cwd, config_file for each service type
        let (exe, args, config_file) = match service_id {
            "redis" => {
                let exe_path = install_dir.join("redis-server.exe");
                let conf_path = install_dir.join("redis.conf");
                // Ensure a default redis.conf exists so users always have an
                // editable config (both visual form and direct-file editing).
                if !conf_path.exists() {
                    let _ = fs::write(&conf_path, default_redis_conf());
                    info!("register_generic_service: wrote default redis.conf at {}", conf_path.display());
                }
                let exe_str = exe_path.to_string_lossy().replace('\\', "/");
                let conf_str = conf_path.to_string_lossy().replace('\\', "/");
                let args = if conf_path.exists() {
                    format!("redis.conf")
                } else {
                    String::new()
                };
                // Register the config file so it appears in the config editor.
                let _ = self.db.upsert_config_file("redis.conf", "redis.conf", &conf_str);
                (exe_str, args, conf_str)
            }
            "minio" => {
                let exe_path = install_dir.join("minio.exe");
                let data_path = install_dir.join("data");
                let _ = fs::create_dir_all(&data_path);
                // Ensure a default minio.env exists (root credentials + ports).
                let env_path = install_dir.join("minio.env");
                if !env_path.exists() {
                    let _ = fs::write(&env_path, default_minio_env());
                    info!("register_generic_service: wrote default minio.env at {}", env_path.display());
                }
                let exe_str = exe_path.to_string_lossy().replace('\\', "/");
                let data_str = data_path.to_string_lossy().replace('\\', "/");
                let args = format!("server {} --address :9000 --console-address :9001", data_str);
                let env_str = env_path.to_string_lossy().replace('\\', "/");
                // Register the config file so it appears in the config editor.
                let _ = self.db.upsert_config_file("minio.env", "minio.env", &env_str);
                (exe_str, args, env_str)
            }
            "mysql80" | "mysql57" => {
                let exe_path = install_dir.join("bin").join("mysqld.exe");
                let ini_path = install_dir.join("my.ini");
                let exe_str = exe_path.to_string_lossy().replace('\\', "/");
                let ini_str = ini_path.to_string_lossy().replace('\\', "/");
                let args = format!("--defaults-file=\"{}\"", ini_str);
                (exe_str, args, ini_str)
            }
            "pgsql" => {
                let exe_path = install_dir.join("bin").join("pg_ctl.exe");
                let conf_path = install_dir.join("data").join("postgresql.conf");
                let exe_str = exe_path.to_string_lossy().replace('\\', "/");
                (exe_str, String::new(), conf_path.to_string_lossy().replace('\\', "/"))
            }
            "apache" => {
                let exe_path = install_dir.join("bin").join("httpd.exe");
                let conf_path = install_dir.join("conf").join("httpd.conf");
                let exe_str = exe_path.to_string_lossy().replace('\\', "/");
                (exe_str, String::new(), conf_path.to_string_lossy().replace('\\', "/"))
            }
            _ => {
                self.db.update_service_path(service_id, install_path)?;
                self.db.update_service_installed(service_id, true)?;
                return Ok(());
            }
        };

        let exe_exists = Path::new(&exe).exists();
        info!("register_generic_service: exe={}, exists={}", exe, exe_exists);
        if !exe_exists {
            anyhow::bail!("安装目录缺少服务可执行文件：{}", exe);
        }

        self.db.conn(|conn| {
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE service_instances SET exe = ?1, args = ?2, cwd = ?3, config_file = ?4, installed = ?5, updated_at = ?6 WHERE id = ?7",
                rusqlite::params![exe, args, dir_str, config_file, exe_exists as i32, &now, service_id],
            )?;
            Ok(())
        })?;

        info!("Generic service registered: service_id={} installed={}", service_id, exe_exists);
        Ok(())
    }

    // ---- Private scanning methods ----

    /// Scan a PHP runtime directory.
    fn scan_php_runtime(&self, path: &Path, dir_name: &str) -> Option<RuntimeManifest> {
        let php_cgi = path.join("php-cgi.exe");
        let php_exe = path.join("php.exe");
        let php_ini = path.join("php.ini");

        let entrypoint = if php_cgi.exists() {
            php_cgi.to_string_lossy().to_string()
        } else if php_exe.exists() {
            php_exe.to_string_lossy().to_string()
        } else {
            warn!("PHP runtime found but no php-cgi.exe or php.exe in {:?}", path);
            return None;
        };

        let version = self.extract_version(dir_name, "php");

        Some(RuntimeManifest {
            id: dir_name.to_lowercase(),
            runtime_type: RuntimeType::Php,
            version,
            install_path: path.to_string_lossy().to_string(),
            entrypoint,
            config_template: if php_ini.exists() {
                Some(php_ini.to_string_lossy().to_string())
            } else {
                None
            },
            installed: true,
        })
    }

    /// Scan an Nginx runtime directory.
    fn scan_nginx_runtime(&self, path: &Path, dir_name: &str) -> Option<RuntimeManifest> {
        let nginx_exe = path.join("nginx.exe");
        let nginx_conf = path.join("conf").join("nginx.conf");

        if !nginx_exe.exists() {
            warn!("Nginx runtime found but no nginx.exe in {:?}", path);
            return None;
        }

        let version = self.extract_version(dir_name, "nginx");

        Some(RuntimeManifest {
            id: dir_name.to_lowercase(),
            runtime_type: RuntimeType::Nginx,
            version,
            install_path: path.to_string_lossy().to_string(),
            entrypoint: nginx_exe.to_string_lossy().to_string(),
            config_template: if nginx_conf.exists() {
                Some(nginx_conf.to_string_lossy().to_string())
            } else {
                None
            },
            installed: true,
        })
    }

    /// Scan an Apache runtime directory.
    fn scan_apache_runtime(&self, path: &Path, dir_name: &str) -> Option<RuntimeManifest> {
        let httpd_exe = path.join("bin").join("httpd.exe");
        let httpd_conf = path.join("conf").join("httpd.conf");

        if !httpd_exe.exists() {
            warn!("Apache runtime found but no httpd.exe in {:?}", path);
            return None;
        }

        let version = self.extract_version(dir_name, "apache");

        Some(RuntimeManifest {
            id: dir_name.to_lowercase(),
            runtime_type: RuntimeType::Apache,
            version,
            install_path: path.to_string_lossy().to_string(),
            entrypoint: httpd_exe.to_string_lossy().to_string(),
            config_template: if httpd_conf.exists() {
                Some(httpd_conf.to_string_lossy().to_string())
            } else {
                None
            },
            installed: true,
        })
    }

    /// Scan a MySQL runtime directory.
    fn scan_mysql_runtime(&self, path: &Path, dir_name: &str) -> Option<RuntimeManifest> {
        let mysqld_exe = path.join("bin").join("mysqld.exe");

        if !mysqld_exe.exists() {
            warn!("MySQL runtime found but no mysqld.exe in {:?}", path);
            return None;
        }

        let version = self.extract_version(dir_name, "mysql");

        Some(RuntimeManifest {
            id: dir_name.to_lowercase(),
            runtime_type: RuntimeType::Mysql,
            version,
            install_path: path.to_string_lossy().to_string(),
            entrypoint: mysqld_exe.to_string_lossy().to_string(),
            config_template: None,
            installed: true,
        })
    }

    /// Scan a Redis runtime directory.
    fn scan_redis_runtime(&self, path: &Path, dir_name: &str) -> Option<RuntimeManifest> {
        let redis_exe = path.join("redis-server.exe");

        if !redis_exe.exists() {
            warn!("Redis runtime found but no redis-server.exe in {:?}", path);
            return None;
        }

        let version = self.extract_version(dir_name, "redis");

        Some(RuntimeManifest {
            id: dir_name.to_lowercase(),
            runtime_type: RuntimeType::Redis,
            version,
            install_path: path.to_string_lossy().to_string(),
            entrypoint: redis_exe.to_string_lossy().to_string(),
            config_template: None,
            installed: true,
        })
    }

    /// Extract a version string from a directory name like "php73" or "nginx1.26.3".
    fn extract_version(&self, dir_name: &str, prefix: &str) -> String {
        let lower_name = dir_name.to_lowercase();
        let stripped = lower_name
            .strip_prefix(prefix)
            .unwrap_or(&lower_name);

        let version_part = stripped.trim_start_matches(|c: char| !c.is_ascii_digit());

        if version_part.is_empty() {
            return "unknown".to_string();
        }

        // Try to format version numbers nicely (e.g., "7.3" from "73", "1.26.3" from "1.26.3")
        if version_part.contains('.') {
            version_part.to_string()
        } else {
            // For short version strings like "73", insert dots
            let chars: Vec<char> = version_part.chars().collect();
            match chars.len() {
                2 => format!("{}.{}", chars[0], chars[1]),
                3 => format!("{}.{}.{}", chars[0], chars[1], chars[2]),
                _ => version_part.to_string(),
            }
        }
    }

    fn import_nginx(&self, root: &Path, port: u16) -> Result<RuntimeManifest> {
        let nginx_exe = root.join("nginx.exe");
        let nginx_conf = root.join("conf").join("nginx.conf");

        ensure_file(&nginx_exe, "nginx.exe")?;
        ensure_file(&nginx_conf, "conf/nginx.conf")?;

        if port == 0 {
            anyhow::bail!("Nginx 端口必须在 1-65535 范围内");
        }

        // Update the listen directive in nginx.conf if port is not 80
        if port != 80 {
            if let Ok(content) = fs::read_to_string(&nginx_conf) {
                let updated = content
                    .replace("listen       80;", &format!("listen       {};", port))
                    .replace("listen 80;", &format!("listen {};", port))
                    .replace("listen 80", &format!("listen {}", port));
                let _ = fs::write(&nginx_conf, updated);
            }
        }

        // Ensure nginx.conf includes the vhosts directory
        if let Ok(content) = fs::read_to_string(&nginx_conf) {
            if !content.contains("vhosts") {
                // Add include directive before the last closing brace of the http block
                let updated = content.replace(
                    "    }\n\n\n}",
                    "    }\n\n    include vhosts/*.conf;\n}",
                );
                if updated != content {
                    let _ = fs::write(&nginx_conf, updated);
                } else {
                    // Fallback: add before the last }
                    if let Some(pos) = content.rfind('}') {
                        let mut new_content = content.clone();
                        new_content.insert_str(pos, "\n    include vhosts/*.conf;\n");
                        let _ = fs::write(&nginx_conf, new_content);
                    }
                }
            }
            // Create vhosts directory
            let vhost_dir = root.join("conf").join("vhosts");
            let _ = fs::create_dir_all(&vhost_dir);
        }

        let version_output = run_version_command(&nginx_exe, &["-v"])
            .with_context(|| format!("Nginx 版本检测失败：{}", nginx_exe.display()))?;
        let version = parse_version(&version_output).unwrap_or_else(|| {
            root.file_name()
                .map(|name| self.extract_version(&name.to_string_lossy(), "nginx"))
                .unwrap_or_else(|| "unknown".to_string())
        });

        let id = format!("nginx-{}", sanitize_id(&version));
        let exe = nginx_exe.to_string_lossy().to_string();
        let cwd = root.to_string_lossy().to_string();
        let config_path = nginx_conf.to_string_lossy().to_string();

        self.db.upsert_service_runtime(
            "nginx",
            &format!("Nginx {}", version),
            "nginx",
            "nginx.exe",
            port,
            &exe,
            "",
            &cwd,
            Some(&config_path),
        )?;
        self.db
            .upsert_config_file("nginx.conf", "nginx.conf", &config_path)?;
        self.db
            .update_software_installed("nginx", true)
            .ok();
        self.db
            .add_log("runtime.import", "nginx", true, &format!("导入 Nginx {}", version))
            .ok();

        Ok(RuntimeManifest {
            id,
            runtime_type: RuntimeType::Nginx,
            version,
            install_path: cwd,
            entrypoint: exe,
            config_template: Some(config_path),
            installed: true,
        })
    }

    fn import_php(&self, root: &Path, requested_port: Option<u16>) -> Result<RuntimeManifest> {
        let php_exe = root.join("php.exe");
        let php_cgi = root.join("php-cgi.exe");
        let php_ini = root.join("php.ini");

        ensure_file(&php_exe, "php.exe")?;
        ensure_file(&php_cgi, "php-cgi.exe")?;
        ensure_file(&php_ini, "php.ini")?;

        let version_output = run_version_command(&php_exe, &["-v"])
            .with_context(|| format!("PHP 版本检测失败：{}", php_exe.display()))?;
        let version = parse_version(&version_output).unwrap_or_else(|| {
            root.file_name()
                .map(|name| self.extract_version(&name.to_string_lossy(), "php"))
                .unwrap_or_else(|| "unknown".to_string())
        });

        let port = requested_port.unwrap_or_else(|| self.next_php_port());
        if port == 0 {
            anyhow::bail!("PHP FastCGI 端口必须在 1-65535 范围内");
        }

        let base_id = format!("php-{}", sanitize_id(&version));
        let service_id = self.unique_service_id(&base_id)?;
        let php_cgi_path = php_cgi.to_string_lossy().to_string();
        let cwd = root.to_string_lossy().to_string();
        let php_ini_path = php_ini.to_string_lossy().to_string();
        let config_id = format!("php.ini:{}", service_id);
        let args = format!("-b 127.0.0.1:{}", port);

        self.db.upsert_service_runtime(
            &service_id,
            &format!("PHP {} CGI", version),
            "php",
            "php-cgi.exe",
            port,
            &php_cgi_path,
            &args,
            &cwd,
            Some(&php_ini_path),
        )?;
        self.db
            .upsert_config_file(&config_id, &format!("php.ini ({})", version), &php_ini_path)?;
        self.db
            .add_log("runtime.import", &service_id, true, &format!("导入 PHP {}", version))
            .ok();

        Ok(RuntimeManifest {
            id: service_id,
            runtime_type: RuntimeType::Php,
            version,
            install_path: cwd,
            entrypoint: php_cgi_path,
            config_template: Some(php_ini_path),
            installed: true,
        })
    }

    fn next_php_port(&self) -> u16 {
        let used_ports = self
            .db
            .list_service_instances()
            .unwrap_or_default()
            .into_iter()
            .map(|svc| svc.port)
            .collect::<std::collections::HashSet<_>>();

        for port in 9073..=9173 {
            if !used_ports.contains(&port) {
                return port;
            }
        }
        9073
    }

    fn unique_service_id(&self, base_id: &str) -> Result<String> {
        let existing = self
            .db
            .list_service_instances()?
            .into_iter()
            .map(|svc| svc.id)
            .collect::<std::collections::HashSet<_>>();

        if !existing.contains(base_id) {
            return Ok(base_id.to_string());
        }

        for index in 2..=99 {
            let candidate = format!("{}-{}", base_id, index);
            if !existing.contains(&candidate) {
                return Ok(candidate);
            }
        }

        anyhow::bail!("无法生成唯一 PHP 服务 ID：{}", base_id);
    }
}

fn ensure_file(path: &Path, label: &str) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!("运行环境缺少关键文件 {}：{}", label, path.display());
    }
    Ok(())
}

/// Build a `Command` that never flashes a console window on Windows.
/// We use this for every helper subprocess (version probes, `where`, mysqld
/// initialization) so that service setup is fully silent.
fn silent_command(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    let _ = &mut cmd; // silence unused-mut on non-windows
    cmd
}

/// Build a silent `Command` from a `Path` (overload for `&Path` exe paths).
fn silent_command_path(exe: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    let _ = &mut cmd;
    cmd
}

fn run_version_command(exe: &Path, args: &[&str]) -> Result<String> {
    let output = silent_command_path(exe)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    if !output.status.success() {
        anyhow::bail!("{}", combined.trim());
    }

    Ok(combined)
}

fn parse_version(output: &str) -> Option<String> {
    let lower = output.to_lowercase();
    if let Some(index) = lower.find("nginx/") {
        return take_version(&output[index + "nginx/".len()..]);
    }
    if let Some(index) = lower.find("php ") {
        return take_version(&output[index + "php ".len()..]);
    }
    if let Some(index) = output.find('v') {
        if let Some(version) = take_version(&output[index + 1..]) {
            return Some(version);
        }
    }
    take_version(output)
}

fn take_version(text: &str) -> Option<String> {
    let start = text.find(|c: char| c.is_ascii_digit())?;
    let mut version = String::new();
    for ch in text[start..].chars() {
        if ch.is_ascii_digit() || ch == '.' {
            version.push(ch);
        } else {
            break;
        }
    }
    if version.is_empty() {
        None
    } else {
        Some(version.trim_matches('.').to_string())
    }
}

fn sanitize_id(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed
    }
}

/// Default redis.conf written when registering the redis service, so users
/// always have an editable config. Fields here drive the visual config form.
fn default_redis_conf() -> String {
    r#"# Redis configuration (managed by WinServer)
# Editable via WinServer visual config or this file directly.

port 6379
bind 127.0.0.1
protected-mode yes
# requirepass <password>     # uncomment and set to enable authentication
maxmemory 256mb
maxmemory-policy allkeys-lru
appendonly no
save ""
"#
    .to_string()
}

/// Default minio.env written when registering the minio service.
/// Holds the MinIO root credentials read at startup.
fn default_minio_env() -> String {
    r#"# MinIO configuration (managed by WinServer)
# Editable via WinServer visual config or this file directly.

MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
MINIO_API_PORT=9000
MINIO_CONSOLE_PORT=9001
"#
    .to_string()
}

fn render_managed_mysql_ini(service_id: &str, install_dir: &Path) -> String {
    let dir_str = install_dir.to_string_lossy().replace('\\', "/");
    let port = if service_id == "mysql57" { 3307 } else { 3306 };
    format!(
r#"[mysql]
default-character-set=utf8

[mysqld]
port={}
default_authentication_plugin=mysql_native_password
basedir="{}"
datadir="{}/data"
character-set-server=utf8
default-storage-engine=InnoDB
max_connections=200
innodb_buffer_pool_size=64M

[client]
port={}
default-character-set=utf8
"#, port, dir_str, dir_str, port)
}

/// Recursively copy the contents of `src` into `dst` (preserving relative paths).
/// Used when a bundled runtime ships as an unpacked directory.
fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    let stack: Vec<(std::path::PathBuf, std::path::PathBuf)> =
        vec![(src.to_path_buf(), dst.to_path_buf())];
    let mut work = stack;
    while let Some((from, to)) = work.pop() {
        fs::create_dir_all(&to)?;
        for entry in fs::read_dir(&from)? {
            let entry = entry?;
            let from_path = entry.path();
            let to_path = to.join(entry.file_name());
            if from_path.is_dir() {
                work.push((from_path, to_path));
            } else {
                fs::copy(&from_path, &to_path)
                    .with_context(|| format!("failed to copy {}", from_path.display()))?;
            }
        }
    }
    Ok(())
}

fn normalize_zip_entry_name(name: &str) -> String {
    name.replace('\\', "/")
}

fn service_marker_path(service_id: &str) -> Option<&'static str> {
    match service_id {
        "nginx" => Some("nginx.exe"),
        "apache" => Some("bin/httpd.exe"),
        "mysql80" | "mysql57" => Some("bin/mysqld.exe"),
        "redis" => Some("redis-server.exe"),
        "minio" => Some("minio.exe"),
        "pgsql" => Some("bin/pg_ctl.exe"),
        "php" | "php73" => Some("php-cgi.exe"),
        s if s.starts_with("php") => Some("php-cgi.exe"),
        _ => None,
    }
}

fn has_service_marker(root: &Path, service_id: &str) -> bool {
    service_marker_path(service_id)
        .map(|marker| root.join(marker).exists())
        .unwrap_or(false)
}

fn normalize_service_root(root: &Path, service_id: &str) -> Result<()> {
    if !root.is_dir() || service_marker_path(service_id).is_none() {
        return Ok(());
    }

    for _ in 0..8 {
        if has_service_marker(root, service_id) {
            return Ok(());
        }

        let entries = fs::read_dir(root)?.collect::<std::result::Result<Vec<_>, _>>()?;
        let has_files = entries.iter().any(|entry| entry.path().is_file());
        let dirs = entries.iter().filter(|entry| entry.path().is_dir()).collect::<Vec<_>>();

        if has_files || dirs.len() != 1 {
            return Ok(());
        }

        let child = dirs[0].path();
        let child_for_log = child.clone();
        let mut staging = root.join(".winserver-flattening");
        let mut suffix = 0u8;
        while staging.exists() {
            suffix += 1;
            staging = root.join(format!(".winserver-flattening-{}", suffix));
        }

        fs::rename(&child, &staging).with_context(|| {
            format!(
                "failed to stage wrapper directory {} as {}",
                child.display(),
                staging.display()
            )
        })?;

        let child_entries = fs::read_dir(&staging)?.collect::<std::result::Result<Vec<_>, _>>()?;
        for entry in child_entries {
            let source = entry.path();
            let destination = root.join(entry.file_name());
            if destination.exists() {
                anyhow::bail!("无法归一运行目录，目标已存在：{}", destination.display());
            }
            fs::rename(&source, &destination).with_context(|| {
                format!(
                    "failed to move {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
        fs::remove_dir(&staging)
            .with_context(|| format!("failed to remove {}", staging.display()))?;
        info!(
            "normalize_service_root: flattened {} wrapper directory {}",
            service_id,
            child_for_log.display()
        );
    }

    Ok(())
}

/// Tracks download/install progress for a software download operation.
pub struct DownloadProgress {
    total: AtomicU64,
    downloaded: AtomicU64,
    phase: std::sync::Mutex<String>,
}

impl DownloadProgress {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            downloaded: AtomicU64::new(0),
            phase: std::sync::Mutex::new("idle".to_string()),
        }
    }

    pub fn set_total(&self, total: u64) {
        self.total.store(total, Ordering::Relaxed);
    }

    pub fn set_downloaded(&self, downloaded: u64) {
        self.downloaded.store(downloaded, Ordering::Relaxed);
    }

    pub fn set_phase(&self, phase: &str) {
        if let Ok(mut p) = self.phase.lock() {
            *p = phase.to_string();
        }
    }

    pub fn get_progress(&self) -> (u64, u64, String) {
        let total = self.total.load(Ordering::Relaxed);
        let downloaded = self.downloaded.load(Ordering::Relaxed);
        let phase = self.phase.lock().map(|p| p.clone()).unwrap_or_default();
        (total, downloaded, phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_db() -> Arc<Database> {
        let dir = std::env::temp_dir().join(format!("winserver-runtime-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create temp dir");
        let db = Database::new(&dir.join("winserver.db")).expect("open db");
        db.run_migrations().expect("migrate db");
        Arc::new(db)
    }

    fn make_data_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("winserver-runtime-data-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("create data dir");
        dir
    }

    fn node_exe() -> Option<PathBuf> {
        std::process::Command::new("node")
            .arg("--version")
            .output()
            .ok()
            .filter(|output| output.status.success())?;

        #[cfg(windows)]
        {
            let output = std::process::Command::new("where")
                .arg("node")
                .output()
                .ok()?;
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| line.to_ascii_lowercase().ends_with(".exe"))
                .map(PathBuf::from)
        }

        #[cfg(not(windows))]
        {
            let output = std::process::Command::new("which")
                .arg("node")
                .output()
                .ok()?;
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(PathBuf::from)
        }
    }

    #[test]
    fn import_nginx_validates_and_updates_service_config() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping nginx import test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("nginx-1.26.3");
        fs::create_dir_all(root.join("conf")).expect("create nginx conf dir");
        fs::copy(&node, root.join("nginx.exe")).expect("copy fake nginx exe");
        fs::write(root.join("conf").join("nginx.conf"), "events {}\nhttp {}\n")
            .expect("write nginx conf");

        let manager = RuntimeManager::new(data_dir, db.clone());
        let manifest = manager
            .import_runtime(RuntimeType::Nginx, &root.to_string_lossy(), Some(8088))
            .expect("import nginx");

        assert_eq!(manifest.runtime_type, RuntimeType::Nginx);
        assert!(manifest.installed);
        assert!(manifest.entrypoint.ends_with("nginx.exe"));

        let service = db.get_service_config("nginx").expect("nginx service config");
        assert!(service.installed);
        assert_eq!(service.port, 8088);
        assert!(service.exe.ends_with("nginx.exe"));

        let config_files = db.list_config_files().expect("config files");
        assert!(config_files.iter().any(|file| {
            file.id == "nginx.conf" && file.path.ends_with("conf\\nginx.conf")
                || file.id == "nginx.conf" && file.path.ends_with("conf/nginx.conf")
        }));
    }

    #[test]
    fn import_php_creates_versioned_fastcgi_service() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping php import test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("php-8.2.12");
        fs::create_dir_all(&root).expect("create php dir");
        fs::copy(&node, root.join("php.exe")).expect("copy fake php exe");
        fs::copy(&node, root.join("php-cgi.exe")).expect("copy fake php cgi");
        fs::write(root.join("php.ini"), "memory_limit=128M\n").expect("write php.ini");

        let manager = RuntimeManager::new(data_dir, db.clone());
        let manifest = manager
            .import_runtime(RuntimeType::Php, &root.to_string_lossy(), Some(9088))
            .expect("import php");

        assert_eq!(manifest.runtime_type, RuntimeType::Php);
        assert!(manifest.id.starts_with("php-"));
        assert!(manifest.entrypoint.ends_with("php-cgi.exe"));

        let service = db
            .list_service_instances()
            .expect("services")
            .into_iter()
            .find(|svc| svc.id == manifest.id)
            .expect("php service");
        assert_eq!(service.service_type, "php");
        assert_eq!(service.port, 9088);
        assert!(service.installed);

        let config = db.get_service_config(&manifest.id).expect("php config");
        assert!(config.args.unwrap_or_default().contains("127.0.0.1:9088"));
    }

    #[test]
    fn import_php_reports_missing_key_file() {
        let Some(node) = node_exe() else {
            eprintln!("Skipping missing php file test because node is unavailable");
            return;
        };

        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("php-broken");
        fs::create_dir_all(&root).expect("create php dir");
        fs::copy(&node, root.join("php.exe")).expect("copy fake php exe");

        let manager = RuntimeManager::new(data_dir, db);
        let error = manager
            .import_runtime(RuntimeType::Php, &root.to_string_lossy(), Some(9089))
            .expect_err("missing php-cgi should fail");

        assert!(error.to_string().contains("php-cgi.exe"));
    }

    #[test]
    fn register_generic_redis_creates_editable_config_and_service_row() {
        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("redis-7.2.4");
        fs::create_dir_all(&root).expect("create redis dir");
        fs::write(root.join("redis-server.exe"), "").expect("write redis exe");

        let manager = RuntimeManager::new(data_dir, db.clone());
        manager
            .register_generic_service("redis", "redis", &root.to_string_lossy())
            .expect("register redis");

        let conf = root.join("redis.conf");
        assert!(conf.exists(), "redis.conf should be created when missing");
        let service = db.get_service_config("redis").expect("redis service config");
        assert!(service.installed);
        assert_eq!(service.args.as_deref(), Some("redis.conf"));
        let conf_str = conf.to_string_lossy().replace('\\', "/");
        assert_eq!(service.config_file.as_deref(), Some(conf_str.as_str()));
        assert!(db
            .list_config_files()
            .expect("config files")
            .iter()
            .any(|file| file.id == "redis.conf" && file.exists));
    }

    #[test]
    fn register_generic_minio_creates_env_and_full_start_args() {
        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("minio");
        fs::create_dir_all(&root).expect("create minio dir");
        fs::write(root.join("minio.exe"), "").expect("write minio exe");

        let manager = RuntimeManager::new(data_dir, db.clone());
        manager
            .register_generic_service("minio", "minio", &root.to_string_lossy())
            .expect("register minio");

        let env = root.join("minio.env");
        assert!(env.exists(), "minio.env should be created when missing");
        let service = db.get_service_config("minio").expect("minio service config");
        assert!(service.installed);
        let args = service.args.unwrap_or_default();
        let data_arg = root.join("data").to_string_lossy().replace('\\', "/");
        assert!(args.contains(&format!("server {}", data_arg)));
        assert!(args.contains("--address :9000"));
        assert!(args.contains("--console-address :9001"));
        let env_str = env.to_string_lossy().replace('\\', "/");
        assert_eq!(service.config_file.as_deref(), Some(env_str.as_str()));
        assert!(db
            .list_config_files()
            .expect("config files")
            .iter()
            .any(|file| file.id == "minio.env" && file.exists));
    }

    #[test]
    fn install_bundled_runtime_flattens_nested_wrappers_and_registers_service() {
        let db = make_db();
        let data_dir = make_data_dir();
        let runtime_dir = make_data_dir().join("bundles");
        let bundled_leaf = runtime_dir.join("redis-up").join("_up").join("_up");
        fs::create_dir_all(&bundled_leaf).expect("create nested bundled dir");
        fs::write(bundled_leaf.join("redis-server.exe"), "").expect("write redis exe");
        fs::write(bundled_leaf.join("redis.conf"), "port 6379\n").expect("write redis conf");

        let manager = RuntimeManager::new(data_dir.clone(), db.clone());
        let manifest = manager
            .install_bundled_runtime("redis", &runtime_dir)
            .expect("install redis bundled runtime");

        let target = data_dir.join("server").join("redis");
        assert!(manifest.installed);
        assert!(target.join("redis-server.exe").exists());
        assert!(target.join("redis.conf").exists());
        assert!(!target.join("_up").exists());

        let service = db.get_service_config("redis").expect("redis service config");
        assert!(service.installed);
        let target_str = target.to_string_lossy().replace('\\', "/");
        assert_eq!(service.cwd.as_deref(), Some(target_str.as_str()));
        assert!(db.get_software("redis").expect("redis software row").installed);
    }

    #[test]
    fn install_bundled_minio_marks_client_tool_in_same_managed_dir() {
        let db = make_db();
        let data_dir = make_data_dir();
        let runtime_dir = make_data_dir().join("bundles");
        let bundled_minio = runtime_dir.join("minio");
        fs::create_dir_all(&bundled_minio).expect("create minio bundle");
        fs::write(bundled_minio.join("minio.exe"), "").expect("write minio exe");
        fs::write(bundled_minio.join("mc.exe"), "").expect("write mc exe");
        fs::write(bundled_minio.join("minio.env"), default_minio_env()).expect("write minio env");

        let manager = RuntimeManager::new(data_dir.clone(), db.clone());
        manager
            .install_bundled_runtime("minio", &runtime_dir)
            .expect("install minio bundled runtime");

        let target = data_dir.join("server").join("minio");
        let target_str = target.to_string_lossy().to_string();
        let minio = db.get_software("minio").expect("minio software row");
        let mc = db.get_software("mc").expect("mc software row");

        assert!(target.join("minio.exe").exists());
        assert!(target.join("mc.exe").exists());
        assert!(minio.installed);
        assert!(mc.installed);
        assert_eq!(minio.install_path.as_deref(), Some(target_str.as_str()));
        assert_eq!(mc.install_path.as_deref(), Some(target_str.as_str()));
    }

    #[test]
    fn managed_mysql_ini_uses_current_install_dir() {
        let install_dir = PathBuf::from(r"C:\Users\12062\Desktop\WinServer\data\server\mysql80");
        let ini = render_managed_mysql_ini("mysql80", &install_dir);

        assert!(ini.contains("port=3306"));
        assert!(ini.contains("basedir=\"C:/Users/12062/Desktop/WinServer/data/server/mysql80\""));
        assert!(ini.contains("datadir=\"C:/Users/12062/Desktop/WinServer/data/server/mysql80/data\""));
        assert!(!ini.contains("phpstudy"));
        assert!(!ini.contains("D:/"));
    }

    #[test]
    fn register_generic_service_rejects_missing_executable() {
        let db = make_db();
        let data_dir = make_data_dir();
        let root = data_dir.join("redis-missing-exe");
        fs::create_dir_all(&root).expect("create redis dir");

        let manager = RuntimeManager::new(data_dir, db.clone());
        let error = manager
            .register_generic_service("redis", "redis", &root.to_string_lossy())
            .expect_err("missing redis-server.exe should fail");

        assert!(error.to_string().contains("服务可执行文件"));
        let service = db.get_service_config("redis").expect("redis service config");
        assert!(!service.installed);
    }
}
