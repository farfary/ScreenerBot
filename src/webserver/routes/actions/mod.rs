//! Actions route — handles user-triggered actions like start, stop, and reset.

mod handlers;
mod types;

pub use types::*;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create actions routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/actions/stream", get(handlers::stream_actions))
        .route("/actions/active", get(handlers::get_active_actions))
        .route("/actions/all", get(handlers::get_all_actions))
        .route("/actions/history", get(handlers::get_action_history))
        .route("/actions/:action_id", get(handlers::get_action_by_id))
        .route("/actions/subscribers", get(handlers::get_subscriber_count))
}
