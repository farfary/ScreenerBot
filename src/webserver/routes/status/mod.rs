//! Status route — reports overall bot health and component readiness.

mod handlers;
pub mod types;

pub use types::*;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create status routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/status", get(handlers::system_status))
        .route("/status/services", get(handlers::service_status))
        .route("/status/metrics", get(handlers::system_metrics))
}
