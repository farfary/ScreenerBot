//! Header route — renders the dashboard header bar with connection and system info.

mod handlers;
mod types;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

/// Create header metrics routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/header/metrics", get(handlers::get_header_metrics))
}
