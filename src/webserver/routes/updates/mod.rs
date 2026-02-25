//! Update management API endpoints
//!
//! Provides endpoints for version info, update checking, downloading, and status.

mod handlers;
pub mod types;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

// =============================================================================
// Routes
// =============================================================================

/// Create update check routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/version", get(handlers::get_version))
        .route("/updates/check", get(handlers::check_updates))
        .route("/updates/download", post(handlers::download_update))
        .route("/updates/status", get(handlers::get_status))
        .route("/updates/install", post(handlers::install_update))
}
