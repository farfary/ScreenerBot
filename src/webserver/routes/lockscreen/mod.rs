//! Lockscreen API routes for dashboard security
//!
//! Provides REST API endpoints for managing lockscreen password and settings.

mod handlers;
pub mod types;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::webserver::state::AppState;

pub use types::*;

// =============================================================================
// ROUTES
// =============================================================================

/// Create lockscreen management routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::get_status))
        .route("/verify", post(handlers::verify_password_handler))
        .route("/set-password", post(handlers::set_password))
        .route("/clear-password", post(handlers::clear_password))
        .route("/settings", post(handlers::update_settings))
}
