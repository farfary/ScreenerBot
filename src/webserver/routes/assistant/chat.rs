//! Assistant chat sessions, messages and confirmations (`/api/assistant/chat`).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, Response, Sse},
    Json,
};
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};

use crate::apis::llm::{try_get_llm_manager, ChatMessage, ChatRequest, Provider};
use crate::assistant::chat::database as chat_db;
use crate::assistant::chat::ChatProgressEvent;
use crate::assistant::{try_get_chat_engine, ChatRequest as ChatEngineRequest};
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// ============================================================================
// CHAT HANDLERS
// ============================================================================

/// POST /api/assistant/chat - Send a message to the Assistant
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
        regenerate_message_id: req.regenerate_message_id,
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

/// POST /api/assistant/chat/stream - Stream agent progress and the final response.
pub async fn stream_chat_message(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SendChatMessageRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, Response> {
    if req.message.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_MESSAGE",
            "Message cannot be empty",
            None,
        ));
    }
    if req.message.len() > 10000 {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "MESSAGE_TOO_LONG",
            "Message exceeds maximum length of 10,000 characters",
            None,
        ));
    }
    let pool = chat_db::get_chat_pool().ok_or_else(|| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CHAT_DB_NOT_INITIALIZED",
            "Chat database not initialized",
            None,
        )
    })?;
    match chat_db::get_session(&pool, req.session_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(error_response(
                StatusCode::NOT_FOUND,
                "SESSION_NOT_FOUND",
                &format!("Chat session {} not found", req.session_id),
                None,
            ));
        }
        Err(error) => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DB_ERROR",
                &format!("Failed to validate session: {error}"),
                None,
            ));
        }
    }
    let engine = try_get_chat_engine().ok_or_else(|| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CHAT_NOT_INITIALIZED",
            "Chat engine not initialized",
            None,
        )
    })?;
    let chat_request = ChatEngineRequest {
        session_id: req.session_id,
        message: req.message,
        regenerate_message_id: req.regenerate_message_id,
        context: req.context,
        headless: false,
        tool_mode: Default::default(),
    };
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let error_sender = sender.clone();
    tokio::spawn(async move {
        if let Err(error) = engine.process_message_streaming(chat_request, sender).await {
            let _ = error_sender.send(ChatProgressEvent::Error {
                message: format!("{error}"),
            });
            logger::error(LogTag::Api, &format!("Streaming chat failed: {error}"));
        }
    });

    let stream = UnboundedReceiverStream::new(receiver).map(|event| {
        let data = serde_json::to_string(&event).unwrap_or_else(|_| {
            r#"{"type":"error","message":"Failed to serialize chat event"}"#.to_owned()
        });
        Ok(Event::default().data(data))
    });
    Ok(Sse::new(stream))
}

/// GET /api/assistant/chat/sessions - List all chat sessions
pub async fn list_chat_sessions(State(_state): State<Arc<AppState>>) -> Response {
    // Return promotional fixtures only for owner-initiated media capture — the real
    // list is the operator's own conversations.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return success_response(crate::webserver::promo::get_promo_chat_sessions());
    }

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

/// POST /api/assistant/chat/sessions - Create new chat session
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
            logger::info(LogTag::Api, &format!("Created chat session: {session_id}"));
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

/// GET /api/assistant/chat/sessions/:id - Get session with messages
pub async fn get_chat_session(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Response {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return match crate::webserver::promo::get_promo_chat_session(id) {
            Some(response) => success_response(response),
            None => error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                &format!("Chat session {id} not found"),
                None,
            ),
        };
    }

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
                &format!("Chat session {id} not found"),
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

/// DELETE /api/assistant/chat/sessions/:id - Delete session
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

/// POST /api/assistant/chat/sessions/:id/summarize - Summarize session
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

    let provider_name = with_config(|cfg| cfg.llm.default_provider.clone());
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
    let model = get_model_for_provider(provider);

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

/// POST /api/assistant/chat/sessions/:id/generate-title - Generate an Assistant session title
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

    let provider_name = with_config(|cfg| cfg.llm.default_provider.clone());
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
    let model = get_model_for_provider(provider);

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

/// POST /api/assistant/chat/confirm/:confirmation_id - Confirm/deny tool execution
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
            let pool = match chat_db::get_chat_pool() {
                Some(pool) => pool,
                None => {
                    return error_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "CHAT_DB_NOT_INITIALIZED",
                        "Chat database not initialized",
                        None,
                    )
                }
            };
            let tool_calls = serde_json::to_string(&response.tool_calls).ok();
            if let Err(e) = chat_db::add_message(
                &pool,
                req.session_id,
                "assistant",
                &response.content,
                tool_calls.as_deref(),
            ) {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DB_ERROR",
                    &format!("Failed to save confirmation response: {e}"),
                    None,
                );
            }
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
// HELPER FUNCTIONS
// ============================================================================

/// Get the appropriate model for a provider from config
fn get_model_for_provider(provider: Provider) -> String {
    with_config(|cfg| {
        let provider_config = match provider {
            Provider::OpenAi => &cfg.llm.providers.openai,
            Provider::Anthropic => &cfg.llm.providers.anthropic,
            Provider::Groq => &cfg.llm.providers.groq,
            Provider::DeepSeek => &cfg.llm.providers.deepseek,
            Provider::Gemini => &cfg.llm.providers.gemini,
            Provider::Together => &cfg.llm.providers.together,
            Provider::OpenRouter => &cfg.llm.providers.openrouter,
            Provider::Mistral => &cfg.llm.providers.mistral,
            Provider::Ollama => {
                return cfg.llm.providers.ollama.model.clone();
            }
        };

        if !provider_config.model.is_empty() {
            provider_config.model.clone()
        } else {
            // Default models for each provider
            match provider {
                Provider::OpenAi => "gpt-4".to_owned(),
                Provider::Anthropic => "claude-3-5-sonnet-20241022".to_owned(),
                Provider::Groq => "llama-3.1-70b-versatile".to_owned(),
                Provider::DeepSeek => "deepseek-chat".to_owned(),
                Provider::Gemini => "gemini-pro".to_owned(),
                Provider::Ollama => "llama3.2".to_owned(),
                Provider::Together => "meta-llama/Llama-3-70b-chat-hf".to_owned(),
                Provider::OpenRouter => "openai/gpt-4".to_owned(),
                Provider::Mistral => "mistral-large-latest".to_owned(),
            }
        }
    })
}
