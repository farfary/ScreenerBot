//! Together AI API client (raw HTTP via reqwest)
//!
//! API Documentation: https://docs.together.ai/reference/chat-completions
//!
//! Together AI provides OpenAI-compatible endpoints with fast inference
//! for open-source models like Llama, Mixtral, and more.
//!
//! Endpoints:
//! - POST https://api.together.xyz/v1/chat/completions
//!
//! Features:
//! - OpenAI-compatible API format
//! - Fast inference on open-source models
//! - JSON mode support via response_format
//! - Free $1 credit on signup

pub mod types;

pub use self::types::{
    TogetherChoice, TogetherMessage, TogetherRequest, TogetherResponse, TogetherResponseFormat,
    TogetherResponseMessage, TogetherUsage,
};

use crate::apis::client::RateLimiter;
use crate::apis::llm::{ChatRequest, ChatResponse, LlmClient, LlmError, Provider};
use crate::apis::stats::ApiStatsTracker;
use crate::logger::{self, LogTag};
use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ============================================================================
// API CONFIGURATION
// ============================================================================

const TOGETHER_BASE_URL: &str = "https://api.together.xyz/v1";
const ENDPOINT_CHAT: &str = "/chat/completions";
const DEFAULT_MODEL: &str = "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Together AI API client
pub struct TogetherClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl TogetherClient {
    /// Create a new Together AI client
    ///
    /// # Arguments
    /// * `api_key` - Together AI API key (from https://api.together.xyz/settings/api-keys)
    /// * `model` - Optional model override (defaults to "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, LlmError> {
        if api_key.trim().is_empty() {
            return Err(LlmError::MissingApiKey {
                provider: "together".to_owned(),
            });
        }

        Ok(Self {
            api_key,
            client: crate::net::client(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: Duration::from_secs(TIMEOUT_SECS),
            rate_limiter: RateLimiter::new(DEFAULT_RATE_LIMIT_PER_MINUTE),
            stats: Arc::new(ApiStatsTracker::new()),
            enabled,
        })
    }

    /// Convert unified ChatRequest to Together-specific format
    fn build_together_request(&self, request: ChatRequest) -> TogetherRequest {
        request.into()
    }

    /// Convert Together response to unified ChatResponse
    fn parse_together_response(
        &self,
        response: TogetherResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        response.into_chat_response("together", latency_ms)
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: TogetherRequest,
    ) -> Result<(TogetherResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "together".to_owned(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "together".to_owned(),
                message: format!("Rate limiter error: {e}"),
            })?;

        let url = format!("{TOGETHER_BASE_URL}{ENDPOINT_CHAT}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[TOGETHER] Calling chat completions: model={}",
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

        let response = response_result.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    provider: "together".to_owned(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "together".to_owned(),
                    message: format!("Request failed: {e}"),
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
                    provider: "together".to_owned(),
                    message: "Invalid API key".to_owned(),
                },
                429 => LlmError::RateLimited {
                    provider: "together".to_owned(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "together".to_owned(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let together_response =
            response
                .json::<TogetherResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "together".to_owned(),
                    message: format!("Failed to parse response: {e}"),
                })?;

        Ok((together_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for TogetherClient {
    fn provider(&self) -> Provider {
        Provider::Together
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn call(&self, mut request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Use the model from request, or fallback to client's default
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        // Build Together-specific request
        let together_request = self.build_together_request(request);

        // Execute the request
        let (together_response, latency_ms) = match self.execute_request(together_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Together", "chat_completion", e.to_string())
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_together_response(together_response, latency_ms)
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
        let client = TogetherClient::new(
            "test-key".to_owned(),
            Some("meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_owned()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = TogetherClient::new("test-key".to_owned(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = TogetherClient::new("".to_owned(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_together_request() {
        let client = TogetherClient::new("test-key".to_owned(), None, true).unwrap();

        let request = ChatRequest::new(
            "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let together_req = client.build_together_request(request);

        assert_eq!(
            together_req.model,
            "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo"
        );
        assert_eq!(together_req.messages.len(), 2);
        assert_eq!(together_req.messages[0].role, "system");
        assert_eq!(together_req.messages[1].role, "user");
        assert_eq!(together_req.temperature, Some(0.7));
        assert_eq!(together_req.max_tokens, Some(100));
    }

    #[test]
    fn test_provider() {
        let client = TogetherClient::new("test-key".to_owned(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::Together);
    }
}
