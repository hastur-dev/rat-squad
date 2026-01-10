//! Ratterm REST API client
//!
//! Provides communication with the ratterm extension API.

use crate::error::{RatSquadError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Maximum request timeout in seconds
const MAX_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Maximum retry attempts for failed requests
const MAX_RETRY_ATTEMPTS: usize = 3;

/// Configuration for the ratterm client
#[derive(Debug, Clone)]
pub struct RattermClientConfig {
    /// API base URL (e.g., "http://127.0.0.1:7878")
    pub api_url: String,
    /// Bearer token for authentication
    pub api_token: String,
}

impl RattermClientConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self> {
        let api_url = std::env::var("RATTERM_API_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:7878".to_string());
        let api_token = std::env::var("RATTERM_API_TOKEN")
            .map_err(|_| RatSquadError::config("RATTERM_API_TOKEN not set"))?;

        assert!(!api_url.is_empty(), "API URL must not be empty");
        assert!(!api_token.is_empty(), "API token must not be empty");

        Ok(Self { api_url, api_token })
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        if self.api_url.is_empty() {
            return Err(RatSquadError::validation("API URL must not be empty"));
        }
        if self.api_token.is_empty() {
            return Err(RatSquadError::validation("API token must not be empty"));
        }
        if !self.api_url.starts_with("http://") && !self.api_url.starts_with("https://") {
            return Err(RatSquadError::validation("API URL must be HTTP or HTTPS"));
        }
        Ok(())
    }
}

/// Terminal buffer response
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalBuffer {
    /// Buffer content
    pub content: String,
    /// Number of rows
    pub rows: u16,
    /// Number of columns
    pub cols: u16,
}

/// Terminal size response
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalSize {
    /// Number of columns
    pub cols: u16,
    /// Number of rows
    pub rows: u16,
}

/// Terminal tab info
#[derive(Debug, Clone, Deserialize)]
pub struct TerminalTab {
    /// Tab identifier
    pub tab_id: u32,
    /// Tab title
    pub title: String,
}

/// Send keys request
#[derive(Debug, Serialize)]
struct SendKeysRequest {
    keys: String,
}

/// Create tab request
#[derive(Debug, Serialize)]
struct CreateTabRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

/// Set status request
#[derive(Debug, Serialize)]
struct SetStatusRequest {
    message: String,
}

/// Register command request
#[derive(Debug, Serialize)]
struct RegisterCommandRequest {
    name: String,
    description: String,
}

/// Generic success response
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SuccessResponse {
    success: bool,
}

/// Ratterm REST API client
#[derive(Debug, Clone)]
pub struct RattermClient {
    client: Client,
    base_url: String,
    token: String,
}

impl RattermClient {
    /// Create a new ratterm client
    pub fn new(config: RattermClientConfig) -> Result<Self> {
        config.validate()?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(MAX_REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| RatSquadError::api(format!("Failed to create HTTP client: {e}")))?;

        assert!(!config.api_url.is_empty(), "API URL validated but empty");
        assert!(!config.api_token.is_empty(), "API token validated but empty");

        Ok(Self {
            client,
            base_url: config.api_url,
            token: config.api_token,
        })
    }

    /// Build authorization header
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Build full URL for an endpoint
    fn url(&self, endpoint: &str) -> String {
        format!("{}/api/v1{}", self.base_url, endpoint)
    }

    /// Perform a GET request with retry logic
    async fn get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        let url = self.url(endpoint);

