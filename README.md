# Kafka Eye 🔍

A powerful TUI (Terminal User Interface) Kafka client built in Rust. Manage and inspect Kafka clusters directly from your terminal — no JVM required.

## Features

- **Multi-cluster Management** — Add, edit, delete cluster configs with support for SASL/PLAIN, SCRAM-SHA-256/512, SSL/TLS authentication
- **Cluster Dashboard** — View broker health, controller info, topic and partition counts
- **Topic Management** — List, create, delete topics; inspect partition details (leader, ISR, replicas)
- **Message Browser** — Real-time message consuming with offset modes (earliest/latest), JSON pretty-printing, key/value filtering
- **Message Producer** — Send test messages with key, value, and headers
- **Consumer Group Monitoring** — View group states, member counts, and partition lag
- **Keyboard-driven** — Vim-style navigation (j/k), Tab for panel switching, shortcuts for all actions

## Prerequisites

- Rust 1.70+
- CMake
- C compiler (gcc/clang on Linux/macOS, MSVC on Windows)

### Install build dependencies

**Ubuntu/Debian:**
```bash
apt-get install cmake build-essential libcurl4-openssl-dev pkg-config
```

**macOS:**
```bash
brew install cmake
```

**Windows (PowerShell as Admin):**
```powershell
# Install CMake
winget install Kitware.CMake

# Install Strawberry Perl (required to compile OpenSSL from source)
winget install StrawberryPerl.StrawberryPerl

# Restart PowerShell after installing, then build
cargo run
```

> **Note for Windows**: OpenSSL is compiled from source automatically via `ssl-vendored`.
> This requires Perl. [Strawberry Perl](https://strawberryperl.com) is the standard
> choice in the Rust/Windows ecosystem. After installing, restart your terminal so
> `perl` is on `PATH`.

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --release
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `?` | Toggle help overlay |
| `Tab` | Switch focus (sidebar ↔ content) |
| `j/↓` | Move down |
| `k/↑` | Move up |
| `Enter` | Select / drill in |
| `Esc` | Back / close dialog |
| `/` | Focus search bar |
| `q` | Quit |

### Topics
| Key | Action |
|-----|--------|
| `c` | Create topic |
| `d` | Delete topic |
| `m` | Browse messages |
| `r` | Refresh |

### Messages
| Key | Action |
|-----|--------|
| `s` | Start/stop consuming |
| `p` | Produce message |
| `1` | Consume from earliest |
| `2` | Consume from latest |
| `Enter` | Toggle detail view |

## Configuration

Cluster configurations are stored at `~/.config/kafka-eye/config.toml`.

## License

MIT
