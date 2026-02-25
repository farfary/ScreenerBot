//! Connectivity route — reports RPC and WebSocket connection status to the dashboard.

mod handlers;
pub mod types;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

/// Create connectivity routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::get_connectivity_status))
        .route("/status/:endpoint", get(handlers::get_endpoint_status))
}
