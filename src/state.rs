//! Application state management
//!
//! Manages the overall state of rat-squad including sessions, worktrees, and UI.

use crate::agent::AgentRegistry;
use crate::config::Config;
use crate::error::{RatSquadError, Result};
use crate::ratterm_client::RattermClient;
use crate::session::{SessionConfig, SessionManager, SessionState};
use crate::worktree::{WorktreeConfig, WorktreeManager};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of state snapshots to keep (reserved for future use)
#[allow(dead_code)]
const MAX_STATE_HISTORY: usize = 100;

/// Application state
pub struct AppState {
    /// Configuration
    config: Config,
    /// Session manager
    session_manager: SessionManager,
    /// Worktree manager (optional, only when in git repo)
    worktree_manager: Option<WorktreeManager>,
    /// Agent registry
    agent_registry: AgentRegistry,
    /// Ratterm API client
    client: Option<RattermClient>,
    /// Current working directory
    cwd: String,
    /// State version (for change detection)
    version: u64,
}

impl AppState {
    /// Create a new application state
    pub fn new(config: Config, cwd: &str) -> Result<Self> {
        config.validate()?;

        let data_dir = Path::new(&config.data_dir);
        let session_manager = SessionManager::new(data_dir)?;

        let worktree_manager = WorktreeManager::new(Path::new(cwd)).ok();

        let mut agent_registry = AgentRegistry::new();
        for (name, _) in &config.agents {
            if let Some(ac) = config.get_agent_config(name) {
                let _ = agent_registry.register(name, ac);
            }
        }

        assert!(!cwd.is_empty(), "CWD must not be empty");

        Ok(Self {
            config,
            session_manager,
            worktree_manager,
            agent_registry,
            client: None,
            cwd: cwd.to_string(),
            version: 0,
        })
    }

    /// Initialize the ratterm API client
    pub fn init_client(&mut self, client: RattermClient) {
        self.client = Some(client);
        self.version += 1;
    }

    /// Get the ratterm client
    pub fn client(&self) -> Option<&RattermClient> {
        self.client.as_ref()
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the session manager
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get the session manager mutably
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    /// Get the worktree manager
    pub fn worktree_manager(&self) -> Option<&WorktreeManager> {
        self.worktree_manager.as_ref()
    }

    /// Get the worktree manager mutably
    pub fn worktree_manager_mut(&mut self) -> Option<&mut WorktreeManager> {
        self.worktree_manager.as_mut()
    }

    /// Get the agent registry
    pub fn agent_registry(&self) -> &AgentRegistry {
        &self.agent_registry
    }

    /// Get the current working directory
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    /// Get the state version
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Create a new session with the given configuration
    pub async fn create_session(&mut self, name: &str, agent_name: &str, yolo: bool) -> Result<String> {
        if name.is_empty() {
            return Err(RatSquadError::validation("Session name must not be empty"));
        }
        if agent_name.is_empty() {
            return Err(RatSquadError::validation("Agent name must not be empty"));
        }

        let _agent_exists = self
            .agent_registry
            .get(agent_name)
            .ok_or_else(|| RatSquadError::not_found(format!("Agent not found: {agent_name}")))?;

        let branch_name = format!("rat-squad/{}", name.replace(' ', "-"));
        let base_branch = self.config.default_base_branch.clone();
        let cwd_fallback = self.cwd.clone();

        let worktree_path = if let Some(wm) = self.worktree_manager_mut() {
            let wt_config = WorktreeConfig {
                branch_name: branch_name.clone(),
                base_branch,
            };
            let worktree = wm.create_worktree(wt_config)?;
            worktree.path().to_string_lossy().to_string()
        } else {
            cwd_fallback
        };

        let session_config = SessionConfig {
            name: name.to_string(),
            agent_type: agent_name.to_string(),
            work_dir: worktree_path,
            auto_accept: yolo || self.config.default_auto_accept,
        };

        let session_id = self.session_manager.create_session(session_config)?;

        if let Some(session) = self.session_manager.get_session_mut(&session_id) {
            session.set_branch_name(branch_name);
        }

        self.version += 1;

        Ok(session_id)
    }

    /// Start a session
    pub async fn start_session(&mut self, session_id: &str) -> Result<()> {
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| RatSquadError::not_found(format!("Session not found: {session_id}")))?;

        if !session.can_start() {
            return Err(RatSquadError::session(format!(
                "Session cannot be started in state: {:?}",
                session.state()
            )));
        }

        session.set_state(SessionState::Starting);

        if let Some(client) = &self.client {
            let tab = client.create_terminal_tab(Some(&session.config().name)).await?;
            session.set_tab_id(tab.tab_id);

            let agent_config = self
                .agent_registry
                .get(&session.config().agent_type)
                .ok_or_else(|| {
                    RatSquadError::not_found(format!(
                        "Agent not found: {}",
                        session.config().agent_type
                    ))
                })?;

            let mut cmd_parts = vec![agent_config.command.clone()];
            cmd_parts.extend(agent_config.args.clone());

            if session.config().auto_accept && !agent_config.args.iter().any(|a| {
                a.contains("--dangerously-skip-permissions")
                    || a.contains("--yes")
                    || a.contains("-y")
            }) {
                if agent_config.agent_type == crate::agent::AgentType::Claude {
                    cmd_parts.push("--dangerously-skip-permissions".to_string());
                } else if agent_config.agent_type == crate::agent::AgentType::Aider {
                    cmd_parts.push("--yes".to_string());
                }
            }

            let cd_cmd = format!("cd {}\n", session.config().work_dir);
            client.send_keys(&cd_cmd).await?;

            let agent_cmd = format!("{}\n", cmd_parts.join(" "));
            client.send_keys(&agent_cmd).await?;

            session.set_state(SessionState::Running);
            client.set_status(&format!("rat-squad: {} running", session.config().name)).await?;
        }

        self.version += 1;
        Ok(())
    }

