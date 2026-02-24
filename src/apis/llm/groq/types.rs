//! Groq API request/response types
//!
//! Groq uses OpenAI-compatible format, so we reuse the OpenAI types.
//! API Documentation: https://console.groq.com/docs/api-reference#chat
//!
//! Since the API is fully compatible, we re-export OpenAI types.

use crate::apis::llm::openai::types::{
    OpenAiMessage, OpenAiRequest, OpenAiResponse, OpenAiResponseFormat,
};

// Re-export as Groq types (aliases)
pub type GroqRequest = OpenAiRequest;
pub type GroqMessage = OpenAiMessage;
pub type GroqResponseFormat = OpenAiResponseFormat;
pub type GroqResponse = OpenAiResponse;
