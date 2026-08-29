//! Shared agent-control API (`/api/agent-control`).
//!
//! Owns the capability registry's tool list and the per-category permission
//! policy. Connection/pairing endpoints are intentionally not added yet.

use axum::{
    routing::{get, patch},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod handlers;

use handlers::{get_permissions, list_tools, update_permissions};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tools", get(list_tools))
        .route("/permissions", get(get_permissions))
        .route("/permissions", patch(update_permissions))
}
