/// Mistral AI API request/response types
///
/// Mistral AI uses OpenAI-compatible API format, so we can reuse OpenAI types directly.
/// API Documentation: https://docs.mistral.ai/api/
// Re-export OpenAI types since Mistral API is OpenAI-compatible
pub use crate::apis::llm::openai::types::{
    OpenAiChoice as MistralChoice, OpenAiMessage as MistralMessage,
    OpenAiRequest as MistralRequest, OpenAiResponse as MistralResponse,
    OpenAiResponseFormat as MistralResponseFormat, OpenAiResponseMessage as MistralResponseMessage,
    OpenAiUsage as MistralUsage,
};
