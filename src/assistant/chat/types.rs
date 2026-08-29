//! Assistant chat data types — request/response structs and tool-call types.

use serde::{Deserialize, Serialize};

/// Chat request from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: i64,
    pub message: String,
    #[serde(default)]
    pub regenerate_message_id: Option<i64>,
    pub context: Option<ChatContext>,
    /// When true, auto-approve tool calls (for scheduled tasks)
    #[serde(default)]
    pub headless: bool,
    /// Tool permission mode for headless execution
    #[serde(default)]
    pub tool_mode: ToolMode,
}

/// Optional context for chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    pub current_token: Option<String>,
    pub current_position: Option<i64>,
}

/// Response to chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message_id: i64,
    pub content: String,
    pub tool_calls: Vec<ToolCallInfo>,
    pub pending_confirmations: Vec<PendingConfirmation>,
    pub is_complete: bool,
}

/// Incremental events emitted while an agent turn is running.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatProgressEvent {
    Thinking { iteration: usize },
    ToolStarted { tool_name: String },
    ToolFinished { tool_call: ToolCallInfo },
    Complete { response: ChatResponse },
    Error { message: String },
}

/// Information about a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub status: ToolCallStatus,
}

/// Status of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Executed,
    PendingConfirmation,
    Denied,
    Failed,
}

/// Tool execution mode for headless/scheduled runs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ToolMode {
    /// Only allow read-only tools (analysis, portfolio, system info)
    #[default]
    ReadOnly,
    /// Allow all tools including trading (auto-approve confirmations)
    Full,
}

/// Pending confirmation for a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingConfirmation {
    pub confirmation_id: String,
    pub tool_name: String,
    pub description: String,
    pub input: serde_json::Value,
}

/// Parsed tool call from LLM response
#[derive(Debug, Clone)]
pub(super) struct ToolCall {
    pub(super) name: String,
    pub(super) arguments: serde_json::Value,
}

/// Pending confirmation in memory
#[derive(Debug, Clone)]
pub(super) struct ConfirmationState {
    pub(super) session_id: i64,
    pub(super) message_id: i64,
    pub(super) tool_calls: Vec<ToolCall>,
    pub(super) current_index: usize,
    pub(super) created_at: std::time::Instant,
}