    /// Stop a session
    pub async fn stop_session(&mut self, session_id: &str) -> Result<()> {
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| RatSquadError::not_found(format!("Session not found: {session_id}")))?;

        if !session.can_stop() {
            return Err(RatSquadError::session(format!(
                "Session cannot be stopped in state: {:?}",
                session.state()
            )));
        }

        if let (Some(client), Some(tab_id)) = (&self.client, session.tab_id()) {
            client.send_keys("\x03").await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            client.close_terminal_tab(tab_id).await?;
        }

        session.set_state(SessionState::Stopped);
        self.version += 1;

        Ok(())
    }

    /// Remove a session and its worktree
    pub async fn remove_session(&mut self, session_id: &str) -> Result<()> {
        let (is_running, branch_name) = {
            let session = self
                .session_manager
                .get_session(session_id)
                .ok_or_else(|| RatSquadError::not_found(format!("Session not found: {session_id}")))?;
            (session.is_running(), session.branch_name().map(String::from))
        };

        if is_running {
            self.stop_session(session_id).await?;
        }

        if let (Some(wm), Some(branch)) = (self.worktree_manager_mut(), branch_name.as_deref()) {
            let _ = wm.remove_worktree(branch);
        }

        self.session_manager.remove_session(session_id)?;
        self.version += 1;

        Ok(())
    }

    /// Get a summary of running sessions
    pub fn get_status_summary(&self) -> String {
        let running = self.session_manager.find_running().len();
        let total = self.session_manager.session_count();
        format!("rat-squad: {running}/{total} agents")
    }
}

/// Thread-safe application state wrapper
pub type SharedState = Arc<RwLock<AppState>>;

/// Create a new shared state
pub fn create_shared_state(config: Config, cwd: &str) -> Result<SharedState> {
    let state = AppState::new(config, cwd)?;
    Ok(Arc::new(RwLock::new(state)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let config = Config::default();
        let state = AppState::new(config, "/tmp");
        assert!(state.is_ok());
    }

    #[test]
    fn test_app_state_version() {
        let config = Config::default();
        let state = AppState::new(config, "/tmp").unwrap();
        assert_eq!(state.version(), 0);
    }

    #[test]
    fn test_status_summary() {
        let config = Config::default();
        let state = AppState::new(config, "/tmp").unwrap();
        let summary = state.get_status_summary();
        assert!(summary.contains("rat-squad"));
    }
}
