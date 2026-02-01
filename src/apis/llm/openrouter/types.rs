/// OpenRouter API request/response types
///
/// OpenRouter uses OpenAI-compatible format, so we re-export OpenAI types.
/// API Documentation: https://openrouter.ai/docs#quick-start
///
/// The API is fully compatible with OpenAI's Chat Completions API,
/// with optional additional headers for site identification.
// Re-export OpenAI types as they are fully compatible
pub use crate::apis::llm::openai::types::{
    OpenAiChoice, OpenAiMessage, OpenAiRequest, OpenAiResponse, OpenAiResponseFormat,
    OpenAiResponseMessage, OpenAiUsage,
};
