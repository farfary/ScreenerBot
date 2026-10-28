// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Chain access route — reports the install's chain access and switches it between the own RPC and the ScreenerBot service.

mod handlers;
pub mod types;

use axum::routing::get;
use axum::Router;
use std::sync::Arc;

use crate::webserver::state::AppState;

pub use types::*;

/// Create chain access routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/access",
        get(handlers::get_chain_access).post(handlers::switch_chain_access),
    )
}
