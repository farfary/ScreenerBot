// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard page routes (home, overview, and related utilities).
use axum::{routing::get, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod calendar;
mod home;
mod overview;
mod status_cards;
mod types;
mod utils;

pub use calendar::get_portfolio_calendar;
pub use home::get_home_dashboard;
pub use overview::get_dashboard_overview;
pub use status_cards::{auto_trader_card, copy_trading_card};
pub use types::*;

/// Create dashboard routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dashboard/overview", get(get_dashboard_overview))
        .route("/dashboard/home", get(get_home_dashboard))
        .route("/dashboard/portfolio-calendar", get(get_portfolio_calendar))
}
