//! Request/response types for the assistant API (`/api/assistant`): chat plus
//! scheduled automation.

use serde::{Deserialize, Serialize};

use crate::assistant::chat::database as chat_db;
use crate::assistant::ChatContext;
use crate::assistant::ChatSession;

// ============================================================================
// CHAT
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SendChatMessageRequest {
    pub session_id: i64,
    pub message: String,
    pub regenerate_message_id: Option<i64>,
    pub context: Option<ChatContext>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateChatSessionResponse {
    pub session_id: i64,
}

#[derive(Debug, Serialize)]
pub struct GetChatSessionResponse {
    pub session: ChatSession,
    pub messages: Vec<chat_db::ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmToolExecutionRequest {
    pub approved: bool,
    pub session_id: i64,
}

// ============================================================================
// AUTOMATION
// ============================================================================

#[derive(Deserialize)]
pub struct CreateAutomationTaskRequest {
    pub name: String,
    pub instruction: String,
    pub schedule_type: String,
    pub schedule_value: String,
    #[serde(default = "default_read_only")]
    pub tool_permissions: String,
    #[serde(default = "default_low")]
    pub priority: String,
    #[serde(default = "default_true")]
    pub notify_telegram: bool,
    #[serde(default = "default_true")]
    pub notify_on_success: bool,
    #[serde(default = "default_true")]
    pub notify_on_failure: bool,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

fn default_read_only() -> String {
    "readonly".to_owned()
}
fn default_low() -> String {
    "low".to_owned()
}
fn default_true() -> bool {
    true
}

#[derive(Deserialize)]
pub struct UpdateAutomationTaskRequest {
    pub name: Option<String>,
    pub instruction: Option<String>,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub tool_permissions: Option<String>,
    pub priority: Option<String>,
    pub notify_telegram: Option<bool>,
    pub notify_on_success: Option<bool>,
    pub notify_on_failure: Option<bool>,
    pub max_retries: Option<i32>,
    pub timeout_seconds: Option<i64>,
    pub instruction_ids: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleTaskRequest {
    pub enabled: bool,
}
