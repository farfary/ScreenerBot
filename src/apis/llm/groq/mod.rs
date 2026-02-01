/// Groq API client (raw HTTP via reqwest)
///
/// API Documentation: https://console.groq.com/docs/api-reference
///
/// Groq uses an OpenAI-compatible API, so the implementation is very similar.
///
/// Endpoints:
/// - POST https://api.groq.com/openai/v1/chat/completions
///
/// Free Tier Rate Limits:
/// - 30 requests per minute
/// - 14,400 requests per day
/// - 6,000 tokens per minute
///
/// Models:
/// - llama-3.1-8b-instant (fast, small)
/// - mixtral-8x7b-32768 (good balance)
/// - llama-3.3-70b-versatile (most capable)
pub mod types;

pub use self::types::{
    GroqChoice, GroqMessage, GroqRequest, GroqResponse, GroqResponseFormat, GroqResponseMessage,
    GroqUsage,
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

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";
const ENDPOINT_CHAT: &str = "/chat/completions";
const DEFAULT_MODEL: &str = "llama-3.1-8b-instant";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 30; // Free tier: 30 RPM

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Groq API client
pub struct GroqClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl GroqClient {
    /// Create a new Groq client
    ///
    /// # Arguments
    /// * `api_key` - Groq API key (from https://console.groq.com/keys)
    /// * `model` - Optional model override (defaults to "llama-3.1-8b-instant")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Groq API key cannot be empty".to_string());
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

    /// Convert unified ChatRequest to Groq-specific format (same as OpenAI)
    fn build_groq_request(&self, request: ChatRequest) -> GroqRequest {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| GroqMessage {
                role: match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content,
            })
            .collect();

        GroqRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request
                .response_format
                .map(|rf| GroqResponseFormat { type_: rf.type_ }),
        }
    }

    /// Convert Groq response to unified ChatResponse
    fn parse_groq_response(
        &self,
        response: GroqResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Get the first choice
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "groq".to_string(),
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
    async fn execute_request(&self, request: GroqRequest) -> Result<(GroqResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "groq".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "groq".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        let url = format!("{}{}", GROQ_BASE_URL, ENDPOINT_CHAT);

        logger::debug(
            LogTag::Api,
            &format!("[GROQ] Calling chat completions: model={}", request.model),
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
                    provider: "groq".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "groq".to_string(),
                    message: format!("Request failed: {}", e),
                }
            }
        })?;

        let status = response.status();

        // Handle error status codes
        if !status.is_success() {
            // Parse retry-after header BEFORE consuming body
            // Groq uses both "retry-after" and "x-ratelimit-reset-requests"
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|s| s * 1000); // Convert seconds to ms

            let error_body = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                401 => LlmError::AuthError {
                    provider: "groq".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "groq".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "groq".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let groq_response =
            response
                .json::<GroqResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "groq".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((groq_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for GroqClient {
    fn provider(&self) -> Provider {
        Provider::Groq
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

        // Build Groq-specific request
        let groq_request = self.build_groq_request(request);

        // Execute the request
        let (groq_response, latency_ms) = match self.execute_request(groq_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Groq", "chat_completion", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_groq_response(groq_response, latency_ms)
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
        let client = GroqClient::new(
            "gsk-test-key".to_string(),
            Some("llama-3.3-70b-versatile".to_string()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "llama-3.3-70b-versatile");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = GroqClient::new("gsk-test-key".to_string(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = GroqClient::new("".to_string(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_groq_request() {
        let client = GroqClient::new("gsk-test".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "llama-3.1-8b-instant",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let groq_req = client.build_groq_request(request);

        assert_eq!(groq_req.model, "llama-3.1-8b-instant");
        assert_eq!(groq_req.messages.len(), 2);
        assert_eq!(groq_req.messages[0].role, "system");
        assert_eq!(groq_req.messages[1].role, "user");
        assert_eq!(groq_req.temperature, Some(0.7));
        assert_eq!(groq_req.max_tokens, Some(100));
    }

    #[test]
    fn test_provider() {
        let client = GroqClient::new("gsk-test".to_string(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::Groq);
    }
}
