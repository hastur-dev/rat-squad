//! AI agent definitions and spawning
//!
//! Supports Claude, ChatGPT, Gemini, Aider, Codex, and custom agents.

use crate::error::{RatSquadError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

/// Maximum number of registered agents
const MAX_AGENTS: usize = 20;

/// Maximum number of arguments per agent
const MAX_ARGS: usize = 50;

/// Agent type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Anthropic's Claude Code CLI
    Claude,
    /// OpenAI ChatGPT (via shell-gpt or similar)
    ChatGPT,
    /// Google Gemini CLI
    Gemini,
    /// Aider AI coding assistant
    Aider,
    /// OpenAI Codex
    Codex,
    /// Custom agent
    Custom,
}

impl AgentType {
    /// Parse agent type from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" => Ok(Self::Claude),
            "chatgpt" | "gpt" | "openai" | "sgpt" => Ok(Self::ChatGPT),
            "gemini" | "google" => Ok(Self::Gemini),
            "aider" => Ok(Self::Aider),
            "codex" => Ok(Self::Codex),
            "custom" => Ok(Self::Custom),
            _ => Err(RatSquadError::agent(format!("Unknown agent type: {s}"))),
        }
    }

    /// Get display name for the agent
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::ChatGPT => "ChatGPT",
            Self::Gemini => "Gemini",
            Self::Aider => "Aider",
            Self::Codex => "Codex",
            Self::Custom => "Custom",
        }
    }

    /// Get the default command for this agent type
    pub fn default_command(&self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::ChatGPT => "sgpt",  // shell-gpt CLI
            Self::Gemini => "gemini",
            Self::Aider => "aider",
            Self::Codex => "codex",
            Self::Custom => "",
        }
    }

    /// Get the yolo/auto-accept flag for this agent
    pub fn yolo_flag(&self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("--dangerously-skip-permissions"),
            Self::ChatGPT => Some("--no-interaction"),
            Self::Gemini => None,  // Gemini doesn't have a yolo flag yet
            Self::Aider => Some("--yes"),
            Self::Codex => Some("--yes"),
            Self::Custom => None,
        }
    }

    /// Get environment variable name for API key
    pub fn api_key_env(&self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("ANTHROPIC_API_KEY"),
            Self::ChatGPT => Some("OPENAI_API_KEY"),
            Self::Gemini => Some("GOOGLE_API_KEY"),
            Self::Aider => None,  // Uses OpenAI key internally
            Self::Codex => Some("OPENAI_API_KEY"),
            Self::Custom => None,
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Claude => "claude",
            Self::ChatGPT => "chatgpt",
            Self::Gemini => "gemini",
            Self::Aider => "aider",
            Self::Codex => "codex",
            Self::Custom => "custom",
        };
        write!(f, "{name}")
    }
}

/// Configuration for an AI agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Type of agent
    pub agent_type: AgentType,
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
}

impl AgentConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.command.is_empty() {
            return Err(RatSquadError::validation("Agent command must not be empty"));
        }
        if self.args.len() > MAX_ARGS {
            return Err(RatSquadError::validation(format!(
                "Too many arguments (max {MAX_ARGS})"
            )));
        }
        assert!(!self.command.is_empty(), "Command validated but empty");
        Ok(())
    }
}

/// Command to execute for an agent
#[derive(Debug, Clone)]
pub struct AgentCommand {
    /// Program to execute
    pub program: String,
    /// Arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: Vec<(String, String)>,
    /// Working directory
    pub working_dir: String,
}

/// Represents an AI agent instance
#[derive(Debug, Clone)]
pub struct Agent {
    /// Unique identifier
    id: String,
    /// Agent configuration
    config: AgentConfig,
}

impl Agent {
    /// Create a new agent
    pub fn new(config: AgentConfig) -> Result<Self> {
        config.validate()?;

        let id = uuid::Uuid::new_v4().to_string();
        assert!(!id.is_empty(), "UUID generation failed");

        Ok(Self { id, config })
    }

