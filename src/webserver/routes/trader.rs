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
// TRAILING STOP PREVIEW TYPES (Phase 2)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TrailingStopPreviewResponse {
    // Position state
    pub position_id: Option<i64>,
    pub symbol: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub peak_price: f64,
    pub current_profit_pct: f64,
    pub unrealized_pnl: f64,
    
    // Trail state with CURRENT settings
    pub trail_active: bool,
    pub trail_activated_at_pct: Option<f64>,
    pub trail_stop_price: Option<f64>,
    pub distance_to_exit_pct: Option<f64>,
    pub estimated_exit_price: f64,
    pub estimated_exit_profit_pct: f64,
    
    // What-if scenarios
    pub what_if_scenarios: Vec<WhatIfScenario>,
}

#[derive(Debug, Serialize)]
pub struct WhatIfScenario {
    pub description: String,
    pub activation_pct: f64,
    pub distance_pct: f64,
    pub trail_active: bool,
    pub exit_price: f64,
    pub exit_profit_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct TrailingStopPreviewQuery {
    pub position_id: Option<i64>,
    pub activation_pct: Option<f64>,
    pub distance_pct: Option<f64>,
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

/// GET /api/trader/preview-trailing-stop - Preview trailing stop for a position
async fn get_trailing_stop_preview(
    axum::extract::Query(query): axum::extract::Query<TrailingStopPreviewQuery>,
) -> Response {
    use crate::pools::get_pool_price;
    
    // Get config values (or use query overrides)
    let (activation_pct, distance_pct) = with_config(|cfg| {
        let act = query.activation_pct.unwrap_or(cfg.positions.trailing_stop_activation_pct);
        let dist = query.distance_pct.unwrap_or(cfg.positions.trailing_stop_distance_pct);
        (act, dist)
    });
    
    // Get position data (or create simulation)
    let (position_id, symbol, entry_price, current_price, peak_price) = if let Some(pos_id) = query.position_id {
        // Try to get real position
        let positions = positions::get_open_positions().await;
        if let Some(pos) = positions.iter().find(|p| p.id == Some(pos_id)) {
            let current = get_pool_price(&pos.mint)
                .map(|pr| pr.price_sol)
                .unwrap_or(pos.entry_price);
            let peak = if pos.price_highest > 0.0 {
                pos.price_highest
            } else {
                current.max(pos.entry_price)
            };
            (
                Some(pos_id),
                pos.symbol.clone(),
                pos.entry_price,
                current,
                peak,
            )
        } else {
            // Position not found, use simulation
            (None, "SIMULATED".to_string(), 0.001, 0.00119, 0.00123)
        }
    } else {
        // No position_id, use simulation
        (None, "SIMULATED".to_string(), 0.001, 0.00119, 0.00123)
    };
    
    // Calculate current profit
    let current_profit_pct = ((current_price - entry_price) / entry_price) * 100.0;
    let peak_profit_pct = ((peak_price - entry_price) / entry_price) * 100.0;
    
    // Calculate trail state
    let trail_active = peak_profit_pct >= activation_pct;
    let trail_activated_at_pct = if trail_active { Some(activation_pct) } else { None };
    let trail_stop_price = if trail_active {
        Some(peak_price * (1.0 - distance_pct / 100.0))
    } else {
        None
    };
    let distance_to_exit_pct = if let Some(stop_price) = trail_stop_price {
        Some(((current_price - stop_price) / current_price) * 100.0)
    } else {
        None
    };
    
    // Estimated exit
    let estimated_exit_price = trail_stop_price.unwrap_or(entry_price);
    let estimated_exit_profit_pct = ((estimated_exit_price - entry_price) / entry_price) * 100.0;
    
    // Calculate unrealized P&L (assuming 0.01 SOL position for simulation)
    let position_size = 0.01;
    let unrealized_pnl = (current_price - entry_price) * (position_size / entry_price);
    
    // Generate what-if scenarios
    let what_if_scenarios = generate_what_if_scenarios(
        entry_price,
        current_price,
        peak_price,
        activation_pct,
        distance_pct,
    );
    
    let preview = TrailingStopPreviewResponse {
        position_id,
        symbol,
        entry_price,
        current_price,
        peak_price,
        current_profit_pct,
        unrealized_pnl,
        trail_active,
        trail_activated_at_pct,
        trail_stop_price,
        distance_to_exit_pct,
        estimated_exit_price,
        estimated_exit_profit_pct,
        what_if_scenarios,
    };
    
    success_response(preview)
}

fn generate_what_if_scenarios(
    entry_price: f64,
    current_price: f64,
    peak_price: f64,
    base_activation: f64,
    base_distance: f64,
) -> Vec<WhatIfScenario> {
    let mut scenarios = Vec::new();
    
    // Helper to calculate scenario
    let calc_scenario = |act: f64, dist: f64| -> WhatIfScenario {
        let peak_profit = ((peak_price - entry_price) / entry_price) * 100.0;
        let trail_active = peak_profit >= act;
        let exit_price = if trail_active {
            peak_price * (1.0 - dist / 100.0)
        } else {
            entry_price
        };
        let exit_profit = ((exit_price - entry_price) / entry_price) * 100.0;
        
        WhatIfScenario {
            description: format!("Activation {}%, Distance {}%", act, dist),
            activation_pct: act,
            distance_pct: dist,
            trail_active,
            exit_price,
            exit_profit_pct: exit_profit,
        }
    };
    
    // Scenario 1: Current settings
    scenarios.push(calc_scenario(base_activation, base_distance));
    
    // Scenario 2: Tighter activation (current - 5%)
    if base_activation > 5.0 {
        scenarios.push(calc_scenario(base_activation - 5.0, base_distance));
    }
    
    // Scenario 3: Looser activation (current + 5%)
    scenarios.push(calc_scenario(base_activation + 5.0, base_distance));
    
    // Scenario 4: Tighter distance (current - 2%)
    if base_distance > 2.0 {
        scenarios.push(calc_scenario(base_activation, base_distance - 2.0));
    }
    
    scenarios
}

// =============================================================================
// PRESET TEMPLATES (Phase 2)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<Template>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trading_style: String,
    pub config: TemplateConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateConfig {
    pub trailing_stop_enabled: bool,
    pub trailing_stop_activation_pct: f64,
    pub trailing_stop_distance_pct: f64,
    pub roi_enabled: bool,
    pub roi_target_pct: f64,
    pub time_override_enabled: bool,
    pub time_override_max_age_hours: f64,
    pub time_override_loss_threshold_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub template_id: String,
}

/// GET /api/trader/templates - List available preset templates
async fn get_templates() -> Response {
    let templates = get_all_templates();
    success_response(TemplateListResponse { templates })
}

/// POST /api/trader/apply-template - Apply a preset template
async fn apply_template(Json(request): Json<ApplyTemplateRequest>) -> Response {
    use crate::config::update_config_section;

    // Get all templates (TODO: DRY this up with get_templates)
    let templates = get_all_templates();

    // Find the requested template
    let template = templates.iter().find(|t| t.id == request.template_id);
    
    if template.is_none() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "TemplateNotFound",
            &format!("Template '{}' not found", request.template_id),
            None,
        );
    }
    
