//! AI agent definitions and spawning
//!
//! Supports Claude, Aider, Codex, Gemini, and custom agents.

use crate::error::{RatSquadError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum number of registered agents
const MAX_AGENTS: usize = 20;

/// Maximum number of arguments per agent
const MAX_ARGS: usize = 50;

/// Agent type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Anthropic's Claude Code
    Claude,
    /// Aider AI coding assistant
    Aider,
    /// OpenAI Codex
    Codex,
    /// Google Gemini
    Gemini,
    /// Custom agent
    Custom,
}

impl AgentType {
    /// Parse agent type from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(Self::Claude),
            "aider" => Ok(Self::Aider),
            "codex" => Ok(Self::Codex),
            "gemini" => Ok(Self::Gemini),
            "custom" => Ok(Self::Custom),
            _ => Err(RatSquadError::agent(format!("Unknown agent type: {s}"))),
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Claude => "claude",
            Self::Aider => "aider",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
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

/// Registry of available AI agents
pub struct AgentRegistry {
    /// Registered agents by name
    agents: HashMap<String, AgentConfig>,
}

impl AgentRegistry {
    /// Create a new registry with default agents
    pub fn new() -> Self {
        let mut agents = HashMap::with_capacity(MAX_AGENTS);

        agents.insert(
            "claude".to_string(),
            AgentConfig {
                agent_type: AgentType::Claude,
                command: "claude".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        agents.insert(
            "aider".to_string(),
            AgentConfig {
                agent_type: AgentType::Aider,
                command: "aider".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        agents.insert(
            "codex".to_string(),
            AgentConfig {
                agent_type: AgentType::Codex,
                command: "codex".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        agents.insert(
            "gemini".to_string(),
            AgentConfig {
                agent_type: AgentType::Gemini,
                command: "gemini".to_string(),
                args: vec![],
                env: vec![],
            },
        );

        Self { agents }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type_from_str() {
        assert_eq!(AgentType::from_str("claude").unwrap(), AgentType::Claude);
        assert_eq!(AgentType::from_str("CLAUDE").unwrap(), AgentType::Claude);
        assert_eq!(AgentType::from_str("aider").unwrap(), AgentType::Aider);
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::Claude.to_string(), "claude");
        assert_eq!(AgentType::Aider.to_string(), "aider");
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
        assert!(registry.get("aider").is_some());
    }
}
