//! Events route — Server-Sent Events (SSE) endpoint for real-time UI updates.

mod handlers;
mod types;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

/// Create events routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/events/head", get(handlers::get_events_head))
        .route("/events/since", get(handlers::get_events_since))
        .route("/events/before", get(handlers::get_events_before))
        .route("/events/categories", get(handlers::get_categories))
}
