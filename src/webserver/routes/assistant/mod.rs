//! Dashboard assistant API (`/api/assistant`).
//!
//! Owns interactive chat (sessions, messages, streaming, titles, summaries,
//! tool-execution confirmations) and scheduled automation (task CRUD, manual
//! run, run history and stats). Provider credentials are `/api/llm`; analysis
//! is `/api/llm-analysis`; the shared tool permission policy is
//! `/api/agent-control`.

use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod automation;
mod chat;
pub mod types;

use automation::{
    create_automation_task, delete_automation_task, get_automation_recent_runs,
    get_automation_run_detail, get_automation_stats_handler, get_automation_task,
    get_automation_task_runs, list_automation_tasks, run_automation_task, toggle_automation_task,
    update_automation_task,
};
use chat::{
    confirm_tool_execution, create_chat_session, delete_chat_session, generate_session_title,
    get_chat_session, list_chat_sessions, send_chat_message, stream_chat_message,
    summarize_chat_session,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Chat
        .route("/chat", post(send_chat_message))
        .route("/chat/stream", post(stream_chat_message))
        .route("/chat/sessions", get(list_chat_sessions))
        .route("/chat/sessions", post(create_chat_session))
        .route("/chat/sessions/:id", get(get_chat_session))
        .route("/chat/sessions/:id", delete(delete_chat_session))
        .route("/chat/sessions/:id/summarize", post(summarize_chat_session))
        .route(
            "/chat/sessions/:id/generate-title",
            post(generate_session_title),
        )
        .route(
            "/chat/confirm/:confirmation_id",
            post(confirm_tool_execution),
        )
        // Scheduled automation
        .route(
            "/automation",
            get(list_automation_tasks).post(create_automation_task),
        )
        .route("/automation/runs", get(get_automation_recent_runs))
        .route("/automation/stats", get(get_automation_stats_handler))
        .route(
            "/automation/:id",
            get(get_automation_task)
                .patch(update_automation_task)
                .delete(delete_automation_task),
        )
        .route("/automation/:id/toggle", post(toggle_automation_task))
        .route("/automation/:id/run", post(run_automation_task))
        .route("/automation/:id/runs", get(get_automation_task_runs))
        .route("/automation/runs/:id", get(get_automation_run_detail))
}
