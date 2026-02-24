//! AI chat sessions, messages, tools, and permissions handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde::Serialize;
use std::sync::Arc;

use crate::ai::chat_db;
use crate::ai::permissions::ToolPermissions;
use crate::ai::{try_get_chat_engine, ChatRequest as ChatEngineRequest};
use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// ============================================================================
// CHAT HANDLERS
// ============================================================================

/// POST /api/ai/chat - Send a message to AI chat
pub async fn send_chat_message(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SendChatMessageRequest>,
) -> Response {
    // Validate message
    if req.message.trim().is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MESSAGE",
            "Message cannot be empty",
            None,
        );
    }

    if req.message.len() > 10000 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "MESSAGE_TOO_LONG",
            "Message exceeds maximum length of 10,000 characters",
            None,
        );
    }

    // Validate session exists
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::get_session(&pool, req.session_id) {
        Ok(Some(_)) => {
            // Session exists, continue
        }
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "SESSION_NOT_FOUND",
                &format!("Chat session {} not found", req.session_id),
                None,
            )
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to validate session: {e}"),
                None,
            )
        }
    }

    // Get chat engine
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Create chat request
    let chat_request = ChatEngineRequest {
        session_id: req.session_id,
        message: req.message,
        context: req.context,
        headless: false,
        tool_mode: Default::default(),
    };

    // Process message
    match engine.process_message(chat_request).await {
        Ok(response) => {
            logger::info(
                LogTag::Api,
                &format!(
                    "Chat message processed for session {} (message {})",
                    req.session_id, response.message_id
                ),
            );
            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_ERROR",
            &format!("Failed to process chat message: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/chat/sessions - List all chat sessions
pub async fn list_chat_sessions(State(_state): State<Arc<AppState>>) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::get_sessions(&pool) {
        Ok(sessions) => success_response(sessions),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to list chat sessions: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions - Create new chat session
pub async fn create_chat_session(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CreateChatSessionRequest>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    let title = req.title.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("Chat {}", now.format("%Y-%m-%d %H:%M"))
    });

    match chat_db::create_session(&pool, &title) {
        Ok(session_id) => {
            logger::info(
                LogTag::Api,
                &format!("Created chat session: {session_id}"),
            );
            success_response(CreateChatSessionResponse { session_id })
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to create chat session: {e}"),
            None,
        ),
    }
}

/// GET /api/ai/chat/sessions/:id - Get session with messages
pub async fn get_chat_session(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get session
    let session = match chat_db::get_session(&pool, id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Chat session {} not found", id),
                None,
            )
        }
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get chat session: {e}"),
                None,
            )
        }
    };

    // Get messages
    match chat_db::get_messages(&pool, id) {
        Ok(messages) => success_response(GetChatSessionResponse { session, messages }),
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to get chat messages: {e}"),
            None,
        ),
    }
}

