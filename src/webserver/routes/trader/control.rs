//! Trader control and safety endpoints -- transports over `trader::controller`.

use axum::{extract::State, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::errors::ErrorClass;
use crate::trader::{self, Monitor};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

fn trader_error(code: &str, error: &trader::Error) -> Response {
    let status =
        StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let code = match error {
        trader::Error::TraderUnavailable => "TraderUnavailable",
        trader::Error::ForceStopActive => "ForceStopActive",
        trader::Error::ConfigUpdate { .. } => "ConfigUpdateFailed",
        _ => code,
    };
    error_response(status, code, &error.to_string(), None)
}

// =============================================================================
// TRADER CONTROL HANDLERS
// =============================================================================

/// GET /api/trader/status - Get current trader status
pub async fn get_trader_status() -> Response {
    success_response(trader::trader_status())
}

/// POST /api/trader/start - Start the trader
pub async fn start_trader_handler() -> Response {
    match trader::start_trader_checked().await {
        Ok(status) => success_response(TraderControlResponse {
            success: true,
            message: "Trader started successfully".to_owned(),
            status,
        }),
        Err(error) => trader_error("Trader Error", &error),
    }
}

/// POST /api/trader/stop - Stop the trader
pub async fn stop_trader_handler() -> Response {
    match trader::stop_trader_checked().await {
        Ok(status) => success_response(TraderControlResponse {
            success: true,
            message: "Trader stopped successfully".to_owned(),
            status,
        }),
        Err(error) => trader_error("Trader Error", &error),
    }
}

// =============================================================================
// FORCE STOP HANDLERS
// =============================================================================

pub async fn force_stop_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ForceStopRequest>,
) -> Response {
    let reason = payload
        .reason
        .unwrap_or_else(|| "Manual force stop".to_owned());
    match trader::engage_force_stop(&reason).await {
        Ok(status) => success_response(status),
        Err(error) => trader_error("ConfigUpdateFailed", &error),
    }
}

/// POST /api/trader/resume - Clear force stop state
pub async fn resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    trader::clear_force_stop(None).await;
    success_response(serde_json::json!({
        "resumed": true,
        "message": "Force stop cleared. Use Start Trading to resume."
    }))
}

/// GET /api/trader/force-stop/status - Get force stop status
pub async fn force_stop_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    success_response(crate::global::get_force_stop_status())
}

// =============================================================================
// MONITOR CONTROL HANDLERS
// =============================================================================

/// GET /api/trader/monitors/status - Get monitor status
pub async fn monitors_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    success_response(trader::monitors_status())
}

async fn toggle_monitor(monitor: Monitor, enabled: bool, field: &str) -> Response {
    match trader::set_monitor_enabled(monitor, enabled) {
        Ok(()) => success_response(serde_json::json!({ field: enabled })),
        Err(error) => trader_error("ConfigUpdateFailed", &error),
    }
}

/// POST /api/trader/monitors/entry/toggle - Toggle entry monitor
pub async fn toggle_entry_monitor_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ToggleMonitorRequest>,
) -> Response {
    toggle_monitor(Monitor::Entry, payload.enabled, "entry_monitor_enabled").await
}

/// POST /api/trader/monitors/exit/toggle - Toggle exit monitor
pub async fn toggle_exit_monitor_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ToggleMonitorRequest>,
) -> Response {
    toggle_monitor(Monitor::Exit, payload.enabled, "exit_monitor_enabled").await
}

// =============================================================================
// LOSS LIMIT HANDLERS
// =============================================================================

/// GET /api/trader/loss-limit/status - Get loss limit status
pub async fn loss_limit_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    success_response(trader::loss_limit_snapshot())
}

/// POST /api/trader/loss-limit/resume - Resume trading after loss limit
pub async fn loss_limit_resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    trader::safety::loss_limit::resume_from_loss_limit();
    success_response(serde_json::json!({ "resumed": true }))
}

/// POST /api/trader/loss-limit/reset - Reset loss limit state
pub async fn loss_limit_reset_handler(State(_state): State<Arc<AppState>>) -> Response {
    trader::safety::loss_limit::reset_loss_limit_state();
    success_response(serde_json::json!({ "reset": true }))
}
