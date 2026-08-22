//! OpenRouter API client (raw HTTP via reqwest)
//!
//! API Documentation: https://openrouter.ai/docs
//!
//! OpenRouter is a unified gateway to 100+ LLM models including:
//! - OpenAI models (gpt-4o, gpt-4-turbo, etc.)
//! - Anthropic Claude models
//! - Google Gemini models
//! - Meta Llama models
//! - And many more
//!
//! The API is OpenAI-compatible with optional site identification headers.
//!
//! Endpoints:
//! - POST https://openrouter.ai/api/v1/chat/completions

pub mod types;

pub use self::types::{
    OpenAiChoice as OpenRouterChoice, OpenAiMessage as OpenRouterMessage,
    OpenAiRequest as OpenRouterRequest, OpenAiResponse as OpenRouterResponse,
    OpenAiResponseFormat as OpenRouterResponseFormat,
    OpenAiResponseMessage as OpenRouterResponseMessage, OpenAiUsage as OpenRouterUsage,
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

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const ENDPOINT_CHAT: &str = "/chat/completions";
const DEFAULT_MODEL: &str = "meta-llama/llama-3.1-8b-instruct:free";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60;

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// OpenRouter API client
pub struct OpenRouterClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
    site_url: Option<String>,
    site_name: Option<String>,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client
    ///
    /// # Arguments
    /// * `api_key` - OpenRouter API key (from https://openrouter.ai/keys)
    /// * `model` - Optional model override (defaults to "meta-llama/llama-3.1-8b-instruct:free")
    /// * `enabled` - Whether the client is enabled
    /// * `site_url` - Optional site URL for HTTP-Referer header (helps with ranking)
    /// * `site_name` - Optional site name for X-Title header (helps with ranking)
    pub fn new(
        api_key: String,
        model: Option<String>,
        enabled: bool,
        site_url: Option<String>,
        site_name: Option<String>,
    ) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("OpenRouter API key cannot be empty".to_owned());
        }

        Ok(Self {
            api_key,
            client: crate::net::client(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: Duration::from_secs(TIMEOUT_SECS),
            rate_limiter: RateLimiter::new(DEFAULT_RATE_LIMIT_PER_MINUTE),
            stats: Arc::new(ApiStatsTracker::new()),
            enabled,
            site_url,
            site_name,
        })
    }

    /// Convert unified ChatRequest to OpenRouter-specific format
    fn build_openrouter_request(&self, request: ChatRequest) -> OpenRouterRequest {
        request.into()
    }

    /// Convert OpenRouter response to unified ChatResponse
    fn parse_openrouter_response(
        response: OpenRouterResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        response.into_chat_response("openrouter", latency_ms)
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: OpenRouterRequest,
    ) -> Result<(OpenRouterResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "openrouter".to_owned(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "openrouter".to_owned(),
                message: format!("Rate limiter error: {e}"),
            })?;

        let url = format!("{OPENROUTER_BASE_URL}{ENDPOINT_CHAT}");

        logger::debug(
            LogTag::Api,
            &format!(
                "[OPENROUTER] Calling chat completions: model={}",
                request.model
            ),
        );

        let start = Instant::now();

        // Build request with headers
        let mut req_builder = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "ScreenerBot/1.0 (https://screenerbot.io)")
            .header(
                "HTTP-Referer",
                self.site_url.as_deref().unwrap_or("https://screenerbot.io"),
            )
            .header(
                "X-Title",
                self.site_name.as_deref().unwrap_or("ScreenerBot"),
            );

        // Add optional site identification headers (override defaults)
        if let Some(ref site_url) = self.site_url {
            req_builder = req_builder.header("HTTP-Referer", site_url);
        }
        if let Some(ref site_name) = self.site_name {
            req_builder = req_builder.header("X-Title", site_name);
        }

        let response_result = req_builder
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await;

        drop(guard);
        let elapsed = start.elapsed().as_millis() as f64;

        let response = response_result.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    provider: "openrouter".to_owned(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "openrouter".to_owned(),
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
                    provider: "openrouter".to_owned(),
                    message: "Invalid API key".to_owned(),
                },
                429 => LlmError::RateLimited {
                    provider: "openrouter".to_owned(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "openrouter".to_owned(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let openrouter_response =
            response
                .json::<OpenRouterResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "openrouter".to_owned(),
                    message: format!("Failed to parse response: {e}"),
                })?;

        Ok((openrouter_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    fn provider(&self) -> Provider {
        Provider::OpenRouter
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn call(&self, mut request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Use the model from request, or fallback to client's default
        if request.model.is_empty() {
            request.model = self.model.clone();
        }

        // Build OpenRouter-specific request
        let openrouter_request = self.build_openrouter_request(request);

        // Execute the request
        let (openrouter_response, latency_ms) = match self.execute_request(openrouter_request).await
        {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("OpenRouter", "chat_completion", e.to_string())
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        Self::parse_openrouter_response(openrouter_response, latency_ms)
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
        let client = OpenRouterClient::new(
            "sk-or-test-key".to_owned(),
            Some("openai/gpt-4o".to_owned()),
            true,
            None,
            None,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "openai/gpt-4o");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = OpenRouterClient::new("sk-or-test-key".to_owned(), None, true, None, None);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_with_site_info() {
        let client = OpenRouterClient::new(
            "sk-or-test-key".to_owned(),
            None,
            true,
            Some("https://example.com".to_owned()),
            Some("My App".to_owned()),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.site_url, Some("https://example.com".to_owned()));
        assert_eq!(client.site_name, Some("My App".to_owned()));
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = OpenRouterClient::new("".to_owned(), None, true, None, None);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_openrouter_request() {
        let client =
            OpenRouterClient::new("sk-or-test".to_owned(), None, true, None, None).unwrap();

        let request = ChatRequest::new(
            "openai/gpt-4o",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let openrouter_req = client.build_openrouter_request(request);

        assert_eq!(openrouter_req.model, "openai/gpt-4o");
        assert_eq!(openrouter_req.messages.len(), 2);
        assert_eq!(openrouter_req.messages[0].role, "system");
        assert_eq!(openrouter_req.messages[1].role, "user");
        assert_eq!(openrouter_req.temperature, Some(0.7));
        assert_eq!(openrouter_req.max_tokens, Some(100));
    }

    #[test]
    fn test_provider() {
        let client =
            OpenRouterClient::new("sk-or-test".to_owned(), None, true, None, None).unwrap();
        assert_eq!(client.provider(), Provider::OpenRouter);
    }

    #[test]
    fn test_reasoning_is_not_exposed_as_answer() {
        let response: OpenRouterResponse = serde_json::from_str(
            r#"{
                "id":"test","object":"chat.completion","created":1,"model":"test/model",
                "choices":[{"index":0,"message":{"role":"assistant","content":null,"reasoning":"Reasoned answer"},"finish_reason":"stop"}],
                "usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}
            }"#,
        )
        .unwrap();
        let error = OpenRouterClient::parse_openrouter_response(response, 12.0).unwrap_err();
        assert!(matches!(
            error,
            LlmError::InvalidResponse { provider, message }
                if provider == "openrouter" && message.contains("reasoning but no answer")
        ));
    }

    #[test]
    fn test_null_content_without_reasoning_is_invalid_response() {
        let response: OpenRouterResponse = serde_json::from_str(
            r#"{
                "id":"test","object":"chat.completion","created":1,"model":"test/model",
                "choices":[{"index":0,"message":{"role":"assistant","content":null},"finish_reason":"stop"}],
                "usage":{"prompt_tokens":2,"completion_tokens":0,"total_tokens":2}
            }"#,
        )
        .unwrap();
        let error = OpenRouterClient::parse_openrouter_response(response, 12.0).unwrap_err();

        assert!(matches!(
            error,
            LlmError::InvalidResponse { provider, message }
                if provider == "openrouter" && message.contains("no answer or tool calls")
        ));
    }

    #[test]
    fn test_native_tool_call_with_null_content() {
        let response: OpenRouterResponse = serde_json::from_str(
            r#"{
                "id":"test","object":"chat.completion","created":1,"model":"test/model",
                "choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":"get_balance","arguments":"{}"}}]},"finish_reason":"tool_calls"}],
                "usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}
            }"#,
        )
        .unwrap();
        let parsed = OpenRouterClient::parse_openrouter_response(response, 12.0).unwrap();

        assert!(parsed.content.is_empty());
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].name, "get_balance");
        assert_eq!(parsed.tool_calls[0].arguments, serde_json::json!({}));
    }
}