    /// Get the agent ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the agent type
    pub fn agent_type(&self) -> AgentType {
        self.config.agent_type
    }

    /// Check if yolo mode is enabled (auto-accept changes)
    pub fn is_yolo_mode(&self) -> bool {
        self.config.args.iter().any(|arg| {
            arg.contains("--dangerously-skip-permissions")
                || arg.contains("--yes")
                || arg.contains("-y")
                || arg.contains("--auto-accept")
        })
    }

    /// Build the command to execute the agent
    pub fn build_command(&self, working_dir: &str) -> AgentCommand {
        assert!(!working_dir.is_empty(), "Working directory must not be empty");

        AgentCommand {
            program: self.config.command.clone(),
            args: self.config.args.clone(),
            env: self.config.env.clone(),
            working_dir: working_dir.to_string(),
        }
    }
}

/// Information about agent availability
#[derive(Debug, Clone)]
pub struct AgentAvailability {
    /// Agent type
    pub agent_type: AgentType,
    /// Whether the CLI is installed
    pub cli_available: bool,
    /// Whether the API key is configured
    pub api_key_configured: bool,
    /// Path to the CLI if found
    pub cli_path: Option<String>,
}

impl AgentAvailability {
    /// Check if the agent is ready to use
    pub fn is_ready(&self) -> bool {
        self.cli_available
    }

    /// Get status message
    pub fn status_message(&self) -> String {
        if !self.cli_available {
            format!("{} CLI not found", self.agent_type.display_name())
        } else if !self.api_key_configured {
            if let Some(env_var) = self.agent_type.api_key_env() {
                format!("{} ready (set {} for API access)", self.agent_type.display_name(), env_var)
            } else {
                format!("{} ready", self.agent_type.display_name())
            }
        } else {
            format!("{} ready", self.agent_type.display_name())
        }
    }
}

/// Check if a command is available in PATH
fn command_exists(cmd: &str) -> Option<String> {
    #[cfg(windows)]
    let which_cmd = "where";
    #[cfg(not(windows))]
    let which_cmd = "which";

    Command::new(which_cmd)
        .arg(cmd)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.lines().next().unwrap_or("").trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
}

/// Check if an environment variable is set and non-empty
fn env_var_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Registry of available AI agents
pub struct AgentRegistry {
    /// Registered agents by name
    agents: HashMap<String, AgentConfig>,
}

