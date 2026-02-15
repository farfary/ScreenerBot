//! Trader Module API Routes
//!
//! Endpoints for trader control, stats, manual trading, templates, and previews.

use axum::{
    response::Response,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::webserver::state::AppState;

// Module declarations
mod control;
mod manual;
mod preview;
pub mod types;

// Re-export handler functions for use by the router
use control::{
    force_stop_handler, force_stop_status_handler, get_trader_status, loss_limit_reset_handler,
    loss_limit_resume_handler, loss_limit_status_handler, monitors_status_handler,
    resume_handler, start_trader_handler, stop_trader_handler, toggle_entry_monitor_handler,
    toggle_exit_monitor_handler,
};
use manual::{manual_add_handler, manual_buy_handler, manual_sell_handler, quote_preview_handler};
use preview::{apply_template, get_templates, get_trader_stats, get_trailing_stop_preview};

// ============================================================================
// ROUTES
// ============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(get_trader_status))
        .route("/stats", get(get_trader_stats))
        .route("/preview-trailing-stop", get(get_trailing_stop_preview))
        .route("/templates", get(get_templates))
        .route("/apply-template", post(apply_template))
        .route("/start", post(start_trader_handler))
        .route("/stop", post(stop_trader_handler))
        // Quote preview endpoint
        .route("/quote", get(quote_preview_handler))
        // Manual trading endpoints (for dashboard actions)
        .route("/manual/buy", post(manual_buy_handler))
        .route("/manual/add", post(manual_add_handler))
        .route("/manual/sell", post(manual_sell_handler))
        // Force stop endpoints
        .route("/force-stop", post(force_stop_handler))
        .route("/resume", post(resume_handler))
        .route("/force-stop/status", get(force_stop_status_handler))
        // Monitor control endpoints
        .route("/monitors/status", get(monitors_status_handler))
        .route("/monitors/entry/toggle", post(toggle_entry_monitor_handler))
        .route("/monitors/exit/toggle", post(toggle_exit_monitor_handler))
        // Loss limit endpoints
        .route("/loss-limit/status", get(loss_limit_status_handler))
        .route("/loss-limit/resume", post(loss_limit_resume_handler))
        .route("/loss-limit/reset", post(loss_limit_reset_handler))
}
