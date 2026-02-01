/// Google Gemini API client (raw HTTP via reqwest)
///
/// API Documentation: https://ai.google.dev/api/rest
///
/// Endpoints:
/// - POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
pub mod types;

pub use self::types::{
    GeminiCandidate, GeminiContent, GeminiGenerationConfig, GeminiPart, GeminiRequest,
    GeminiResponse, GeminiResponseContent, GeminiResponsePart, GeminiSystemInstruction,
    GeminiUsageMetadata,
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

const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const DEFAULT_MODEL: &str = "gemini-1.5-flash";
const TIMEOUT_SECS: u64 = 30;
const DEFAULT_RATE_LIMIT_PER_MINUTE: usize = 60; // Free tier: 60 RPM

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Google Gemini API client
pub struct GeminiClient {
    api_key: String,
    client: Client,
    model: String,
    timeout: Duration,
    rate_limiter: RateLimiter,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl GeminiClient {
    /// Create a new Gemini client
    ///
    /// # Arguments
    /// * `api_key` - Gemini API key (from https://aistudio.google.com/app/apikey)
    /// * `model` - Optional model override (defaults to "gemini-1.5-flash")
    /// * `enabled` - Whether the client is enabled
    pub fn new(api_key: String, model: Option<String>, enabled: bool) -> Result<Self, String> {
        if api_key.trim().is_empty() {
            return Err("Gemini API key cannot be empty".to_string());
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

    /// Convert unified ChatRequest to Gemini-specific format
    fn build_gemini_request(&self, request: ChatRequest) -> GeminiRequest {
        let mut system_instruction = None;
        let mut contents = Vec::new();

        // Separate system messages from user/assistant messages
        for msg in request.messages {
            match msg.role {
                MessageRole::System => {
                    // Gemini uses systemInstruction field for system prompts
                    system_instruction = Some(GeminiSystemInstruction {
                        parts: vec![GeminiPart { text: msg.content }],
                    });
                }
                MessageRole::User => {
                    contents.push(GeminiContent {
                        parts: vec![GeminiPart { text: msg.content }],
                        role: Some("user".to_string()),
                    });
                }
                MessageRole::Assistant => {
                    contents.push(GeminiContent {
                        parts: vec![GeminiPart { text: msg.content }],
                        role: Some("model".to_string()),
                    });
                }
            }
        }

        // Build generation config
        let mut generation_config = None;
        if request.temperature.is_some()
            || request.max_tokens.is_some()
            || request.response_format.is_some()
        {
            generation_config = Some(GeminiGenerationConfig {
                response_mime_type: request
                    .response_format
                    .map(|_| "application/json".to_string()),
                max_output_tokens: request.max_tokens,
                temperature: request.temperature,
            });
        }

        GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        }
    }

    /// Convert Gemini response to unified ChatResponse
    fn parse_gemini_response(
        &self,
        response: GeminiResponse,
        model: String,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Get the first candidate
        let candidate = response
            .candidates
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "gemini".to_string(),
                message: "No candidates in response".to_string(),
            })?;

        // Get the first part of the content
        let content = candidate
            .content
            .parts
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: "gemini".to_string(),
                message: "No content parts in response".to_string(),
            })?;

        // Extract usage metadata
        let usage = if let Some(metadata) = &response.usage_metadata {
            Usage::new(metadata.prompt_token_count, metadata.candidates_token_count)
        } else {
            Usage::new(0, 0)
        };

        // Get finish reason
        let finish_reason = candidate
            .finish_reason
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());

        Ok(ChatResponse::new(
            content.text.clone(),
            usage,
            finish_reason,
            model,
            latency_ms,
        ))
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: GeminiRequest,
        model: &str,
    ) -> Result<(GeminiResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "gemini".to_string(),
            });
        }

        // Acquire rate limiter
        let guard = self
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| LlmError::NetworkError {
                provider: "gemini".to_string(),
                message: format!("Rate limiter error: {}", e),
            })?;

        // Build URL with model and API key as query parameter
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_BASE_URL, model, self.api_key
        );

        logger::debug(
            LogTag::Api,
            &format!("[GEMINI] Calling generateContent: model={}", model),
        );

        let start = Instant::now();
        let response_result = self
            .client
            .post(&url)
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
                    provider: "gemini".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else {
                LlmError::NetworkError {
                    provider: "gemini".to_string(),
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
                401 | 403 => LlmError::AuthError {
                    provider: "gemini".to_string(),
                    message: "Invalid API key".to_string(),
                },
                429 => LlmError::RateLimited {
                    provider: "gemini".to_string(),
                    retry_after_ms: retry_after,
                },
                _ => LlmError::ApiError {
                    provider: "gemini".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let gemini_response =
            response
                .json::<GeminiResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "gemini".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((gemini_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for GeminiClient {
    fn provider(&self) -> Provider {
        Provider::Gemini
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn call(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // Use the model from request, or fallback to client's default
        let model = if request.model.is_empty() {
            self.model.clone()
        } else {
            request.model.clone()
        };

        // Build Gemini-specific request
        let gemini_request = self.build_gemini_request(request);

        // Execute the request
        let (gemini_response, latency_ms) = match self.execute_request(gemini_request, &model).await
        {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Gemini", "generate_content", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_gemini_response(gemini_response, model, latency_ms)
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
        let client = GeminiClient::new(
            "test-api-key".to_string(),
            Some("gemini-1.5-pro".to_string()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "gemini-1.5-pro");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = GeminiClient::new("test-api-key".to_string(), None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = GeminiClient::new("".to_string(), None, true);
        assert!(client.is_err());
    }

    #[test]
    fn test_build_gemini_request_with_system() {
        let client = GeminiClient::new("test-key".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "gemini-1.5-flash",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let gemini_req = client.build_gemini_request(request);

        // System message should be in systemInstruction
        assert!(gemini_req.system_instruction.is_some());
        assert_eq!(
            gemini_req.system_instruction.unwrap().parts[0].text,
            "You are helpful"
        );

        // User message should be in contents
        assert_eq!(gemini_req.contents.len(), 1);
        assert_eq!(gemini_req.contents[0].role, Some("user".to_string()));
        assert_eq!(gemini_req.contents[0].parts[0].text, "Hello");

        // Generation config should be set
        assert!(gemini_req.generation_config.is_some());
        let config = gemini_req.generation_config.unwrap();
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_output_tokens, Some(100));
    }

    #[test]
    fn test_build_gemini_request_with_assistant() {
        let client = GeminiClient::new("test-key".to_string(), None, true).unwrap();

        let request = ChatRequest::new(
            "gemini-1.5-flash",
            vec![
                ChatMessage::user("Hello"),
                ChatMessage::assistant("Hi there!"),
                ChatMessage::user("How are you?"),
            ],
        );

        let gemini_req = client.build_gemini_request(request);

        // Should have 3 contents
        assert_eq!(gemini_req.contents.len(), 3);
        assert_eq!(gemini_req.contents[0].role, Some("user".to_string()));
        assert_eq!(gemini_req.contents[1].role, Some("model".to_string()));
        assert_eq!(gemini_req.contents[2].role, Some("user".to_string()));
    }

    #[test]
    fn test_build_gemini_request_json_mode() {
        let client = GeminiClient::new("test-key".to_string(), None, true).unwrap();

        let request =
            ChatRequest::new("gemini-1.5-flash", vec![ChatMessage::user("Test")]).with_json_mode();

        let gemini_req = client.build_gemini_request(request);

        assert!(gemini_req.generation_config.is_some());
        assert_eq!(
            gemini_req
                .generation_config
                .unwrap()
                .response_mime_type
                .unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_provider() {
        let client = GeminiClient::new("test-key".to_string(), None, true).unwrap();
        assert_eq!(client.provider(), Provider::Gemini);
    }
}