impl AgentRegistry {
    /// Create a new registry with default agents
    pub fn new() -> Self {
        let mut agents = HashMap::with_capacity(MAX_AGENTS);

        // Claude Code - Anthropic's official CLI
        agents.insert(
            "claude".to_string(),
            AgentConfig {
                agent_type: AgentType::Claude,
                command: "claude".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        // ChatGPT via shell-gpt (sgpt) - popular CLI for ChatGPT
        agents.insert(
            "chatgpt".to_string(),
            AgentConfig {
                agent_type: AgentType::ChatGPT,
                command: "sgpt".to_string(),
                args: vec!["--repl".to_string(), "temp".to_string()],
                env: vec![],
            },
        );

        // Gemini - Google's CLI
        agents.insert(
            "gemini".to_string(),
            AgentConfig {
                agent_type: AgentType::Gemini,
                command: "gemini".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        // Aider - AI pair programming
        agents.insert(
            "aider".to_string(),
            AgentConfig {
                agent_type: AgentType::Aider,
                command: "aider".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        // Codex - OpenAI Codex
        agents.insert(
            "codex".to_string(),
            AgentConfig {
                agent_type: AgentType::Codex,
                command: "codex".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        Self { agents }
    }

    /// Check availability of all registered agents
    pub fn check_availability(&self) -> Vec<AgentAvailability> {
        self.agents
            .values()
            .map(|config| {
                let cli_path = command_exists(&config.command);
                let api_key_configured = config
                    .agent_type
                    .api_key_env()
                    .map(env_var_set)
                    .unwrap_or(true);

                AgentAvailability {
                    agent_type: config.agent_type,
                    cli_available: cli_path.is_some(),
                    api_key_configured,
                    cli_path,
                }
            })
            .collect()
    }

    /// Check availability of a specific agent
    pub fn check_agent_availability(&self, name: &str) -> Option<AgentAvailability> {
        self.agents.get(name).map(|config| {
            let cli_path = command_exists(&config.command);
            let api_key_configured = config
                .agent_type
                .api_key_env()
                .map(env_var_set)
                .unwrap_or(true);

            AgentAvailability {
                agent_type: config.agent_type,
                cli_available: cli_path.is_some(),
                api_key_configured,
                cli_path,
            }
        })
    }

    /// Get list of available (ready to use) agents
    pub fn available_agents(&self) -> Vec<String> {
        self.check_availability()
            .into_iter()
            .filter(|a| a.is_ready())
            .map(|a| a.agent_type.to_string())
            .collect()
    }

    /// Get an agent configuration by name
    pub fn get(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.get(name)
    }

    /// Register a custom agent
    pub fn register(&mut self, name: &str, config: AgentConfig) -> Result<()> {
        if name.is_empty() {
            return Err(RatSquadError::validation("Agent name must not be empty"));
        }
        if self.agents.contains_key(name) {
            return Err(RatSquadError::already_exists(format!(
                "Agent already registered: {name}"
            )));
        }
        if self.agents.len() >= MAX_AGENTS {
            return Err(RatSquadError::limit_exceeded(format!(
                "Maximum agents ({MAX_AGENTS}) reached"
            )));
        }

        config.validate()?;
        self.agents.insert(name.to_string(), config);
        Ok(())
    }

    /// Unregister an agent
    pub fn unregister(&mut self, name: &str) -> Result<()> {
        if self.agents.remove(name).is_none() {
            return Err(RatSquadError::not_found(format!("Agent not found: {name}")));
        }
        Ok(())
    }

    /// List all registered agent names
    pub fn list(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create default Claude agent config
pub fn claude_default() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec![],
        env: vec![],
    }
}

/// Create Claude agent config with yolo mode
pub fn claude_yolo() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec!["--dangerously-skip-permissions".to_string()],
        env: vec![],
    }
}

/// Create default Aider agent config
pub fn aider_default() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::Aider,
        command: "aider".to_string(),
        args: vec![],
        env: vec![],
    }
}

/// Create Aider agent config with yolo mode
pub fn aider_yolo() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::Aider,
        command: "aider".to_string(),
        args: vec!["--yes".to_string()],
        env: vec![],
    }
}

/// Create default ChatGPT agent config (using shell-gpt)
pub fn chatgpt_default() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::ChatGPT,
        command: "sgpt".to_string(),
        args: vec!["--repl".to_string(), "temp".to_string()],
        env: vec![],
    }
}

/// Create ChatGPT agent config with code mode
pub fn chatgpt_code() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::ChatGPT,
        command: "sgpt".to_string(),
        args: vec!["--repl".to_string(), "temp".to_string(), "--code".to_string()],
        env: vec![],
    }
}

/// Create default Gemini agent config
pub fn gemini_default() -> AgentConfig {
    AgentConfig {
        agent_type: AgentType::Gemini,
        command: "gemini".to_string(),
        args: vec![],
        env: vec![],
    }
}

