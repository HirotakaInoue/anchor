use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TunnelConfig {
    pub name: String,
    pub ssh_host: String,
    pub local_port: u16,
    pub remote_target: String, // host:port format

    #[serde(skip)]
    pub process: Option<u32>, // PID of the SSH process
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            ssh_host: String::new(),
            local_port: 0,
            remote_target: String::new(),
            process: None,
        }
    }
}

impl TunnelConfig {
    pub fn connect(&mut self) -> Result<()> {
        // Build SSH command for local port forwarding
        // ssh -L local_port:remote_host:remote_port -N -f ssh_host
        let forward_spec = format!("{}:{}", self.local_port, self.remote_target);

        let child: Child = Command::new("ssh")
            .args([
                "-L",
                &forward_spec,
                "-N",          // No remote command
                "-f",          // Go to background
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "ServerAliveInterval=60",
                "-o",
                "ServerAliveCountMax=3",
                &self.ssh_host,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        // Give SSH a moment to establish or fail
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Check if the port is now in use (indicating success)
        if self.is_connected() {
            // Find the actual SSH process PID
            if let Some(pid) = self.find_ssh_pid() {
                self.process = Some(pid);
            }
            Ok(())
        } else {
            // Try to get error message
            let output = child.wait_with_output()?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!(
                "SSH tunnel failed to establish: {}",
                stderr.trim()
            ))
        }
    }

    pub fn disconnect(&mut self) -> Result<()> {
        if let Some(pid) = self.process {
            Command::new("kill").arg(pid.to_string()).output()?;
            self.process = None;
        } else if let Some(pid) = self.find_ssh_pid() {
            Command::new("kill").arg(pid.to_string()).output()?;
        }

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        // Check if there's an SSH process listening on our local port
        self.find_ssh_pid().is_some()
    }

    fn find_ssh_pid(&self) -> Option<u32> {
        // Use lsof to find SSH process on our local port
        let output = Command::new("lsof")
            .args(["-iTCP", "-P", "-n", &format!("-i:{}", self.local_port)])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == "ssh" {
                if let Ok(pid) = parts[1].parse::<u32>() {
                    return Some(pid);
                }
            }
        }

        None
    }

    pub fn status_string(&self) -> &'static str {
        if self.is_connected() {
            "● Connected"
        } else {
            "○ Disconnected"
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TunnelManager {
    pub tunnels: Vec<TunnelConfig>,

    #[serde(skip)]
    config_path: PathBuf,
}

impl TunnelManager {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let mut manager: TunnelManager = serde_json::from_str(&content)?;
            manager.config_path = config_path;

            // Update connection status for each tunnel
            for tunnel in &mut manager.tunnels {
                if let Some(pid) = tunnel.find_ssh_pid() {
                    tunnel.process = Some(pid);
                }
            }

            Ok(manager)
        } else {
            Ok(Self {
                tunnels: Vec::new(),
                config_path,
            })
        }
    }

    pub fn save(&self) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self)?;
        fs::write(&self.config_path, content)?;

        Ok(())
    }

    pub fn add(&mut self, tunnel: TunnelConfig) {
        // Remove existing tunnel with same name
        self.tunnels.retain(|t| t.name != tunnel.name);
        self.tunnels.push(tunnel);
    }

    pub fn remove(&mut self, name: &str) {
        // Disconnect first if connected
        if let Some(tunnel) = self.tunnels.iter_mut().find(|t| t.name == name) {
            let _ = tunnel.disconnect();
        }
        self.tunnels.retain(|t| t.name != name);
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

        Ok(config_dir.join("anchor").join("tunnels.json"))
    }
}
