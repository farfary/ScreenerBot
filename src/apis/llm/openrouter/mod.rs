/// OpenRouter API client (raw HTTP via reqwest)
///
/// API Documentation: https://openrouter.ai/docs
///
/// OpenRouter is a unified gateway to 100+ LLM models including:
/// - OpenAI models (gpt-4o, gpt-4-turbo, etc.)
/// - Anthropic Claude models
/// - Google Gemini models
/// - Meta Llama models
/// - And many more
///
/// The API is OpenAI-compatible with optional site identification headers.
///
/// Endpoints:
/// - POST https://openrouter.ai/api/v1/chat/completions
pub mod types;

pub use self::types::{
    OpenAiChoice as OpenRouterChoice, OpenAiMessage as OpenRouterMessage,
    OpenAiRequest as OpenRouterRequest, OpenAiResponse as OpenRouterResponse,
    OpenAiResponseFormat as OpenRouterResponseFormat,
    OpenAiResponseMessage as OpenRouterResponseMessage, OpenAiUsage as OpenRouterUsage,
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
            return Err("OpenRouter API key cannot be empty".to_string());
        }

        Ok(Self {
            api_key,
            client: Client::new(),
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
        let messages = request
            .messages
            .into_iter()
            .map(|msg| OpenRouterMessage {
                role: match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content,
            })
            .collect();

        OpenRouterRequest {
            model: request.model,
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request
                .response_format
                .map(|rf| OpenRouterResponseFormat { type_: rf.type_ }),
        }
    }

    /// Convert OpenRouter response to unified ChatResponse
    fn parse_openrouter_response(
        &self,
        response: OpenRouterResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Get the first choice
        let choice = response
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "openrouter".to_string(),
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
        request: OpenRouterRequest,
    ) -> Result<(OpenRouterResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "openrouter".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "openrouter".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        let url = format!("{}{}", OPENROUTER_BASE_URL, ENDPOINT_CHAT);

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
            .header("HTTP-Referer", self.site_url.as_deref().unwrap_or("https://screenerbot.io"))
            .header("X-Title", self.site_name.as_deref().unwrap_or("ScreenerBot"));

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

        let mut response = response_result.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    provider: "openrouter".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "openrouter".to_string(),
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
                    provider: "openrouter".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "openrouter".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "openrouter".to_string(),
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
                    provider: "openrouter".to_string(),
                    message: format!("Failed to parse response: {}", e),
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

    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Use the model from request, or fallback to client's default
        let mut request = request;
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
                    .record_error_with_event("OpenRouter", "chat_completion", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_openrouter_response(openrouter_response, latency_ms)
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
            "sk-or-test-key".to_string(),
            Some("openai/gpt-4o".to_string()),
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
        let client = OpenRouterClient::new("sk-or-test-key".to_string(), None, true, None, None);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_with_site_info() {
        let client = OpenRouterClient::new(
            "sk-or-test-key".to_string(),
            None,
            true,
            Some("https://example.com".to_string()),
            Some("My App".to_string()),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.site_url, Some("https://example.com".to_string()));
        assert_eq!(client.site_name, Some("My App".to_string()));
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = OpenRouterClient::new("".to_string(), None, true, None, None);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_openrouter_request() {
        let client =
            OpenRouterClient::new("sk-or-test".to_string(), None, true, None, None).unwrap();

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
            OpenRouterClient::new("sk-or-test".to_string(), None, true, None, None).unwrap();
        assert_eq!(client.provider(), Provider::OpenRouter);
    }
}
