use anyhow::Result;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct PortInfo {
    pub port: u16,
    pub pid: i32,
    pub process_name: String,
    pub protocol: String,
    pub state: String,
    pub local_address: String,
    pub foreign_address: String,
}

pub fn get_listening_ports() -> Result<Vec<PortInfo>> {
    let mut ports = Vec::new();

    // Run lsof to get listening ports
    // -iTCP -iUDP: Show TCP and UDP
    // -sTCP:LISTEN,ESTABLISHED: Show listen and established states
    // -P: Don't convert port numbers to names
    // -n: Don't convert IP addresses to names
    let output = Command::new("lsof")
        .args(["-iTCP", "-iUDP", "-P", "-n"])
        .output()?;

    if !output.status.success() {
        // lsof might require sudo for some ports, but we'll work with what we get
        return Ok(ports);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines().skip(1) {
        // Skip header line
        if let Some(port_info) = parse_lsof_line(line) {
            // Avoid duplicates
            if !ports.iter().any(|p: &PortInfo| {
                p.port == port_info.port
                    && p.pid == port_info.pid
                    && p.state == port_info.state
            }) {
                ports.push(port_info);
            }
        }
    }

    // Sort by port number
    ports.sort_by(|a, b| a.port.cmp(&b.port));

    Ok(ports)
}

fn parse_lsof_line(line: &str) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 9 {
        return None;
    }

    let process_name = parts[0].to_string();
    let pid: i32 = parts[1].parse().ok()?;

    // Find the NAME column (usually last or second to last)
    // Format is typically: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
    let name_idx = parts.len() - 1;
    let name = parts[name_idx];

    // Parse the address:port
    // Format can be: *:port, localhost:port, 127.0.0.1:port, [::]:port, etc.
    let (local_address, port) = if name.contains("->") {
        // Established connection: local->remote
        let conn_parts: Vec<&str> = name.split("->").collect();
        let local = conn_parts.first()?;
        parse_address_port(local)?
    } else {
        parse_address_port(name)?
    };

    // Determine protocol from TYPE column
    let protocol = if parts.len() > 4 {
        if parts[4].contains("TCP") || parts[7].contains("TCP") {
            "TCP".to_string()
        } else if parts[4].contains("UDP") || parts[7].contains("UDP") {
            "UDP".to_string()
        } else {
            "???".to_string()
        }
    } else {
        "???".to_string()
    };

    // Determine state
    let state = if name.contains("->") {
        "ESTABLISHED".to_string()
    } else if parts.iter().any(|p| p.contains("LISTEN")) {
        "LISTEN".to_string()
    } else if parts.iter().any(|p| p.contains("ESTABLISHED")) {
        "ESTABLISHED".to_string()
    } else {
        // Check the last or second-to-last part for state info
        let state_part = if parts.len() > name_idx + 1 {
            parts[name_idx]
        } else if parts.len() > 1 {
            parts[parts.len() - 2]
        } else {
            ""
        };

        if state_part.contains("LISTEN") {
            "LISTEN".to_string()
        } else {
            "UNKNOWN".to_string()
        }
    };

    // Get foreign address for established connections
    let foreign_address = if name.contains("->") {
        let conn_parts: Vec<&str> = name.split("->").collect();
        conn_parts.get(1).unwrap_or(&"").to_string()
    } else {
        String::new()
    };

    Some(PortInfo {
        port,
        pid,
        process_name,
        protocol,
        state,
        local_address,
        foreign_address,
    })
}

fn parse_address_port(addr: &str) -> Option<(String, u16)> {
    // Handle IPv6 format like [::1]:port or [::]:port
    if addr.starts_with('[') {
        let bracket_end = addr.find(']')?;
        let ip = &addr[1..bracket_end];
        let port_str = addr.get(bracket_end + 2..)?; // Skip ']:' 
        let port: u16 = port_str.parse().ok()?;
        return Some((ip.to_string(), port));
    }

    // Handle IPv4 format like 127.0.0.1:port or *:port
    let last_colon = addr.rfind(':')?;
    let ip = &addr[..last_colon];
    let port_str = &addr[last_colon + 1..];

    // Handle cases where port might have extra info
    let port_str = port_str.split('(').next()?;
    let port: u16 = port_str.parse().ok()?;

    Some((ip.to_string(), port))
}

pub fn check_port(port: u16) -> Result<Option<PortInfo>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-iUDP", "-P", "-n", &format!("-i:{}", port)])
        .output()?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines().skip(1) {
        if let Some(port_info) = parse_lsof_line(line) {
            return Ok(Some(port_info));
        }
    }

    Ok(None)
}
