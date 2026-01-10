//! Error types for rat-squad extension
//!
//! Provides unified error handling across all modules.

use thiserror::Error;

/// Maximum error message length for validation
const MAX_ERROR_MSG_LEN: usize = 1024;

/// Unified error type for rat-squad operations
#[derive(Error, Debug)]
pub enum RatSquadError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Session management errors
    #[error("Session error: {0}")]
    Session(String),

    /// Worktree operation errors
    #[error("Worktree error: {0}")]
    Worktree(String),

    /// Agent operation errors
    #[error("Agent error: {0}")]
    Agent(String),

    /// Ratterm API client errors
    #[error("API error: {0}")]
    Api(String),

    /// HTTP request errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Git operation errors
    #[error("Git error: {0}")]
    Git(String),

    /// Process spawn errors
    #[error("Process error: {0}")]
    Process(String),

    /// Resource limit exceeded
    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Resource already exists
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}

impl RatSquadError {
    /// Create a config error with message validation
    pub fn config(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Config(msg)
    }

    /// Create a session error with message validation
    pub fn session(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Session(msg)
    }

    /// Create a worktree error with message validation
    pub fn worktree(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Worktree(msg)
    }

    /// Create an agent error with message validation
    pub fn agent(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Agent(msg)
    }

    /// Create an API error with message validation
    pub fn api(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Api(msg)
    }

    /// Create a validation error with message validation
    pub fn validation(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Validation(msg)
    }

    /// Create a git error with message validation
    pub fn git(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Git(msg)
    }

    /// Create a process error with message validation
    pub fn process(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::Process(msg)
    }

    /// Create a limit exceeded error with message validation
    pub fn limit_exceeded(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::LimitExceeded(msg)
    }

    /// Create a not found error with message validation
    pub fn not_found(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::NotFound(msg)
    }

    /// Create an already exists error with message validation
    pub fn already_exists(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        assert!(msg.len() <= MAX_ERROR_MSG_LEN, "Error message too long");
        Self::AlreadyExists(msg)
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Http(_) | Self::Api(_) | Self::Process(_))
    }
}

impl From<serde_json::Error> for RatSquadError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for RatSquadError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::Serialization(err.to_string())
    }
}

/// Result type alias for rat-squad operations
pub type Result<T> = std::result::Result<T, RatSquadError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RatSquadError::config("test error");
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("test error"));
    }

    #[test]
    fn test_error_retryable() {
        let api_err = RatSquadError::api("timeout");
        assert!(api_err.is_retryable(), "API errors should be retryable");

        let config_err = RatSquadError::config("invalid");
        assert!(!config_err.is_retryable(), "Config errors should not be retryable");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: RatSquadError = io_err.into();
        assert!(matches!(err, RatSquadError::Io(_)));
    }
}
