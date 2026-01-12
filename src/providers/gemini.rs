//! Google Gemini API client
//!
//! Provides direct API access to Google's Gemini models.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{Message, Provider, ProviderConfig, ProviderResponse, Role, TokenUsage};
use crate::error::{RatSquadError, Result};

/// Gemini API base URL
const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Gemini generate content request
#[derive(Debug, Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemInstruction")]
    system_instruction: Option<Content>,
}

/// Gemini content object
#[derive(Debug, Serialize, Deserialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<Part>,
}

/// Gemini content part
#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

/// Gemini generation configuration
#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "topK")]
    top_k: u32,
}

/// Gemini generate content response
#[derive(Debug, Deserialize)]
struct GenerateResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

/// Gemini response candidate
#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

/// Gemini usage metadata
#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

/// Gemini error response
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: ErrorInfo,
}

/// Gemini error details
#[derive(Debug, Deserialize)]
struct ErrorInfo {
    message: String,
    status: String,
}

/// Google Gemini API provider
pub struct GeminiProvider {
    config: ProviderConfig,
    client: Client,
}

impl GeminiProvider {
    /// Create a new Gemini provider
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
        let api_key = std::env::var("GOOGLE_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .map_err(|_| RatSquadError::config("GOOGLE_API_KEY or GEMINI_API_KEY not set"))?;

        let model =
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.0-flash".to_string());

        let config = ProviderConfig::new(api_key, model);
        Self::new(config)
    }

    /// Create with a specific model
    pub fn with_model(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let config = ProviderConfig::new(api_key, model);
        Self::new(config)
    }

    /// Convert messages to Gemini content format
    fn build_contents(&self, messages: &[Message]) -> (Vec<Content>, Option<Content>) {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        // Check for system prompt in config
        if let Some(ref prompt) = self.config.system_prompt {
            system_instruction = Some(Content {
                role: None,
                parts: vec![Part {
                    text: prompt.clone(),
                }],
            });
        }

        for msg in messages {
            match msg.role {
                Role::System => {
                    // Gemini uses systemInstruction for system prompts
                    system_instruction = Some(Content {
                        role: None,
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                    });
                }
                Role::User => {
                    contents.push(Content {
                        role: Some("user".to_string()),
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                    });
                }
                Role::Assistant => {
                    contents.push(Content {
                        role: Some("model".to_string()),
                        parts: vec![Part {
                            text: msg.content.clone(),
                        }],
                    });
                }
            }
        }

        (contents, system_instruction)
    }

    /// Send a generate request to Gemini
    async fn send_generate_request(
        &self,
        contents: Vec<Content>,
        system_instruction: Option<Content>,
    ) -> Result<ProviderResponse> {
        let request = GenerateRequest {
            contents,
            generation_config: GenerationConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
                top_p: 0.95,
                top_k: 40,
            },
            system_instruction,
        };

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_URL, self.config.model, self.config.api_key
        );

        let response = self
            .client
            .post(&url)
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
                    "Gemini API error: {} ({})",
                    error.error.message, error.error.status
                )));
            }
            return Err(RatSquadError::api(format!(
                "Gemini API error: {} - {}",
                status, body
            )));
        }

        let generate_response: GenerateResponse = serde_json::from_str(&body)
            .map_err(|e| RatSquadError::api(format!("Failed to parse response: {}", e)))?;

        let candidate = generate_response
            .candidates
            .first()
            .ok_or_else(|| RatSquadError::api("No response candidates returned"))?;

        let content = candidate
            .content
            .parts
            .first()
            .map(|p| p.text.clone())
            .unwrap_or_default();

        let truncated = candidate.finish_reason.as_deref() == Some("MAX_TOKENS");

        let usage = generate_response.usage_metadata.map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
        });

        Ok(ProviderResponse {
            content,
            model: self.config.model.clone(),
            usage,
            truncated,
        })
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &'static str {
        "Gemini"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    async fn complete(&self, prompt: &str) -> Result<ProviderResponse> {
        let contents = vec![Content {
            role: Some("user".to_string()),
            parts: vec![Part {
                text: prompt.to_string(),
            }],
        }];

        let system_instruction = self.config.system_prompt.as_ref().map(|p| Content {
            role: None,
            parts: vec![Part { text: p.clone() }],
        });

        self.send_generate_request(contents, system_instruction)
            .await
    }

    async fn chat(&self, messages: &[Message]) -> Result<ProviderResponse> {
        let (contents, system_instruction) = self.build_contents(messages);
        self.send_generate_request(contents, system_instruction)
            .await
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
    fn test_provider_creation() {
        let config = ProviderConfig::new("test-key", "gemini-2.0-flash");
        let provider = GeminiProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_provider_is_ready() {
        let config = ProviderConfig::new("test-key", "gemini-2.0-flash");
        let provider = GeminiProvider::new(config).unwrap();
        assert!(provider.is_ready());
    }

    #[test]
    fn test_content_conversion_user() {
        let config = ProviderConfig::new("test-key", "gemini-2.0-flash");
        let provider = GeminiProvider::new(config).unwrap();

        let messages = vec![Message::user("Hello")];
        let (contents, _) = provider.build_contents(&messages);

        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("user".to_string()));
    }

    #[test]
    fn test_content_conversion_with_system() {
        let config = ProviderConfig::new("test-key", "gemini-2.0-flash")
            .with_system_prompt("You are helpful");
        let provider = GeminiProvider::new(config).unwrap();

        let messages = vec![Message::user("Hello")];
        let (contents, system) = provider.build_contents(&messages);

        assert_eq!(contents.len(), 1);
        assert!(system.is_some());
    }

    #[test]
    fn test_content_conversion_assistant() {
        let config = ProviderConfig::new("test-key", "gemini-2.0-flash");
        let provider = GeminiProvider::new(config).unwrap();

        let messages = vec![Message::assistant("Hello there")];
        let (contents, _) = provider.build_contents(&messages);

        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0].role, Some("model".to_string()));
    }
}
