use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::{info, warn};

const HOSTS_PATH: &str = "C:/Windows/System32/drivers/etc/hosts";
const MARKER_PREFIX: &str = "# winserver";

/// Manages the Windows hosts file for local domain resolution.
pub struct HostsManager {
    hosts_path: PathBuf,
}

impl HostsManager {
    pub fn new() -> Self {
        Self {
            hosts_path: PathBuf::from(HOSTS_PATH),
        }
    }

    /// Create a HostsManager with a custom hosts file path (useful for testing).
    pub fn with_path(hosts_path: PathBuf) -> Self {
        Self { hosts_path }
    }

    /// Add or update a domain entry in the hosts file.
    /// Uses marker comments for idempotent management.
    /// Skips localhost and raw IP addresses.
    pub fn sync_hosts(&self, domain: &str) -> Result<()> {
        let clean_domain = domain.trim();

        // Skip localhost and IP addresses
        if clean_domain.is_empty()
            || clean_domain.eq_ignore_ascii_case("localhost")
            || self.is_ip_address(clean_domain)
        {
            info!("Skipping hosts sync for: {}", clean_domain);
            return Ok(());
        }

        let marker = format!("{} {}", MARKER_PREFIX, clean_domain);
        let mut lines = self.read_hosts_lines()?;

        // Remove any existing entry with our marker
        lines.retain(|line| !line.contains(&marker));

        // Add the new entry
        let entry = format!("127.0.0.1 {} {}", clean_domain, marker);
        lines.push(entry);

        self.write_hosts_lines(&lines)?;

        info!("Hosts synced for domain: {}", clean_domain);
        Ok(())
    }

    /// Remove a domain entry from the hosts file.
    pub fn remove_hosts(&self, domain: &str) -> Result<()> {
        let clean_domain = domain.trim();

        if clean_domain.is_empty() {
            return Ok(());
        }

        let marker = format!("{} {}", MARKER_PREFIX, clean_domain);
        let mut lines = self.read_hosts_lines()?;

        let original_len = lines.len();
        lines.retain(|line| !line.contains(&marker));

        if lines.len() != original_len {
            self.write_hosts_lines(&lines)?;
            info!("Hosts removed for domain: {}", clean_domain);
        } else {
            info!("No hosts entry found for domain: {}", clean_domain);
        }

        Ok(())
    }

    // ---- Private helpers ----

    /// Read the hosts file into a vector of lines.
    fn read_hosts_lines(&self) -> Result<Vec<String>> {
        if !self.hosts_path.exists() {
            warn!("Hosts file not found: {:?}", self.hosts_path);
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&self.hosts_path)
            .with_context(|| format!("Failed to read hosts file: {:?}", self.hosts_path))?;

        Ok(content.lines().map(|l| l.to_string()).collect())
    }

    /// Write lines back to the hosts file, with a backup first.
    fn write_hosts_lines(&self, lines: &[String]) -> Result<()> {
        // Backup before writing
        self.backup_hosts_file()?;

        // Ensure parent directory exists
        if let Some(parent) = self.hosts_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Join lines with Windows-style line endings
        let content = lines.join("\r\n");
        // Ensure file ends with a newline
        let content = if content.ends_with("\r\n") {
            content
        } else {
            format!("{}\r\n", content)
        };

        fs::write(&self.hosts_path, content)
            .with_context(|| format!("Failed to write hosts file: {:?}", self.hosts_path))?;

        Ok(())
    }

    /// Create a backup of the hosts file before modification.
    fn backup_hosts_file(&self) -> Result<()> {
        if !self.hosts_path.exists() {
            return Ok(());
        }

        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = self.hosts_path.with_extension(format!("bak.{}", timestamp));

        fs::copy(&self.hosts_path, &backup_path)
            .with_context(|| {
                format!(
                    "Failed to backup hosts file from {:?} to {:?}",
                    self.hosts_path, backup_path
                )
            })?;

        info!("Hosts file backed up to {:?}", backup_path);
        Ok(())
    }

    /// Check if a string looks like an IPv4 address.
    fn is_ip_address(&self, s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u8>().is_ok())
    }
}

impl Default for HostsManager {
    fn default() -> Self {
        Self::new()
    }
}
