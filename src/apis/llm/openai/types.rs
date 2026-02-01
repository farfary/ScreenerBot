/// OpenAI API request/response types
///
/// These types match the OpenAI Chat Completions API format exactly.
/// API Documentation: https://platform.openai.com/docs/api-reference/chat/create
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// OpenAI Chat Completion Request
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiRequest {
    /// Model ID (e.g., "gpt-4o-mini", "gpt-4", "gpt-4-turbo")
    pub model: String,

    /// Array of messages in the conversation
    pub messages: Vec<OpenAiMessage>,

    /// Sampling temperature (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Response format (for JSON mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<OpenAiResponseFormat>,
}

/// Message in OpenAI format
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiMessage {
    /// Role: "system", "user", or "assistant"
    pub role: String,

    /// Message content
    pub content: String,
}

/// Response format specification
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiResponseFormat {
    /// Format type: "text" or "json_object"
    #[serde(rename = "type")]
    pub type_: String,
}

impl OpenAiResponseFormat {
    /// Create JSON object format
    pub fn json_object() -> Self {
        Self {
            type_: "json_object".to_string(),
        }
    }

    /// Create text format (default)
    pub fn text() -> Self {
        Self {
            type_: "text".to_string(),
        }
    }
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// OpenAI Chat Completion Response
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiResponse {
    /// Unique identifier for the completion
    pub id: String,

    /// Object type (always "chat.completion")
    pub object: String,

    /// Unix timestamp of creation
    pub created: u64,

    /// Model used for generation
    pub model: String,

    /// Array of completion choices
    pub choices: Vec<OpenAiChoice>,

    /// Token usage statistics
    pub usage: OpenAiUsage,
}

/// A single choice in the response
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChoice {
    /// Index of this choice
    pub index: u32,

    /// The generated message
    pub message: OpenAiResponseMessage,

    /// Reason for stopping ("stop", "length", "content_filter", etc.)
    pub finish_reason: String,
}

/// Response message from the assistant
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiResponseMessage {
    /// Role (always "assistant")
    pub role: String,

    /// Generated content
    pub content: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,

    /// Tokens in the completion
    pub completion_tokens: u32,

    /// Total tokens used
    pub total_tokens: u32,
}
