/// Google Gemini API request/response types
///
/// These types match the Gemini API format exactly.
/// API Documentation: https://ai.google.dev/api/rest
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST TYPES
// ============================================================================

/// Gemini Chat Completion Request
#[derive(Debug, Clone, Serialize)]
pub struct GeminiRequest {
    /// Array of content parts in the conversation
    pub contents: Vec<GeminiContent>,

    /// System instruction (separate from contents)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemInstruction")]
    pub system_instruction: Option<GeminiSystemInstruction>,

    /// Generation configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "generationConfig")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

/// Content in Gemini format (user message)
#[derive(Debug, Clone, Serialize)]
pub struct GeminiContent {
    /// Array of parts (text, images, etc.)
    pub parts: Vec<GeminiPart>,

    /// Role: "user" or "model" (no "system" role in contents)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// System instruction format
#[derive(Debug, Clone, Serialize)]
pub struct GeminiSystemInstruction {
    /// Array of parts for system instruction
    pub parts: Vec<GeminiPart>,
}

/// A single part of content (text)
#[derive(Debug, Clone, Serialize)]
pub struct GeminiPart {
    /// Text content
    pub text: String,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize)]
pub struct GeminiGenerationConfig {
    /// Response MIME type (for JSON mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "responseMimeType")]
    pub response_mime_type: Option<String>,

    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxOutputTokens")]
    pub max_output_tokens: Option<u32>,

    /// Sampling temperature (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

// ============================================================================
// RESPONSE TYPES
// ============================================================================

/// Gemini Chat Completion Response
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiResponse {
    /// Array of candidate responses
    pub candidates: Vec<GeminiCandidate>,

    /// Token usage metadata
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<GeminiUsageMetadata>,
}

/// A single candidate response
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiCandidate {
    /// Generated content
    pub content: GeminiResponseContent,

    /// Reason for finishing
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,

    /// Index of this candidate
    pub index: Option<u32>,
}

/// Response content from the model
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiResponseContent {
    /// Array of parts
    pub parts: Vec<GeminiResponsePart>,

    /// Role (always "model")
    pub role: String,
}

/// A single part of response content
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiResponsePart {
    /// Text content
    pub text: String,
}

/// Token usage metadata
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiUsageMetadata {
    /// Tokens in the prompt
    #[serde(rename = "promptTokenCount")]
    pub prompt_token_count: u32,

    /// Tokens in the candidates
    #[serde(rename = "candidatesTokenCount")]
    pub candidates_token_count: u32,

    /// Total tokens used
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<u32>,
}
