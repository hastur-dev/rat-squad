//! Tests for configuration management
//!
//! Tests cover: config loading, validation, defaults, and serialization.

use rat_squad::config::{Config, ConfigManager};
use std::io::Write;
use tempfile::TempDir;

/// Test default config creation
#[test]
fn test_default_config() {
    let config = Config::default();

    assert!(!config.data_dir.is_empty(), "Data dir must not be empty");
    assert!(config.max_sessions > 0, "Max sessions must be positive");
    assert!(config.max_sessions <= 50, "Max sessions must be reasonable");
    assert!(config.default_agent.is_some(), "Should have default agent");
}

/// Test config manager creation
#[test]
fn test_config_manager_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let manager = ConfigManager::new(&config_path);
    assert!(manager.is_ok(), "Config manager should be created");
}

/// Test loading config from file
#[test]
fn test_load_config_from_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let yaml_content = r#"
data_dir: "/custom/data"
max_sessions: 10
default_agent: "aider"
agents:
  claude:
    command: "claude"
    args:
      - "--dangerously-skip-permissions"
  aider:
    command: "aider"
    args:
      - "--yes"
"#;

    std::fs::write(&config_path, yaml_content).expect("Failed to write config");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");
    let config = manager.load();

    assert!(config.is_ok(), "Should load config from file");

    let config = config.unwrap();
    assert_eq!(config.data_dir, "/custom/data", "Data dir should match");
    assert_eq!(config.max_sessions, 10, "Max sessions should match");
    assert_eq!(config.default_agent.unwrap(), "aider", "Default agent should match");
}

/// Test config file not found uses defaults
#[test]
fn test_config_file_not_found_uses_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("nonexistent.yaml");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");
    let config = manager.load();

    assert!(config.is_ok(), "Should return default config when file not found");

    let config = config.unwrap();
    let default = Config::default();
    assert_eq!(config.max_sessions, default.max_sessions, "Should use default max sessions");
}

/// Test config validation - max_sessions must be positive
#[test]
fn test_config_validation_max_sessions_positive() {
    let mut config = Config::default();
    config.max_sessions = 0;

    let result = config.validate();
    assert!(result.is_err(), "Zero max_sessions should fail validation");
}

/// Test config validation - max_sessions must be reasonable
#[test]
fn test_config_validation_max_sessions_reasonable() {
    let mut config = Config::default();
    config.max_sessions = 1000;

    let result = config.validate();
    assert!(result.is_err(), "Excessive max_sessions should fail validation");
}

/// Test config validation - data_dir must not be empty
#[test]
fn test_config_validation_data_dir_not_empty() {
    let mut config = Config::default();
    config.data_dir = String::new();

    let result = config.validate();
    assert!(result.is_err(), "Empty data_dir should fail validation");
}

/// Test saving config to file
#[test]
fn test_save_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");

    let mut config = Config::default();
    config.max_sessions = 25;
    config.default_agent = Some("codex".to_string());

    let result = manager.save(&config);
    assert!(result.is_ok(), "Should save config to file");

    assert!(config_path.exists(), "Config file should exist");

    let loaded = manager.load().expect("Should load saved config");
    assert_eq!(loaded.max_sessions, 25, "Loaded max_sessions should match");
    assert_eq!(loaded.default_agent.unwrap(), "codex", "Loaded default_agent should match");
}

/// Test config with custom agents
#[test]
fn test_config_custom_agents() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let yaml_content = r#"
data_dir: "/data"
max_sessions: 5
agents:
  my-custom-agent:
    command: "/path/to/agent"
    args:
      - "--flag1"
      - "--flag2"
    env:
      MY_KEY: "my_value"
"#;

    std::fs::write(&config_path, yaml_content).expect("Failed to write config");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");
    let config = manager.load().expect("Should load config");

    assert!(config.agents.contains_key("my-custom-agent"), "Should have custom agent");

    let agent = config.agents.get("my-custom-agent").unwrap();
    assert_eq!(agent.command, "/path/to/agent", "Command should match");
    assert_eq!(agent.args.len(), 2, "Should have 2 args");
}

/// Test config merge with defaults
#[test]
fn test_config_merge_with_defaults() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let yaml_content = r#"
max_sessions: 15
"#;

    std::fs::write(&config_path, yaml_content).expect("Failed to write config");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");
    let config = manager.load().expect("Should load config");

    assert_eq!(config.max_sessions, 15, "Custom max_sessions should be used");
    let default = Config::default();
    assert_eq!(config.data_dir, default.data_dir, "Default data_dir should be used");
}

/// Test invalid YAML fails gracefully
#[test]
fn test_invalid_yaml_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("config.yaml");

    let invalid_yaml = r#"
data_dir: [invalid
  yaml content
"#;

    std::fs::write(&config_path, invalid_yaml).expect("Failed to write config");

    let manager = ConfigManager::new(&config_path).expect("Manager creation should succeed");
    let result = manager.load();

    assert!(result.is_err(), "Invalid YAML should fail to load");
}

/// Test config example file generation
#[test]
fn test_config_example_generation() {
    let example = Config::generate_example();

    assert!(example.contains("data_dir"), "Example should contain data_dir");
    assert!(example.contains("max_sessions"), "Example should contain max_sessions");
    assert!(example.contains("agents"), "Example should contain agents section");
}

/// Property tests for config values
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn max_sessions_must_be_in_range(max in 0usize..1000usize) {
            let mut config = Config::default();
            config.max_sessions = max;
            let result = config.validate();
            if max == 0 || max > 50 {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }

        #[test]
        fn data_dir_must_be_non_empty(dir in ".*") {
            let mut config = Config::default();
            config.data_dir = dir.clone();
            let result = config.validate();
            if dir.is_empty() {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }
    }
}
