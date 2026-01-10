# Dependencies

This document lists all dependencies required by rat-squad, their purpose, and installation instructions.

## Runtime Dependencies

### Required System Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| Git  | Worktree management | System package manager |
| ratterm | Host application | See ratterm docs |

### AI Agents (at least one required)

| Agent | Purpose | Installation |
|-------|---------|--------------|
| Claude Code | Anthropic's AI coding assistant | `npm install -g @anthropic-ai/claude-code` |
| Aider | AI pair programming tool | `pip install aider-chat` |
| Codex | OpenAI's coding assistant | See OpenAI docs |
| Gemini | Google's AI assistant | See Google docs |

## Build Dependencies

### Rust Toolchain

Requires Rust 1.85 or later.

**Installation (all platforms):**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
```

**Windows (alternative):**
```powershell
winget install Rustlang.Rustup
```

## Rust Crate Dependencies

Listed in `Cargo.toml`, automatically managed by Cargo.

### Core Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.12 | HTTP client for ratterm REST API |
| `serde` | 1.0 | Serialization/deserialization |
| `serde_json` | 1.0 | JSON handling |
| `serde_yaml` | 0.9 | YAML config parsing |
| `tokio` | 1.0 | Async runtime |
| `thiserror` | 2.0 | Error derive macros |
| `anyhow` | 1.0 | Error handling |
| `tracing` | 0.1 | Logging/tracing |
| `tracing-subscriber` | 0.3 | Log output formatting |
| `clap` | 4.0 | CLI argument parsing |
| `uuid` | 1.0 | Session ID generation |
| `dirs` | 6.0 | Platform directories |
| `chrono` | 0.4 | Date/time handling |
| `regex` | 1.0 | Regular expressions |

### Development Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `proptest` | 1.0 | Property-based testing |
| `mockall` | 0.13 | Mocking for unit tests |
| `tempfile` | 3.0 | Temporary directories for tests |
| `wiremock` | 0.6 | HTTP mocking |
| `tokio-test` | 0.4 | Async test utilities |

## Installation Instructions

### POSIX (Linux/macOS)

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 2. Install Git (if not present)
# Debian/Ubuntu:
sudo apt install git
# macOS:
brew install git
# Fedora:
sudo dnf install git

# 3. Install at least one AI agent
# Claude Code:
npm install -g @anthropic-ai/claude-code
# Aider:
pip install aider-chat

# 4. Clone and build rat-squad
git clone https://github.com/LecherousCthulhu/rat-squad
cd rat-squad
cargo build --release

# 5. Install to ratterm extensions
mkdir -p ~/.ratterm/extensions/rat-squad
cp target/release/rat-squad ~/.ratterm/extensions/rat-squad/
cp extension.toml ~/.ratterm/extensions/rat-squad/

# 6. Copy example config
mkdir -p ~/.rat-squad
cp config.yaml.example ~/.rat-squad/config.yaml
```

### Windows (PowerShell)

```powershell
# 1. Install Rust
winget install Rustlang.Rustup
# Or download from https://rustup.rs

# 2. Install Git (if not present)
winget install Git.Git

# 3. Install at least one AI agent
# Claude Code:
npm install -g @anthropic-ai/claude-code
# Aider:
pip install aider-chat

# 4. Clone and build rat-squad
git clone https://github.com/LecherousCthulhu/rat-squad
cd rat-squad
cargo build --release

# 5. Install to ratterm extensions
$extDir = "$env:USERPROFILE\.ratterm\extensions\rat-squad"
New-Item -ItemType Directory -Force -Path $extDir
Copy-Item "target\release\rat-squad.exe" "$extDir\"
Copy-Item "extension.toml" "$extDir\"

# 6. Copy example config
$configDir = "$env:USERPROFILE\.rat-squad"
New-Item -ItemType Directory -Force -Path $configDir
Copy-Item "config.yaml.example" "$configDir\config.yaml"
```

## Verifying Installation

```bash
# Check Rust version
rustc --version  # Should be >= 1.85

# Check Git version
git --version

# Check agent installations
claude --version  # or
aider --version

# Test rat-squad
./target/release/rat-squad --help
```

## Updating Dependencies

```bash
# Update Rust toolchain
rustup update

# Update Cargo dependencies
cargo update

# Rebuild
cargo build --release
```

## Troubleshooting

### Cargo build fails

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Missing native TLS

On some Linux systems:
```bash
# Debian/Ubuntu
sudo apt install pkg-config libssl-dev

# Fedora
sudo dnf install pkg-config openssl-devel
```

### Agent not in PATH

Ensure the agent binary is in your system PATH:
```bash
# Check PATH
echo $PATH

# Add to PATH (bash)
export PATH="$PATH:/path/to/agent"

# Make permanent (add to ~/.bashrc or ~/.zshrc)
```
