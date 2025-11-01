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
use crate::{
    global::{are_core_services_ready, get_pending_services},
    logger::{self, LogTag},
    positions, trader,
};
use axum::Json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

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
// MANUAL TRADING REQUEST/RESPONSE TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
struct ManualBuyRequest {
    mint: String,
    #[serde(default)]
    size_sol: Option<f64>,
    #[serde(default)]
    force: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ManualAddRequest {
    mint: String,
    #[serde(default)]
    size_sol: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ManualSellRequest {
    mint: String,
    #[serde(default)]
    percentage: Option<f64>,
    #[serde(default)]
    close_all: Option<bool>,
    #[serde(default)]
    force: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ManualTradeSuccess {
    success: bool,
    mint: String,
    signature: Option<String>,
    effective_price_sol: Option<f64>,
    size_sol: Option<f64>,
    position_id: Option<String>,
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct TraderStatsResponse {
    open_positions_count: usize,
    locked_sol: f64,
    win_rate_pct: f64,
    total_trades: usize,
    avg_hold_time_hours: f64,
    best_trade_pct: f64,
    exit_breakdown: Vec<ExitBreakdown>,
}

#[derive(Debug, Serialize)]
struct ExitBreakdown {
    exit_type: String,
    count: usize,
    avg_profit_pct: f64,
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

/// GET /api/trader/stats - Get trader performance statistics
async fn get_trader_stats() -> Response {
    // Get open positions
    let open_positions = positions::get_open_positions().await;
    let open_positions_count = open_positions.len();
    
    // Calculate locked SOL (use total_size_sol for DCA support)
    let locked_sol: f64 = open_positions.iter().map(|p| p.total_size_sol).sum();
    
    // Query closed positions from database for statistics
    let closed_positions = {
        let db_ref = positions::db::get_positions_database().await.ok();
        if let Some(db_arc) = db_ref {
            let db_guard = db_arc.lock().await;
            if let Some(db) = db_guard.as_ref() {
                db.get_closed_positions().await.unwrap_or_else(|_| Vec::new())
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };
    
    // Filter to last 30 days
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    let recent_closed: Vec<_> = closed_positions
        .into_iter()
        .filter(|p| {
            if let Some(exit_time) = p.exit_time {
                exit_time >= thirty_days_ago
            } else {
                false
            }
        })
        .collect();
    
    let total_trades = recent_closed.len();
    
    // Calculate win rate (using pnl_percent for closed positions)
    let winners = recent_closed.iter().filter(|p| p.pnl_percent.unwrap_or(0.0) > 0.0).count();
    let win_rate_pct = if total_trades > 0 {
        (winners as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };
    
    // Calculate average hold time
    let total_hold_seconds: i64 = recent_closed
        .iter()
        .filter_map(|p| {
            p.exit_time.and_then(|exit_time| {
                Some((exit_time - p.entry_time).num_seconds())
            })
        })
        .sum();
    
    let avg_hold_time_hours = if total_trades > 0 {
        (total_hold_seconds as f64 / total_trades as f64) / 3600.0
    } else {
        0.0
    };
    
    // Find best trade (using pnl_percent)
    let best_trade_pct = recent_closed
        .iter()
        .filter_map(|p| p.pnl_percent)
        .fold(f64::NEG_INFINITY, f64::max);
    
    let best_trade_pct = if best_trade_pct == f64::NEG_INFINITY {
        0.0
    } else {
        best_trade_pct
    };
    
    // Build exit breakdown from closed_reason in closed positions
    use std::collections::HashMap;
    let mut exit_stats: HashMap<String, (usize, Vec<f64>)> = HashMap::new();
    
    for pos in &recent_closed {
        let exit_type = pos.closed_reason.clone().unwrap_or_else(|| "unknown".to_string());
        let entry = exit_stats.entry(exit_type).or_insert((0, Vec::new()));
        entry.0 += 1;
        if let Some(pnl_pct) = pos.pnl_percent {
            entry.1.push(pnl_pct);
        }
    }
    
    let mut exit_breakdown = Vec::new();
    for (exit_type, (count, profits)) in exit_stats {
        let avg_profit_pct = if !profits.is_empty() {
            profits.iter().sum::<f64>() / profits.len() as f64
        } else {
            0.0
        };
        
        exit_breakdown.push(ExitBreakdown {
            exit_type,
            count,
            avg_profit_pct,
        });
    }
    
    // Sort by count descending
    exit_breakdown.sort_by(|a, b| b.count.cmp(&a.count));
    
    let stats = TraderStatsResponse {
        open_positions_count,
        locked_sol,
        win_rate_pct,
        total_trades,
        avg_hold_time_hours,
        best_trade_pct,
        exit_breakdown,
    };
    
    success_response(stats)
}

// =============================================================================
// ROUTER
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(get_trader_status))
        .route("/stats", get(get_trader_stats))
        .route("/start", post(start_trader_handler))
        .route("/stop", post(stop_trader_handler))
        // Manual trading endpoints (for dashboard actions)
        .route("/manual/buy", post(manual_buy_handler))
        .route("/manual/add", post(manual_add_handler))
        .route("/manual/sell", post(manual_sell_handler))
}

// =============================================================================
// MANUAL TRADING HANDLERS
// =============================================================================

fn validate_mint(mint: &str) -> Result<(), Response> {
    if Pubkey::from_str(mint).is_err() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            "Invalid token mint address",
            Some("Mint must be a valid base58 pubkey"),
        ));
    }
    Ok(())
}

fn ensure_ready() -> Result<(), Response> {
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        return Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={}", pending)),
        ));
    }
    Ok(())
}

