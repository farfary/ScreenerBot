//! Features API route
//!
//! Exposes feature availability to the dashboard.

mod handlers;
pub mod types;

pub use types::*;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create features routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::get_all_features))
        .route("/tool/{tool_id}", get(handlers::check_tool))
        .route(
            "/trading/{feature_id}",
            get(handlers::check_trading_feature),
        )
}