    let template = template.unwrap();
    let cfg = template.config.clone();

    // Update positions config
    let result = update_config_section(
        |config| {
            config.positions.trailing_stop_enabled = cfg.trailing_stop_enabled;
            config.positions.trailing_stop_activation_pct = cfg.trailing_stop_activation_pct;
            config.positions.trailing_stop_distance_pct = cfg.trailing_stop_distance_pct;
        },
        false, // Don't save yet
    );

    if let Err(e) = result {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to update positions config: {}", e),
            None,
        );
    }

    // Update trader config
    let result = update_config_section(
        |config| {
            config.trader.min_profit_threshold_enabled = cfg.roi_enabled;
            config.trader.min_profit_threshold_percent = cfg.roi_target_pct;
            config.trader.time_override_duration_hours = cfg.time_override_max_age_hours;
            config.trader.time_override_loss_threshold_percent = cfg.time_override_loss_threshold_pct;
        },
        true, // Save to disk
    );

    if let Err(e) = result {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ConfigUpdateFailed",
            &format!("Failed to update trader config: {}", e),
            None,
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!("Applied template '{}' ({})", template.name, template.id),
    );

    success_response(serde_json::json!({
        "message": format!("Template '{}' applied successfully", template.name),
        "template": template,
    }))
}

fn get_all_templates() -> Vec<Template> {
    vec![
        Template {
            id: "conservative".to_string(),
            name: "Conservative".to_string(),
            description: "Low risk, secure profits early".to_string(),
            trading_style: "conservative".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 3.0,
                roi_enabled: true,
                roi_target_pct: 10.0,
                time_override_enabled: true,
                time_override_max_age_hours: 72.0,
                time_override_loss_threshold_pct: -20.0,
            },
        },
        Template {
            id: "balanced".to_string(),
            name: "Balanced".to_string(),
            description: "Balanced risk/reward".to_string(),
            trading_style: "balanced".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 10.0,
                trailing_stop_distance_pct: 5.0,
                roi_enabled: true,
                roi_target_pct: 20.0,
                time_override_enabled: true,
                time_override_max_age_hours: 168.0,
                time_override_loss_threshold_pct: -40.0,
            },
        },
        Template {
            id: "aggressive".to_string(),
            name: "Aggressive".to_string(),
            description: "High risk, chase large gains".to_string(),
            trading_style: "aggressive".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 15.0,
                trailing_stop_distance_pct: 7.0,
                roi_enabled: true,
                roi_target_pct: 50.0,
                time_override_enabled: true,
                time_override_max_age_hours: 336.0,
                time_override_loss_threshold_pct: -60.0,
            },
        },
        Template {
            id: "day_trade".to_string(),
            name: "Day Trade".to_string(),
            description: "Quick exits, tight stops".to_string(),
            trading_style: "day_trade".to_string(),
            config: TemplateConfig {
                trailing_stop_enabled: true,
                trailing_stop_activation_pct: 5.0,
                trailing_stop_distance_pct: 2.0,
                roi_enabled: true,
                roi_target_pct: 5.0,
                time_override_enabled: true,
                time_override_max_age_hours: 24.0,
                time_override_loss_threshold_pct: -15.0,
            },
        },
    ]
}

// =============================================================================
// ROUTER
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(get_trader_status))
        .route("/stats", get(get_trader_stats))
        .route("/preview-trailing-stop", get(get_trailing_stop_preview))
        .route("/templates", get(get_templates))
        .route("/apply-template", post(apply_template))
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
