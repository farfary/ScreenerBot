/// DeepSeek API request/response types
///
/// DeepSeek uses OpenAI-compatible format, so we can reuse the OpenAI types.
/// API Documentation: https://api-docs.deepseek.com/
///
/// Since DeepSeek is OpenAI-compatible, we simply re-export OpenAI types.
pub use crate::apis::llm::openai::types::{
    OpenAiChoice as DeepSeekChoice, OpenAiMessage as DeepSeekMessage,
    OpenAiRequest as DeepSeekRequest, OpenAiResponse as DeepSeekResponse,
    OpenAiResponseFormat as DeepSeekResponseFormat,
    OpenAiResponseMessage as DeepSeekResponseMessage, OpenAiUsage as DeepSeekUsage,
};
