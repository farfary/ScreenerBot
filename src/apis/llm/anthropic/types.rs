/// Anthropic API request/response types
///
/// These types match the Anthropic Messages API format exactly.
/// API Documentation: https://docs.anthropic.com/en/api/messages
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Anthropic Messages Request
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    /// Model ID (e.g., "claude-3-haiku-20240307", "claude-3-5-sonnet-20241022")
    pub model: String,

    /// Maximum tokens to generate (REQUIRED by Anthropic API)
    pub max_tokens: u32,

    /// System prompt (SEPARATE from messages array in Anthropic API)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Array of messages in the conversation (NO system messages here!)
    pub messages: Vec<AnthropicMessage>,

    /// Sampling temperature (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

/// Message in Anthropic format
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicMessage {
    /// Role: "user" or "assistant" (NO "system" role!)
    pub role: String,

    /// Message content
    pub content: String,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Anthropic Messages Response
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    /// Unique identifier for the message
    pub id: String,

    /// Object type (always "message")
    #[serde(rename = "type")]
    pub type_: String,

    /// Role (always "assistant")
    pub role: String,

    /// Content array (DIFFERENT from OpenAI - content is an ARRAY!)
    pub content: Vec<AnthropicContent>,

    /// Reason for stopping ("end_turn", "max_tokens", "stop_sequence", etc.)
    pub stop_reason: String,

    /// Model used for generation
    pub model: String,

    /// Token usage statistics
    pub usage: AnthropicUsage,
}

/// Content block in Anthropic response
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicContent {
    /// Content type (usually "text")
    #[serde(rename = "type")]
    pub type_: String,

    /// The actual text content
    pub text: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicUsage {
    /// Tokens in the input
    pub input_tokens: u32,

    /// Tokens in the output
    pub output_tokens: u32,
}