/// Create agent config with yolo mode for any agent type
pub fn with_yolo(mut config: AgentConfig) -> AgentConfig {
    if let Some(flag) = config.agent_type.yolo_flag() {
        if !config.args.iter().any(|a| a == flag) {
            config.args.push(flag.to_string());
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!(AgentType::from_str("claude").unwrap(), AgentType::Claude);
        assert_eq!(AgentType::from_str("CLAUDE").unwrap(), AgentType::Claude);
        assert_eq!(AgentType::from_str("claude-code").unwrap(), AgentType::Claude);
        assert_eq!(AgentType::from_str("chatgpt").unwrap(), AgentType::ChatGPT);
        assert_eq!(AgentType::from_str("gpt").unwrap(), AgentType::ChatGPT);
        assert_eq!(AgentType::from_str("sgpt").unwrap(), AgentType::ChatGPT);
        assert_eq!(AgentType::from_str("gemini").unwrap(), AgentType::Gemini);
        assert_eq!(AgentType::from_str("aider").unwrap(), AgentType::Aider);
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::Claude.to_string(), "claude");
        assert_eq!(AgentType::ChatGPT.to_string(), "chatgpt");
        assert_eq!(AgentType::Gemini.to_string(), "gemini");
        assert_eq!(AgentType::Aider.to_string(), "aider");
    }

    #[test]
    fn test_agent_type_display_name() {
        assert_eq!(AgentType::Claude.display_name(), "Claude Code");
        assert_eq!(AgentType::ChatGPT.display_name(), "ChatGPT");
        assert_eq!(AgentType::Gemini.display_name(), "Gemini");
    }

    #[test]
    fn test_agent_type_yolo_flags() {
        assert_eq!(AgentType::Claude.yolo_flag(), Some("--dangerously-skip-permissions"));
        assert_eq!(AgentType::ChatGPT.yolo_flag(), Some("--no-interaction"));
        assert_eq!(AgentType::Aider.yolo_flag(), Some("--yes"));
        assert_eq!(AgentType::Gemini.yolo_flag(), None);
    }

    #[test]
    fn test_agent_type_api_keys() {
        assert_eq!(AgentType::Claude.api_key_env(), Some("ANTHROPIC_API_KEY"));
        assert_eq!(AgentType::ChatGPT.api_key_env(), Some("OPENAI_API_KEY"));
        assert_eq!(AgentType::Gemini.api_key_env(), Some("GOOGLE_API_KEY"));
    }

    #[test]
    fn test_agent_config_validation() {
        let config = AgentConfig {
            agent_type: AgentType::Claude,
            command: "claude".to_string(),
            args: vec![],
            env: vec![],
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_agent_yolo_detection() {
        let config = AgentConfig {
            agent_type: AgentType::Claude,
            command: "claude".to_string(),
            args: vec!["--dangerously-skip-permissions".to_string()],
            env: vec![],
        };
        let agent = Agent::new(config).unwrap();
        assert!(agent.is_yolo_mode());
    }

    #[test]
    fn test_agent_registry_default() {
        let registry = AgentRegistry::new();
        assert!(registry.get("claude").is_some());
        assert!(registry.get("chatgpt").is_some());
        assert!(registry.get("gemini").is_some());
        assert!(registry.get("aider").is_some());
    }

    #[test]
    fn test_with_yolo() {
        let config = claude_default();
        assert!(!config.args.contains(&"--dangerously-skip-permissions".to_string()));

        let yolo_config = with_yolo(config);
        assert!(yolo_config.args.contains(&"--dangerously-skip-permissions".to_string()));
    }

    #[test]
    fn test_chatgpt_configs() {
        let default = chatgpt_default();
        assert_eq!(default.command, "sgpt");
        assert!(default.args.contains(&"--repl".to_string()));

        let code = chatgpt_code();
        assert!(code.args.contains(&"--code".to_string()));
    }

    #[test]
    fn test_agent_availability_status() {
        let avail = AgentAvailability {
            agent_type: AgentType::Claude,
            cli_available: true,
            api_key_configured: true,
            cli_path: Some("/usr/bin/claude".to_string()),
        };
        assert!(avail.is_ready());
        assert!(avail.status_message().contains("ready"));
    }
}
