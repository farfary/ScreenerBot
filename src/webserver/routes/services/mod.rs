//! Services route — manages background service controls (start, stop, restart).

pub mod handlers;
pub mod types;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;
pub use handlers::gather_services_overview_snapshot;
pub use types::*;

// ================================================================================================
// Route Handlers
// ================================================================================================

/// Create services management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/services", get(handlers::list_services))
        .route("/services/:name", get(handlers::get_service))
        .route("/services/overview", get(handlers::services_overview))
}
