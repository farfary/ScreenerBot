// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading route — manages trading engine controls and configuration via the UI.

mod handlers;
pub mod types;

pub use types::*;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create trading routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/config", get(handlers::get_trading_config))
}
