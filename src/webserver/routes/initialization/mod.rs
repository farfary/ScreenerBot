// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Initialization route — reports bot startup progress and initialization status.

mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod types;
pub(in crate::webserver::routes) mod validation;

use crate::webserver::middleware::setup_only_gate;
use crate::webserver::state::AppState;
use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub use types::*;

/// Create initialization routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::initialization_status))
        .route("/validate", post(handlers::validate_credentials))
        .route("/wallet/generate", post(handlers::generate_wallet))
        .route(
            "/complete",
            post(handlers::complete_initialization).route_layer(from_fn(setup_only_gate)),
        )
        .route(
            "/explore",
            post(handlers::enter_explore_mode).route_layer(from_fn(setup_only_gate)),
        )
        .route("/progress", get(handlers::initialization_progress))
        .route("/onboarding/complete", post(handlers::complete_onboarding))
}
