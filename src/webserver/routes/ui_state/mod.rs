//! UI state route — persists and retrieves user interface state (collapsed panels, etc.).

mod handlers;
mod types;

pub use types::*;

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create UI state persistence routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ui-state/all", get(handlers::load_all_state))
        .route("/ui-state/save", post(handlers::save_state))
        .route("/ui-state/batch-save", post(handlers::batch_save_state))
        .route("/ui-state/load", post(handlers::load_state))
        .route("/ui-state/remove", post(handlers::remove_state))
        .route("/ui-state/clear", post(handlers::clear_state))
}
