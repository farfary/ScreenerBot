use axum::{ extract::State, http::StatusCode, response::Response, routing::{ get, post }, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::config::{ update_config_section, with_config };
use crate::trader::{ is_trader_running, start_trader, stop_trader_gracefully };
use crate::webserver::state::AppState;
use crate::webserver::utils::{ error_response, success_response };

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
    // Check if already enabled in config
    let enabled = with_config(|cfg| cfg.trader.enabled);

    if !enabled {
        // Update config to enable trader
        if
            let Err(e) = update_config_section(|cfg| {
                cfg.trader.enabled = true;
            }, true)
        {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Config Error",
                &format!("Failed to update config: {}", e),
                None
            );
        }
    }

    // Start the trader
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
        Err(e) =>
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Trader Error",
                &format!("Failed to start trader: {}", e),
                None
            ),
    }
}

/// POST /api/trader/stop - Stop the trader
async fn stop_trader_handler() -> Response {
    // Update config to disable trader
    if
        let Err(e) = update_config_section(|cfg| {
            cfg.trader.enabled = false;
        }, true)
    {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Config Error",
            &format!("Failed to update config: {}", e),
            None
        );
    }

    // Stop the trader
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
        Err(e) =>
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Trader Error",
                &format!("Failed to stop trader: {}", e),
                None
            ),
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
