//! Configuration management
//!
//! Handles loading, validation, and persistence of rat-squad configuration.

use crate::agent::AgentConfig;
use crate::error::{RatSquadError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Maximum allowed sessions
const MAX_ALLOWED_SESSIONS: usize = 50;

/// Minimum allowed sessions
const MIN_ALLOWED_SESSIONS: usize = 1;

/// Agent-specific configuration in config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFileConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Data directory for sessions and worktrees
    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    /// Maximum number of concurrent sessions
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,

    /// Default agent to use
    #[serde(default)]
    pub default_agent: Option<String>,

    /// Agent configurations
    #[serde(default)]
    pub agents: HashMap<String, AgentFileConfig>,

    /// Whether to auto-accept changes by default
    #[serde(default)]
    pub default_auto_accept: bool,

    /// Default base branch for worktrees
    #[serde(default = "default_base_branch")]
    pub default_base_branch: String,
}

fn default_data_dir() -> String {
    dirs::data_local_dir()
        .map(|p| p.join("rat-squad").to_string_lossy().to_string())
        .unwrap_or_else(|| ".rat-squad".to_string())
}

fn default_max_sessions() -> usize {
    10
}

fn default_base_branch() -> String {
    "main".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            max_sessions: default_max_sessions(),
            default_agent: Some("claude".to_string()),
            agents: HashMap::new(),
            default_auto_accept: false,
            default_base_branch: default_base_branch(),
        }
    }
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.data_dir.is_empty() {
            return Err(RatSquadError::validation("data_dir must not be empty"));
        }
        if self.max_sessions < MIN_ALLOWED_SESSIONS {
            return Err(RatSquadError::validation(format!(
                "max_sessions must be at least {MIN_ALLOWED_SESSIONS}"
            )));
        }
        if self.max_sessions > MAX_ALLOWED_SESSIONS {
            return Err(RatSquadError::validation(format!(
                "max_sessions must be at most {MAX_ALLOWED_SESSIONS}"
            )));
        }

        for (name, agent) in &self.agents {
            if agent.command.is_empty() {
                return Err(RatSquadError::validation(format!(
                    "Agent '{name}' has empty command"
                )));
            }
        }

        assert!(!self.data_dir.is_empty(), "data_dir validated but empty");
        assert!(self.max_sessions >= MIN_ALLOWED_SESSIONS, "max_sessions below minimum");
        assert!(self.max_sessions <= MAX_ALLOWED_SESSIONS, "max_sessions above maximum");

        Ok(())
    }

    /// Merge with defaults for missing fields
    pub fn merge_with_defaults(&mut self) {
        let defaults = Config::default();
        if self.data_dir.is_empty() {
            self.data_dir = defaults.data_dir;
        }
        if self.max_sessions == 0 {
            self.max_sessions = defaults.max_sessions;
        }
        if self.default_base_branch.is_empty() {
            self.default_base_branch = defaults.default_base_branch;
        }
    }

    /// Generate example configuration YAML
    pub fn generate_example() -> String {
        r#"# rat-squad configuration
# Place this file at ~/.rat-squad/config.yaml

# Data directory for sessions and worktrees
data_dir: "~/.rat-squad/data"

# Maximum number of concurrent sessions
max_sessions: 10

# Default agent to use when creating new sessions
default_agent: "claude"

# Whether to auto-accept changes by default (yolo mode)
default_auto_accept: false

# Default base branch for creating worktrees
default_base_branch: "main"

# Agent configurations
agents:
  claude:
    command: "claude"
    args:
      - "--dangerously-skip-permissions"
    env:
      ANTHROPIC_API_KEY: "${ANTHROPIC_API_KEY}"

  aider:
    command: "aider"
    args:
      - "--yes"
    env:
      OPENAI_API_KEY: "${OPENAI_API_KEY}"

  codex:
    command: "codex"
    args: []
    env:
      OPENAI_API_KEY: "${OPENAI_API_KEY}"
"#
        .to_string()
    }

    /// Convert agent file config to agent config
    pub fn get_agent_config(&self, name: &str) -> Option<AgentConfig> {
        self.agents.get(name).map(|fc| {
            let agent_type = crate::agent::AgentType::from_str(name).unwrap_or(crate::agent::AgentType::Custom);
            AgentConfig {
                agent_type,
                command: fc.command.clone(),
                args: fc.args.clone(),
                env: fc.env.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            }
        })
    }
}

/// Configuration manager for loading and saving config
pub struct ConfigManager {
    /// Path to config file
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new config manager
    pub fn new(config_path: &Path) -> Result<Self> {
        Ok(Self {
            config_path: config_path.to_path_buf(),
        })
    }

    /// Load configuration from file
    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Config::default());
        }

        let content = std::fs::read_to_string(&self.config_path)?;
        let mut config: Config = serde_yaml::from_str(&content)?;
        config.merge_with_defaults();
        config.validate()?;

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, config: &Config) -> Result<()> {
        config.validate()?;

        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let yaml = serde_yaml::to_string(config)?;
        std::fs::write(&self.config_path, yaml)?;

        assert!(self.config_path.exists(), "Config file not created");

        Ok(())
    }

    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Create example config file if it doesn't exist
    pub fn create_example(&self) -> Result<PathBuf> {
        let example_path = self.config_path.with_extension("yaml.example");

        if !example_path.exists() {
            if let Some(parent) = example_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&example_path, Config::generate_example())?;
        }

        Ok(example_path)
    }
}

/// Get the default config directory
pub fn default_config_dir() -> PathBuf {
    dirs::config_dir()
        .map(|p| p.join("rat-squad"))
        .unwrap_or_else(|| PathBuf::from(".rat-squad"))
}

/// Get the default config file path
pub fn default_config_path() -> PathBuf {
    default_config_dir().join("config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.data_dir.is_empty());
        assert!(config.max_sessions > 0);
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_data_dir() {
        let mut config = Config::default();
        config.data_dir = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_max_sessions_zero() {
        let mut config = Config::default();
        config.max_sessions = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_max_sessions_excessive() {
        let mut config = Config::default();
        config.max_sessions = 1000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_generate_example() {
        let example = Config::generate_example();
        assert!(example.contains("data_dir"));
        assert!(example.contains("max_sessions"));
        assert!(example.contains("agents"));
    }
}
