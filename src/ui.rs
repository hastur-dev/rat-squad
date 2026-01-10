//! UI handling for rat-squad
//!
//! Manages the user interface within ratterm's terminal.

use crate::error::{RatSquadError, Result};
use crate::session::Session;
use crate::state::SharedState;

/// Maximum menu items to display
const MAX_MENU_ITEMS: usize = 20;

/// UI command that can be executed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    /// Create a new session
    NewSession {
        /// Session name
        name: String,
        /// Agent type to use
        agent: String,
        /// Whether to enable yolo mode
        yolo: bool,
    },
    /// Start a session
    StartSession {
        /// Session ID to start
        session_id: String,
    },
    /// Stop a session
    StopSession {
        /// Session ID to stop
        session_id: String,
    },
    /// Remove a session
    RemoveSession {
        /// Session ID to remove
        session_id: String,
    },
    /// Switch to a session tab
    SwitchSession {
        /// Session ID to switch to
        session_id: String,
    },
    /// List all sessions
    ListSessions,
    /// Show help
    Help,
    /// Quit the extension
    Quit,
}

impl UiCommand {
    /// Parse a command from string input
    pub fn parse(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err(RatSquadError::validation("Empty command"));
        }

        match parts[0] {
            "new" | "n" => {
                if parts.len() < 2 {
                    return Err(RatSquadError::validation("Usage: new <name> [agent] [--yolo]"));
                }
                let name = parts[1].to_string();
                let agent = parts.get(2).map(|s| s.to_string()).unwrap_or_else(|| "claude".to_string());
                let yolo = parts.iter().any(|p| *p == "--yolo" || *p == "-y");

                assert!(!name.is_empty(), "Name parsed but empty");

                Ok(Self::NewSession { name, agent, yolo })
            }
            "start" | "s" => {
                if parts.len() < 2 {
                    return Err(RatSquadError::validation("Usage: start <session_id>"));
                }
                Ok(Self::StartSession {
                    session_id: parts[1].to_string(),
                })
            }
            "stop" | "x" => {
                if parts.len() < 2 {
                    return Err(RatSquadError::validation("Usage: stop <session_id>"));
                }
                Ok(Self::StopSession {
                    session_id: parts[1].to_string(),
                })
            }
            "remove" | "rm" | "r" => {
                if parts.len() < 2 {
                    return Err(RatSquadError::validation("Usage: remove <session_id>"));
                }
                Ok(Self::RemoveSession {
                    session_id: parts[1].to_string(),
                })
            }
            "switch" | "sw" => {
                if parts.len() < 2 {
                    return Err(RatSquadError::validation("Usage: switch <session_id>"));
                }
                Ok(Self::SwitchSession {
                    session_id: parts[1].to_string(),
                })
            }
            "list" | "ls" | "l" => Ok(Self::ListSessions),
            "help" | "h" | "?" => Ok(Self::Help),
            "quit" | "q" | "exit" => Ok(Self::Quit),
            _ => Err(RatSquadError::validation(format!("Unknown command: {}", parts[0]))),
        }
    }
}

/// Format a session for display
pub fn format_session(session: &Session) -> String {
    let state_icon = match session.state() {
        crate::session::SessionState::Pending => "⏸",
        crate::session::SessionState::Starting => "🔄",
        crate::session::SessionState::Running => "▶",
        crate::session::SessionState::Paused => "⏸",
        crate::session::SessionState::Stopped => "⏹",
        crate::session::SessionState::Error => "❌",
    };

    let yolo_indicator = if session.config().auto_accept {
        " [YOLO]"
    } else {
        ""
    };

    format!(
        "{} {} ({}) - {}{} [{}]",
        state_icon,
        session.config().name,
        session.config().agent_type,
        session.config().work_dir,
        yolo_indicator,
        &session.id()[..8]
    )
}

/// Format the help message
pub fn format_help() -> String {
    r#"
rat-squad - Multi-agent AI squad manager

Commands:
  new <name> [agent] [--yolo]  Create a new agent session
  start <id>                   Start a session
  stop <id>                    Stop a session
  remove <id>                  Remove a session and its worktree
  switch <id>                  Switch to a session's terminal tab
  list                         List all sessions
  help                         Show this help message
  quit                         Exit rat-squad

Agents:
  claude  - Anthropic's Claude Code (default)
  aider   - Aider AI coding assistant
  codex   - OpenAI Codex
  gemini  - Google Gemini
  custom  - Custom agent (configure in config.yaml)

Options:
  --yolo, -y  Auto-accept all changes (dangerous!)

Shortcuts:
  n  = new
  s  = start
  x  = stop
  r  = remove
  sw = switch
  l  = list
  h  = help
  q  = quit

Examples:
  new feature-auth claude --yolo
  new bugfix aider
  start abc12345
  list
"#
    .to_string()
}

