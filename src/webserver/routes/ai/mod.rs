//! AI Module API Routes
//!
//! Endpoints for AI analysis, provider management, chat, and testing.

use axum::{
    response::Response,
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod automation;
mod chat;
mod instructions;
mod providers;
mod status;
pub mod types;

// Re-export handler functions for use by the router
use automation::{
    create_automation_task, delete_automation_task, get_automation_recent_runs,
    get_automation_run_detail, get_automation_stats_handler, get_automation_task,
    get_automation_task_runs, list_automation_tasks, run_automation_task, toggle_automation_task,
    update_automation_task,
};
use chat::{
    confirm_tool_execution, create_chat_session, delete_chat_session, generate_session_title,
    get_chat_session, get_permissions, list_chat_sessions, list_tools, send_chat_message,
    stream_chat_message, summarize_chat_session, update_permissions,
};
use instructions::{
    create_instruction, delete_instruction, get_history_detail, get_instruction, list_history,
    list_instructions, list_templates, reorder_instructions, update_instruction,
};
use providers::{list_providers, test_provider, update_provider};
use status::{
    clear_cache, get_ai_config, get_ai_stats, get_ai_status, get_cache_stats, test_evaluate,
    update_ai_config,
};

// ============================================================================
// ROUTES
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Status & Stats
        .route("/status", get(get_ai_status))
        .route("/stats", get(get_ai_stats))
        // Provider Management
        .route("/providers", get(list_providers))
        .route("/providers/:provider", patch(update_provider))
        .route("/providers/:provider/test", post(test_provider))
        // Configuration
        .route("/config", get(get_ai_config))
        .route("/config", patch(update_ai_config))
        // Cache
        .route("/cache/clear", post(clear_cache))
        .route("/cache/stats", get(get_cache_stats))
        // Testing
        .route("/test/evaluate", post(test_evaluate))
        // Instructions
        .route("/instructions", get(list_instructions))
        .route("/instructions", post(create_instruction))
        .route("/instructions/:id", get(get_instruction))
        .route("/instructions/:id", patch(update_instruction))
        .route("/instructions/:id", delete(delete_instruction))
        .route("/instructions/reorder", post(reorder_instructions))
        // Templates
        .route("/templates", get(list_templates))
        // History
        .route("/history", get(list_history))
        .route("/history/:id", get(get_history_detail))
        // Chat Routes
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
        // Tools & Permissions
        .route("/tools", get(list_tools))
        .route("/permissions", get(get_permissions))
        .route("/permissions", patch(update_permissions))
        // Automation routes
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
