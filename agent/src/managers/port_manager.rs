use std::net::{TcpStream, Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use tracing::{debug, info};

use shared::types::PortCheckResult;

/// Manages port availability checks.
pub struct PortManager;

impl PortManager {
    pub fn new() -> Self {
        Self
    }

    /// Check if a port is open (i.e., something is listening on it).
    /// Returns `true` if a TCP connection can be established, meaning the port
    /// is in use.
    pub fn is_port_open(&self, port: u16, host: &str, timeout_ms: u64) -> bool {
        let addr = format!("{}:{}", host, port);
        let timeout = Duration::from_millis(timeout_ms);

        match host.parse::<Ipv4Addr>() {
            Ok(ip) => {
                let socket_addr = SocketAddrV4::new(ip, port);
                TcpStream::connect_timeout(&socket_addr.into(), timeout).is_ok()
            }
            Err(_) => {
                // Fallback: try parsing as full socket address
                debug!("Parsing host '{}' as IPv4 failed, trying as SocketAddr", host);
                let socket_addr: std::net::SocketAddr = match addr.parse() {
                    Ok(a) => a,
                    Err(_) => {
                        debug!("Failed to parse address: {}", addr);
                        return false;
                    }
                };
                TcpStream::connect_timeout(&socket_addr, timeout).is_ok()
            }
        }
    }

    /// Check a port and return a structured result.
    pub fn check_port(&self, port: u16) -> PortCheckResult {
        let is_open = self.is_port_open(port, "127.0.0.1", 150);

        info!(
            "Port check: {} is {}",
            port,
            if is_open { "open/in use" } else { "closed/available" }
        );

        // Attempt to find the process using the port via netstat
        let (pid, process_name) = if is_open {
            self.find_port_owner(port)
        } else {
            (None, None)
        };

        PortCheckResult {
            port,
            is_open,
            available: !is_open,
            pid,
            process_name,
            owner_type: if is_open {
                Some("external_process".to_string())
            } else {
                None
            },
            owner_id: None,
        }
    }

    /// Try to find the process that owns a port using `netstat -ano`.
    fn find_port_owner(&self, port: u16) -> (Option<u32>, Option<String>) {
        let output = match std::process::Command::new("netstat.exe")
            .args(["-ano", "-p", "TCP"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
        {
            Ok(o) => o,
            Err(_) => return (None, None),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let listen_pattern = format!(":{}", port);

        for line in stdout.lines() {
            // Look for LISTENING lines that match our port
            if line.contains("LISTENING") && line.contains(&listen_pattern) {
                // netstat -ano output format: Proto  Local Address  Foreign Address  State  PID
                // The PID is the last column
                if let Some(pid_str) = line.split_whitespace().last() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        // Try to get the process name
                        let process_name = self.get_process_name(pid);
                        return (Some(pid), process_name);
                    }
                }
            }
        }

        (None, None)
    }

    /// Get the process name for a given PID using tasklist.
    fn get_process_name(&self, pid: u32) -> Option<String> {
        let output = std::process::Command::new("tasklist.exe")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?;

        // CSV format: "name.exe","pid","session","session#","mem"
        let name = first_line.split(',').next()?;
        Some(name.trim_matches('"').to_string())
    }

    /// Kill a process by PID using taskkill.
    pub fn kill_process(&self, pid: u32) -> anyhow::Result<()> {
        info!("Attempting to kill process PID {}", pid);

        let output = std::process::Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;

        if output.status.success() {
            info!("Successfully killed process PID {}", pid);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("无法终止进程 {}: {}", pid, stderr)
        }
    }
}

impl Default for PortManager {
    fn default() -> Self {
        Self::new()
    }
}
