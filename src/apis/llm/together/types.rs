/// Together AI API types
///
/// Together AI uses OpenAI-compatible format, so we can re-export OpenAI types.
/// API Documentation: https://docs.together.ai/reference/chat-completions
///
/// The Together API is fully compatible with OpenAI's chat completions format,
/// including JSON mode via response_format: { "type": "json_object" }
// Re-export OpenAI types since Together uses the same format
pub use crate::apis::llm::openai::types::{
    OpenAiChoice as TogetherChoice, OpenAiMessage as TogetherMessage,
    OpenAiRequest as TogetherRequest, OpenAiResponse as TogetherResponse,
    OpenAiResponseFormat as TogetherResponseFormat,
    OpenAiResponseMessage as TogetherResponseMessage, OpenAiUsage as TogetherUsage,
};