/// Format the session list
pub fn format_session_list(sessions: &[&Session]) -> String {
    if sessions.is_empty() {
        return "No sessions found. Use 'new <name> [agent]' to create one.".to_string();
    }

    let mut output = String::from("Sessions:\n");
    output.push_str("─────────────────────────────────────────────────\n");

    for (i, session) in sessions.iter().take(MAX_MENU_ITEMS).enumerate() {
        output.push_str(&format!("{:2}. {}\n", i + 1, format_session(session)));
    }

    if sessions.len() > MAX_MENU_ITEMS {
        output.push_str(&format!("... and {} more\n", sessions.len() - MAX_MENU_ITEMS));
    }

    output.push_str("─────────────────────────────────────────────────");
    output
}

/// UI handler for processing commands
pub struct UiHandler {
    state: SharedState,
}

impl UiHandler {
    /// Create a new UI handler
    pub fn new(state: SharedState) -> Self {
        Self { state }
    }

    /// Process a command and return output
    pub async fn process(&self, command: UiCommand) -> Result<String> {
        match command {
            UiCommand::NewSession { name, agent, yolo } => {
                let mut state = self.state.write().await;
                let session_id = state.create_session(&name, &agent, yolo).await?;
                Ok(format!("Created session: {} ({})", name, &session_id[..8]))
            }
            UiCommand::StartSession { session_id } => {
                let mut state = self.state.write().await;
                let full_id = self.resolve_session_id(&state, &session_id)?;
                state.start_session(&full_id).await?;
                Ok(format!("Started session: {}", &full_id[..8]))
            }
            UiCommand::StopSession { session_id } => {
                let mut state = self.state.write().await;
                let full_id = self.resolve_session_id(&state, &session_id)?;
                state.stop_session(&full_id).await?;
                Ok(format!("Stopped session: {}", &full_id[..8]))
            }
            UiCommand::RemoveSession { session_id } => {
                let mut state = self.state.write().await;
                let full_id = self.resolve_session_id(&state, &session_id)?;
                state.remove_session(&full_id).await?;
                Ok(format!("Removed session: {}", &full_id[..8]))
            }
            UiCommand::SwitchSession { session_id } => {
                let state = self.state.read().await;
                let full_id = self.resolve_session_id(&state, &session_id)?;
                if let Some(session) = state.session_manager().get_session(&full_id) {
                    if let (Some(client), Some(tab_id)) = (state.client(), session.tab_id()) {
                        client.switch_terminal_tab(tab_id).await?;
                        return Ok(format!("Switched to session: {}", session.config().name));
                    }
                }
                Err(RatSquadError::session("Session has no associated tab"))
            }
            UiCommand::ListSessions => {
                let state = self.state.read().await;
                let sessions = state.session_manager().list_sessions();
                Ok(format_session_list(&sessions))
            }
            UiCommand::Help => Ok(format_help()),
            UiCommand::Quit => Ok("Goodbye!".to_string()),
        }
    }

    /// Resolve a partial session ID to full ID
    fn resolve_session_id(
        &self,
        state: &crate::state::AppState,
        partial: &str,
    ) -> Result<String> {
        if partial.is_empty() {
            return Err(RatSquadError::validation("Session ID must not be empty"));
        }

        let sessions = state.session_manager().list_sessions();
        let matches: Vec<_> = sessions
            .iter()
            .filter(|s| s.id().starts_with(partial) || s.config().name == partial)
            .collect();

        match matches.len() {
            0 => Err(RatSquadError::not_found(format!("Session not found: {partial}"))),
            1 => Ok(matches[0].id().to_string()),
            _ => Err(RatSquadError::validation(format!(
                "Ambiguous session ID: {partial} (matches {} sessions)",
                matches.len()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_new_command() {
        let cmd = UiCommand::parse("new feature-auth claude --yolo").unwrap();
        assert!(matches!(cmd, UiCommand::NewSession { name, agent, yolo } if name == "feature-auth" && agent == "claude" && yolo));
    }

    #[test]
    fn test_parse_list_command() {
        let cmd = UiCommand::parse("list").unwrap();
        assert_eq!(cmd, UiCommand::ListSessions);
    }

    #[test]
    fn test_parse_help_command() {
        let cmd = UiCommand::parse("help").unwrap();
        assert_eq!(cmd, UiCommand::Help);
    }

    #[test]
    fn test_parse_shortcut() {
        let cmd = UiCommand::parse("l").unwrap();
        assert_eq!(cmd, UiCommand::ListSessions);
    }

    #[test]
    fn test_parse_unknown_fails() {
        let result = UiCommand::parse("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_help() {
        let help = format_help();
        assert!(help.contains("rat-squad"));
        assert!(help.contains("new"));
        assert!(help.contains("list"));
    }
}
