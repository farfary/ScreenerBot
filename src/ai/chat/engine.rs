//! AI Chat Engine Module
//!
//! Main orchestrator for AI chat with MCP-like tool calling.
//! Handles conversation flow, tool execution, and permission management.

use super::database;
use crate::ai::tools::{create_tool_registry, ToolRegistry};
use crate::ai::types::AiError;
use crate::apis::llm::ChatMessage as LlmChatMessage;
use crate::logger::{self, LogTag};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{OnceCell, RwLock};

// =============================================================================
// CONSTANTS
// =============================================================================

const MAX_TOOL_ITERATIONS: usize = 5;

// =============================================================================
// REGEX PATTERNS (Compiled once at startup)
// =============================================================================

/// Regex for JSON code blocks in LLM responses
pub(super) static JSON_CODE_BLOCK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)```json\s*(\{.+?\})\s*```").expect("Invalid JSON pattern regex")
});

/// Regex for loose JSON tool calls without code blocks
pub(super) static LOOSE_JSON_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)\{[^{}]*"tool_calls"[^{}]*\[.+?\]\s*\}"#)
        .expect("Invalid loose JSON pattern regex")
});

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

/// Global chat engine singleton
static CHAT_ENGINE: OnceCell<Arc<ChatEngine>> = OnceCell::const_new();

/// Initialize the global chat engine
pub async fn init_chat_engine() -> Result<(), String> {
    let engine = ChatEngine::new();
    CHAT_ENGINE
        .set(Arc::new(engine))
        .map_err(|_| "Chat engine already initialized".to_owned())
}

/// Get the global chat engine
pub fn get_chat_engine() -> Arc<ChatEngine> {
    CHAT_ENGINE
        .get()
        .expect("Chat engine not initialized - call init_chat_engine() first")
        .clone()
}

/// Try to get the global chat engine (non-panicking version)
pub fn try_get_chat_engine() -> Option<Arc<ChatEngine>> {
    CHAT_ENGINE.get().cloned()
}

// =============================================================================
// TYPES
// =============================================================================

/// Chat request from user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: i64,
    pub message: String,
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

// =============================================================================
// CONFIRMATION MANAGER
// =============================================================================

/// Simple confirmation manager for tool calls requiring user approval
pub(super) struct ConfirmationManager {
    pub(super) pending: Arc<RwLock<HashMap<String, ConfirmationState>>>,
}

impl ConfirmationManager {
    pub(super) fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(super) async fn create_confirmation(
        &self,
        session_id: i64,
        message_id: i64,
        tool_calls: Vec<ToolCall>,
    ) -> String {
        let confirmation_id = uuid::Uuid::new_v4().to_string();
        let state = ConfirmationState {
            session_id,
            message_id,
            tool_calls,
            current_index: 0,
            created_at: std::time::Instant::now(),
        };

        let mut pending = self.pending.write().await;

        // Cleanup expired confirmations (older than 10 minutes)
        let timeout = Duration::from_secs(600);
        pending.retain(|_, v| v.created_at.elapsed() < timeout);

        // Limit max pending confirmations per session (prevent DoS)
        let session_count = pending
            .values()
            .filter(|v| v.session_id == session_id)
            .count();
        if session_count >= 10 {
            // Evict oldest confirmation for this session to prevent unbounded growth
            if let Some(oldest_key) = pending
                .iter()
                .filter(|(_, v)| v.session_id == session_id)
                .min_by_key(|(_, v)| v.created_at)
                .map(|(k, _)| k.clone())
            {
                pending.remove(&oldest_key);
            }
        }

        pending.insert(confirmation_id.clone(), state);

        confirmation_id
    }

    pub(super) async fn get_confirmation(&self, confirmation_id: &str) -> Option<ConfirmationState> {
        let mut pending = self.pending.write().await;
        let state = pending.get(confirmation_id)?;

        // Check if confirmation has expired (10 minutes)
        if state.created_at.elapsed() > Duration::from_secs(600) {
            pending.remove(confirmation_id);
            return None;
        }

        Some(state.clone())
    }

    pub(super) async fn remove_confirmation(&self, confirmation_id: &str) {
        let mut pending = self.pending.write().await;
        pending.remove(confirmation_id);
    }
}

