use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::with_config;
use crate::trader::{is_trader_running, start_trader, stop_trader_gracefully, TraderControlError};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TraderStatusResponse {
    pub enabled: bool,
    pub running: bool,
}

#[derive(Debug, Serialize)]
pub struct TraderControlResponse {
    pub success: bool,
    pub message: String,
    pub status: TraderStatusResponse,
}

#[derive(Debug, Deserialize)]
pub struct TraderControlRequest {
    pub enabled: bool,
}

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// GET /api/trader/status - Get current trader status
async fn get_trader_status() -> Response {
    let enabled = with_config(|cfg| cfg.trader.enabled);
    let running = is_trader_running();

    let status = TraderStatusResponse { enabled, running };

    success_response(status)
}

/// POST /api/trader/start - Start the trader
async fn start_trader_handler() -> Response {
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
            };

            let response = TraderControlResponse {
                success: true,
                message: "Trader started successfully".to_string(),
                status,
            };

            success_response(response)
        }
        Err(err) => {
            let (status, message) = match err {
                TraderControlError::ConfigUpdate(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update trader config: {}", e),
                ),
                other => (StatusCode::BAD_REQUEST, other.to_string()),
            };

            error_response(status, "Trader Error", &message, None)
        }
    }
}

/// POST /api/trader/stop - Stop the trader
async fn stop_trader_handler() -> Response {
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
            };

            let response = TraderControlResponse {
                success: true,
                message: "Trader stopped successfully".to_string(),
                status,
            };

            success_response(response)
        }
        Err(err) => {
            let (status, message) = match err {
                TraderControlError::ConfigUpdate(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to update trader config: {}", e),
                ),
                TraderControlError::AlreadyStopped => (
                    StatusCode::BAD_REQUEST,
                    "Trader is already stopped".to_string(),
                ),
                TraderControlError::AlreadyRunning => (
                    StatusCode::BAD_REQUEST,
                    "Trader is already running".to_string(),
                ),
            };

            error_response(status, "Trader Error", &message, None)
        }
    }
}

// =============================================================================
// ROUTER
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(get_trader_status))
        .route("/start", post(start_trader_handler))
        .route("/stop", post(stop_trader_handler))
}