        for attempt in 0..MAX_RETRY_ATTEMPTS {
            let response = self
                .client
                .get(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let body = resp
                        .json::<T>()
                        .await
                        .map_err(|e| RatSquadError::api(format!("Failed to parse response: {e}")))?;
                    return Ok(body);
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt < MAX_RETRY_ATTEMPTS - 1 && status.is_server_error() {
                        continue;
                    }
                    return Err(RatSquadError::api(format!("Request failed with status: {status}")));
                }
                Err(e) if attempt < MAX_RETRY_ATTEMPTS - 1 && e.is_timeout() => {
                    continue;
                }
                Err(e) => {
                    return Err(RatSquadError::api(format!("Request failed: {e}")));
                }
            }
        }

        Err(RatSquadError::api("Max retry attempts exceeded"))
    }

    /// Perform a POST request with retry logic
    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<R> {
        let url = self.url(endpoint);

        for attempt in 0..MAX_RETRY_ATTEMPTS {
            let response = self
                .client
                .post(&url)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .json(body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let body = resp
                        .json::<R>()
                        .await
                        .map_err(|e| RatSquadError::api(format!("Failed to parse response: {e}")))?;
                    return Ok(body);
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt < MAX_RETRY_ATTEMPTS - 1 && status.is_server_error() {
                        continue;
                    }
                    return Err(RatSquadError::api(format!("Request failed with status: {status}")));
                }
                Err(e) if attempt < MAX_RETRY_ATTEMPTS - 1 && e.is_timeout() => {
                    continue;
                }
                Err(e) => {
                    return Err(RatSquadError::api(format!("Request failed: {e}")));
                }
            }
        }

        Err(RatSquadError::api("Max retry attempts exceeded"))
    }

    /// Perform a PUT request with retry logic
    async fn put<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<R> {
        let url = self.url(endpoint);

        for attempt in 0..MAX_RETRY_ATTEMPTS {
            let response = self
                .client
                .put(&url)
                .header("Authorization", self.auth_header())
                .header("Content-Type", "application/json")
                .json(body)
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let body = resp
                        .json::<R>()
                        .await
                        .map_err(|e| RatSquadError::api(format!("Failed to parse response: {e}")))?;
                    return Ok(body);
                }
                Ok(resp) => {
                    let status = resp.status();
                    if attempt < MAX_RETRY_ATTEMPTS - 1 && status.is_server_error() {
                        continue;
                    }
                    return Err(RatSquadError::api(format!("Request failed with status: {status}")));
                }
                Err(e) if attempt < MAX_RETRY_ATTEMPTS - 1 && e.is_timeout() => {
                    continue;
                }
                Err(e) => {
                    return Err(RatSquadError::api(format!("Request failed: {e}")));
                }
            }
        }

        Err(RatSquadError::api("Max retry attempts exceeded"))
    }

    /// Get terminal buffer content
    pub async fn get_terminal_buffer(&self) -> Result<TerminalBuffer> {
        self.get("/terminal/buffer").await
    }

    /// Send keys to terminal
    pub async fn send_keys(&self, keys: &str) -> Result<()> {
        let request = SendKeysRequest {
            keys: keys.to_string(),
        };
        let _: SuccessResponse = self.post("/terminal/send_keys", &request).await?;
        Ok(())
    }

    /// Get terminal size
    pub async fn get_terminal_size(&self) -> Result<TerminalSize> {
        self.get("/terminal/size").await
    }

    /// Set status bar message
    pub async fn set_status(&self, message: &str) -> Result<()> {
        let request = SetStatusRequest {
            message: message.to_string(),
        };
        let _: SuccessResponse = self.put("/system/status", &request).await?;
        Ok(())
    }

    /// Create a new terminal tab
    pub async fn create_terminal_tab(&self, title: Option<&str>) -> Result<TerminalTab> {
        let request = CreateTabRequest {
            title: title.map(String::from),
        };
        self.post("/tabs/terminal/new", &request).await
    }

    /// Switch to a terminal tab
    pub async fn switch_terminal_tab(&self, tab_id: u32) -> Result<()> {
        #[derive(Serialize)]
        struct SwitchRequest {
            tab_id: u32,
        }
        let request = SwitchRequest { tab_id };
        let _: SuccessResponse = self.post("/tabs/terminal/switch", &request).await?;
        Ok(())
    }

    /// Close a terminal tab
    pub async fn close_terminal_tab(&self, tab_id: u32) -> Result<()> {
        #[derive(Serialize)]
        struct CloseRequest {
            tab_id: u32,
        }
        let request = CloseRequest { tab_id };
        let _: SuccessResponse = self
            .client
            .delete(&self.url("/tabs/terminal/close"))
            .header("Authorization", self.auth_header())
            .json(&request)
            .send()
            .await
            .map_err(|e| RatSquadError::api(format!("Request failed: {e}")))?
            .json()
            .await
            .map_err(|e| RatSquadError::api(format!("Failed to parse response: {e}")))?;
        Ok(())
    }

    /// Register a custom command
    pub async fn register_command(&self, name: &str, description: &str) -> Result<()> {
        let request = RegisterCommandRequest {
            name: name.to_string(),
            description: description.to_string(),
        };
        let _: SuccessResponse = self.post("/commands/register", &request).await?;
        Ok(())
    }

    /// Show a notification
    pub async fn notify(&self, message: &str) -> Result<()> {
        #[derive(Serialize)]
        struct NotifyRequest {
            message: String,
        }
        let request = NotifyRequest {
            message: message.to_string(),
        };
        let _: SuccessResponse = self.post("/system/notify", &request).await?;
        Ok(())
    }

    /// Get current working directory
    pub async fn get_cwd(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct CwdResponse {
            cwd: String,
        }
        let resp: CwdResponse = self.get("/system/cwd").await?;
        Ok(resp.cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_valid() {
        let config = RattermClientConfig {
            api_url: "http://127.0.0.1:7878".to_string(),
            api_token: "test-token".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_url() {
        let config = RattermClientConfig {
            api_url: String::new(),
            api_token: "test-token".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_empty_token() {
        let config = RattermClientConfig {
            api_url: "http://127.0.0.1:7878".to_string(),
            api_token: String::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_protocol() {
        let config = RattermClientConfig {
            api_url: "ftp://127.0.0.1:7878".to_string(),
            api_token: "test-token".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_url_building() {
        let config = RattermClientConfig {
            api_url: "http://127.0.0.1:7878".to_string(),
            api_token: "test".to_string(),
        };
        let client = RattermClient::new(config).unwrap();
        assert_eq!(
            client.url("/terminal/buffer"),
            "http://127.0.0.1:7878/api/v1/terminal/buffer"
        );
    }
}