// =============================================================================
// CHAT ENGINE
// =============================================================================

/// Main chat engine that orchestrates conversation and tool calling
pub struct ChatEngine {
    pub(super) tool_registry: Arc<ToolRegistry>,
    pub(super) confirmation_manager: Arc<ConfirmationManager>,
}

impl ChatEngine {
    /// Create a new chat engine
    pub fn new() -> Self {
        let tool_registry = Arc::new(create_tool_registry());
        let confirmation_manager = Arc::new(ConfirmationManager::new());

        Self {
            tool_registry,
            confirmation_manager,
        }
    }

    /// Process a user message and generate response
    pub async fn process_message(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        // Get database pool
        let pool = database::get_chat_pool()
            .ok_or_else(|| AiError::ValidationError("Chat database not initialized".to_owned()))?;

        // Add user message to history
        let user_message_id =
            database::add_message(&pool, request.session_id, "user", &request.message, None)
                .map_err(|e| AiError::ParseError(format!("Failed to save user message: {e}")))?;

        logger::debug(
            LogTag::Api,
            &format!(
                "Processing chat message for session {} (message {})",
                request.session_id, user_message_id
            ),
        );

        // Load conversation history
        let history = database::get_messages(&pool, request.session_id)
            .map_err(|e| AiError::ParseError(format!("Failed to load history: {e}")))?;

        // Build messages for LLM (system + history)
        let mut messages = self.build_messages(&history, &request.context)?;

        // Execute tool calling loop
        let mut tool_calls_info = Vec::new();
        let mut iteration = 0;

        let final_content = loop {
            if iteration >= MAX_TOOL_ITERATIONS {
                logger::warning(
                    LogTag::Api,
                    &format!("Max tool iterations ({MAX_TOOL_ITERATIONS}) reached"),
                );
                break "I've reached the maximum number of tool calls. Please try breaking this down into smaller requests.".to_owned();
            }

            // Call LLM
            let llm_response = self.call_llm(&messages).await?;
            let content = llm_response.content.trim();

            logger::debug(LogTag::Api, &format!("LLM response: {content}"));

            // Parse tool calls from response
            let tool_calls = self.parse_tool_calls(content);

            if tool_calls.is_empty() {
                // No more tool calls, we're done
                break content.to_string();
            }

            logger::debug(
                LogTag::Api,
                &format!("Parsed {} tool calls", tool_calls.len()),
            );

            // Execute tools
            let (results, pending) = self
                .execute_tools(
                    tool_calls,
                    request.session_id,
                    user_message_id,
                    &pool,
                    request.headless,
                    &request.tool_mode,
                )
                .await;

            // If there are pending confirmations, return early
            if !pending.is_empty() {
                logger::info(
                    LogTag::Api,
                    &format!("Waiting for {} confirmations", pending.len()),
                );

                return Ok(ChatResponse {
                    message_id: user_message_id,
                    content: content.to_string(),
                    tool_calls: results,
                    pending_confirmations: pending,
                    is_complete: false,
                });
            }

            // Add tool results to conversation
            tool_calls_info.extend(results.clone());

            // Build tool results message
            let tool_results_text = self.format_tool_results(&results);
            messages.push(LlmChatMessage::assistant(content));
            messages.push(LlmChatMessage::user(format!(
                "Tool execution results:\n{}",
                tool_results_text
            )));

            // Trim messages if getting too long (keep first system message + last 20)
            if messages.len() > 50 {
                let system_msg = messages[0].clone();
                let keep_last = messages.split_off(messages.len() - 20);
                messages.clear();
                messages.push(system_msg);
                messages.extend(keep_last);
            }

            iteration += 1;
        };

        // Save assistant response
        let tool_calls_json = if tool_calls_info.is_empty() {
            None
        } else {
            match serde_json::to_string(&tool_calls_info) {
                Ok(json) => Some(json),
                Err(e) => {
                    logger::warning(LogTag::Api, &format!("Failed to serialize tool calls: {e}"));
                    None
                }
            }
        };

        let assistant_message_id = database::add_message(
            &pool,
            request.session_id,
            "assistant",
            &final_content,
            tool_calls_json.as_deref(),
        )
        .map_err(|e| AiError::ParseError(format!("Failed to save assistant message: {e}")))?;

        logger::info(
            LogTag::Api,
            &format!(
                "Chat response generated for session {} (message {})",
                request.session_id, assistant_message_id
            ),
        );

        Ok(ChatResponse {
            message_id: assistant_message_id,
            content: final_content,
            tool_calls: tool_calls_info,
            pending_confirmations: Vec::new(),
            is_complete: true,
        })
    }

