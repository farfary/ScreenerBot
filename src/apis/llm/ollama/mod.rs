/// Ollama API client (raw HTTP via reqwest)
///
/// API Documentation: https://github.com/ollama/ollama/blob/main/docs/api.md
///
/// Endpoints:
/// - POST http://localhost:11434/api/chat
pub mod types;

pub use self::types::{OllamaMessage, OllamaRequest, OllamaResponse, OllamaResponseMessage};

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

const DEFAULT_BASE_URL: &str = "http://localhost:11434";
const ENDPOINT_CHAT: &str = "/api/chat";
const DEFAULT_MODEL: &str = "llama3.2";
const TIMEOUT_SECS: u64 = 120; // Local inference can be slower

// ============================================================================
// CLIENT IMPLEMENTATION
// ============================================================================

/// Ollama API client
pub struct OllamaClient {
    base_url: String,
    client: Client,
    model: String,
    timeout: Duration,
    stats: Arc<ApiStatsTracker>,
    enabled: bool,
}

impl OllamaClient {
    /// Create a new Ollama client
    ///
    /// # Arguments
    /// * `base_url` - Optional base URL (defaults to http://localhost:11434)
    /// * `model` - Optional model override (defaults to "llama3.2")
    /// * `enabled` - Whether the client is enabled
    pub fn new(
        base_url: Option<String>,
        model: Option<String>,
        enabled: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
            client: Client::new(),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
            timeout: Duration::from_secs(TIMEOUT_SECS),
            stats: Arc::new(ApiStatsTracker::new()),
            enabled,
        })
    }

    /// Convert unified ChatRequest to Ollama-specific format
    fn build_ollama_request(&self, request: ChatRequest) -> OllamaRequest {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| OllamaMessage {
                role: match msg.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: msg.content,
            })
            .collect();

        // Ollama uses "json" string for JSON mode, not an object!
        let format = request.response_format.map(|_| "json".to_string());

        OllamaRequest {
            model: request.model,
            messages,
            format,
            stream: false,
            temperature: request.temperature,
            num_predict: request.max_tokens,
        }
    }

    /// Convert Ollama response to unified ChatResponse
    fn parse_ollama_response(
        &self,
        response: OllamaResponse,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        // Extract token counts (Ollama may not always provide them)
        let prompt_tokens = response.prompt_eval_count.unwrap_or(0);
        let completion_tokens = response.eval_count.unwrap_or(0);

        // Get finish reason
        let finish_reason = response
            .done_reason
            .clone()
            .unwrap_or_else(|| if response.done { "stop" } else { "unknown" }.to_string());

        Ok(ChatResponse::new(
            response.message.content,
            Usage::new(prompt_tokens, completion_tokens),
            finish_reason,
            response.model,
            latency_ms,
        ))
    }

    /// Execute the API call
    async fn execute_request(
        &self,
        request: OllamaRequest,
    ) -> Result<(OllamaResponse, f64), LlmError> {
        if !self.enabled {
            return Err(LlmError::ProviderDisabled {
                provider: "ollama".to_string(),
            });
        }

        let url = format!("{}{}", self.base_url, ENDPOINT_CHAT);

        logger::debug(
            LogTag::Api,
            &format!("[OLLAMA] Calling chat API: model={}", request.model),
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

        let elapsed = start.elapsed().as_millis() as f64;

        let mut response = response_result.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    provider: "ollama".to_string(),
                    timeout_ms: self.timeout.as_millis() as u64,
                }
            } else if e.is_connect() {
                LlmError::NetworkError {
                    provider: "ollama".to_string(),
                    message: format!(
                        "Cannot connect to Ollama at {}. Is Ollama running?",
                        self.base_url
                    ),
                }
            } else {
                LlmError::NetworkError {
                    provider: "ollama".to_string(),
                    message: format!("Request failed: {}", e),
                }
            }
        })?;

        let status = response.status();

        // Handle error status codes
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();

            return Err(match status.as_u16() {
                404 => LlmError::InvalidResponse {
                    provider: "ollama".to_string(),
                    message: format!(
                        "Model not found. Pull the model first with: ollama pull {}",
                        request.model
                    ),
                },
                _ => LlmError::ApiError {
                    provider: "ollama".to_string(),
                    status_code: status.as_u16(),
                    message: error_body,
                },
            });
        }

        // Parse successful response
        let ollama_response =
            response
                .json::<OllamaResponse>()
                .await
                .map_err(|e| LlmError::ParseError {
                    provider: "ollama".to_string(),
                    message: format!("Failed to parse response: {}", e),
                })?;

        Ok((ollama_response, elapsed))
    }
}

#[async_trait]
impl LlmClient for OllamaClient {
    fn provider(&self) -> Provider {
        Provider::Ollama
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

        // Build Ollama-specific request
        let ollama_request = self.build_ollama_request(request);

        // Execute the request
        let (ollama_response, latency_ms) = match self.execute_request(ollama_request).await {
            Ok((resp, lat)) => {
                self.stats.record_request(true, lat).await;
                (resp, lat)
            }
            Err(e) => {
                self.stats.record_request(false, 0.0).await;
                self.stats
                    .record_error_with_event("Ollama", "chat_completion", format!("{}", e))
                    .await;
                return Err(e);
            }
        };

        // Parse and convert response
        self.parse_ollama_response(ollama_response, latency_ms)
    }

    async fn get_stats(&self) -> crate::apis::stats::ApiStats {
        self.stats.get_stats().await
    }

    fn rate_limit_info(&self) -> (usize, Duration) {
        // No rate limiting for local Ollama
        (usize::MAX, Duration::from_millis(0))
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
        let client = OllamaClient::new(
            Some("http://localhost:11434".to_string()),
            Some("llama3.2".to_string()),
            true,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, "llama3.2");
        assert!(client.is_enabled());
    }

    #[test]
    fn test_client_creation_with_defaults() {
        let client = OllamaClient::new(None, None, true);
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.model, DEFAULT_MODEL);
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn test_build_ollama_request() {
        let client = OllamaClient::new(None, None, true).unwrap();

        let request = ChatRequest::new(
            "llama3.2",
            vec![
                ChatMessage::system("You are helpful"),
                ChatMessage::user("Hello"),
            ],
        )
        .with_temperature(0.7)
        .with_max_tokens(100);

        let ollama_req = client.build_ollama_request(request);

        assert_eq!(ollama_req.model, "llama3.2");
        assert_eq!(ollama_req.messages.len(), 2);
        assert_eq!(ollama_req.messages[0].role, "system");
        assert_eq!(ollama_req.messages[1].role, "user");
        assert_eq!(ollama_req.temperature, Some(0.7));
        assert_eq!(ollama_req.num_predict, Some(100));
        assert_eq!(ollama_req.stream, false);
    }

    #[test]
    fn test_build_ollama_request_with_json_mode() {
        let client = OllamaClient::new(None, None, true).unwrap();

        let request =
            ChatRequest::new("llama3.2", vec![ChatMessage::user("Test")]).with_json_mode();

        let ollama_req = client.build_ollama_request(request);

        // JSON mode should be "json" string in Ollama
        assert_eq!(ollama_req.format, Some("json".to_string()));
    }

    #[test]
    fn test_provider() {
        let client = OllamaClient::new(None, None, true).unwrap();
        assert_eq!(client.provider(), Provider::Ollama);
    }
}
