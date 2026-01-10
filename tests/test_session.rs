//! Tests for session management
//!
//! Tests cover: session creation, lifecycle, state transitions, and cleanup.

use rat_squad::session::{Session, SessionConfig, SessionManager, SessionState};
use tempfile::TempDir;

const MAX_SESSIONS: usize = 50;

/// Test creating a new session with valid config
#[test]
fn test_session_creation_valid() {
    let config = SessionConfig {
        name: "test-session".to_string(),
        agent_type: "claude".to_string(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: false,
    };

    assert!(!config.name.is_empty(), "Name must not be empty");
    assert!(!config.agent_type.is_empty(), "Agent type must not be empty");

    let session = Session::new(config);
    assert!(session.is_ok(), "Session should be created successfully");

    let session = session.unwrap();
    assert_eq!(session.state(), SessionState::Pending, "Initial state should be Pending");
    assert!(!session.id().is_empty(), "Session ID must not be empty");
}

/// Test session creation with empty name fails
#[test]
fn test_session_creation_empty_name_fails() {
    let config = SessionConfig {
        name: String::new(),
        agent_type: "claude".to_string(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: false,
    };

    let session = Session::new(config);
    assert!(session.is_err(), "Session creation should fail with empty name");
}

/// Test session creation with empty agent type fails
#[test]
fn test_session_creation_empty_agent_fails() {
    let config = SessionConfig {
        name: "test".to_string(),
        agent_type: String::new(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: false,
    };

    let session = Session::new(config);
    assert!(session.is_err(), "Session creation should fail with empty agent type");
}

/// Test session state transitions
#[test]
fn test_session_state_transitions() {
    let config = SessionConfig {
        name: "state-test".to_string(),
        agent_type: "claude".to_string(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: false,
    };

    let mut session = Session::new(config).expect("Session creation should succeed");

    assert_eq!(session.state(), SessionState::Pending, "Initial state");

    session.set_state(SessionState::Starting);
    assert_eq!(session.state(), SessionState::Starting, "After start");

    session.set_state(SessionState::Running);
    assert_eq!(session.state(), SessionState::Running, "After running");

    session.set_state(SessionState::Paused);
    assert_eq!(session.state(), SessionState::Paused, "After pause");

    session.set_state(SessionState::Stopped);
    assert_eq!(session.state(), SessionState::Stopped, "After stop");
}

/// Test session manager creation
#[test]
fn test_session_manager_creation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manager = SessionManager::new(temp_dir.path());

    assert!(manager.is_ok(), "Manager should be created successfully");

    let manager = manager.unwrap();
    assert_eq!(manager.session_count(), 0, "Initial count should be 0");
}

/// Test adding sessions to manager
#[test]
fn test_session_manager_add_session() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut manager = SessionManager::new(temp_dir.path()).expect("Manager creation should succeed");

    let config = SessionConfig {
        name: "add-test".to_string(),
        agent_type: "claude".to_string(),
        work_dir: temp_dir.path().to_string_lossy().to_string(),
        auto_accept: false,
    };

    let session_id = manager.create_session(config);
    assert!(session_id.is_ok(), "Should add session successfully");
    assert_eq!(manager.session_count(), 1, "Count should be 1");
}

/// Test session manager enforces max sessions
#[test]
fn test_session_manager_max_sessions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut manager = SessionManager::new(temp_dir.path()).expect("Manager creation should succeed");

    for i in 0..MAX_SESSIONS {
        let config = SessionConfig {
            name: format!("session-{}", i),
            agent_type: "claude".to_string(),
            work_dir: temp_dir.path().to_string_lossy().to_string(),
            auto_accept: false,
        };
        let result = manager.create_session(config);
        assert!(result.is_ok(), "Should create session {}", i);
    }

    assert_eq!(manager.session_count(), MAX_SESSIONS, "Should have max sessions");

    let overflow_config = SessionConfig {
        name: "overflow".to_string(),
        agent_type: "claude".to_string(),
        work_dir: temp_dir.path().to_string_lossy().to_string(),
        auto_accept: false,
    };

    let result = manager.create_session(overflow_config);
    assert!(result.is_err(), "Should reject session over max");
}

/// Test removing session from manager
#[test]
fn test_session_manager_remove_session() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut manager = SessionManager::new(temp_dir.path()).expect("Manager creation should succeed");

    let config = SessionConfig {
        name: "remove-test".to_string(),
        agent_type: "claude".to_string(),
        work_dir: temp_dir.path().to_string_lossy().to_string(),
        auto_accept: false,
    };

    let session_id = manager.create_session(config).expect("Should create session");
    assert_eq!(manager.session_count(), 1, "Count should be 1");

    let result = manager.remove_session(&session_id);
    assert!(result.is_ok(), "Should remove session successfully");
    assert_eq!(manager.session_count(), 0, "Count should be 0");
}

