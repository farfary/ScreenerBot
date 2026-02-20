use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod analytics;
mod helpers;
mod stats;
mod tokens;
mod types;

use analytics::get_analytics;
use stats::{get_rejection_stats, get_stats, trigger_refresh};
use tokens::{export_rejected_tokens, get_rejected_tokens_handler};

/// Filtering management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/filtering/refresh", post(trigger_refresh))
        .route("/filtering/stats", get(get_stats))
        .route("/filtering/rejection-stats", get(get_rejection_stats))
        .route("/filtering/analytics", get(get_analytics))
        .route(
            "/filtering/rejected-tokens",
            get(get_rejected_tokens_handler),
        )
        .route(
            "/filtering/export-rejected-tokens",
            get(export_rejected_tokens),
        )
}
