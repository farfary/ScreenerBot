// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet route — manages wallet operations (add, remove, set main) via the UI.

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::database::catalog;
use crate::webserver::middleware::store_gate;
use crate::webserver::state::AppState;

mod handlers;
mod types;

pub use types::*;

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/wallet/qr/{address}", get(handlers::get_wallet_qr))
        .route("/wallet/current", get(handlers::get_wallet_current))
        .route("/wallet/balance", get(handlers::get_wallet_balance)) // Alias for /wallet/current
        .route("/wallet/tokens", get(handlers::get_wallet_tokens))
        .route(
            "/wallet/buy-account-rent/{mint}",
            get(handlers::get_buy_account_rent),
        )
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
        .route_layer(from_fn_with_state(&catalog::WALLET, store_gate))
}
