//! Trader control and safety endpoints -- transports over `trader::controller`.

use axum::{
    extract::State,
    response::{IntoResponse as _, Response},
    Json,
};
use std::sync::Arc;

use crate::errors::ErrorClass;
use crate::i18n::{ids, UiArg};
use crate::trader::{self, Monitor};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;

use super::types::*;

/// Response for a failed trader operation. The category and HTTP status come
/// from the error value (`http_status`); the message is the catalog entry for
/// the variant with its domain values as arguments, and the error's own text
/// goes to `details`.
pub(super) fn trader_failure(error: &trader::Error) -> Response {
    use trader::Error;
    let code = ApiErrorCode::for_status(error.http_status());
    let text = match error {
        Error::AlreadyRunning => ApiError::new(code, ids::ERRORS_TRADE_ALREADY_RUNNING),
        Error::AlreadyStopped => ApiError::new(code, ids::ERRORS_TRADE_ALREADY_STOPPED),
        Error::ConfigUpdate { .. } => ApiError::new(code, ids::ERRORS_TRADE_CONFIG_UPDATE_FAILED),
        Error::TraderUnavailable => ApiError::new(code, ids::ERRORS_TRADE_TRADER_UNAVAILABLE),
        Error::ForceStopActive => ApiError::new(code, ids::ERRORS_TRADE_FORCE_STOP_ACTIVE),
        Error::TemplateNotFound { template_id } => {
            ApiError::new(code, ids::ERRORS_TRADE_TEMPLATE_NOT_FOUND)
                .text_arg("template", template_id.clone())
        }
        Error::ForceStopped => ApiError::new(code, ids::ERRORS_TRADE_MANUAL_FORCE_STOPPED),
        Error::CoreServicesNotReady { pending } => {
            ApiError::new(code, ids::ERRORS_TRADE_CORE_SERVICES_NOT_READY)
                .text_arg("pending", pending.clone())
        }
        Error::InvalidMint { mint } => {
            ApiError::new(code, ids::ERRORS_TRADE_MINT_INVALID).text_arg("mint", mint.clone())
        }
        Error::Blacklisted { mint } => {
            ApiError::new(code, ids::ERRORS_TRADE_BLACKLISTED).text_arg("mint", mint.clone())
        }
        Error::InvalidSlippage {
            slippage_pct,
            maximum_pct,
        } => ApiError::new(code, ids::ERRORS_TRADE_SLIPPAGE_INVALID)
            .arg("slippage", UiArg::Number(*slippage_pct))
            .arg("maximum", UiArg::Number(*maximum_pct)),
        Error::InvalidPercentage { percentage } => {
            ApiError::new(code, ids::ERRORS_TRADE_PERCENTAGE_INVALID)
                .arg("percentage", UiArg::Number(*percentage))
        }
        Error::Database(_) | Error::Positions(_) | Error::Transactions(_) => {
            ApiError::new(code, ids::ERRORS_TRADE_STORAGE_FAILED)
        }
        Error::CopyTaskNotFound { .. }
        | Error::CopyHoldingNotFound { .. }
        | Error::CopyTaskDecode { .. }
        | Error::CopySerialize { .. }
        | Error::CopyReconciliation { .. }
        | Error::CopyValidation { .. }
        | Error::CopyDatabaseUnavailable { .. }
        | Error::CopyTaskRejected { .. }
        | Error::CopyTaskLimit { .. }
        | Error::CopyWatchRejected { .. }
        | Error::CopyTaskLive { .. }
        | Error::CopyTaskOwnsPositions { .. }
        | Error::CopyLiveUnavailable { .. } => ApiError::new(code, ids::ERRORS_COPY_REQUEST_FAILED),
        Error::ManualTradeRecord { .. } => ApiError::new(code, ids::ERRORS_TRADE_RECORD_FAILED),
        Error::NoOpenPosition { mint } => {
            ApiError::new(code, ids::ERRORS_TRADE_NO_OPEN_POSITION).text_arg("mint", mint.clone())
        }
        Error::InvalidSolAmount { amount_sol, .. } => {
            ApiError::new(code, ids::ERRORS_TRADE_SIZE_INVALID)
                .arg("amount", UiArg::Sol(amount_sol.to_string()))
        }
        Error::InvalidManagement { management, .. } => {
            ApiError::new(code, ids::ERRORS_TRADE_MANAGEMENT_INVALID)
                .text_arg("management", management.clone())
        }
        Error::StrategyEvaluation { mint, .. } => {
            ApiError::new(code, ids::ERRORS_TRADE_STRATEGY_EVALUATION_FAILED)
                .text_arg("mint", mint.clone())
        }
        Error::TokenDataMissing { mint } => {
            ApiError::new(code, ids::ERRORS_TRADE_TOKEN_DATA_MISSING).text_arg("mint", mint.clone())
        }
        Error::UnhealthyEndpoints { .. } => {
            ApiError::new(code, ids::ERRORS_TRADE_ENDPOINTS_UNHEALTHY)
        }
        Error::Dependency { dependency, .. } => {
            ApiError::new(code, ids::ERRORS_TRADE_DEPENDENCY_FAILED)
                .text_arg("dependency", *dependency)
        }
    };
    text.details(error.to_string()).into_response()
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
            status,
        }),
        Err(error) => trader_failure(&error),
    }
}

/// POST /api/trader/stop - Stop the trader
pub async fn stop_trader_handler() -> Response {
    match trader::stop_trader_checked().await {
        Ok(status) => success_response(TraderControlResponse {
            success: true,
            status,
        }),
        Err(error) => trader_failure(&error),
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
        Err(error) => trader_failure(&error),
    }
}

/// POST /api/trader/resume - Clear force stop state
pub async fn resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    trader::clear_force_stop(None).await;
    success_response(serde_json::json!({
        "resumed": true
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
        Err(error) => trader_failure(&error),
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
