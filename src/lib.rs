//! rat-squad: Multi-agent AI squad manager for ratterm
//!
//! This extension enables running multiple AI coding agents (Claude, Aider, Codex, Gemini)
//! in parallel with isolated git worktrees, similar to claude-squad but integrated
//! with ratterm's extension system.
//!
//! # Features
//!
//! - Run multiple AI agents in parallel
//! - Each agent gets an isolated git worktree
//! - "Yolo mode" for auto-accepting changes
//! - Review changes before merging
//! - Integrated with ratterm's terminal tabs
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────┐
//! │                      rat-squad                            │
//! ├──────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │
//! │  │  Session 1  │  │  Session 2  │  │  Session N  │       │
//! │  │  (Claude)   │  │   (Aider)   │  │  (Codex)    │       │
//! │  │  worktree/1 │  │  worktree/2 │  │  worktree/n │       │
//! │  └─────────────┘  └─────────────┘  └─────────────┘       │
//! ├──────────────────────────────────────────────────────────┤
//! │                   Ratterm REST API                        │
//! │              http://127.0.0.1:7878/api/v1                 │
//! └──────────────────────────────────────────────────────────┘
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

pub mod agent;
pub mod config;
pub mod error;
pub mod providers;
pub mod ratterm_client;
pub mod session;
pub mod state;
pub mod ui;
pub mod worktree;

pub use error::{RatSquadError, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default API port for ratterm
pub const DEFAULT_API_PORT: u16 = 7878;

/// Maximum number of concurrent sessions
pub const MAX_SESSIONS: usize = 50;

/// Maximum number of worktrees per repository
pub const MAX_WORKTREES: usize = 20;

/// Pre-allocated buffer sizes for performance
pub mod buffers {
    /// Terminal buffer size (lines)
    pub const TERMINAL_BUFFER_LINES: usize = 10000;

    /// Command buffer size (bytes)
    pub const COMMAND_BUFFER_SIZE: usize = 4096;

    /// Response buffer size (bytes)
    pub const RESPONSE_BUFFER_SIZE: usize = 65536;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_not_empty() {
        assert!(!VERSION.is_empty(), "Version must not be empty");
    }

    #[test]
    fn test_constants_valid() {
        assert!(DEFAULT_API_PORT > 0, "API port must be positive");
        assert!(MAX_SESSIONS > 0, "Max sessions must be positive");
        assert!(MAX_WORKTREES > 0, "Max worktrees must be positive");
    }

    #[test]
    fn test_buffer_sizes_valid() {
        assert!(buffers::TERMINAL_BUFFER_LINES > 0);
        assert!(buffers::COMMAND_BUFFER_SIZE > 0);
        assert!(buffers::RESPONSE_BUFFER_SIZE > 0);
    }
}