/// DELETE /api/ai/chat/sessions/:id - Delete session
pub async fn delete_chat_session(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    match chat_db::delete_session(&pool, id) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Deleted chat session: {id}"));
            success_response(serde_json::json!({
                "message": "Chat session deleted successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to delete chat session: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions/:id/summarize - Summarize session
pub async fn summarize_chat_session(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get messages: {e}"),
                None,
            )
        }
    };

    if messages.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EMPTY_SESSION",
            "Cannot summarize empty chat session",
            None,
        );
    }

    // Build conversation text
    let conversation: Vec<String> = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect();
    let conversation_text = conversation.join("\n");

    // Ask LLM to summarize
    let llm_manager = match try_get_llm_manager() {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "LLM_NOT_CONFIGURED",
                "LLM manager not initialized",
                None,
            )
        }
    };

    let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER",
                &format!("Invalid provider: {provider_name}"),
                None,
            )
        }
    };

    // Get the model for the configured provider
    let model = super::Assistant_auth::get_model_for_provider(provider);

    let request = ChatRequest::new(
        model,
        vec![
            ChatMessage::system(
                "You are a helpful assistant that creates concise summaries of chat conversations."
                    .to_string(),
            ),
            ChatMessage::user(format!(
                "Please provide a brief 1-2 sentence summary of this conversation:\n\n{}",
                conversation_text
            )),
        ],
    )
    .with_temperature(0.5)
    .with_max_tokens(150);

    match llm_manager.call(provider, request).await {
        Ok(response) => {
            let summary = response.content.trim().to_owned();

            // Save summary to session
            if let Err(e) = chat_db::update_session_summary(&pool, id, &summary) {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to save summary: {e}"),
                    None,
                );
            }

            logger::info(LogTag::Api, &format!("Summarized chat session: {id}"));
            success_response(serde_json::json!({
                "summary": summary
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "LLM_ERROR",
            &format!("Failed to generate summary: {e}"),
            None,
        ),
    }
}

/// POST /api/ai/chat/sessions/:id/generate-title - Generate AI title for session
pub async fn generate_session_title(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_DB_NOT_INITIALIZED",
                "Chat database not initialized",
                None,
            )
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to get messages: {e}"),
                None,
            )
        }
    };

    if messages.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "EMPTY_SESSION",
            "Cannot generate title for empty chat session",
            None,
        );
    }

    // Get the first 2-3 messages (user + assistant exchanges)
    let mut first_user_msg = String::new();
    let mut first_assistant_msg = String::new();

    for msg in messages.iter().take(5) {
        if msg.role == "user" && first_user_msg.is_empty() {
            first_user_msg = msg.content.clone();
        } else if msg.role == "assistant"
            && first_assistant_msg.is_empty()
            && !first_user_msg.is_empty()
        {
            first_assistant_msg = msg.content.clone();
            break; // We have enough context
        }
    }

    if first_user_msg.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_USER_MESSAGE",
            "No user messages found in session",
            None,
        );
    }

    // Build the title generation prompt
    let assistant_part = if !first_assistant_msg.is_empty() {
        format!("\nAssistant: {first_assistant_msg}")
    } else {
        String::new()
    };

    let prompt = format!(
        "Generate a short, descriptive title (3-8 words) for this conversation. Output only the title, no quotes or formatting.\n\nUser: {}{}

Rules:
- Keep it concise (3-8 words max)
- Focus on the main topic or intent
- Match the language of the conversation
- If it's a generic greeting, use something like \"Quick Chat\" or \"General Question\"",
        first_user_msg, assistant_part
    );

    // Call LLM to generate title
    let llm_manager = match try_get_llm_manager() {
        Some(m) => m,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "LLM_NOT_CONFIGURED",
                "LLM manager not initialized",
                None,
            )
        }
    };

    let provider_name = with_config(|cfg| cfg.ai.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_PROVIDER",
                &format!("Invalid provider: {provider_name}"),
                None,
            )
        }
    };

    // Get the model for the configured provider
    let model = super::Assistant_auth::get_model_for_provider(provider);

    let request = ChatRequest::new(model, vec![ChatMessage::user(prompt)])
        .with_temperature(0.7)
        .with_max_tokens(50);

    let title = match llm_manager.call(provider, request).await {
        Ok(response) => {
            let raw_title = response.content.trim();

            // Remove quotes if present
            let cleaned_title = raw_title.trim_matches('"').trim_matches('\'').trim();

            // Ensure title is within 50 characters
            if cleaned_title.len() > 50 {
                cleaned_title.chars().take(47).collect::<String>() + "..."
            } else {
                cleaned_title.to_string()
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Api,
                &format!("Failed to generate title with LLM: {e}"),
            );
            // Fallback: use first few words of user message
            let words: Vec<&str> = first_user_msg.split_whitespace().take(5).collect();
            let fallback = words.join(" ");
            if fallback.len() > 50 {
                fallback.chars().take(47).collect::<String>() + "..."
            } else {
                fallback
            }
        }
    };

    // Update session title in database
    if let Err(e) = chat_db::update_session_title(&pool, id, &title) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DB_ERROR",
            &format!("Failed to update session title: {e}"),
            None,
        );
    }

    logger::info(
        LogTag::Api,
        &format!("Generated title for session {id}: {title}"),
    );

    #[derive(Serialize)]
    pub struct GenerateTitleResponse {
        pub title: String,
    }

    success_response(GenerateTitleResponse { title })
}

/// POST /api/ai/chat/confirm/:confirmation_id - Confirm/deny tool execution
pub async fn confirm_tool_execution(
    State(_state): State<Arc<AppState>>,
    Path(confirmation_id): Path<String>,
    Json(req): Json<ConfirmToolExecutionRequest>,
) -> Response {
    // Get chat engine
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Process confirmation with optional session_id validation
    match engine
        .process_confirmation(&confirmation_id, req.approved, req.session_id)
        .await
    {
        Ok(response) => {
            logger::info(
                LogTag::Api,
                &format!(
                    "Tool execution confirmation processed: {} (approved: {})",
                    confirmation_id, req.approved
                ),
            );
            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CHAT_ERROR",
            &format!("Failed to process confirmation: {e}"),
            None,
        ),
    }
}

// ============================================================================
// TOOLS & PERMISSIONS HANDLERS
// ============================================================================

/// GET /api/ai/tools - List available tools
pub async fn list_tools(State(_state): State<Arc<AppState>>) -> Response {
    // Get chat engine to access tool registry
    let _engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "CHAT_NOT_INITIALIZED",
                "Chat engine not initialized",
                None,
            )
        }
    };

    // Use the tool registry from the engine (we'll need to expose this method)
    // For now, create a temporary registry
    let registry = crate::ai::create_tool_registry();
    let tools = registry.list_definitions();

    success_response(tools)
}

/// GET /api/ai/permissions - Get tool permissions
pub async fn get_permissions(State(_state): State<Arc<AppState>>) -> Response {
    let permissions = with_config(|cfg| ToolPermissions {
        analysis: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_analysis),
        portfolio: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_portfolio),
        trading: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_trading),
        config: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_config),
        system: crate::ai::PermissionLevel::from_str(&cfg.ai.tool_permissions_system),
    });

    success_response(permissions)
}

/// PATCH /api/ai/permissions - Update permissions
pub async fn update_permissions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ToolPermissions>,
) -> Response {
    match update_config_section(
        |cfg| {
            cfg.ai.tool_permissions_analysis = req.analysis.to_str().to_string();
            cfg.ai.tool_permissions_portfolio = req.portfolio.to_str().to_string();
            cfg.ai.tool_permissions_trading = req.trading.to_str().to_string();
            cfg.ai.tool_permissions_config = req.config.to_str().to_string();
            cfg.ai.tool_permissions_system = req.system.to_str().to_string();
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Api, "Updated AI tool permissions");
            success_response(serde_json::json!({
                "message": "Tool permissions updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update permissions: {e}"),
            None,
        ),
    }
}
