//! Model-scored analysis API (`/api/llm-analysis`).
//!
//! Owns analysis status/stats/config, the evaluation cache, ad-hoc test
//! evaluation, and the instruction / template / decision-history surfaces.
//! Provider credentials are `/api/llm`; the dashboard assistant is
//! `/api/assistant`; tool permissions are `/api/agent-control`.

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod instructions;
mod status;
pub mod types;

use instructions::{
    create_instruction, delete_instruction, get_history_detail, get_instruction, list_history,
    list_instructions, list_templates, reorder_instructions, update_instruction,
};
use status::{
    clear_cache, get_analysis_config, get_analysis_stats, get_analysis_status, get_cache_stats,
    test_evaluate, update_analysis_config,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Status & stats
        .route("/status", get(get_analysis_status))
        .route("/stats", get(get_analysis_stats))
        // Configuration
        .route("/config", get(get_analysis_config))
        .route("/config", patch(update_analysis_config))
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
}
