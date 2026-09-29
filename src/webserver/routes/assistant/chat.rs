//! Assistant chat sessions, messages and confirmations (`/api/assistant/chat`).

use axum::{
    extract::{Path, State},
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
use crate::i18n::ids;
use crate::logger::{self, LogTag};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;
use axum::response::IntoResponse as _;

use crate::webserver::routes::llm::{assistant_failure, provider_failure};

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
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_CHAT_MESSAGE_EMPTY)
            .into_response();
    }

    if req.message.len() > 10000 {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CHAT_MESSAGE_TOO_LONG,
        )
        .into_response();
    }

    // Validate session exists
    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    match chat_db::get_session(&pool, req.session_id) {
        Ok(Some(_)) => {
            // Session exists, continue
        }
        Ok(None) => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_CHAT_SESSION_NOT_FOUND)
                .text_arg("id", req.session_id.to_string())
                .into_response()
        }
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::DatabaseError,
                ids::ERRORS_CHAT_SESSION_VALIDATE_FAILED,
            )
            .details(e.to_string())
            .into_response()
        }
    }

    // Get chat engine
    let engine = match try_get_chat_engine() {
        Some(e) => e,
        None => {
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_ENGINE_UNAVAILABLE,
            )
            .into_response()
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
        Err(e) => assistant_failure(ids::ERRORS_CHAT_PROCESS_FAILED, &e).into_response(),
    }
}

/// POST /api/assistant/chat/stream - Stream agent progress and the final response.
pub async fn stream_chat_message(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SendChatMessageRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, Response> {
    if req.message.trim().is_empty() {
        return Err(
            ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_CHAT_MESSAGE_EMPTY)
                .into_response(),
        );
    }
    if req.message.len() > 10000 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CHAT_MESSAGE_TOO_LONG,
        )
        .into_response());
    }
    let pool = chat_db::get_chat_pool().ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::ServiceUnavailable,
            ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
        )
        .into_response()
    })?;
    match chat_db::get_session(&pool, req.session_id) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(
                ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_CHAT_SESSION_NOT_FOUND)
                    .text_arg("id", req.session_id.to_string())
                    .into_response(),
            );
        }
        Err(error) => {
            return Err(ApiError::new(
                ApiErrorCode::DatabaseError,
                ids::ERRORS_CHAT_SESSION_VALIDATE_FAILED,
            )
            .details(error.to_string())
            .into_response());
        }
    }
    let engine = try_get_chat_engine().ok_or_else(|| {
        ApiError::new(
            ApiErrorCode::ServiceUnavailable,
            ids::ERRORS_CHAT_ENGINE_UNAVAILABLE,
        )
        .into_response()
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    match chat_db::get_sessions(&pool) {
        Ok(sessions) => success_response(sessions),
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_CHAT_SESSIONS_LIST_FAILED,
        )
        .details(e.to_string())
        .into_response(),
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
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
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_CHAT_SESSION_CREATE_FAILED,
        )
        .details(e.to_string())
        .into_response(),
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
            None => ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_CHAT_SESSION_NOT_FOUND)
                .text_arg("id", id.to_string())
                .into_response(),
        };
    }

    let pool = match chat_db::get_chat_pool() {
        Some(p) => p,
        None => {
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    // Get session
    let session = match chat_db::get_session(&pool, id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_CHAT_SESSION_NOT_FOUND)
                .text_arg("id", id.to_string())
                .into_response()
        }
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::DatabaseError,
                ids::ERRORS_CHAT_SESSION_GET_FAILED,
            )
            .details(e.to_string())
            .into_response()
        }
    };

    // Get messages
    match chat_db::get_messages(&pool, id) {
        Ok(messages) => success_response(GetChatSessionResponse { session, messages }),
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_CHAT_MESSAGES_GET_FAILED,
        )
        .details(e.to_string())
        .into_response(),
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    match chat_db::delete_session(&pool, id) {
        Ok(()) => {
            logger::info(LogTag::Api, &format!("Deleted chat session: {id}"));
            success_response(serde_json::json!({
                "message": "Chat session deleted successfully"
            }))
        }
        Err(e) => ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_CHAT_SESSION_DELETE_FAILED,
        )
        .details(e.to_string())
        .into_response(),
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::DatabaseError,
                ids::ERRORS_CHAT_MESSAGES_LOAD_FAILED,
            )
            .details(e.to_string())
            .into_response()
        }
    };

    if messages.is_empty() {
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_CHAT_SUMMARIZE_EMPTY)
            .into_response();
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_LLM_MANAGER_UNAVAILABLE,
            )
            .into_response()
        }
    };

    let provider_name = with_config(|cfg| cfg.llm.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_CHAT_PROVIDER_INVALID,
            )
            .text_arg("provider", provider_name.clone())
            .into_response()
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
                return ApiError::new(
                    ApiErrorCode::DatabaseError,
                    ids::ERRORS_CHAT_SUMMARY_SAVE_FAILED,
                )
                .details(e.to_string())
                .into_response();
            }

            logger::info(LogTag::Api, &format!("Summarized chat session: {id}"));
            success_response(serde_json::json!({
                "summary": summary
            }))
        }
        Err(e) => provider_failure(ids::ERRORS_CHAT_SUMMARY_FAILED, &e).into_response(),
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
            )
            .into_response()
        }
    };

    // Get messages for this session
    let messages = match chat_db::get_messages(&pool, id) {
        Ok(m) => m,
        Err(e) => {
            return ApiError::new(
                ApiErrorCode::DatabaseError,
                ids::ERRORS_CHAT_MESSAGES_LOAD_FAILED,
            )
            .details(e.to_string())
            .into_response()
        }
    };

    if messages.is_empty() {
        return ApiError::new(
            ApiErrorCode::InvalidInput,
            ids::ERRORS_CHAT_TITLE_EMPTY_SESSION,
        )
        .into_response();
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
        return ApiError::new(ApiErrorCode::InvalidInput, ids::ERRORS_CHAT_NO_USER_MESSAGE)
            .into_response();
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_LLM_MANAGER_UNAVAILABLE,
            )
            .into_response()
        }
    };

    let provider_name = with_config(|cfg| cfg.llm.default_provider.clone());
    let provider = match Provider::from_str(&provider_name) {
        Some(p) => p,
        None => {
            return ApiError::new(
                ApiErrorCode::InvalidInput,
                ids::ERRORS_CHAT_PROVIDER_INVALID,
            )
            .text_arg("provider", provider_name.clone())
            .into_response()
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
        return ApiError::new(
            ApiErrorCode::DatabaseError,
            ids::ERRORS_CHAT_TITLE_SAVE_FAILED,
        )
        .details(e.to_string())
        .into_response();
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
            return ApiError::new(
                ApiErrorCode::ServiceUnavailable,
                ids::ERRORS_CHAT_ENGINE_UNAVAILABLE,
            )
            .into_response()
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
                    return ApiError::new(
                        ApiErrorCode::ServiceUnavailable,
                        ids::ERRORS_CHAT_DATABASE_UNAVAILABLE,
                    )
                    .into_response()
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
                return ApiError::new(
                    ApiErrorCode::DatabaseError,
                    ids::ERRORS_CHAT_CONFIRMATION_SAVE_FAILED,
                )
                .details(e.to_string())
                .into_response();
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
        Err(e) => assistant_failure(ids::ERRORS_CHAT_CONFIRMATION_FAILED, &e).into_response(),
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
