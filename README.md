# Anchor 🚀

A beautiful TUI (Terminal User Interface) tool for managing ports and SSH tunnels on macOS.

![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-blue.svg)

## Features

- **Port Management**
  - View all listening and established ports
  - Filter ports by port number, process name, or PID
  - Kill processes occupying specific ports

- **SSH Tunnel Management**
  - Save frequently used SSH tunnel configurations
  - One-key connect/disconnect
  - Persistent configuration storage

## Installation

```bash
# Clone or download the project
cd anchor

# Build release version
cargo build --release

# Install to your PATH (optional)
cp target/release/anchor /usr/local/bin/
```

## Usage

```bash
# Run the application
anchor

# Or if not installed globally
./target/release/anchor
```

## Keyboard Shortcuts

### Global
| Key | Action |
|-----|--------|
| `Tab` | Switch between Ports/Tunnels tabs |
| `1` / `2` | Jump to Ports / Tunnels tab |
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `g` / `Home` | Go to first item |
| `G` / `End` | Go to last item |
| `q` | Quit |
| `Ctrl+C` | Force quit |

### Ports Tab
| Key | Action |
|-----|--------|
| `r` / `F5` | Refresh port list |
| `/` | Filter ports |
| `K` | Kill selected process |
| `Esc` | Clear filter |

### Tunnels Tab
| Key | Action |
|-----|--------|
| `a` | Add new tunnel |
| `c` | Connect selected tunnel |
| `d` | Disconnect selected tunnel |
| `x` | Delete selected tunnel |

## SSH Tunnel Configuration

When adding a new tunnel, you'll be prompted for:

1. **Tunnel name**: A friendly name for this tunnel (e.g., "dev-db")
2. **SSH host**: The SSH server (e.g., "user@jumphost.example.com")
3. **Local port**: The port on your Mac (e.g., "3306")
4. **Remote target**: The target host:port (e.g., "db-server:3306")

This creates an SSH local port forward equivalent to:
```bash
ssh -L 3306:db-server:3306 user@jumphost.example.com
```

## Configuration

Tunnel configurations are stored in:
```
~/.config/anchor/tunnels.json
```

## Requirements

- macOS (uses `lsof` for port detection)
- Rust 1.70 or later
- SSH client (for tunnel functionality)

## Tips

- Use **Shift+K** (capital K) to kill a process to avoid accidental termination
- Tunnels persist across restarts - just reconnect them
- Filter accepts port numbers, process names, and PIDs
- Connected tunnels show a green `●` indicator

## License

MIT
