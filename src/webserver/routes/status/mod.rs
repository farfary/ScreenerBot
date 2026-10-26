// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Status route — reports overall bot health and component readiness.

mod handlers;
#[cfg(test)]
mod handlers_tests;
pub mod types;

pub use types::*;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// The loopback-only restart check, relative to `/api`. Exempt from the dashboard
/// password and the GUI token: it answers one boolean, and only to this machine.
pub const RESTART_SAFETY_PATH: &str = "/health/restart-safety";

/// Create status routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route(RESTART_SAFETY_PATH, get(handlers::restart_safety))
        .route("/status", get(handlers::system_status))
        .route("/status/services", get(handlers::service_status))
        .route("/status/metrics", get(handlers::system_metrics))
}
