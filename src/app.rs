use crate::port::PortInfo;
use crate::tunnel::{TunnelConfig, TunnelManager};
use anyhow::Result;
use std::process::Command;

#[derive(Clone, Copy, PartialEq)]
pub enum AppTab {
    Ports,
    Tunnels,
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    None,
    TunnelName,
    TunnelHost,
    TunnelLocalPort,
    TunnelRemotePort,
}

pub struct App {
    pub current_tab: AppTab,

    // Port list
    pub ports: Vec<PortInfo>,
    pub filtered_ports: Vec<PortInfo>,
    pub port_selected: usize,

    // Tunnel management
    pub tunnel_manager: TunnelManager,
    pub tunnel_selected: usize,

    // Filter
    pub show_filter: bool,
    pub filter_text: String,

    // Input dialog
    pub show_input: bool,
    pub input_mode: InputMode,
    pub input_prompt: String,
    pub input_buffer: String,

    // Confirmation dialog
    pub show_confirm: bool,
    pub confirm_message: String,
    pub pending_action: Option<PendingAction>,

    // New tunnel being created
    pub new_tunnel: Option<TunnelConfig>,

    // Status message
    pub status_message: String,
}

#[derive(Clone)]
pub enum PendingAction {
    KillProcess(i32),
    DeleteTunnel(String),
}

impl App {
    pub fn new() -> Result<Self> {
        let tunnel_manager = TunnelManager::load()?;

        Ok(Self {
            current_tab: AppTab::Ports,
            ports: Vec::new(),
            filtered_ports: Vec::new(),
            port_selected: 0,
            tunnel_manager,
            tunnel_selected: 0,
            show_filter: false,
            filter_text: String::new(),
            show_input: false,
            input_mode: InputMode::None,
            input_prompt: String::new(),
            input_buffer: String::new(),
            show_confirm: false,
            confirm_message: String::new(),
            pending_action: None,
            new_tunnel: None,
            status_message: String::from("Press ? for help"),
        })
    }

    pub fn refresh_ports(&mut self) -> Result<()> {
        self.ports = crate::port::get_listening_ports()?;
        self.apply_filter();
        self.status_message = format!("Found {} ports", self.ports.len());
        Ok(())
    }

