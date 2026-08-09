//! OpenAI API request/response types
//!
//! These types match the OpenAI Chat Completions API format exactly.
//! API Documentation: https://platform.openai.com/docs/api-reference/chat/create

use serde::{Deserialize, Serialize};

use crate::apis::llm::{ChatRequest, ChatResponse, LlmError, MessageRole, ToolCall, Usage};

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAiTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,
}

impl From<ChatRequest> for OpenAiRequest {
    fn from(request: ChatRequest) -> Self {
        let tools = request.tools.map(|definitions| {
            definitions
                .into_iter()
                .map(|definition| OpenAiTool {
                    type_: "function".to_owned(),
                    function: OpenAiFunction {
                        name: definition.name,
                        description: definition.description,
                        parameters: definition.parameters,
                    },
                })
                .collect::<Vec<_>>()
        });
        let tool_choice = tools.as_ref().map(|_| "auto".to_owned());

        Self {
            model: request.model,
            messages: request
                .messages
                .into_iter()
                .map(|message| OpenAiMessage {
                    role: match message.role {
                        MessageRole::System => "system".to_owned(),
                        MessageRole::User => "user".to_owned(),
                        MessageRole::Assistant => "assistant".to_owned(),
                    },
                    content: message.content,
                })
                .collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            response_format: request.response_format.map(|format| OpenAiResponseFormat {
                type_: format.type_,
            }),
            tools,
            tool_choice,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiTool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: OpenAiFunction,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
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
            type_: "json_object".to_owned(),
        }
    }

    /// Create text format (default)
    pub fn text() -> Self {
        Self {
            type_: "text".to_owned(),
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

    /// Generated content. OpenAI-compatible gateways may return null for
    /// reasoning-only choices.
    #[serde(default)]
    pub content: Option<String>,

    /// Reasoning text returned by models that separate it from final content.
    #[serde(default, alias = "reasoning_content")]
    pub reasoning: Option<String>,

    #[serde(default)]
    pub tool_calls: Vec<OpenAiToolCall>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiToolCall {
    #[serde(default)]
    pub id: String,
    pub function: OpenAiToolCallFunction,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiToolCallFunction {
    pub name: String,
    pub arguments: String,
}

impl OpenAiResponseMessage {
    pub fn text(&self) -> Option<&str> {
        self.content
            .as_deref()
            .filter(|content| !content.trim().is_empty())
    }
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

impl OpenAiResponse {
    pub fn into_chat_response(
        self,
        provider: &str,
        latency_ms: f64,
    ) -> Result<ChatResponse, LlmError> {
        let choice = self
            .choices
            .first()
            .ok_or_else(|| LlmError::InvalidResponse {
                provider: provider.to_owned(),
                message: "No choices in response".to_owned(),
            })?;
        let tool_calls = choice
            .message
            .tool_calls
            .iter()
            .map(|call| {
                let arguments =
                    serde_json::from_str(&call.function.arguments).map_err(|error| {
                        LlmError::InvalidResponse {
                            provider: provider.to_owned(),
                            message: format!(
                                "Invalid arguments for tool {}: {error}",
                                call.function.name
                            ),
                        }
                    })?;
                Ok(ToolCall {
                    id: call.id.clone(),
                    name: call.function.name.clone(),
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, LlmError>>()?;
        let content = choice.message.text().unwrap_or_default().to_owned();
        if content.trim().is_empty() && tool_calls.is_empty() {
            return Err(LlmError::InvalidResponse {
                provider: provider.to_owned(),
                message: "Choice contained reasoning but no answer or tool calls".to_owned(),
            });
        }

        Ok(ChatResponse::new(
            content,
            Usage::new(self.usage.prompt_tokens, self.usage.completion_tokens),
            choice.finish_reason.clone(),
            self.model,
            latency_ms,
        )
        .with_reasoning(choice.message.reasoning.clone())
        .with_tool_calls(tool_calls))
    }
}
