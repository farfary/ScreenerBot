/// OpenAI API client (raw HTTP via reqwest)
///
/// API Documentation: https://platform.openai.com/docs/api-reference/chat
///
/// Endpoints:
/// - POST https://api.openai.com/v1/chat/completions
pub mod types;

pub use self::types::{
    OpenAiChoice, OpenAiMessage, OpenAiRequest, OpenAiResponse, OpenAiResponseFormat,
    OpenAiResponseMessage, OpenAiUsage,
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

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const ENDPOINT_CHAT: &str = "/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// OpenAI API client
pub struct OpenAiClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl OpenAiClient {
    /// Create a new OpenAI client
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key (from https://platform.openai.com/api-keys)
    /// * `model` - Optional model override (defaults to "gpt-4o-mini")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("OpenAI API key cannot be empty".to_string());
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

    /// Convert unified ChatRequest to OpenAI-specific format
    fn build_openai_request(&self, request: ChatRequest) -> OpenAiRequest {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| OpenAiMessage {
                role: match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content,
            })
            .collect();

        OpenAiRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request
                .response_format
                .map(|rf| OpenAiResponseFormat { type_: rf.type_ }),
        }
    }

    /// Convert OpenAI response to unified ChatResponse
    fn parse_openai_response(
        &self,
        response: OpenAiResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Get the first choice
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "openai".to_string(),
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
        request: OpenAiRequest,
    ) -> Result<(OpenAiResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "openai".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "openai".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        let url = format!("{}{}", OPENAI_BASE_URL, ENDPOINT_CHAT);

        logger::debug(
            LogTag::Api,
            &format!("[OPENAI] Calling chat completions: model={}", request.model),
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
                    provider: "openai".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "openai".to_string(),
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
                    provider: "openai".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "openai".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "openai".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let openai_response =
            response
                .json::<OpenAiResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "openai".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((openai_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    fn provider(&self) -> Provider {
        Provider::OpenAi
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

        // Build OpenAI-specific request
        let openai_request = self.build_openai_request(request);

        // Execute the request
        let (openai_response, latency_ms) = match self.execute_request(openai_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("OpenAI", "chat_completion", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_openai_response(openai_response, latency_ms)
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
        let client = OpenAiClient::new("sk-test-key".to_string(), Some("gpt-4".to_string()), true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "gpt-4");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = OpenAiClient::new("sk-test-key".to_string(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = OpenAiClient::new("".to_string(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_openai_request() {
        let client = OpenAiClient::new("sk-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "gpt-4",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let openai_req = client.build_openai_request(request);

        assert_eq!(openai_req.model, "gpt-4");
        assert_eq!(openai_req.messages.len(), 2);
        assert_eq!(openai_req.messages[0].role, "system");
        assert_eq!(openai_req.messages[1].role, "user");
        assert_eq!(openai_req.temperature, Some(0.7));
        assert_eq!(openai_req.max_tokens, Some(100));
    }

    #[test]
    fn test_provider() {
        let client = OpenAiClient::new("sk-test".to_string(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::OpenAi);
    }
}
