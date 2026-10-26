// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet API routes: the wallet list, summary and main wallet, one wallet's
//! record and its private-key export, the trading-wallet replacement and its
//! backups, and the watched wallets.

use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::middleware::full_setup_gate;
use crate::webserver::state::AppState;

pub mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod types;
pub mod watch;

// Re-export types for external consumers
pub use types::*;

/// Create wallet routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::list_wallets))
        .route("/summary", get(handlers::get_summary))
        .route("/main", get(handlers::get_main_wallet))
        .route("/replace", post(handlers::replace_wallet))
        .route("/backups", get(handlers::list_wallet_backups))
        .route("/{id}", get(handlers::get_wallet))
        .route("/{id}/export", post(handlers::export_wallet))
        .nest("/watch", watch::routes())
        .route_layer(from_fn(full_setup_gate))
}
