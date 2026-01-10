//! Tests for AI agent definitions and spawning
//!
//! Tests cover: agent types, command generation, and process management.

use rat_squad::agent::{Agent, AgentConfig, AgentRegistry, AgentType};

const MAX_AGENTS: usize = 10;

/// Test agent type parsing
#[test]
fn test_agent_type_parsing() {
    assert_eq!(AgentType::from_str("claude").unwrap(), AgentType::Claude);
    assert_eq!(AgentType::from_str("aider").unwrap(), AgentType::Aider);
    assert_eq!(AgentType::from_str("codex").unwrap(), AgentType::Codex);
    assert_eq!(AgentType::from_str("gemini").unwrap(), AgentType::Gemini);
    assert_eq!(AgentType::from_str("custom").unwrap(), AgentType::Custom);
}

/// Test agent type parsing case insensitive
#[test]
fn test_agent_type_case_insensitive() {
    assert_eq!(AgentType::from_str("CLAUDE").unwrap(), AgentType::Claude);
    assert_eq!(AgentType::from_str("Claude").unwrap(), AgentType::Claude);
    assert_eq!(AgentType::from_str("cLaUdE").unwrap(), AgentType::Claude);
}

/// Test unknown agent type fails
#[test]
fn test_agent_type_unknown_fails() {
    let result = AgentType::from_str("unknown-agent");
    assert!(result.is_err(), "Unknown agent type should fail");
}

/// Test agent config validation
#[test]
fn test_agent_config_valid() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec!["--dangerously-skip-permissions".to_string()],
        env: vec![("ANTHROPIC_API_KEY".to_string(), "test".to_string())],
    };

    assert!(!config.command.is_empty(), "Command must not be empty");

    let result = config.validate();
    assert!(result.is_ok(), "Valid config should pass validation");
}

/// Test agent config with empty command fails
#[test]
fn test_agent_config_empty_command_fails() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: String::new(),
        args: vec![],
        env: vec![],
    };

    let result = config.validate();
    assert!(result.is_err(), "Empty command should fail validation");
}

/// Test agent creation
#[test]
fn test_agent_creation() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec![],
        env: vec![],
    };

    let agent = Agent::new(config);
    assert!(agent.is_ok(), "Agent should be created successfully");

    let agent = agent.unwrap();
    assert!(!agent.id().is_empty(), "Agent ID must not be empty");
    assert_eq!(agent.agent_type(), AgentType::Claude, "Agent type should match");
}

/// Test agent command generation for Claude
#[test]
fn test_agent_command_claude() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec!["--dangerously-skip-permissions".to_string()],
        env: vec![],
    };

    let agent = Agent::new(config).expect("Agent creation should succeed");
    let cmd = agent.build_command("/work/dir");

    assert_eq!(cmd.program, "claude", "Program should be claude");
    assert!(cmd.args.contains(&"--dangerously-skip-permissions".to_string()), "Should have skip permissions arg");
    assert_eq!(cmd.working_dir, "/work/dir", "Working dir should match");
}

/// Test agent command generation for Aider
#[test]
fn test_agent_command_aider() {
    let config = AgentConfig {
        agent_type: AgentType::Aider,
        command: "aider".to_string(),
        args: vec!["--yes".to_string()],
        env: vec![],
    };

    let agent = Agent::new(config).expect("Agent creation should succeed");
    let cmd = agent.build_command("/work/dir");

    assert_eq!(cmd.program, "aider", "Program should be aider");
    assert!(cmd.args.contains(&"--yes".to_string()), "Should have yes arg");
}

/// Test agent registry creation
#[test]
fn test_agent_registry_creation() {
    let registry = AgentRegistry::new();

    assert!(registry.get("claude").is_some(), "Claude should be registered by default");
    assert!(registry.get("aider").is_some(), "Aider should be registered by default");
}

/// Test agent registry get by name
#[test]
fn test_agent_registry_get_by_name() {
    let registry = AgentRegistry::new();

    let claude = registry.get("claude");
    assert!(claude.is_some(), "Should find claude");

    let claude = claude.unwrap();
    assert_eq!(claude.agent_type, AgentType::Claude, "Type should be Claude");
}

/// Test agent registry custom registration
#[test]
fn test_agent_registry_custom_registration() {
    let mut registry = AgentRegistry::new();

    let custom_config = AgentConfig {
        agent_type: AgentType::Custom,
        command: "my-agent".to_string(),
        args: vec!["--custom-flag".to_string()],
        env: vec![],
    };

    let result = registry.register("my-agent", custom_config);
    assert!(result.is_ok(), "Should register custom agent");

    let agent = registry.get("my-agent");
    assert!(agent.is_some(), "Should find custom agent");
}

/// Test agent registry prevents duplicate registration
#[test]
fn test_agent_registry_duplicate_fails() {
    let mut registry = AgentRegistry::new();

    let config = AgentConfig {
        agent_type: AgentType::Custom,
        command: "claude".to_string(),
        args: vec![],
        env: vec![],
    };

    let result = registry.register("claude", config);
    assert!(result.is_err(), "Duplicate registration should fail");
}

/// Test agent registry list all
#[test]
fn test_agent_registry_list_all() {
    let registry = AgentRegistry::new();
    let agents = registry.list();

    assert!(agents.len() >= 4, "Should have at least 4 default agents");
    assert!(agents.contains(&"claude".to_string()), "Should contain claude");
    assert!(agents.contains(&"aider".to_string()), "Should contain aider");
}

/// Test agent with environment variables
#[test]
fn test_agent_with_env_vars() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec![],
        env: vec![
            ("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string()),
            ("CLAUDE_MODEL".to_string(), "claude-3-opus".to_string()),
        ],
    };

    let agent = Agent::new(config).expect("Agent creation should succeed");
    let cmd = agent.build_command("/work");

    assert_eq!(cmd.env.len(), 2, "Should have 2 env vars");
    assert!(cmd.env.iter().any(|(k, _)| k == "ANTHROPIC_API_KEY"), "Should have API key env");
}

/// Test agent yolo mode configuration
#[test]
fn test_agent_yolo_mode() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec!["--dangerously-skip-permissions".to_string()],
        env: vec![],
    };

    let agent = Agent::new(config).expect("Agent creation should succeed");
    assert!(agent.is_yolo_mode(), "Should detect yolo mode from args");
}

/// Test agent non-yolo mode
#[test]
fn test_agent_non_yolo_mode() {
    let config = AgentConfig {
        agent_type: AgentType::Claude,
        command: "claude".to_string(),
        args: vec![],
        env: vec![],
    };

    let agent = Agent::new(config).expect("Agent creation should succeed");
    assert!(!agent.is_yolo_mode(), "Should not be yolo mode without flag");
}

/// Property tests for agent types
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn agent_type_roundtrip(agent_type in prop_oneof![
            Just(AgentType::Claude),
            Just(AgentType::Aider),
            Just(AgentType::Codex),
            Just(AgentType::Gemini),
            Just(AgentType::Custom),
        ]) {
            let name = agent_type.to_string();
            let parsed = AgentType::from_str(&name).unwrap();
            prop_assert_eq!(agent_type, parsed);
        }

        #[test]
        fn command_must_be_non_empty(cmd in ".*") {
            let config = AgentConfig {
                agent_type: AgentType::Custom,
                command: cmd.clone(),
                args: vec![],
                env: vec![],
            };
            let result = config.validate();
            if cmd.is_empty() {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }
    }
}
