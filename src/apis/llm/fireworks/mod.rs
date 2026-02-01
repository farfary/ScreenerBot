/// Fireworks AI API client (raw HTTP via reqwest)
///
/// API Documentation: https://docs.fireworks.ai/api-reference/chat-completions
///
/// Endpoints:
/// - POST https://api.fireworks.ai/inference/v1/chat/completions
///
/// Fireworks AI provides an OpenAI-compatible API, so we use the same request/response types.
pub mod types;

pub use self::types::{
    FireworksChoice, FireworksMessage, FireworksRequest, FireworksResponse,
    FireworksResponseFormat, FireworksResponseMessage, FireworksUsage,
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

const FIREWORKS_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";
const ENDPOINT_CHAT: &str = "/chat/completions";
const DEFAULT_MODEL: &str = "accounts/fireworks/models/llama-v3p1-8b-instruct";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Fireworks AI API client
pub struct FireworksClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl FireworksClient {
    /// Create a new Fireworks AI client
    ///
    /// # Arguments
    /// * `api_key` - Fireworks AI API key (from https://fireworks.ai/account)
    /// * `model` - Optional model override (defaults to "accounts/fireworks/models/llama-v3p1-8b-instruct")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Fireworks AI API key cannot be empty".to_string());
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

    /// Convert unified ChatRequest to Fireworks-specific format
    fn build_fireworks_request(&self, request: ChatRequest) -> FireworksRequest {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| FireworksMessage {
                role: match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content,
            })
            .collect();

        FireworksRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request
                .response_format
                .map(|rf| FireworksResponseFormat { type_: rf.type_ }),
        }
    }

    /// Convert Fireworks response to unified ChatResponse
    fn parse_fireworks_response(
        &self,
        response: FireworksResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Get the first choice
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "fireworks".to_string(),
                message: "No choices in response".to_string(),
            })?;

        Ok(ChatResponse::new(
            choice.message.content.clone(),
            Usage::new(
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
            ),
            choice.finish_reason.clone(),
            response.model,
            latency_ms,
        ))
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: FireworksRequest,
    ) -> Result<(FireworksResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "fireworks".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "fireworks".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        let url = format!("{}{}", FIREWORKS_BASE_URL, ENDPOINT_CHAT);

        logger::debug(
            LogTag::Api,
            &format!(
                "[FIREWORKS] Calling chat completions: model={}",
                request.model
            ),
        );

        let start = Instant::now();
        let response_result = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
                    provider: "fireworks".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "fireworks".to_string(),
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
                    provider: "fireworks".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "fireworks".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "fireworks".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let fireworks_response =
            response
                .json::<FireworksResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "fireworks".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((fireworks_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for FireworksClient {
    fn provider(&self) -> Provider {
        Provider::Fireworks
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

        // Build Fireworks-specific request
        let fireworks_request = self.build_fireworks_request(request);

        // Execute the request
        let (fireworks_response, latency_ms) = match self.execute_request(fireworks_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Fireworks", "chat_completion", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_fireworks_response(fireworks_response, latency_ms)
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
        let client = FireworksClient::new(
            "fw-test-key".to_string(),
            Some("accounts/fireworks/models/mixtral-8x7b-instruct".to_string()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(
            client.model,
            "accounts/fireworks/models/mixtral-8x7b-instruct"
        );
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = FireworksClient::new("fw-test-key".to_string(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = FireworksClient::new("".to_string(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_fireworks_request() {
        let client = FireworksClient::new("fw-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "accounts/fireworks/models/llama-v3p1-8b-instruct",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let fireworks_req = client.build_fireworks_request(request);

        assert_eq!(
            fireworks_req.model,
            "accounts/fireworks/models/llama-v3p1-8b-instruct"
        );
        assert_eq!(fireworks_req.messages.len(), 2);
        assert_eq!(fireworks_req.messages[0].role, "system");
        assert_eq!(fireworks_req.messages[1].role, "user");
        assert_eq!(fireworks_req.temperature, Some(0.7));
        assert_eq!(fireworks_req.max_tokens, Some(100));
    }

    #[test]
    fn test_provider() {
        let client = FireworksClient::new("fw-test".to_string(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::Fireworks);
    }
}
