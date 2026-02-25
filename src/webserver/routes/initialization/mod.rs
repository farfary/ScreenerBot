//! Initialization route — reports bot startup progress and initialization status.

mod handlers;
pub mod types;

use crate::webserver::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub use types::*;

/// Create initialization routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::initialization_status))
        .route("/validate", post(handlers::validate_credentials))
        .route("/complete", post(handlers::complete_initialization))
        .route("/progress", get(handlers::initialization_progress))
        .route(
            "/onboarding/complete",
            post(handlers::complete_onboarding),
        )
}