    /// Process a confirmation response from user
    pub async fn process_confirmation(
        &self,
        confirmation_id: &str,
        approved: bool,
        session_id: Option<i64>,
    ) -> Result<ChatResponse, AiError> {
        // Get confirmation state
        let state = self
            .confirmation_manager
            .get_confirmation(confirmation_id)
            .await
            .ok_or_else(|| {
                AiError::ValidationError("Confirmation not found or expired".to_owned())
            })?;

        // Validate session_id if provided
        if let Some(sid) = session_id {
            if state.session_id != sid {
                return Err(AiError::ValidationError(
                    "Confirmation does not belong to this session".to_owned(),
                ));
            }
        }

        // Remove confirmation from pending
        self.confirmation_manager
            .remove_confirmation(confirmation_id)
            .await;

        // Bounds check before accessing tool_calls
        if state.current_index >= state.tool_calls.len() {
            return Err(AiError::ValidationError(
                "Invalid confirmation state: index out of bounds".to_owned(),
            ));
        }

        if !approved {
            // User denied the tool call
            logger::info(LogTag::Api, "User denied tool execution");

            return Ok(ChatResponse {
                message_id: state.message_id,
                content: "Tool execution was denied by user.".to_owned(),
                tool_calls: vec![ToolCallInfo {
                    tool_name: state.tool_calls[state.current_index].name.clone(),
                    input: state.tool_calls[state.current_index].arguments.clone(),
                    output: None,
                    status: ToolCallStatus::Denied,
                }],
                pending_confirmations: Vec::new(),
                is_complete: true,
            });
        }

        // Get database pool
        let pool = database::get_chat_pool()
            .ok_or_else(|| AiError::ValidationError("Chat database not initialized".to_owned()))?;

        // Execute the approved tool
        let tool_call = &state.tool_calls[state.current_index];
        let result = self
            .execute_single_tool(tool_call, state.message_id, &pool)
            .await;

        logger::info(
            LogTag::Api,
            &format!("Tool {} executed after approval", tool_call.name),
        );

        // Check if there are more tools to confirm
        let has_more_tools = state.current_index + 1 < state.tool_calls.len();

        if has_more_tools {
            // Update state with incremented index and re-insert
            let mut updated_state = state.clone();
            updated_state.current_index += 1;

            let new_confirmation_id = uuid::Uuid::new_v4().to_string();
            let mut pending = self.confirmation_manager.pending.write().await;
            pending.insert(new_confirmation_id.clone(), updated_state);

            // Get tool definition for description
            let next_tool = &state.tool_calls[state.current_index + 1];
            let description = self
                .tool_registry
                .get(&next_tool.name)
                .map(|t| t.definition().description.clone())
                .unwrap_or_else(|| "No description available".to_owned());

            // Return with next pending confirmation
            return Ok(ChatResponse {
                message_id: state.message_id,
                content: format!(
                    "Tool {} executed. Waiting for next confirmation.",
                    tool_call.name
                ),
                tool_calls: vec![result.clone()],
                pending_confirmations: vec![PendingConfirmation {
                    confirmation_id: new_confirmation_id,
                    tool_name: next_tool.name.clone(),
                    description,
                    input: next_tool.arguments.clone(),
                }],
                is_complete: false,
            });
        }

        // Return continuation message (all tools processed)
        Ok(ChatResponse {
            message_id: state.message_id,
            content: format!("Tool {} executed successfully.", tool_call.name),
            tool_calls: vec![result],
            pending_confirmations: Vec::new(),
            is_complete: true,
        })
    }
}

impl Default for ChatEngine {
    fn default() -> Self {
        Self::new()
    }
}
