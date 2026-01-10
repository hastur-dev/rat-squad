//! Git worktree management
//!
//! Provides isolated git worktrees for each AI agent session.

use crate::error::{RatSquadError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum number of worktrees
const MAX_WORKTREES: usize = 20;

/// Worktree directory prefix
const WORKTREE_PREFIX: &str = ".rat-squad-worktrees";

/// Configuration for creating a worktree
#[derive(Debug, Clone)]
pub struct WorktreeConfig {
    /// Branch name for the worktree
    pub branch_name: String,
    /// Base branch to create from
    pub base_branch: String,
}

impl WorktreeConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.branch_name.is_empty() {
            return Err(RatSquadError::validation("Branch name must not be empty"));
        }
        if self.base_branch.is_empty() {
            return Err(RatSquadError::validation("Base branch must not be empty"));
        }
        assert!(!self.branch_name.is_empty(), "Branch name validated but empty");
        assert!(!self.base_branch.is_empty(), "Base branch validated but empty");
        Ok(())
    }
}

/// Worktree status information
#[derive(Debug, Clone)]
pub struct WorktreeStatus {
    /// Whether the worktree has uncommitted changes
    pub is_clean: bool,
    /// Number of modified files
    pub modified_count: usize,
    /// Number of untracked files
    pub untracked_count: usize,
}

/// Represents a git worktree
#[derive(Debug, Clone)]
pub struct Worktree {
    /// Path to the worktree
    path: PathBuf,
    /// Branch name
    branch: String,
}

impl Worktree {
    /// Get the worktree path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the branch name
    pub fn branch_name(&self) -> &str {
        &self.branch
    }

    /// Get the worktree status
    pub fn status(&self) -> Result<WorktreeStatus> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.path)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to run git status: {e}")))?;

        if !output.status.success() {
            return Err(RatSquadError::git("git status failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        let modified_count = lines.iter().filter(|l| l.starts_with(" M") || l.starts_with("M ")).count();
        let untracked_count = lines.iter().filter(|l| l.starts_with("??")).count();

        assert!(modified_count <= lines.len(), "Modified count exceeds total");
        assert!(untracked_count <= lines.len(), "Untracked count exceeds total");

        Ok(WorktreeStatus {
            is_clean: lines.is_empty(),
            modified_count,
            untracked_count,
        })
    }

    /// Commit all changes in the worktree
    pub fn commit_all(&self, message: &str) -> Result<()> {
        if message.is_empty() {
            return Err(RatSquadError::validation("Commit message must not be empty"));
        }

        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.path)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to run git add: {e}")))?;

        if !add_output.status.success() {
            return Err(RatSquadError::git("git add failed"));
        }

        let commit_output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&self.path)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to run git commit: {e}")))?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            if stderr.contains("nothing to commit") {
                return Ok(());
            }
            return Err(RatSquadError::git(format!("git commit failed: {stderr}")));
        }

        Ok(())
    }
}

/// Manages git worktrees for sessions
pub struct WorktreeManager {
    /// Repository root path
    repo_root: PathBuf,
    /// Worktrees directory
    worktrees_dir: PathBuf,
    /// Active worktrees
    worktrees: Vec<Worktree>,
}

impl WorktreeManager {
    /// Create a new worktree manager for a repository
    pub fn new(repo_path: &Path) -> Result<Self> {
        let is_repo = Command::new("git")
            .args(["rev-parse", "--git-dir"])
            .current_dir(repo_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !is_repo {
            return Err(RatSquadError::git("Not a git repository"));
        }

        let worktrees_dir = repo_path.join(WORKTREE_PREFIX);
        if !worktrees_dir.exists() {
            std::fs::create_dir_all(&worktrees_dir)?;
        }

        assert!(repo_path.exists(), "Repository path must exist");
        assert!(worktrees_dir.exists(), "Worktrees directory must exist");

        Ok(Self {
            repo_root: repo_path.to_path_buf(),
            worktrees_dir,
            worktrees: Vec::with_capacity(MAX_WORKTREES),
        })
    }

    /// Create a new worktree
    pub fn create_worktree(&mut self, config: WorktreeConfig) -> Result<Worktree> {
        config.validate()?;

        if self.worktrees.len() >= MAX_WORKTREES {
            return Err(RatSquadError::limit_exceeded(format!(
                "Maximum worktrees ({MAX_WORKTREES}) reached"
            )));
        }

        let safe_branch = config.branch_name.replace('/', "-");
        let worktree_path = self.worktrees_dir.join(&safe_branch);

        if worktree_path.exists() {
            return Err(RatSquadError::already_exists(format!(
                "Worktree already exists: {}",
                worktree_path.display()
            )));
        }

        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &config.branch_name,
                worktree_path.to_str().unwrap(),
                &config.base_branch,
            ])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to create worktree: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RatSquadError::git(format!("git worktree add failed: {stderr}")));
        }

        assert!(worktree_path.exists(), "Worktree creation succeeded but path doesn't exist");

        let worktree = Worktree {
            path: worktree_path,
            branch: config.branch_name,
        };

        self.worktrees.push(worktree.clone());
        Ok(worktree)
    }

    /// List all worktrees
    pub fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to list worktrees: {e}")))?;

        if !output.status.success() {
            return Err(RatSquadError::git("git worktree list failed"));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;

        for line in stdout.lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path));
            } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                current_branch = Some(branch.to_string());
            } else if line.is_empty() {
                if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                    worktrees.push(Worktree { path, branch });
                }
            }
        }

        if let (Some(path), Some(branch)) = (current_path, current_branch) {
            worktrees.push(Worktree { path, branch });
        }

        Ok(worktrees)
    }

    /// Remove a worktree by branch name
    pub fn remove_worktree(&mut self, branch_name: &str) -> Result<()> {
        if branch_name.is_empty() {
            return Err(RatSquadError::validation("Branch name must not be empty"));
        }

        let safe_branch = branch_name.replace('/', "-");
        let worktree_path = self.worktrees_dir.join(&safe_branch);

        let output = Command::new("git")
            .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to remove worktree: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RatSquadError::git(format!("git worktree remove failed: {stderr}")));
        }

        self.worktrees.retain(|w| w.branch != branch_name);

        let delete_output = Command::new("git")
            .args(["branch", "-D", branch_name])
            .current_dir(&self.repo_root)
            .output();

        if let Ok(out) = delete_output {
            if !out.status.success() {
                tracing::warn!("Failed to delete branch {branch_name}");
            }
        }

        Ok(())
    }

    /// Get a worktree by branch name
    pub fn get_worktree(&self, branch_name: &str) -> Option<&Worktree> {
        self.worktrees.iter().find(|w| w.branch == branch_name)
    }

    /// Prune stale worktrees
    pub fn prune(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["worktree", "prune"])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RatSquadError::git(format!("Failed to prune worktrees: {e}")))?;

        if !output.status.success() {
            return Err(RatSquadError::git("git worktree prune failed"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_config_validation() {
        let config = WorktreeConfig {
            branch_name: "feature/test".to_string(),
            base_branch: "main".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_worktree_config_empty_branch() {
        let config = WorktreeConfig {
            branch_name: String::new(),
            base_branch: "main".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_worktree_config_empty_base() {
        let config = WorktreeConfig {
            branch_name: "feature/test".to_string(),
            base_branch: String::new(),
        };
        assert!(config.validate().is_err());
    }
}
