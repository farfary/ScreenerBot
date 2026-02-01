/// Fireworks AI API request/response types
///
/// Fireworks AI uses the OpenAI-compatible Chat Completions API format.
/// API Documentation: https://docs.fireworks.ai/api-reference/chat-completions
///
/// Since the API is OpenAI-compatible, we re-export the OpenAI types.
use serde::{Deserialize, Serialize};

// ============================================================================
// REQUEST TYPES
// ============================================================================

pub use crate::apis::llm::openai::types::OpenAiMessage as FireworksMessage;
/// Fireworks AI Chat Completion Request (OpenAI-compatible)
pub use crate::apis::llm::openai::types::OpenAiRequest as FireworksRequest;
pub use crate::apis::llm::openai::types::OpenAiResponseFormat as FireworksResponseFormat;

// ============================================================================
// RESPONSE TYPES
// ============================================================================

pub use crate::apis::llm::openai::types::OpenAiChoice as FireworksChoice;
/// Fireworks AI Chat Completion Response (OpenAI-compatible)
pub use crate::apis::llm::openai::types::OpenAiResponse as FireworksResponse;
pub use crate::apis::llm::openai::types::OpenAiResponseMessage as FireworksResponseMessage;
pub use crate::apis::llm::openai::types::OpenAiUsage as FireworksUsage;
