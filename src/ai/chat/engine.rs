//! AI Chat Engine Module
//!
//! Main orchestrator for AI chat with MCP-like tool calling.
//! Handles conversation flow, tool execution, and permission management.

use super::database;
pub use super::types::{
    ChatContext, ChatProgressEvent, ChatRequest, ChatResponse, ToolCallInfo, ToolCallStatus,
    ToolMode,
};
use super::types::{ConfirmationState, PendingConfirmation, ToolCall};
use crate::ai::error::{Error, Result};
use crate::ai::tools::{create_tool_registry, ToolRegistry};
use crate::apis::llm::ChatMessage as LlmChatMessage;
use crate::logger::{self, LogTag};
use async_trait::async_trait;
use regex::Regex;
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
pub async fn init_chat_engine() -> Result<()> {
    let engine = ChatEngine::new();
    CHAT_ENGINE.set(Arc::new(engine)).map_err(|_| {
        Error::Internal(crate::errors::InternalError::InvariantViolation {
            message: "chat engine already initialized".to_owned(),
        })
    })
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

    pub(super) async fn get_confirmation(
        &self,
        confirmation_id: &str,
    ) -> Option<ConfirmationState> {
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
    pub(super) completion: Option<Arc<dyn ChatCompletion>>,
}

#[async_trait]
pub(super) trait ChatCompletion: Send + Sync {
    async fn complete(
        &self,
        request: crate::apis::llm::ChatRequest,
    ) -> std::result::Result<crate::apis::llm::ChatResponse, crate::apis::llm::LlmError>;
}

impl ChatEngine {
    /// Create a new chat engine
    pub fn new() -> Self {
        let tool_registry = Arc::new(create_tool_registry());
        let confirmation_manager = Arc::new(ConfirmationManager::new());

        Self {
            tool_registry,
            confirmation_manager,
            completion: None,
        }
    }

    #[cfg(test)]
    fn with_test_dependencies(
        tool_registry: ToolRegistry,
        completion: Arc<dyn ChatCompletion>,
    ) -> Self {
        Self {
            tool_registry: Arc::new(tool_registry),
            confirmation_manager: Arc::new(ConfirmationManager::new()),
            completion: Some(completion),
        }
    }

    /// Process a user message and generate response
    pub async fn process_message(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.process_message_with_progress(request, None).await
    }

    /// Process a user message while emitting safe, user-visible progress events.
    pub async fn process_message_streaming(
        &self,
        request: ChatRequest,
        progress: tokio::sync::mpsc::UnboundedSender<ChatProgressEvent>,
    ) -> Result<ChatResponse> {
        self.process_message_with_progress(request, Some(progress))
            .await
    }

    async fn process_message_with_progress(
        &self,
        request: ChatRequest,
        progress: Option<tokio::sync::mpsc::UnboundedSender<ChatProgressEvent>>,
    ) -> Result<ChatResponse> {
        let pool = database::get_chat_pool().ok_or_else(|| {
            Error::Internal(crate::errors::InternalError::InvariantViolation {
                message: "chat database not initialized".to_owned(),
            })
        })?;
        self.process_message_with_pool(request, &pool, progress)
            .await
    }

    async fn process_message_with_pool(
        &self,
        request: ChatRequest,
        pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
        progress: Option<tokio::sync::mpsc::UnboundedSender<ChatProgressEvent>>,
    ) -> Result<ChatResponse> {
        let (user_message_id, history, replaced_message_id) = if let Some(message_id) =
            request.regenerate_message_id
        {
            let history = database::get_messages(&pool, request.session_id)?;
            let target_index = history
                .iter()
                .position(|message| message.id == message_id && message.role == "assistant")
                .ok_or_else(|| Error::InvalidParameters {
                    detail: "regeneration target is not an assistant message in this session"
                        .to_owned(),
                })?;
            if target_index + 1 != history.len() || target_index == 0 {
                return Err(Error::InvalidParameters {
                    detail: "only the latest assistant response can be regenerated".to_owned(),
                });
            }
            let user_message = &history[target_index - 1];
            if user_message.role != "user" {
                return Err(Error::InvalidParameters {
                    detail: "regeneration target has no preceding user message".to_owned(),
                });
            }
            (
                user_message.id,
                history[..target_index].to_vec(),
                Some(message_id),
            )
        } else {
            let message_id =
                database::add_message(&pool, request.session_id, "user", &request.message, None)?;
            let history = database::get_messages(&pool, request.session_id)?;
            (message_id, history, None)
        };

        logger::debug(
            LogTag::Api,
            &format!(
                "Processing chat message for session {} (message {})",
                request.session_id, user_message_id
            ),
        );

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

            if let Some(progress) = &progress {
                let _ = progress.send(ChatProgressEvent::Thinking { iteration });
            }

            // Call LLM
            let llm_response = match self.call_llm(&messages).await {
                Ok(response) => response,
                Err(error) => {
                    if replaced_message_id.is_none() && tool_calls_info.is_empty() {
                        if let Err(cleanup_error) = database::delete_message(&pool, user_message_id)
                        {
                            logger::warning(
                                LogTag::Api,
                                &format!(
                                    "Failed to remove incomplete chat turn {user_message_id}: {cleanup_error}"
                                ),
                            );
                        }
                    }
                    return Err(error);
                }
            };
            let content = llm_response.content.trim();

            logger::debug(LogTag::Api, &format!("LLM response: {content}"));

            // Parse tool calls from response
            let tool_calls = if llm_response.tool_calls.is_empty() {
                self.parse_tool_calls(content)
            } else {
                llm_response
                    .tool_calls
                    .into_iter()
                    .map(|call| ToolCall {
                        name: call.name,
                        arguments: call.arguments,
                    })
                    .collect()
            };

            if tool_calls.is_empty() {
                // No more tool calls, we're done
                break content.to_string();
            }

            logger::debug(
                LogTag::Api,
                &format!("Parsed {} tool calls", tool_calls.len()),
            );

            if progress.as_ref().is_some_and(|sender| sender.is_closed()) {
                if replaced_message_id.is_none() && tool_calls_info.is_empty() {
                    let _ = database::delete_message(pool, user_message_id);
                }
                return Err(Error::InvalidParameters {
                    detail: "chat request was cancelled".to_owned(),
                });
            }

            if let Some(progress) = &progress {
                for tool_call in &tool_calls {
                    let _ = progress.send(ChatProgressEvent::ToolStarted {
                        tool_name: tool_call.name.clone(),
                    });
                }
            }

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

            if let Some(progress) = &progress {
                for result in &results {
                    let _ = progress.send(ChatProgressEvent::ToolFinished {
                        tool_call: result.clone(),
                    });
                }
            }

            // If there are pending confirmations, return early
            if !pending.is_empty() {
                logger::info(
                    LogTag::Api,
                    &format!("Waiting for {} confirmations", pending.len()),
                );

                if let Some(message_id) = replaced_message_id {
                    database::delete_message(&pool, message_id)?;
                }
                let tool_calls_json = serde_json::to_string(&results).ok();
                let assistant_message_id = database::add_message(
                    &pool,
                    request.session_id,
                    "assistant",
                    content,
                    tool_calls_json.as_deref(),
                )?;
                let response = ChatResponse {
                    message_id: assistant_message_id,
                    content: content.to_string(),
                    tool_calls: results,
                    pending_confirmations: pending,
                    is_complete: false,
                };
                if let Some(progress) = &progress {
                    let _ = progress.send(ChatProgressEvent::Complete {
                        response: response.clone(),
                    });
                }
                return Ok(response);
            }

            // Add tool results to conversation
            tool_calls_info.extend(results.clone());

            // Build tool results message
            let tool_results_text = self.format_tool_results(&results);
            if !content.is_empty() {
                messages.push(LlmChatMessage::assistant(content));
            }
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
        )?;

        if let Some(message_id) = replaced_message_id {
            database::delete_message(&pool, message_id)?;
        }

        logger::info(
            LogTag::Api,
            &format!(
                "Chat response generated for session {} (message {})",
                request.session_id, assistant_message_id
            ),
        );

        let response = ChatResponse {
            message_id: assistant_message_id,
            content: final_content,
            tool_calls: tool_calls_info,
            pending_confirmations: Vec::new(),
            is_complete: true,
        };
        if let Some(progress) = &progress {
            let _ = progress.send(ChatProgressEvent::Complete {
                response: response.clone(),
            });
        }
        Ok(response)
    }

    /// Process a confirmation response from user
    pub async fn process_confirmation(
        &self,
        confirmation_id: &str,
        approved: bool,
        session_id: i64,
    ) -> Result<ChatResponse> {
        // Get confirmation state
        let state = self
            .confirmation_manager
            .get_confirmation(confirmation_id)
            .await
            .ok_or_else(|| Error::InvalidParameters {
                detail: "confirmation not found or expired".to_owned(),
            })?;

        if state.session_id != session_id {
            return Err(Error::InvalidParameters {
                detail: "confirmation does not belong to this session".to_owned(),
            });
        }

        // Remove confirmation from pending
        self.confirmation_manager
            .remove_confirmation(confirmation_id)
            .await;

        // Bounds check before accessing tool_calls
        if state.current_index >= state.tool_calls.len() {
            return Err(Error::InvalidParameters {
                detail: "invalid confirmation state: index out of bounds".to_owned(),
            });
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
        let pool = database::get_chat_pool().ok_or_else(|| {
            Error::Internal(crate::errors::InternalError::InvariantViolation {
                message: "chat database not initialized".to_owned(),
            })
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::tools::{Tool, ToolCategory, ToolDefinition, ToolResult};
    use crate::apis::llm::{LlmError, ToolCall as LlmToolCall, Usage};
    use async_trait::async_trait;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct MockCompletion {
        responses: Mutex<VecDeque<crate::apis::llm::ChatResponse>>,
    }

    #[async_trait]
    impl ChatCompletion for MockCompletion {
        async fn complete(
            &self,
            request: crate::apis::llm::ChatRequest,
        ) -> std::result::Result<crate::apis::llm::ChatResponse, LlmError> {
            assert!(request
                .tools
                .as_ref()
                .is_some_and(|tools| tools.iter().any(|tool| tool.name == "test_balance")));
            self.responses
                .lock()
                .expect("mock completion lock")
                .pop_front()
                .ok_or_else(|| LlmError::InvalidResponse {
                    provider: "mock".to_owned(),
                    message: "No queued response".to_owned(),
                })
        }
    }

    struct TestBalanceTool;

    #[async_trait]
    impl Tool for TestBalanceTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "test_balance".to_owned(),
                description: "Return a deterministic balance".to_owned(),
                category: ToolCategory::Portfolio,
                parameters: serde_json::json!({"type": "object", "properties": {}}),
                requires_confirmation: false,
            }
        }

        async fn execute(&self, _params: serde_json::Value) -> ToolResult {
            ToolResult::success(serde_json::json!({"sol_balance": 2.5}))
        }
    }

    #[tokio::test]
    async fn full_agent_turn_streams_native_tool_execution_and_persists_result() {
        let pool = database::test_pool();
        let session_id = database::create_session(&pool, "Agent flow").expect("create session");
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestBalanceTool));
        let completion = Arc::new(MockCompletion {
            responses: Mutex::new(VecDeque::from([
                crate::apis::llm::ChatResponse::new(
                    "",
                    Usage::default(),
                    "tool_calls",
                    "mock",
                    1.0,
                )
                .with_tool_calls(vec![LlmToolCall {
                    id: "call-1".to_owned(),
                    name: "test_balance".to_owned(),
                    arguments: serde_json::json!({}),
                }]),
                crate::apis::llm::ChatResponse::new(
                    "Your wallet balance is 2.5 SOL.",
                    Usage::default(),
                    "stop",
                    "mock",
                    1.0,
                ),
            ])),
        });
        let engine = ChatEngine::with_test_dependencies(registry, completion);
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        let response = engine
            .process_message_with_pool(
                ChatRequest {
                    session_id,
                    message: "What is my balance?".to_owned(),
                    regenerate_message_id: None,
                    context: None,
                    headless: false,
                    tool_mode: ToolMode::ReadOnly,
                },
                &pool,
                Some(sender),
            )
            .await
            .expect("complete agent turn");

        assert_eq!(response.content, "Your wallet balance is 2.5 SOL.");
        assert_eq!(response.tool_calls.len(), 1);
        assert!(matches!(
            response.tool_calls[0].status,
            ToolCallStatus::Executed
        ));
        let messages = database::get_messages(&pool, session_id).expect("persisted messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[1].content, response.content);
        let executions =
            database::get_tool_executions(&pool, messages[0].id).expect("persisted tool execution");
        assert_eq!(executions.len(), 1);

        let mut event_types = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            event_types.push(match event {
                ChatProgressEvent::Thinking { .. } => "thinking",
                ChatProgressEvent::ToolStarted { .. } => "tool_started",
                ChatProgressEvent::ToolFinished { .. } => "tool_finished",
                ChatProgressEvent::Complete { .. } => "complete",
                ChatProgressEvent::Error { .. } => "error",
            });
        }
        assert_eq!(
            event_types,
            [
                "thinking",
                "tool_started",
                "tool_finished",
                "thinking",
                "complete"
            ]
        );
    }
}