    pub fn apply_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_ports = self.ports.clone();
        } else {
            let filter_lower = self.filter_text.to_lowercase();
            self.filtered_ports = self.ports
                .iter()
                .filter(|p| {
                    p.port.to_string().contains(&filter_lower)
                        || p.process_name.to_lowercase().contains(&filter_lower)
                        || p.pid.to_string().contains(&filter_lower)
                })
                .cloned()
                .collect();
        }

        // Adjust selection
        if self.port_selected >= self.filtered_ports.len() && !self.filtered_ports.is_empty() {
            self.port_selected = self.filtered_ports.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            AppTab::Ports => AppTab::Tunnels,
            AppTab::Tunnels => AppTab::Ports,
        };
    }

    pub fn prev_tab(&mut self) {
        self.next_tab();
    }

    pub fn select_next(&mut self) {
        match self.current_tab {
            AppTab::Ports => {
                if !self.filtered_ports.is_empty() {
                    self.port_selected = (self.port_selected + 1) % self.filtered_ports.len();
                }
            }
            AppTab::Tunnels => {
                let len = self.tunnel_manager.tunnels.len();
                if len > 0 {
                    self.tunnel_selected = (self.tunnel_selected + 1) % len;
                }
            }
        }
    }

    pub fn select_prev(&mut self) {
        match self.current_tab {
            AppTab::Ports => {
                if !self.filtered_ports.is_empty() {
                    self.port_selected = if self.port_selected == 0 {
                        self.filtered_ports.len() - 1
                    } else {
                        self.port_selected - 1
                    };
                }
            }
            AppTab::Tunnels => {
                let len = self.tunnel_manager.tunnels.len();
                if len > 0 {
                    self.tunnel_selected = if self.tunnel_selected == 0 {
                        len - 1
                    } else {
                        self.tunnel_selected - 1
                    };
                }
            }
        }
    }

    pub fn select_first(&mut self) {
        match self.current_tab {
            AppTab::Ports => self.port_selected = 0,
            AppTab::Tunnels => self.tunnel_selected = 0,
        }
    }

    pub fn select_last(&mut self) {
        match self.current_tab {
            AppTab::Ports => {
                if !self.filtered_ports.is_empty() {
                    self.port_selected = self.filtered_ports.len() - 1;
                }
            }
            AppTab::Tunnels => {
                let len = self.tunnel_manager.tunnels.len();
                if len > 0 {
                    self.tunnel_selected = len - 1;
                }
            }
        }
    }

    pub fn request_kill(&mut self) -> Result<()> {
        if let Some(port) = self.filtered_ports.get(self.port_selected) {
            self.confirm_message = format!(
                "Kill process '{}' (PID {}) on port {}?",
                port.process_name, port.pid, port.port
            );
            self.pending_action = Some(PendingAction::KillProcess(port.pid));
            self.show_confirm = true;
        }
        Ok(())
    }

    pub fn confirm_action(&mut self) -> Result<()> {
        self.show_confirm = false;

        if let Some(action) = self.pending_action.take() {
            match action {
                PendingAction::KillProcess(pid) => {
                    let output = Command::new("kill").arg("-9").arg(pid.to_string()).output()?;

                    if output.status.success() {
                        self.status_message = format!("Killed process {}", pid);
                        self.refresh_ports()?;
                    } else {
                        self.status_message = format!(
                            "Failed to kill process: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
                PendingAction::DeleteTunnel(name) => {
                    self.tunnel_manager.remove(&name);
                    self.tunnel_manager.save()?;
                    self.status_message = format!("Deleted tunnel '{}'", name);

                    if self.tunnel_selected >= self.tunnel_manager.tunnels.len()
                        && !self.tunnel_manager.tunnels.is_empty()
                    {
                        self.tunnel_selected = self.tunnel_manager.tunnels.len() - 1;
                    }
                }
            }
        }

        Ok(())
    }

    pub fn start_add_tunnel(&mut self) {
        self.new_tunnel = Some(TunnelConfig::default());
        self.input_mode = InputMode::TunnelName;
        self.input_prompt = String::from("Tunnel name:");
        self.input_buffer.clear();
        self.show_input = true;
    }

    pub fn submit_input(&mut self) -> Result<()> {
        let input = self.input_buffer.trim().to_string();

        if input.is_empty() {
            self.status_message = String::from("Input cannot be empty");
            return Ok(());
        }

        if let Some(ref mut tunnel) = self.new_tunnel {
            match self.input_mode {
                InputMode::TunnelName => {
                    tunnel.name = input;
                    self.input_mode = InputMode::TunnelHost;
                    self.input_prompt = String::from("SSH host (user@host):");
                    self.input_buffer.clear();
                }
                InputMode::TunnelHost => {
                    tunnel.ssh_host = input;
                    self.input_mode = InputMode::TunnelLocalPort;
                    self.input_prompt = String::from("Local port:");
                    self.input_buffer.clear();
                }
                InputMode::TunnelLocalPort => {
                    if let Ok(port) = input.parse::<u16>() {
                        tunnel.local_port = port;
                        self.input_mode = InputMode::TunnelRemotePort;
                        self.input_prompt = String::from("Remote port (host:port):");
                        self.input_buffer.clear();
                    } else {
                        self.status_message = String::from("Invalid port number");
                    }
                }
                InputMode::TunnelRemotePort => {
                    tunnel.remote_target = input;
                    // Save the tunnel
                    let tunnel_clone = tunnel.clone();
                    self.tunnel_manager.add(tunnel_clone);
                    self.tunnel_manager.save()?;
                    self.status_message = format!("Added tunnel '{}'", tunnel.name);
                    self.new_tunnel = None;
                    self.show_input = false;
                    self.input_mode = InputMode::None;
                }
                InputMode::None => {}
            }
        }

        Ok(())
    }

    pub fn cancel_input(&mut self) {
        self.show_input = false;
        self.input_mode = InputMode::None;
        self.new_tunnel = None;
        self.input_buffer.clear();
    }

    pub fn connect_tunnel(&mut self) -> Result<()> {
        if let Some(tunnel) = self.tunnel_manager.tunnels.get_mut(self.tunnel_selected) {
            if tunnel.is_connected() {
                self.status_message = format!("Tunnel '{}' is already connected", tunnel.name);
                return Ok(());
            }

            match tunnel.connect() {
                Ok(()) => {
                    self.status_message = format!("Connected tunnel '{}'", tunnel.name);
                }
                Err(e) => {
                    self.status_message = format!("Failed to connect: {}", e);
                }
            }
        }
        Ok(())
    }

    pub fn disconnect_tunnel(&mut self) -> Result<()> {
        if let Some(tunnel) = self.tunnel_manager.tunnels.get_mut(self.tunnel_selected) {
            if !tunnel.is_connected() {
                self.status_message = format!("Tunnel '{}' is not connected", tunnel.name);
                return Ok(());
            }

            match tunnel.disconnect() {
                Ok(()) => {
                    self.status_message = format!("Disconnected tunnel '{}'", tunnel.name);
                }
                Err(e) => {
                    self.status_message = format!("Failed to disconnect: {}", e);
                }
            }
        }
        Ok(())
    }

    pub fn request_delete_tunnel(&mut self) -> Result<()> {
        if let Some(tunnel) = self.tunnel_manager.tunnels.get(self.tunnel_selected) {
            self.confirm_message = format!("Delete tunnel '{}'?", tunnel.name);
            self.pending_action = Some(PendingAction::DeleteTunnel(tunnel.name.clone()));
            self.show_confirm = true;
        }
        Ok(())
    }
}
