//! Session management for AI agent instances
//!
//! Each session represents an isolated AI agent working in its own git worktree.

use crate::error::{RatSquadError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Maximum number of sessions
const MAX_SESSIONS: usize = 50;

/// Session state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session created but not started
    Pending,
    /// Session is starting up
    Starting,
    /// Session is running
    Running,
    /// Session is paused
    Paused,
    /// Session has stopped
    Stopped,
    /// Session encountered an error
    Error,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Pending
    }
}

/// Configuration for creating a new session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Human-readable name for the session
    pub name: String,
    /// Type of AI agent (claude, aider, codex, etc.)
    pub agent_type: String,
    /// Working directory for the session
    pub work_dir: String,
    /// Whether to auto-accept changes (yolo mode)
    pub auto_accept: bool,
}

impl SessionConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(RatSquadError::validation("Session name must not be empty"));
        }
        if self.agent_type.is_empty() {
            return Err(RatSquadError::validation("Agent type must not be empty"));
        }
        assert!(!self.name.is_empty(), "Name validated but empty");
        assert!(!self.agent_type.is_empty(), "Agent type validated but empty");
        Ok(())
    }
}

/// Represents a single AI agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    id: String,
    /// Session configuration
    config: SessionConfig,
    /// Current state
    state: SessionState,
    /// Branch name for the worktree
    branch_name: Option<String>,
    /// Terminal tab ID in ratterm
    tab_id: Option<u32>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the given configuration
    pub fn new(config: SessionConfig) -> Result<Self> {
        config.validate()?;

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        assert!(!id.is_empty(), "UUID generation failed");
        assert!(now.timestamp() > 0, "Invalid timestamp");

        Ok(Self {
            id,
            config,
            state: SessionState::Pending,
            branch_name: None,
            tab_id: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the session configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// Get the current state
    pub fn state(&self) -> SessionState {
        self.state
    }

    /// Set the session state
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    /// Get the branch name
    pub fn branch_name(&self) -> Option<&str> {
        self.branch_name.as_deref()
    }

    /// Set the branch name
    pub fn set_branch_name(&mut self, name: String) {
        assert!(!name.is_empty(), "Branch name must not be empty");
        self.branch_name = Some(name);
        self.updated_at = Utc::now();
    }

    /// Get the terminal tab ID
    pub fn tab_id(&self) -> Option<u32> {
        self.tab_id
    }

    /// Set the terminal tab ID
    pub fn set_tab_id(&mut self, id: u32) {
        self.tab_id = Some(id);
        self.updated_at = Utc::now();
    }

    /// Get the creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Get the last update timestamp
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Check if the session is running
    pub fn is_running(&self) -> bool {
        self.state == SessionState::Running
    }

    /// Check if the session can be started
    pub fn can_start(&self) -> bool {
        matches!(self.state, SessionState::Pending | SessionState::Stopped)
    }

    /// Check if the session can be stopped
    pub fn can_stop(&self) -> bool {
        matches!(self.state, SessionState::Running | SessionState::Paused)
    }
}

/// Manages multiple sessions
pub struct SessionManager {
    /// Map of session ID to session
    sessions: HashMap<String, Session>,
    /// Data directory for persistence
    data_dir: std::path::PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(data_dir: &Path) -> Result<Self> {
        if !data_dir.exists() {
            std::fs::create_dir_all(data_dir)?;
        }

        assert!(data_dir.exists(), "Data directory creation failed");

        Ok(Self {
            sessions: HashMap::with_capacity(MAX_SESSIONS),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Get the number of sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Create a new session
    pub fn create_session(&mut self, config: SessionConfig) -> Result<String> {
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(RatSquadError::limit_exceeded(format!(
                "Maximum sessions ({MAX_SESSIONS}) reached"
            )));
        }

        let session = Session::new(config)?;
        let id = session.id().to_string();

        assert!(!id.is_empty(), "Session ID must not be empty");
        assert!(
            !self.sessions.contains_key(&id),
            "Duplicate session ID generated"
        );

        self.sessions.insert(id.clone(), session);
        self.persist()?;

        Ok(id)
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// Remove a session
    pub fn remove_session(&mut self, id: &str) -> Result<()> {
        if self.sessions.remove(id).is_none() {
            return Err(RatSquadError::not_found(format!("Session not found: {id}")));
        }
        self.persist()?;
        Ok(())
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// Find sessions by agent type
    pub fn find_by_agent_type(&self, agent_type: &str) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| s.config.agent_type == agent_type)
            .collect()
    }

    /// Find running sessions
    pub fn find_running(&self) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| s.is_running())
            .collect()
    }

    /// Persist sessions to disk
    fn persist(&self) -> Result<()> {
        let sessions_file = self.data_dir.join("sessions.json");
        let sessions_vec: Vec<&Session> = self.sessions.values().collect();
        let json = serde_json::to_string_pretty(&sessions_vec)?;
        std::fs::write(&sessions_file, json)?;
        Ok(())
    }

    /// Load sessions from disk
    pub fn load(&mut self) -> Result<()> {
        let sessions_file = self.data_dir.join("sessions.json");
        if !sessions_file.exists() {
            return Ok(());
        }

        let json = std::fs::read_to_string(&sessions_file)?;
        let sessions_vec: Vec<Session> = serde_json::from_str(&json)?;

        self.sessions.clear();
        for session in sessions_vec {
            let id = session.id().to_string();
            self.sessions.insert(id, session);
        }

        assert!(
            self.sessions.len() <= MAX_SESSIONS,
            "Loaded more sessions than allowed"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state_default() {
        let state = SessionState::default();
        assert_eq!(state, SessionState::Pending);
    }

    #[test]
    fn test_session_config_validation() {
        let config = SessionConfig {
            name: "test".to_string(),
            agent_type: "claude".to_string(),
            work_dir: "/tmp".to_string(),
            auto_accept: false,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_session_creation() {
        let config = SessionConfig {
            name: "test".to_string(),
            agent_type: "claude".to_string(),
            work_dir: "/tmp".to_string(),
            auto_accept: false,
        };
        let session = Session::new(config).unwrap();
        assert!(!session.id().is_empty());
        assert_eq!(session.state(), SessionState::Pending);
    }

    #[test]
    fn test_session_state_transitions() {
        let config = SessionConfig {
            name: "test".to_string(),
            agent_type: "claude".to_string(),
            work_dir: "/tmp".to_string(),
            auto_accept: false,
        };
        let mut session = Session::new(config).unwrap();

        assert!(session.can_start());
        assert!(!session.can_stop());

        session.set_state(SessionState::Running);
        assert!(!session.can_start());
        assert!(session.can_stop());
        assert!(session.is_running());
    }
}
