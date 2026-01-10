# rat-squad

Multi-agent AI squad manager for ratterm - run Claude, Aider, Codex, and Gemini in parallel with isolated git worktrees.

## Overview

rat-squad is a ratterm extension that enables running multiple AI coding agents simultaneously. Each agent runs in its own isolated git worktree, preventing conflicts and allowing parallel task execution. Inspired by [claude-squad](https://github.com/smtg-ai/claude-squad), reimplemented as a native Rust extension for ratterm.

## Features

- **Multiple Agents**: Run Claude, Aider, Codex, Gemini, or custom agents in parallel
- **Isolated Worktrees**: Each agent gets its own git worktree for conflict-free development
- **Yolo Mode**: Auto-accept changes for hands-free operation (use with caution!)
- **Terminal Tabs**: Each agent runs in its own ratterm terminal tab
- **Session Management**: Create, start, stop, and remove agent sessions
- **Persistent State**: Sessions survive restarts

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                      rat-squad                            │
├──────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
│  │  Session 1  │  │  Session 2  │  │  Session N  │       │
│  │  (Claude)   │  │   (Aider)   │  │  (Codex)    │       │
│  │  worktree/1 │  │  worktree/2 │  │  worktree/n │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
├──────────────────────────────────────────────────────────┤
│                   Ratterm REST API                        │
│              http://127.0.0.1:7878/api/v1                 │
└──────────────────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- Rust 1.85+ (for building)
- ratterm 1.0.0+
- Git (for worktree support)
- At least one AI agent installed (claude, aider, codex, or gemini)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/hastur-dev/rat-squad
cd rat-squad

# Build in release mode
cargo build --release

# The binary will be at target/release/rat-squad
```

### Install as ratterm Extension

```bash
# Copy binary to extensions directory
mkdir -p ~/.ratterm/extensions/rat-squad
cp target/release/rat-squad ~/.ratterm/extensions/rat-squad/
cp extension.toml ~/.ratterm/extensions/rat-squad/

# Or install via ratterm
rat ext install hastur-dev/rat-squad
```

### Hotkey

rat-squad auto-registers **F3** as its default hotkey. Press F3 in ratterm to launch rat-squad in a new terminal tab.

You can customize the hotkey in `~/.ratrc`:

```
addon.rat-squad = f5|~/.ratterm/extensions/rat-squad/rat-squad
```

## Configuration

Copy `config.yaml.example` to `~/.rat-squad/config.yaml` and customize:

```yaml
# Data directory for sessions and worktrees
data_dir: "~/.rat-squad/data"

# Maximum concurrent sessions (1-50)
max_sessions: 10

# Default agent
default_agent: "claude"

# Default yolo mode (dangerous!)
default_auto_accept: false

# Default base branch for worktrees
default_base_branch: "main"

# Agent configurations
agents:
  claude:
    command: "claude"
    args: []
  aider:
    command: "aider"
    args: []
```

## Usage

### Interactive Mode

When run as a ratterm extension, rat-squad starts in interactive mode:

```
rat-squad> help

Commands:
  new <name> [agent] [--yolo]  Create a new agent session
  start <id>                   Start a session
  stop <id>                    Stop a session
  remove <id>                  Remove a session and its worktree
  switch <id>                  Switch to a session's terminal tab
  list                         List all sessions
  help                         Show this help message
  quit                         Exit rat-squad
```

### CLI Mode

```bash
# Create a new session
rat-squad new feature-auth claude --yolo

# List sessions
rat-squad list

# Start a session
rat-squad start abc12345

# Stop a session
rat-squad stop abc12345

# Remove a session
rat-squad remove abc12345
```

### Examples

```bash
# Create multiple agents working on different features
rat-squad new auth-system claude
rat-squad new api-refactor aider --yolo
rat-squad new docs-update gemini

# List all sessions
rat-squad list

# Start all sessions
rat-squad start auth
rat-squad start api
rat-squad start docs

# Switch to a session's tab
rat-squad switch auth
```

## Agents

### Supported Agents

| Agent   | Command | Yolo Flag                         |
|---------|---------|-----------------------------------|
| Claude  | `claude`| `--dangerously-skip-permissions`  |
| Aider   | `aider` | `--yes`                           |
| Codex   | `codex` | (varies)                          |
| Gemini  | `gemini`| (varies)                          |
| Custom  | (user)  | (user-defined)                    |

### Custom Agents

Add custom agents in `config.yaml`:

```yaml
agents:
  my-custom-agent:
    command: "/path/to/agent"
    args:
      - "--custom-flag"
    env:
      MY_API_KEY: "${MY_API_KEY}"
```

## How It Works

1. **Session Creation**: Creates a new git worktree branched from your base branch
2. **Agent Spawning**: Opens a new ratterm terminal tab and starts the agent
3. **Isolation**: Each agent works in its isolated worktree, preventing conflicts
4. **Session Management**: Track, switch between, and manage multiple agents

## Troubleshooting

### Agent not found

Ensure the agent CLI is installed and in your PATH:

```bash
# Check if claude is installed
which claude

# Check if aider is installed
which aider
```

### Worktree creation fails

Ensure you're in a git repository and have committed changes:

```bash
git status
git log --oneline -1
```

### API connection fails

Ensure ratterm is running and the extension has the correct API token:

```bash
# Check environment variables
echo $RATTERM_API_URL
echo $RATTERM_API_TOKEN
```

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=rat_squad=debug cargo run
```

### Code Structure

```
src/
├── main.rs           # Entry point and CLI
├── lib.rs            # Library root
├── error.rs          # Error types
├── config.rs         # Configuration management
├── agent.rs          # AI agent definitions
├── session.rs        # Session management
├── worktree.rs       # Git worktree operations
├── ratterm_client.rs # Ratterm REST API client
├── state.rs          # Application state
└── ui.rs             # UI and command parsing
```

## License

AGPL-3.0

## Acknowledgments

- Inspired by [claude-squad](https://github.com/smtg-ai/claude-squad)
- Built for [ratterm](https://github.com/hastur-dev/ratterm)
