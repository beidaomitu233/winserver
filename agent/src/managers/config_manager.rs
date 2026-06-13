use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

/// Manages reading and writing of configuration files with backup support.
pub struct ConfigManager;

impl ConfigManager {
    pub fn new() -> Self {
        Self
    }

    /// Read a configuration file and return its content as a string.
    pub fn read_config(&self, path: &Path) -> Result<String> {
        if !path.exists() {
            anyhow::bail!("Config file not found: {:?}", path);
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        info!("Read config file: {:?}", path);
        Ok(content)
    }

    /// Write content to a configuration file, creating a backup first.
    pub fn write_config(&self, path: &Path, content: &str) -> Result<()> {
        // Backup existing file before overwriting
        if path.exists() {
            self.backup_file(path)?;
        }

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        fs::write(path, content)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        info!("Wrote config file: {:?}", path);
        Ok(())
    }

    /// Create a timestamped backup of a file.
    fn backup_file(&self, path: &Path) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = path.with_extension(format!("bak.{}", timestamp));

        fs::copy(path, &backup_path)
            .with_context(|| format!("Failed to backup {:?} to {:?}", path, backup_path))?;

        info!("Config backed up: {:?} -> {:?}", path, backup_path);
        Ok(())
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
