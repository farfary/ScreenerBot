/// Anthropic API client (raw HTTP via reqwest)
///
/// API Documentation: https://docs.anthropic.com/en/api/messages
///
/// Endpoints:
/// - POST https://api.anthropic.com/v1/messages
///
/// IMPORTANT DIFFERENCES FROM OPENAI:
/// - Uses `x-api-key` header instead of `Authorization: Bearer`
/// - Requires `anthropic-version: 2023-06-01` header
/// - System prompt is a SEPARATE field, NOT in messages array
/// - Response content is an ARRAY of objects, not a string
/// - max_tokens is REQUIRED (not optional)
pub mod types;

pub use self::types::{
    AnthropicContent, AnthropicMessage, AnthropicRequest, AnthropicResponse, AnthropicUsage,
};

use crate::apis::client::RateLimiter;
use crate::apis::llm::{
    ChatMessage, ChatRequest, ChatResponse, LlmClient, LlmError, MessageRole, Provider, Usage,
};
use crate::apis::stats::ApiStatsTracker;
use crate::logger::{self, LogTag};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION
// ============================================================================

const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
const ENDPOINT_MESSAGES: &str = "/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-3-haiku-20240307";
const DEFAULT_MAX_TOKENS: u32 = 4096;
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Anthropic API client
pub struct AnthropicClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    ///
    /// # Arguments
    /// * `api_key` - Anthropic API key (from https://console.anthropic.com/settings/keys)
    /// * `model` - Optional model override (defaults to "claude-3-haiku-20240307")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Anthropic API key cannot be empty".to_string());
        }

        Ok(Self {
            api_key,
            client: Client::new(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: Duration::from_secs(TIMEOUT_SECS),
            rate_limiter: RateLimiter::new(DEFAULT_RATE_LIMIT_PER_MINUTE),
            stats: Arc::new(ApiStatsTracker::new()),
            enabled,
        })
    }

    /// Convert unified ChatRequest to Anthropic-specific format
    ///
    /// IMPORTANT: Anthropic API has a DIFFERENT format:
    /// - System messages must be extracted to a separate "system" field
    /// - Only user/assistant messages go in the "messages" array
    /// - max_tokens is REQUIRED (we use DEFAULT_MAX_TOKENS if not specified)
    fn build_anthropic_request(&self, request: ChatRequest) -> AnthropicRequest {
        // Extract system message (if any) - Anthropic wants it separate!
        let mut system_prompt: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();

        for msg in request.messages {
            match msg.role {
                MessageRole::System => {
                    // Combine multiple system messages (rare, but possible)
                    if let Some(existing) = system_prompt {
                        system_prompt = Some(format!("{}\n\n{}", existing, msg.content));
                    } else {
                        system_prompt = Some(msg.content);
                    }
                }
                MessageRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content,
                    });
                }
                MessageRole::Assistant => {
                    messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content,
                    });
                }
            }
        }

        AnthropicRequest {
            model: request.model,
            max_tokens: request.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            system: system_prompt,
            messages,
            temperature: request.temperature,
        }
    }

    /// Convert Anthropic response to unified ChatResponse
    ///
    /// IMPORTANT: Anthropic response content is an ARRAY of objects with type and text!
    fn parse_anthropic_response(
        &self,
        response: AnthropicResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Extract text from content array - Anthropic returns array of content blocks
        let content = response
            .content
            .into_iter()
            .filter(|c| c.type_ == "text")
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        if content.is_empty() {
            return Err(LlmError::InvalidResponse {
                provider: "anthropic".to_string(),
                message: "No text content in response".to_string(),
            });
        }

        Ok(ChatResponse::new(
            content,
            Usage::new(response.usage.input_tokens, response.usage.output_tokens),
            response.stop_reason,
            response.model,
            latency_ms,
        ))
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: AnthropicRequest,
    ) -> Result<(AnthropicResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "anthropic".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "anthropic".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        let url = format!("{}{}", ANTHROPIC_BASE_URL, ENDPOINT_MESSAGES);

        logger::debug(
            LogTag::Api,
            &format!("[ANTHROPIC] Calling messages: model={}", request.model),
        );

        let start = Instant::now();
        let response_result = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key) // NOTE: Different from OpenAI!
            .header("anthropic-version", ANTHROPIC_VERSION) // NOTE: Required!
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await;

        drop(guard);
        let elapsed = start.elapsed().as_millis() as f64;

        let mut response = response_result.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    provider: "anthropic".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "anthropic".to_string(),
                    message: format!("Request failed: {}", e),
                }
            }
        })?;

        let status = response.status();

        // Handle error status codes
        if !status.is_success() {
            // Parse retry-after header BEFORE consuming body
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000); // Convert seconds to ms

            let error_body = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                401 => LlmError::AuthError {
                    provider: "anthropic".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "anthropic".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "anthropic".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let anthropic_response =
            response
                .json::<AnthropicResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "anthropic".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((anthropic_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    fn provider(&self) -> Provider {
        Provider::Anthropic
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Use the model from request, or fallback to client's default
        let mut request = request;
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        // Build Anthropic-specific request
        let anthropic_request = self.build_anthropic_request(request);

        // Execute the request
        let (anthropic_response, latency_ms) = match self.execute_request(anthropic_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Anthropic", "messages", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_anthropic_response(anthropic_response, latency_ms)
    }

    async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn rate_limit_info(&self) -> (usize, Duration) {
        (
            self.rate_limiter.max_per_minute(),
            self.rate_limiter.min_interval(),
        )
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apis::llm::ChatMessage;

    #[test]
    fn test_client_creation() {
        let client = AnthropicClient::new(
            "sk-ant-test-key".to_string(),
            Some("claude-3-opus-20240229".to_string()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "claude-3-opus-20240229");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = AnthropicClient::new("sk-ant-test-key".to_string(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = AnthropicClient::new("".to_string(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_anthropic_request_with_system() {
        let client = AnthropicClient::new("sk-ant-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "claude-3-haiku-20240307",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let anthropic_req = client.build_anthropic_request(request);

        // System message should be extracted to system field
        assert_eq!(anthropic_req.system, Some("You are helpful".to_string()));
        // Only user message should be in messages array
        assert_eq!(anthropic_req.messages.len(), 1);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.messages[0].content, "Hello");
        assert_eq!(anthropic_req.temperature, Some(0.7));
        assert_eq!(anthropic_req.max_tokens, 100);
    }

    #[test]
    fn test_build_anthropic_request_without_system() {
        let client = AnthropicClient::new("sk-ant-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "claude-3-haiku-20240307",
            vec![
                ChatMessage::user("Hello"),
                ChatMessage::assistant("Hi there!"),
            ],
        );

        let anthropic_req = client.build_anthropic_request(request);

        // No system message
        assert_eq!(anthropic_req.system, None);
        // Both messages in array
        assert_eq!(anthropic_req.messages.len(), 2);
        assert_eq!(anthropic_req.messages[0].role, "user");
        assert_eq!(anthropic_req.messages[1].role, "assistant");
        // Default max_tokens used
        assert_eq!(anthropic_req.max_tokens, DEFAULT_MAX_TOKENS);
    }

    #[test]
    fn test_build_anthropic_request_multiple_system() {
        let client = AnthropicClient::new("sk-ant-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "claude-3-haiku-20240307",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::system("You speak JSON"),
                ChatMessage::user("Hello"),
            ],
        );

        let anthropic_req = client.build_anthropic_request(request);

        // Multiple system messages should be combined
        assert_eq!(
            anthropic_req.system,
            Some("You are helpful\n\nYou speak JSON".to_string())
        );
        assert_eq!(anthropic_req.messages.len(), 1);
    }

    #[test]
    fn test_provider() {
        let client = AnthropicClient::new("sk-ant-test".to_string(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::Anthropic);
    }
}
