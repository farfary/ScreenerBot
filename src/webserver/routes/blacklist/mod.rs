//! Blacklist route — manages token blacklist additions and removals via the UI.

mod handlers;
mod types;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

/// Create blacklist routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/blacklist/stats", get(handlers::get_blacklist_stats))
        .route("/blacklist/details", get(handlers::get_blacklist_details))
}