async fn manual_buy_handler(Json(req): Json<ManualBuyRequest>) -> Response {
    if let Err(resp) = ensure_ready() {
        return resp;
    }
    if let Err(resp) = validate_mint(&req.mint) {
        return resp;
    }

    // Server-side blacklist enforcement with optional force override
    if let Some(db) = crate::tokens::database::get_global_database() {
        if let Ok(true) = tokio::task::spawn_blocking({
            let db = db.clone();
            let mint = req.mint.clone();
            move || db.is_blacklisted(&mint)
        })
        .await
        .unwrap_or(Ok(false))
        {
            if !req.force.unwrap_or(false) {
                return error_response(
                    StatusCode::FORBIDDEN,
                    "Blacklisted",
                    "Token is blacklisted; set force=true to override",
                    None,
                );
            }
        }
    }

    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => with_config(|cfg| cfg.trader.trade_size_sol),
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} size_sol={} force={}",
            req.mint,
            size,
            req.force.unwrap_or(false)
        ),
    );

    // Use standard manual_buy; force mode is not exposed publicly via manual module
    let result = crate::trader::manual::manual_buy(&req.mint, size).await;

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualBuyFailed",
                    tr.error.as_deref().unwrap_or("Manual buy failed"),
                    None,
                );
            }
            let resp = ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: tr.tx_signature,
                effective_price_sol: tr.executed_price_sol,
                size_sol: tr.executed_size_sol,
                position_id: tr.position_id,
                message: "Manual buy executed".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualBuyError",
            &e,
            None,
        ),
    }
}

async fn manual_add_handler(Json(req): Json<ManualAddRequest>) -> Response {
    // Enforce blacklist for add (no override)
    if let Some(db) = crate::tokens::database::get_global_database() {
        if let Ok(true) = tokio::task::spawn_blocking({
            let db = db.clone();
            let mint = req.mint.clone();
            move || db.is_blacklisted(&mint)
        })
        .await
        .unwrap_or(Ok(false))
        {
            return error_response(
                StatusCode::FORBIDDEN,
                "Blacklisted",
                "Token is blacklisted; cannot add to position",
                None,
            );
        }
    }
    if let Err(resp) = ensure_ready() {
        return resp;
    }
    if let Err(resp) = validate_mint(&req.mint) {
        return resp;
    }

    // Ensure there's an open position for this mint
    let has_open = positions::is_open_position(&req.mint).await;
    if !has_open {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NoOpenPosition",
            "Cannot add to position: no open position for this token",
            None,
        );
    }

    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => with_config(|cfg| cfg.trader.trade_size_sol * 0.5), // default add = 50% size
    };

    logger::info(
        LogTag::Webserver,
        &format!("mint={} size_sol={}", req.mint, size),
    );

    match positions::add_to_position(&req.mint, size).await {
        Ok(signature) => success_response(ManualTradeSuccess {
            success: true,
            mint: req.mint,
            signature: Some(signature),
            effective_price_sol: None,
            size_sol: Some(size),
            position_id: None,
            message: "Added to position".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }),
        Err(e) => error_response(StatusCode::BAD_REQUEST, "ManualAddFailed", &e, None),
    }
}

async fn manual_sell_handler(Json(req): Json<ManualSellRequest>) -> Response {
    if let Err(resp) = ensure_ready() {
        return resp;
    }
    if let Err(resp) = validate_mint(&req.mint) {
        return resp;
    }

    let is_open = positions::is_open_position(&req.mint).await;
    if !is_open {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NoOpenPosition",
            "Cannot sell: no open position for this token",
            None,
        );
    }

    let close_all = req.close_all.unwrap_or(false);
    let pct = req
        .percentage
        .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct));

    // Normalize and validate percentage
    let pct = if close_all { 100.0 } else { pct };
    if !pct.is_finite() || pct <= 0.0 || pct > 100.0 {
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidPercentage",
            "percentage must be in (0, 100]",
            None,
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!("mint={} percentage={}", req.mint, pct),
    );

    let exit_reason = if req.force.unwrap_or(false) {
        "ForceSell"
    } else {
        "ManualExit"
    }
    .to_string();

    // Full vs partial exit
    if (pct - 100.0).abs() < f64::EPSILON {
        match positions::close_position_direct(&req.mint, exit_reason).await {
            Ok(signature) => success_response(ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: Some(signature),
                effective_price_sol: None,
                size_sol: None,
                position_id: None,
                message: "Full position closed".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => error_response(StatusCode::BAD_REQUEST, "ManualSellFailed", &e, None),
        }
    } else {
        match positions::partial_close_position(&req.mint, pct, &exit_reason).await {
            Ok(signature) => success_response(ManualTradeSuccess {
                success: true,
                mint: req.mint,
                signature: Some(signature),
                effective_price_sol: None,
                size_sol: None,
                position_id: None,
                message: format!("Partial sell executed ({}%)", pct),
                timestamp: chrono::Utc::now().to_rfc3339(),
            }),
            Err(e) => error_response(StatusCode::BAD_REQUEST, "ManualSellFailed", &e, None),
        }
    }
}
