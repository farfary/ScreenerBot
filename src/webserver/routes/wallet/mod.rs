//! Wallet route — manages wallet operations (add, remove, set main) via the UI.

use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod handlers;
mod types;

pub use types::*;

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/wallet/current", get(handlers::get_wallet_current))
        .route("/wallet/balance", get(handlers::get_wallet_balance)) // Alias for /wallet/current
        .route("/wallet/tokens", get(handlers::get_wallet_tokens))
        .route(
            "/wallet/tokens/refresh",
            post(handlers::refresh_wallet_tokens),
        )
        .route("/wallet/dashboard", post(handlers::get_wallet_dashboard))
        .route(
            "/wallet/dashboard/refresh",
            post(handlers::refresh_wallet_dashboard),
        )
        .route(
            "/wallet/flow-cache",
            get(handlers::get_wallet_flow_cache_stats),
        )
        .route(
            "/wallet/cache-metrics",
            get(handlers::get_wallet_cache_metrics),
        )
}
