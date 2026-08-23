//! Trader control and safety endpoints

use axum::{extract::State, http::StatusCode, response::Response, Json};
use std::sync::Arc;

use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::trader::{self, is_trader_running, start_trader, stop_trader_gracefully};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

// =============================================================================
// TRADER CONTROL HANDLERS
// =============================================================================

/// GET /api/trader/status - Get current trader status
pub async fn get_trader_status() -> Response {
    let available =
        crate::global::is_initialization_complete() && !crate::global::is_explore_mode();
    let enabled = available && with_config(|cfg| cfg.trader.enabled);
    let running = available && is_trader_running();

    let status = TraderStatusResponse {
        enabled,
        running,
        available,
        unavailable_reason: (!available)
            .then_some("Complete wallet and RPC setup to use Auto Trader"),
    };

    success_response(status)
}

/// POST /api/trader/start - Start the trader
pub async fn start_trader_handler() -> Response {
    if crate::global::is_explore_mode() || !crate::global::is_initialization_complete() {
        return error_response(
            StatusCode::CONFLICT,
            "TraderUnavailable",
            "Complete wallet and RPC setup before enabling Auto Trader",
            None,
        );
    }

    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::CONFLICT,
            "ForceStopActive",
            "Clear the emergency stop before enabling Auto Trader",
            None,
        );
    }

    if is_trader_running() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Trader Error",
            "Trader is already running",
            None,
        );
    }

    match start_trader().await {
        Ok(_) => {
            let status = TraderStatusResponse {
                enabled: true,
                running: is_trader_running(),
                available: true,
                unavailable_reason: None,
            };

            let response = TraderControlResponse {
                success: true,
                message: "Trader started successfully".to_owned(),
                status,
            };

            success_response(response)
        }
        Err(err) => {
            let (status, message) = match err {
                trader::Error::ConfigUpdate { detail } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update trader config: {detail}"),
                ),
                other => (StatusCode::BAD_REQUEST, other.to_string()),
            };

            error_response(status, "Trader Error", &message, None)
        }
    }
}

/// POST /api/trader/stop - Stop the trader
pub async fn stop_trader_handler() -> Response {
    if !is_trader_running() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "Trader Error",
            "Trader is already stopped",
            None,
        );
    }

    match stop_trader_gracefully().await {
        Ok(_) => {
            let status = TraderStatusResponse {
                enabled: false,
                running: is_trader_running(),
                available: true,
                unavailable_reason: None,
            };

            let response = TraderControlResponse {
                success: true,
                message: "Trader stopped successfully".to_owned(),
                status,
            };

            success_response(response)
        }
        Err(err) => {
            let (status, message) = match err {
                trader::Error::ConfigUpdate { detail } => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update trader config: {detail}"),
                ),
                trader::Error::AlreadyStopped => (
                    StatusCode::BAD_REQUEST,
                    "Trader is already stopped".to_owned(),
                ),
                trader::Error::AlreadyRunning => (
                    StatusCode::BAD_REQUEST,
                    "Trader is already running".to_owned(),
                ),
                other => (StatusCode::BAD_REQUEST, other.to_string()),
            };

            error_response(status, "Trader Error", &message, None)
        }
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
    crate::global::set_force_stopped(true, Some(&reason));

    // Also disable trader in config to ensure it stays stopped
    if let Err(e) = update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Force stop activated but config update failed: {e}"),
            None,
        );
    }

    logger::warning(LogTag::Trader, &format!("FORCE STOP activated: {reason}"));
    success_response(crate::global::get_force_stop_status())
}

/// POST /api/trader/resume - Clear force stop state
pub async fn resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    crate::global::set_force_stopped(false, None);

    // Note: Does NOT automatically enable trader - user must start explicitly
    logger::info(
        LogTag::Trader,
        "Force stop cleared - trading can be resumed",
    );
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
    use crate::trader::config;

    let available =
        crate::global::is_initialization_complete() && !crate::global::is_explore_mode();

    success_response(serde_json::json!({
        "entry_monitor": {
            "enabled": config::is_entry_monitor_enabled_standalone(),
            "running": available && config::is_entry_monitor_enabled(),
        },
        "exit_monitor": {
            "enabled": config::is_exit_monitor_enabled_standalone(),
            "running": available && config::is_exit_monitor_enabled(),
        },
        "master_enabled": available && config::is_trader_enabled(),
        "force_stopped": crate::global::is_force_stopped(),
        "available": available,
    }))
}

/// POST /api/trader/monitors/entry/toggle - Toggle entry monitor
pub async fn toggle_entry_monitor_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ToggleMonitorRequest>,
) -> Response {
    if let Err(e) = update_config_section(
        |cfg| {
            cfg.trader.entry_monitor_enabled = payload.enabled;
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to toggle entry monitor: {e}"),
            None,
        );
    }

    let status = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };
    logger::info(LogTag::Trader, &format!("Entry monitor {status}"));
    success_response(serde_json::json!({ "entry_monitor_enabled": payload.enabled }))
}

/// POST /api/trader/monitors/exit/toggle - Toggle exit monitor
pub async fn toggle_exit_monitor_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ToggleMonitorRequest>,
) -> Response {
    if let Err(e) = update_config_section(
        |cfg| {
            cfg.trader.exit_monitor_enabled = payload.enabled;
        },
        true,
    ) {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to toggle exit monitor: {e}"),
            None,
        );
    }

    let status = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };
    logger::info(LogTag::Trader, &format!("Exit monitor {status}"));
    success_response(serde_json::json!({ "exit_monitor_enabled": payload.enabled }))
}

// =============================================================================
// LOSS LIMIT HANDLERS
// =============================================================================

/// GET /api/trader/loss-limit/status - Get loss limit status
pub async fn loss_limit_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::config;
    use crate::trader::safety::loss_limit;

    let status = loss_limit::get_loss_limit_status();
    let limit = config::get_loss_limit_sol();
    let enabled = config::is_loss_limit_enabled();

    let progress_percent = if limit > 0.0 {
        (status.cumulative_loss_sol / limit * 100.0).min(100.0)
    } else {
        0.0
    };

    success_response(serde_json::json!({
        "enabled": enabled,
        "limit_sol": limit,
        "current_loss_sol": status.cumulative_loss_sol,
        "is_limited": status.is_limited,
        "limited_at": status.limited_at,
        "period_start": status.period_start,
        "period_remaining_secs": status.period_remaining_secs,
        "progress_percent": progress_percent,
    }))
}

/// POST /api/trader/loss-limit/resume - Resume trading after loss limit
pub async fn loss_limit_resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::safety::loss_limit;

    loss_limit::resume_from_loss_limit();
    success_response(serde_json::json!({ "resumed": true }))
}

/// POST /api/trader/loss-limit/reset - Reset loss limit state
pub async fn loss_limit_reset_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::safety::loss_limit;

    loss_limit::reset_loss_limit_state();
    success_response(serde_json::json!({ "reset": true }))
}
