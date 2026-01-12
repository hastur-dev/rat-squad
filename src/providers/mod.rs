//! AI Provider API clients
//!
//! Direct API integration for ChatGPT (OpenAI) and Gemini (Google).

pub mod openai;
pub mod gemini;

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Maximum tokens for a single request
pub const MAX_TOKENS: u32 = 4096;

/// Maximum conversation history to maintain
pub const MAX_HISTORY: usize = 50;

/// Role in a conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions)
    System,
    /// User message
    User,
    /// Assistant response
    Assistant,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Response from an AI provider
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    /// The generated text content
    pub content: String,
    /// Model used for generation
    pub model: String,
    /// Token usage statistics
    pub usage: Option<TokenUsage>,
    /// Whether the response was truncated
    pub truncated: bool,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Configuration for an AI provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// API key for authentication
    pub api_key: String,
    /// Model to use (e.g., "gpt-4", "gemini-pro")
    pub model: String,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Temperature for generation (0.0 - 2.0)
    pub temperature: f32,
    /// System prompt/instructions
    pub system_prompt: Option<String>,
}

impl ProviderConfig {
    /// Create a new provider configuration
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: MAX_TOKENS,
            temperature: 0.7,
            system_prompt: None,
        }
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set the system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(crate::error::RatSquadError::validation("API key is required"));
        }
        if self.model.is_empty() {
            return Err(crate::error::RatSquadError::validation("Model is required"));
        }
        if self.max_tokens == 0 {
            return Err(crate::error::RatSquadError::validation("Max tokens must be > 0"));
        }
        Ok(())
    }
}

/// Trait for AI provider implementations
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get the current model
    fn model(&self) -> &str;

    /// Send a single message and get a response
    async fn complete(&self, prompt: &str) -> Result<ProviderResponse>;

    /// Send a conversation and get a response
    async fn chat(&self, messages: &[Message]) -> Result<ProviderResponse>;

    /// Stream a response (returns chunks)
    async fn stream(&self, prompt: &str) -> Result<Vec<String>>;

    /// Check if the provider is configured and ready
    fn is_ready(&self) -> bool;
}

/// Provider type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// OpenAI (ChatGPT)
    OpenAI,
    /// Google Gemini
    Gemini,
    /// Anthropic Claude (for reference)
    Anthropic,
}

impl ProviderType {
    /// Get the environment variable for the API key
    pub fn api_key_env(&self) -> &'static str {
        match self {
            Self::OpenAI => "OPENAI_API_KEY",
            Self::Gemini => "GOOGLE_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
        }
    }

    /// Get the default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-4o",
            Self::Gemini => "gemini-2.0-flash",
            Self::Anthropic => "claude-sonnet-4-20250514",
        }
    }
}

pub use openai::OpenAIProvider;
pub use gemini::GeminiProvider;
