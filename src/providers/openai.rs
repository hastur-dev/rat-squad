//! OpenAI API client for ChatGPT integration
//!
//! Provides direct API access to OpenAI's GPT models.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{Message, Provider, ProviderConfig, ProviderResponse, Role, TokenUsage};
use crate::error::{RatSquadError, Result};

/// OpenAI API base URL
const OPENAI_API_URL: &str = "https://api.openai.com/v1";

/// OpenAI chat completion request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// OpenAI message format
#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

impl From<&Message> for ChatMessage {
    fn from(msg: &Message) -> Self {
        let role = match msg.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        Self {
            role: role.to_string(),
            content: msg.content.clone(),
        }
    }
}

/// OpenAI chat completion response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatResponse {
    id: String,
    model: String,
    choices: Vec<ChatChoice>,
    usage: Option<UsageInfo>,
}

/// OpenAI choice in response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: Option<String>,
}

/// OpenAI usage information
#[derive(Debug, Deserialize)]
struct UsageInfo {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// OpenAI error response
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorInfo,
}

/// OpenAI error details
#[derive(Debug, Deserialize)]
struct ErrorInfo {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

/// OpenAI/ChatGPT API provider
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(config: ProviderConfig) -> Result<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| RatSquadError::api(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { config, client })
    }

    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| RatSquadError::config("OPENAI_API_KEY not set"))?;

        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let config = ProviderConfig::new(api_key, model);
        Self::new(config)
    }

    /// Create with a specific model
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new(api_key, model);
        Self::new(config)
    }

    /// Build messages array with optional system prompt
    fn build_messages(&self, messages: &[Message]) -> Vec<ChatMessage> {
        let mut result = Vec::with_capacity(messages.len() + 1);

        // Add system prompt if configured
        if let Some(ref system_prompt) = self.config.system_prompt {
            result.push(ChatMessage {
                role: "system".to_string(),
                content: system_prompt.clone(),
            });
        }

        // Add conversation messages
        for msg in messages {
            result.push(ChatMessage::from(msg));
        }

        result
    }

    /// Send a chat request to OpenAI
    async fn send_chat_request(&self, messages: Vec<ChatMessage>) -> Result<ProviderResponse> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: None,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", OPENAI_API_URL))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RatSquadError::api(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| RatSquadError::api(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<ErrorResponse>(&body) {
                return Err(RatSquadError::api(format!(
                    "OpenAI API error: {} ({})",
                    error.error.message, error.error.error_type
                )));
            }
            return Err(RatSquadError::api(format!(
                "OpenAI API error: {} - {}",
                status, body
            )));
        }

        let chat_response: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| RatSquadError::api(format!("Failed to parse response: {}", e)))?;

        let choice = chat_response
            .choices
            .first()
            .ok_or_else(|| RatSquadError::api("No response choices returned"))?;

        let truncated = choice.finish_reason.as_deref() == Some("length");

        let usage = chat_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ProviderResponse {
            content: choice.message.content.clone(),
            model: chat_response.model,
            usage,
            truncated,
        })
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    async fn complete(&self, prompt: &str) -> Result<ProviderResponse> {
        self.send_chat_request(self.build_messages(&[Message::user(prompt)]))
            .await
    }

    async fn chat(&self, messages: &[Message]) -> Result<ProviderResponse> {
        self.send_chat_request(self.build_messages(messages)).await
    }

    async fn stream(&self, prompt: &str) -> Result<Vec<String>> {
        // For now, just return the complete response as a single chunk
        // Full streaming would require SSE parsing
        let response = self.complete(prompt).await?;
        Ok(vec![response.content])
    }

    fn is_ready(&self) -> bool {
        !self.config.api_key.is_empty() && !self.config.model.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_validation() {
        let config = ProviderConfig::new("test-key", "gpt-4o");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_provider_config_empty_key_fails() {
        let config = ProviderConfig::new("", "gpt-4o");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_message_conversion() {
        let msg = Message::user("Hello");
        let chat_msg = ChatMessage::from(&msg);
        assert_eq!(chat_msg.role, "user");
        assert_eq!(chat_msg.content, "Hello");
    }

    #[test]
    fn test_provider_creation() {
        let config = ProviderConfig::new("test-key", "gpt-4o");
        let provider = OpenAIProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_provider_is_ready() {
        let config = ProviderConfig::new("test-key", "gpt-4o");
        let provider = OpenAIProvider::new(config).unwrap();
        assert!(provider.is_ready());
    }
}
