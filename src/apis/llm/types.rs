/// Core LLM API types
///
/// Unified request/response types for all LLM providers.
/// Individual providers transform these to/from their specific API formats.
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// Chat message with role and content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Message role in a chat conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::System => write!(f, "system"),
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
        }
    }
}

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Model identifier (e.g., "gpt-4", "claude-3-sonnet-20240229")
    pub model: String,

    /// Conversation messages
    pub messages: Vec<ChatMessage>,

    /// Temperature (0.0-2.0, typically 0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Response format (for JSON mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl ChatRequest {
    /// Create a new chat request
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            response_format: None,
        }
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable JSON mode
    pub fn with_json_mode(mut self) -> Self {
        self.response_format = Some(ResponseFormat::json_object());
        self
    }
}

/// Response format configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub type_: String,
}

impl ResponseFormat {
    /// JSON object mode
    pub fn json_object() -> Self {
        Self {
            type_: "json_object".to_string(),
        }
    }

    /// Text mode (default)
    pub fn text() -> Self {
        Self {
            type_: "text".to_string(),
        }
    }
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Generated text content
    pub content: String,

    /// Token usage statistics
    pub usage: Usage,

    /// Reason for completion finish
    pub finish_reason: String,

    /// Model used for generation
    pub model: String,

    /// Latency in milliseconds
    pub latency_ms: f64,
}

impl ChatResponse {
    /// Create a new response
    pub fn new(
        content: impl Into<String>,
        usage: Usage,
        finish_reason: impl Into<String>,
        model: impl Into<String>,
        latency_ms: f64,
    ) -> Self {
        Self {
            content: content.into(),
            usage,
            finish_reason: finish_reason.into(),
            model: model.into(),
            latency_ms,
        }
    }
}

/// Token usage statistics
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    /// Create new usage stats
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

// ============================================================================
// ERROR TYPES
// ============================================================================

/// LLM API errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmError {
    /// Rate limited by provider
    RateLimited {
        provider: String,
        retry_after_ms: Option<u64>,
    },

    /// Request timeout
    Timeout { provider: String, timeout_ms: u64 },

    /// Invalid response from API
    InvalidResponse { provider: String, message: String },

    /// Authentication error
    AuthError { provider: String, message: String },

    /// Network error
    NetworkError { provider: String, message: String },

    /// JSON parse error
    ParseError { provider: String, message: String },

    /// Generic API error
    ApiError {
        provider: String,
        status_code: u16,
        message: String,
    },

    /// Provider disabled in config
    ProviderDisabled { provider: String },
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::RateLimited {
                provider,
                retry_after_ms,
            } => {
                if let Some(ms) = retry_after_ms {
                    write!(f, "[{}] Rate limited (retry after {}ms)", provider, ms)
                } else {
                    write!(f, "[{}] Rate limited", provider)
                }
            }
            LlmError::Timeout {
                provider,
                timeout_ms,
            } => {
                write!(f, "[{}] Request timeout ({}ms)", provider, timeout_ms)
            }
            LlmError::InvalidResponse { provider, message } => {
                write!(f, "[{}] Invalid response: {}", provider, message)
            }
            LlmError::AuthError { provider, message } => {
                write!(f, "[{}] Auth error: {}", provider, message)
            }
            LlmError::NetworkError { provider, message } => {
                write!(f, "[{}] Network error: {}", provider, message)
            }
            LlmError::ParseError { provider, message } => {
                write!(f, "[{}] Parse error: {}", provider, message)
            }
            LlmError::ApiError {
                provider,
                status_code,
                message,
            } => {
                write!(f, "[{}] API error {}: {}", provider, status_code, message)
            }
            LlmError::ProviderDisabled { provider } => {
                write!(f, "[{}] Provider disabled in config", provider)
            }
        }
    }
}

impl std::error::Error for LlmError {}

// Convert to String for compatibility with Result<T, String>
impl From<LlmError> for String {
    fn from(err: LlmError) -> String {
        err.to_string()
    }
}
