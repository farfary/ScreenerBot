//! Outbound LLM provider API (`/api/llm`).
//!
//! Owns provider discovery, per-provider configuration updates and connectivity
//! tests. Model-scored analysis is `/api/llm-analysis`; the dashboard assistant
//! is `/api/assistant`; tool permissions are `/api/agent-control`.

use axum::{
    routing::{get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod handlers;
pub mod types;

use handlers::{get_config, list_providers, test_provider, update_config, update_provider};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/config", get(get_config))
        .route("/config", patch(update_config))
        .route("/providers", get(list_providers))
        .route("/providers/:provider", patch(update_provider))
        .route("/providers/:provider/test", post(test_provider))
}
