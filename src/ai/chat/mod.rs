//! AI Chat Submodule
//!
//! Groups all chat-related functionality:
//! - `types` — Chat data types (request, response, tool call info)
//! - `engine` — Chat engine singleton and orchestration
//! - `engine_internals` — Private implementation methods for ChatEngine
//! - `database` — SQLite persistence for chat sessions/messages/tool executions
//! - `database_queries` — CRUD query operations

pub mod database;
mod database_queries;
pub mod engine;
mod engine_internals;
pub mod types;

// Re-export database items so callers can use `chat::database::get_chat_pool()` etc.
pub use database::{
    add_message, add_tool_execution, cleanup_hidden_sessions, create_hidden_session,
    create_session, delete_message, delete_session, get_chat_pool, get_message, get_messages,
    get_session, get_sessions, get_tool_executions, init_chat_db, touch_session,
    update_session_summary, update_session_title, update_tool_execution, with_chat_db, ChatMessage,
    ChatSession, ToolExecution,
};

// Re-export engine items
pub use engine::{get_chat_engine, init_chat_engine, try_get_chat_engine, ChatEngine};

// Re-export chat types
pub use types::{
    ChatContext, ChatRequest, ChatResponse, PendingConfirmation, ToolCallInfo, ToolCallStatus,
    ToolMode,
};