/// Test getting session by ID
#[test]
fn test_session_manager_get_by_id() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut manager = SessionManager::new(temp_dir.path()).expect("Manager creation should succeed");

    let config = SessionConfig {
        name: "get-test".to_string(),
        agent_type: "aider".to_string(),
        work_dir: temp_dir.path().to_string_lossy().to_string(),
        auto_accept: true,
    };

    let session_id = manager.create_session(config).expect("Should create session");

    let session = manager.get_session(&session_id);
    assert!(session.is_some(), "Should find session by ID");

    let session = session.unwrap();
    assert_eq!(session.config().name, "get-test", "Name should match");
    assert_eq!(session.config().agent_type, "aider", "Agent type should match");
    assert!(session.config().auto_accept, "Auto accept should be true");
}

/// Test listing all sessions
#[test]
fn test_session_manager_list_sessions() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut manager = SessionManager::new(temp_dir.path()).expect("Manager creation should succeed");

    for i in 0..3 {
        let config = SessionConfig {
            name: format!("list-session-{}", i),
            agent_type: "claude".to_string(),
            work_dir: temp_dir.path().to_string_lossy().to_string(),
            auto_accept: false,
        };
        manager.create_session(config).expect("Should create session");
    }

    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 3, "Should list 3 sessions");
}

/// Test session with yolo mode (auto_accept)
#[test]
fn test_session_yolo_mode() {
    let config = SessionConfig {
        name: "yolo-test".to_string(),
        agent_type: "claude".to_string(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: true,
    };

    let session = Session::new(config).expect("Session creation should succeed");
    assert!(session.config().auto_accept, "Auto accept should be true");
}

/// Test session timestamps
#[test]
fn test_session_timestamps() {
    let config = SessionConfig {
        name: "timestamp-test".to_string(),
        agent_type: "claude".to_string(),
        work_dir: "/tmp/test".to_string(),
        auto_accept: false,
    };

    let session = Session::new(config).expect("Session creation should succeed");
    let created = session.created_at();
    let updated = session.updated_at();

    assert!(created <= updated, "Created should be <= updated");
}

/// Property tests for session names
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn session_name_must_be_non_empty(name in ".*") {
            let config = SessionConfig {
                name: name.clone(),
                agent_type: "claude".to_string(),
                work_dir: "/tmp".to_string(),
                auto_accept: false,
            };
            let session = Session::new(config);
            if name.is_empty() {
                prop_assert!(session.is_err());
            } else {
                prop_assert!(session.is_ok());
            }
        }

        #[test]
        fn session_id_is_unique(_name in "[a-z]{1,10}") {
            let config1 = SessionConfig {
                name: "a".to_string(),
                agent_type: "claude".to_string(),
                work_dir: "/tmp".to_string(),
                auto_accept: false,
            };
            let config2 = SessionConfig {
                name: "b".to_string(),
                agent_type: "claude".to_string(),
                work_dir: "/tmp".to_string(),
                auto_accept: false,
            };

            let s1 = Session::new(config1).unwrap();
            let s2 = Session::new(config2).unwrap();

            prop_assert_ne!(s1.id(), s2.id());
        }
    }
}
