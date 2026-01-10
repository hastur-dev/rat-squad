//! Tests for the ratterm REST API client
//!
//! Tests cover: API connectivity, terminal operations, editor operations,
//! filesystem operations, and system operations.

use rat_squad::ratterm_client::{RattermClient, RattermClientConfig};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MAX_TEST_ITERATIONS: usize = 100;

/// Test client creation with valid configuration
#[tokio::test]
async fn test_client_creation_valid_config() {
    let config = RattermClientConfig {
        api_url: "http://127.0.0.1:7878".to_string(),
        api_token: "test-token".to_string(),
    };

    assert!(!config.api_url.is_empty(), "API URL must not be empty");
    assert!(!config.api_token.is_empty(), "API token must not be empty");

    let client = RattermClient::new(config);
    assert!(client.is_ok(), "Client should be created successfully");
}

/// Test client creation with empty URL fails
#[tokio::test]
async fn test_client_creation_empty_url_fails() {
    let config = RattermClientConfig {
        api_url: String::new(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config);
    assert!(client.is_err(), "Client creation should fail with empty URL");
}

/// Test client creation with empty token fails
#[tokio::test]
async fn test_client_creation_empty_token_fails() {
    let config = RattermClientConfig {
        api_url: "http://127.0.0.1:7878".to_string(),
        api_token: String::new(),
    };

    let client = RattermClient::new(config);
    assert!(client.is_err(), "Client creation should fail with empty token");
}

/// Test getting terminal buffer via mock server
#[tokio::test]
async fn test_get_terminal_buffer() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/terminal/buffer"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "content": "$ echo hello\nhello\n$ ",
            "rows": 24,
            "cols": 80
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let buffer = client.get_terminal_buffer().await;

    assert!(buffer.is_ok(), "Should get terminal buffer");
    let buffer = buffer.unwrap();
    assert!(buffer.content.contains("hello"), "Buffer should contain expected content");
    assert_eq!(buffer.rows, 24, "Rows should match");
    assert_eq!(buffer.cols, 80, "Cols should match");
}

/// Test sending keys to terminal
#[tokio::test]
async fn test_send_keys_to_terminal() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/terminal/send_keys"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.send_keys("echo hello\n").await;

    assert!(result.is_ok(), "Should send keys successfully");
}

/// Test getting terminal size
#[tokio::test]
async fn test_get_terminal_size() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/terminal/size"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "cols": 120,
            "rows": 40
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let size = client.get_terminal_size().await;

    assert!(size.is_ok(), "Should get terminal size");
    let size = size.unwrap();
    assert_eq!(size.cols, 120, "Cols should match");
    assert_eq!(size.rows, 40, "Rows should match");
}

/// Test setting status message
#[tokio::test]
async fn test_set_status_message() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/api/v1/system/status"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.set_status("rat-squad: 3 agents running").await;

    assert!(result.is_ok(), "Should set status successfully");
}

/// Test creating terminal tab
#[tokio::test]
async fn test_create_terminal_tab() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/tabs/terminal/new"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tab_id": 1,
            "title": "Agent-1"
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.create_terminal_tab(Some("Agent-1")).await;

    assert!(result.is_ok(), "Should create terminal tab");
    let tab = result.unwrap();
    assert_eq!(tab.tab_id, 1, "Tab ID should match");
}

/// Test handling API errors gracefully
#[tokio::test]
async fn test_api_error_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/terminal/buffer"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "error": "Internal server error"
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.get_terminal_buffer().await;

    assert!(result.is_err(), "Should return error for 500 response");
}

/// Test handling network errors
#[tokio::test]
async fn test_network_error_handling() {
    let config = RattermClientConfig {
        api_url: "http://127.0.0.1:99999".to_string(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.get_terminal_buffer().await;

    assert!(result.is_err(), "Should return error for network failure");
}

/// Test registering custom command
#[tokio::test]
async fn test_register_command() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/commands/register"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true
        })))
        .mount(&mock_server)
        .await;

    let config = RattermClientConfig {
        api_url: mock_server.uri(),
        api_token: "test-token".to_string(),
    };

    let client = RattermClient::new(config).expect("Client creation should succeed");
    let result = client.register_command("squad-new", "Create new agent session").await;

    assert!(result.is_ok(), "Should register command successfully");
}

/// Property: API URL must be valid HTTP/HTTPS
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn config_rejects_non_http_urls(url in "[a-z]+://[a-z]+\\.[a-z]+") {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                let config = RattermClientConfig {
                    api_url: url,
                    api_token: "token".to_string(),
                };
                let client = RattermClient::new(config);
                prop_assert!(client.is_err());
            }
        }

        #[test]
        fn token_must_be_non_empty(token in ".*") {
            let config = RattermClientConfig {
                api_url: "http://127.0.0.1:7878".to_string(),
                api_token: token.clone(),
            };
            let client = RattermClient::new(config);
            if token.is_empty() {
                prop_assert!(client.is_err());
            } else {
                prop_assert!(client.is_ok());
            }
        }
    }
}
