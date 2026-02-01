/// Ollama API request/response types
///
/// These types match the Ollama Chat API format.
/// API Documentation: https://github.com/ollama/ollama/blob/main/docs/api.md
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Ollama Chat Completion Request
#[derive(Debug, Clone, Serialize)]
pub struct OllamaRequest {
    /// Model ID (e.g., "llama3.2", "mistral", "qwen2.5-coder")
    pub model: String,

    /// Array of messages in the conversation
    pub messages: Vec<OllamaMessage>,

    /// Response format - "json" for JSON mode (string, not object!)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    /// Stream responses (always false for our use case)
    pub stream: bool,

    /// Sampling temperature (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate (num_predict in Ollama)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
}

/// Message in Ollama format
#[derive(Debug, Clone, Serialize)]
pub struct OllamaMessage {
    /// Role: "system", "user", or "assistant"
    pub role: String,

    /// Message content
    pub content: String,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Ollama Chat Completion Response
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaResponse {
    /// Model used for generation
    pub model: String,

    /// ISO 8601 timestamp of creation
    pub created_at: String,

    /// The generated message
    pub message: OllamaResponseMessage,

    /// Whether generation is complete
    pub done: bool,

    /// Reason for completion (e.g., "stop")
    #[serde(default)]
    pub done_reason: Option<String>,

    /// Number of tokens in the prompt
    #[serde(default)]
    pub prompt_eval_count: Option<u32>,

    /// Number of tokens in the response
    #[serde(default)]
    pub eval_count: Option<u32>,

    /// Time spent generating in nanoseconds
    #[serde(default)]
    pub eval_duration: Option<u64>,
}

/// Response message from the assistant
#[derive(Debug, Clone, Deserialize)]
pub struct OllamaResponseMessage {
    /// Role (always "assistant")
    pub role: String,

    /// Generated content
    pub content: String,
}
