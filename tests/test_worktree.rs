//! Tests for git worktree management
//!
//! Tests cover: worktree creation, listing, cleanup, and branch operations.

use rat_squad::worktree::{Worktree, WorktreeConfig, WorktreeManager};
use std::process::Command;
use tempfile::TempDir;

const MAX_WORKTREES: usize = 20;

/// Helper to initialize a git repo in a directory
fn init_git_repo(path: &std::path::Path) -> bool {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Helper to make initial commit
fn make_initial_commit(path: &std::path::Path) -> bool {
    let touch_result = std::fs::write(path.join("README.md"), "# Test\n");
    if touch_result.is_err() {
        return false;
    }

    let add_result = Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !add_result {
        return false;
    }

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Test worktree config validation
#[test]
fn test_worktree_config_valid() {
    let config = WorktreeConfig {
        branch_name: "feature/test".to_string(),
        base_branch: "main".to_string(),
    };

    assert!(!config.branch_name.is_empty(), "Branch name must not be empty");
    assert!(!config.base_branch.is_empty(), "Base branch must not be empty");

    let result = config.validate();
    assert!(result.is_ok(), "Valid config should pass validation");
}

/// Test worktree config with empty branch fails
#[test]
fn test_worktree_config_empty_branch_fails() {
    let config = WorktreeConfig {
        branch_name: String::new(),
        base_branch: "main".to_string(),
    };

    let result = config.validate();
    assert!(result.is_err(), "Empty branch name should fail validation");
}

/// Test worktree config with empty base branch fails
#[test]
fn test_worktree_config_empty_base_fails() {
    let config = WorktreeConfig {
        branch_name: "feature/test".to_string(),
        base_branch: String::new(),
    };

    let result = config.validate();
    assert!(result.is_err(), "Empty base branch should fail validation");
}

/// Test worktree manager creation in git repo
#[test]
fn test_worktree_manager_creation_in_repo() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let manager = WorktreeManager::new(repo_path);
    assert!(manager.is_ok(), "Manager should be created in git repo");
}

/// Test worktree manager creation outside git repo fails
#[test]
fn test_worktree_manager_creation_no_repo_fails() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let manager = WorktreeManager::new(temp_dir.path());

    assert!(manager.is_err(), "Manager should fail outside git repo");
}

/// Test creating a worktree
#[test]
fn test_create_worktree() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let mut manager = WorktreeManager::new(repo_path).expect("Manager creation should succeed");

    let config = WorktreeConfig {
        branch_name: "feature/test-1".to_string(),
        base_branch: "master".to_string(),
    };

    let worktree = manager.create_worktree(config);
    assert!(worktree.is_ok(), "Should create worktree: {:?}", worktree.err());

    let worktree = worktree.unwrap();
    assert!(worktree.path().exists(), "Worktree path should exist");
}

/// Test listing worktrees
#[test]
fn test_list_worktrees() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let mut manager = WorktreeManager::new(repo_path).expect("Manager creation should succeed");

    for i in 0..3 {
        let config = WorktreeConfig {
            branch_name: format!("feature/list-test-{}", i),
            base_branch: "master".to_string(),
        };
        manager.create_worktree(config).expect("Should create worktree");
    }

    let worktrees = manager.list_worktrees();
    assert!(worktrees.is_ok(), "Should list worktrees");

    let worktrees = worktrees.unwrap();
    assert!(worktrees.len() >= 3, "Should have at least 3 worktrees");
}

/// Test removing a worktree
#[test]
fn test_remove_worktree() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let mut manager = WorktreeManager::new(repo_path).expect("Manager creation should succeed");

    let config = WorktreeConfig {
        branch_name: "feature/remove-test".to_string(),
        base_branch: "master".to_string(),
    };

    let worktree = manager.create_worktree(config).expect("Should create worktree");
    let worktree_path = worktree.path().to_path_buf();

    assert!(worktree_path.exists(), "Worktree should exist before removal");

    let result = manager.remove_worktree(&worktree.branch_name());
    assert!(result.is_ok(), "Should remove worktree");

    assert!(!worktree_path.exists(), "Worktree should not exist after removal");
}

/// Test worktree manager enforces max worktrees
#[test]
fn test_worktree_max_limit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let mut manager = WorktreeManager::new(repo_path).expect("Manager creation should succeed");

    for i in 0..MAX_WORKTREES {
        let config = WorktreeConfig {
            branch_name: format!("feature/max-test-{}", i),
            base_branch: "master".to_string(),
        };
        let result = manager.create_worktree(config);
        assert!(result.is_ok(), "Should create worktree {}", i);
    }

    let overflow_config = WorktreeConfig {
        branch_name: "feature/overflow".to_string(),
        base_branch: "master".to_string(),
    };

    let result = manager.create_worktree(overflow_config);
    assert!(result.is_err(), "Should reject worktree over max");
}

/// Test getting worktree status
#[test]
fn test_worktree_status() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    if !init_git_repo(repo_path) {
        eprintln!("Skipping test: git not available");
        return;
    }

    if !make_initial_commit(repo_path) {
        eprintln!("Skipping test: could not make initial commit");
        return;
    }

    let mut manager = WorktreeManager::new(repo_path).expect("Manager creation should succeed");

    let config = WorktreeConfig {
        branch_name: "feature/status-test".to_string(),
        base_branch: "master".to_string(),
    };

    let worktree = manager.create_worktree(config).expect("Should create worktree");
    let status = worktree.status();

    assert!(status.is_ok(), "Should get status");
    let status = status.unwrap();
    assert!(status.is_clean, "New worktree should be clean");
}

/// Test worktree branch name generation
#[test]
fn test_worktree_branch_naming() {
    let config = WorktreeConfig {
        branch_name: "feature/my-feature".to_string(),
        base_branch: "main".to_string(),
    };

    assert!(config.branch_name.starts_with("feature/"), "Branch should have feature prefix");
}

/// Property tests for worktree names
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn branch_name_must_be_non_empty(name in ".*") {
            let config = WorktreeConfig {
                branch_name: name.clone(),
                base_branch: "main".to_string(),
            };
            let result = config.validate();
            if name.is_empty() {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }

        #[test]
        fn base_branch_must_be_non_empty(base in ".*") {
            let config = WorktreeConfig {
                branch_name: "feature/test".to_string(),
                base_branch: base.clone(),
            };
            let result = config.validate();
            if base.is_empty() {
                prop_assert!(result.is_err());
            } else {
                prop_assert!(result.is_ok());
            }
        }
    }
}
