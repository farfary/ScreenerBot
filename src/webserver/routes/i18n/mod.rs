// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Localization routes — the locale registry API and the dashboard message catalog.

pub mod handlers;
mod types;

use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

/// Create locale registry routes (mounted under `/api`).
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/i18n/locales", get(handlers::get_locales))
}
