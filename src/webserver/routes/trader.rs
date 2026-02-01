use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::{update_config_section, with_config};
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
pub struct TraderStatsResponse {
    pub open_positions_count: usize,
    pub locked_sol: f64,
    pub win_rate_pct: f64,
    pub total_trades: usize,
    pub avg_hold_time_hours: f64,
    pub best_trade_pct: f64,
    pub exit_breakdown: Vec<ExitBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct ExitBreakdown {
    pub exit_type: String,
    pub count: usize,
    pub avg_profit_pct: f64,
}

// =============================================================================
// FORCE STOP / MONITOR CONTROL / LOSS LIMIT TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
struct ForceStopRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToggleMonitorRequest {
    enabled: bool,
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
// QUOTE PREVIEW TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct QuotePreviewRequest {
    pub mint: String,
    #[serde(default)]
    pub amount_sol: Option<f64>, // For buy: SOL amount to spend
    #[serde(default)]
    pub amount_tokens: Option<f64>, // For sell: token amount to sell
    #[serde(default)]
    pub direction: String, // "buy" or "sell", defaults to "buy"
}

#[derive(Debug, Serialize)]
pub struct QuotePreviewResponse {
    pub success: bool,
    pub router: String,
    pub direction: String,
    // For buy: input_sol is SOL spent, output is tokens received
    // For sell: input is tokens sold, output_sol is SOL received
    pub input_amount: f64,
    pub input_formatted: String,
    pub output_amount: f64,
    pub output_formatted: String,
    pub price_per_token_sol: f64,
    pub price_impact_pct: f64,
    pub platform_fee_pct: f64,
    pub platform_fee_sol: f64,
    pub network_fee_sol: f64,
    pub route: String,
    pub slippage_bps: u16,
    pub expires_in_secs: u64,
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
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return crate::webserver::utils::success_response(
            crate::webserver::demo::get_demo_trader_stats(),
        );
    }

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
                db.get_closed_positions()
                    .await
                    .unwrap_or_else(|_| Vec::new())
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
    let winners = recent_closed
        .iter()
        .filter(|p| p.pnl_percent.unwrap_or(0.0) > 0.0)
        .count();
    let win_rate_pct = if total_trades > 0 {
        (winners as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    // Calculate average hold time
    let total_hold_seconds: i64 = recent_closed
        .iter()
        .filter_map(|p| {
            p.exit_time
                .and_then(|exit_time| Some((exit_time - p.entry_time).num_seconds()))
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
        let exit_type = pos
            .closed_reason
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
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
        let act = query
            .activation_pct
            .unwrap_or(cfg.positions.trailing_stop_activation_pct);
        let dist = query
            .distance_pct
            .unwrap_or(cfg.positions.trailing_stop_distance_pct);
        (act, dist)
    });

    // Get position data (or create simulation)
    let (position_id, symbol, entry_price, current_price, peak_price) =
        if let Some(pos_id) = query.position_id {
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
    let trail_activated_at_pct = if trail_active {
        Some(activation_pct)
    } else {
        None
    };
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
    pub roi_exit_enabled: bool,
    pub roi_target_pct: f64,
    pub time_override_enabled: bool,
    pub time_override_duration: f64,
    pub time_override_unit: String,
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
            config.trader.roi_exit_enabled = cfg.roi_exit_enabled;
            config.trader.roi_target_percent = cfg.roi_target_pct;

            config.trader.time_override_enabled = cfg.time_override_enabled;
            config.trader.time_override_duration = cfg.time_override_duration;
            config.trader.time_override_unit = cfg.time_override_unit.clone();
            config.trader.time_override_loss_threshold_percent =
                cfg.time_override_loss_threshold_pct;
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
                roi_exit_enabled: true,
                roi_target_pct: 10.0,
                time_override_enabled: true,
                time_override_duration: 3.0,
                time_override_unit: "days".to_string(),
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
                roi_exit_enabled: true,
                roi_target_pct: 20.0,
                time_override_enabled: true,
                time_override_duration: 7.0,
                time_override_unit: "days".to_string(),
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
                roi_exit_enabled: true,
                roi_target_pct: 50.0,
                time_override_enabled: true,
                time_override_duration: 14.0,
                time_override_unit: "days".to_string(),
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
                roi_exit_enabled: true,
                roi_target_pct: 5.0,
                time_override_enabled: true,
                time_override_duration: 4.0,
                time_override_unit: "hours".to_string(),
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

// =============================================================================
// FORCE STOP HANDLERS
// =============================================================================

/// POST /api/trader/force-stop - Emergency stop all trading
async fn force_stop_handler(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<ForceStopRequest>,
) -> Response {
    let reason = payload
        .reason
        .unwrap_or_else(|| "Manual force stop".to_string());
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
            &format!("Force stop activated but config update failed: {}", e),
            None,
        );
    }

    logger::warning(LogTag::Trader, &format!("FORCE STOP activated: {}", reason));
    success_response(crate::global::get_force_stop_status())
}

/// POST /api/trader/resume - Clear force stop state
async fn resume_handler(State(_state): State<Arc<AppState>>) -> Response {
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
async fn force_stop_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    success_response(crate::global::get_force_stop_status())
}

// =============================================================================
// MONITOR CONTROL HANDLERS
// =============================================================================

/// GET /api/trader/monitors/status - Get monitor status
async fn monitors_status_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::config;

    success_response(serde_json::json!({
        "entry_monitor": {
            "enabled": config::is_entry_monitor_enabled_standalone(),
            "running": config::is_entry_monitor_enabled(),
        },
        "exit_monitor": {
            "enabled": config::is_exit_monitor_enabled_standalone(),
            "running": config::is_exit_monitor_enabled(),
        },
        "master_enabled": config::is_trader_enabled(),
        "force_stopped": crate::global::is_force_stopped(),
    }))
}

/// POST /api/trader/monitors/entry/toggle - Toggle entry monitor
async fn toggle_entry_monitor_handler(
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
            &format!("Failed to toggle entry monitor: {}", e),
            None,
        );
    }

    let status = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };
    logger::info(LogTag::Trader, &format!("Entry monitor {}", status));
    success_response(serde_json::json!({ "entry_monitor_enabled": payload.enabled }))
}

/// POST /api/trader/monitors/exit/toggle - Toggle exit monitor
async fn toggle_exit_monitor_handler(
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
            &format!("Failed to toggle exit monitor: {}", e),
            None,
        );
    }

    let status = if payload.enabled {
        "enabled"
    } else {
        "disabled"
    };
    logger::info(LogTag::Trader, &format!("Exit monitor {}", status));
    success_response(serde_json::json!({ "exit_monitor_enabled": payload.enabled }))
}

// =============================================================================
// LOSS LIMIT HANDLERS
// =============================================================================

/// GET /api/trader/loss-limit/status - Get loss limit status
async fn loss_limit_status_handler(State(_state): State<Arc<AppState>>) -> Response {
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
async fn loss_limit_resume_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::safety::loss_limit;

    loss_limit::resume_from_loss_limit();
    success_response(serde_json::json!({ "resumed": true }))
}

/// POST /api/trader/loss-limit/reset - Reset loss limit state
async fn loss_limit_reset_handler(State(_state): State<Arc<AppState>>) -> Response {
    use crate::trader::safety::loss_limit;

    loss_limit::reset_loss_limit_state();
    success_response(serde_json::json!({ "reset": true }))
}

// =============================================================================
// MANUAL TRADING HANDLERS
// =============================================================================

async fn manual_buy_handler(Json(req): Json<ManualBuyRequest>) -> Response {
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {}", pending);
        // Create failed action for visibility
        crate::trader::actions::create_failed_buy_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={}", pending)),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_buy_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
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
                let error_msg = "Token is blacklisted";
                crate::trader::actions::create_failed_buy_action(&req.mint, error_msg).await;
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

    // Use standard manual_buy - action tracking is handled inside
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
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

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
            let error_msg = "Token is blacklisted; cannot add to position";
            crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
            return error_response(StatusCode::FORBIDDEN, "Blacklisted", error_msg, None);
        }
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {}", pending);
        crate::trader::actions::create_failed_add_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={}", pending)),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
    }

    // Ensure there's an open position for this mint
    let has_open = positions::is_open_position(&req.mint).await;
    if !has_open {
        let error_msg = "Cannot add to position: no open position for this token";
        crate::trader::actions::create_failed_add_action(&req.mint, error_msg).await;
        return error_response(StatusCode::BAD_REQUEST, "NoOpenPosition", error_msg, None);
    }

    let size = match req.size_sol {
        Some(v) if v.is_finite() && v > 0.0 => v,
        _ => with_config(|cfg| cfg.trader.trade_size_sol * 0.5), // default add = 50% size
    };

    logger::info(
        LogTag::Webserver,
        &format!("mint={} size_sol={}", req.mint, size),
    );

    // Use trader module - action tracking is handled inside
    let result = crate::trader::manual::manual_add(&req.mint, size).await;

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualAddFailed",
                    tr.error.as_deref().unwrap_or("Manual add failed"),
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
                message: "Added to position".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualAddError",
            &e,
            None,
        ),
    }
}

async fn manual_sell_handler(Json(req): Json<ManualSellRequest>) -> Response {
    // Check force stop
    if crate::global::is_force_stopped() {
        return error_response(
            StatusCode::FORBIDDEN,
            "ForceStopped",
            "Manual trading disabled - Force stop is active",
            None,
        );
    }

    // Check services ready
    if !are_core_services_ready() {
        let pending = get_pending_services().join(", ");
        let error_msg = format!("Core services not ready: {}", pending);
        crate::trader::actions::create_failed_sell_action(&req.mint, &error_msg).await;
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "CoreServicesNotReady",
            "Core services are not ready for trading operations",
            Some(&format!("pending={}", pending)),
        );
    }

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        let error_msg = "Invalid token mint address";
        crate::trader::actions::create_failed_sell_action(&req.mint, error_msg).await;
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            error_msg,
            Some("Mint must be a valid base58 pubkey"),
        );
    }

    let is_open = positions::is_open_position(&req.mint).await;
    if !is_open {
        let error_msg = "Cannot sell: no open position for this token";
        crate::trader::actions::create_failed_sell_action(&req.mint, error_msg).await;
        return error_response(StatusCode::BAD_REQUEST, "NoOpenPosition", error_msg, None);
    }

    // Determine percentage
    let close_all = req.close_all.unwrap_or(false);
    let pct = if close_all {
        None // Full exit (100%)
    } else {
        Some(
            req.percentage
                .unwrap_or_else(|| with_config(|cfg| cfg.positions.partial_exit_default_pct)),
        )
    };

    // Validate percentage if provided
    if let Some(percentage) = pct {
        if !percentage.is_finite() || percentage <= 0.0 || percentage > 100.0 {
            let error_msg = format!("Invalid percentage: {}. Must be in (0, 100]", percentage);
            crate::trader::actions::create_failed_sell_action(&req.mint, &error_msg).await;
            return error_response(
                StatusCode::BAD_REQUEST,
                "InvalidPercentage",
                "percentage must be in (0, 100]",
                None,
            );
        }
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} percentage={:?} force={}",
            req.mint,
            pct,
            req.force.unwrap_or(false)
        ),
    );

    // Route to trader module - action tracking is handled inside
    let result = if req.force.unwrap_or(false) {
        crate::trader::manual::force_sell(&req.mint, pct).await
    } else {
        crate::trader::manual::manual_sell(&req.mint, pct).await
    };

    match result {
        Ok(tr) => {
            if !tr.success {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "ManualSellFailed",
                    tr.error.as_deref().unwrap_or("Manual sell failed"),
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
                message: if pct.unwrap_or(100.0) == 100.0 {
                    "Full position closed".to_string()
                } else {
                    format!("Partial position closed ({}%)", pct.unwrap_or(100.0))
                },
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            success_response(resp)
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ManualSellError",
            &e,
            None,
        ),
    }
}

// =============================================================================
// QUOTE PREVIEW HANDLER
// =============================================================================

/// GET /api/trader/quote - Get quote preview without execution
/// For BUY: requires amount_sol (SOL to spend), returns tokens received
/// For SELL: requires amount_tokens (tokens to sell), returns SOL received
async fn quote_preview_handler(
    axum::extract::Query(req): axum::extract::Query<QuotePreviewRequest>,
) -> Response {
    use crate::constants::SOL_MINT;
    use crate::swaps::operations::get_best_quote;
    use crate::swaps::router::{QuoteRequest, SwapMode};
    use crate::tokens::database::get_token_async;
    use crate::utils::get_wallet_address;

    // Validate mint
    if Pubkey::from_str(&req.mint).is_err() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "InvalidMint",
            "Invalid token mint address",
            None,
        );
    }

    let direction = if req.direction.to_lowercase() == "sell" {
        "sell"
    } else {
        "buy"
    };

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(_) => {
            return error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "WalletNotAvailable",
                "Wallet not configured",
                None,
            );
        }
    };

    // Get token info for decimals (needed for sell)
    let token_decimals = match get_token_async(&req.mint).await {
        Ok(Some(token)) => token.decimals.unwrap_or(9) as u32,
        Ok(None) => 9, // Default to 9 decimals if token not found
        Err(_) => 9,
    };

    // Build quote request based on direction
    let (input_mint, output_mint, input_amount, input_amount_display) = if direction == "buy" {
        // BUY: SOL → Token
        let amount_sol = match req.amount_sol {
            Some(amt) if amt > 0.0 && amt.is_finite() => amt,
            _ => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "amount_sol is required for buy and must be positive",
                    None,
                );
            }
        };
        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;
        (
            SOL_MINT.to_string(),
            req.mint.clone(),
            amount_lamports,
            amount_sol,
        )
    } else {
        // SELL: Token → SOL
        let amount_tokens = match req.amount_tokens {
            Some(amt) if amt > 0.0 && amt.is_finite() => amt,
            _ => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "InvalidAmount",
                    "amount_tokens is required for sell and must be positive",
                    None,
                );
            }
        };
        // Convert token amount to smallest units based on decimals
        let amount_raw = (amount_tokens * 10f64.powi(token_decimals as i32)) as u64;
        (
            req.mint.clone(),
            SOL_MINT.to_string(),
            amount_raw,
            amount_tokens,
        )
    };

    let quote_request = QuoteRequest {
        input_mint,
        output_mint,
        input_amount,
        wallet_address,
        slippage_pct: with_config(|cfg| cfg.swaps.slippage.quote_default_pct),
        swap_mode: SwapMode::ExactIn,
    };

    // Fetch quote
    match get_best_quote(quote_request).await {
        Ok(quote) => {
            // Format input/output based on direction
            let (input_formatted, output_display, output_formatted, price_per_token) = if direction
                == "buy"
            {
                // BUY: input is SOL, output is tokens
                let input_fmt = format!("{:.4} SOL", input_amount_display);
                let output_tokens = quote.output_amount as f64 / 10f64.powi(token_decimals as i32);
                let output_fmt = if output_tokens >= 1_000_000_000.0 {
                    format!("{:.2}B tokens", output_tokens / 1_000_000_000.0)
                } else if output_tokens >= 1_000_000.0 {
                    format!("{:.2}M tokens", output_tokens / 1_000_000.0)
                } else if output_tokens >= 1_000.0 {
                    format!("{:.2}K tokens", output_tokens / 1_000.0)
                } else {
                    format!("{:.4} tokens", output_tokens)
                };
                let price = if output_tokens > 0.0 {
                    input_amount_display / output_tokens
                } else {
                    0.0
                };
                (input_fmt, output_tokens, output_fmt, price)
            } else {
                // SELL: input is tokens, output is SOL
                let input_fmt = if input_amount_display >= 1_000_000_000.0 {
                    format!("{:.2}B tokens", input_amount_display / 1_000_000_000.0)
                } else if input_amount_display >= 1_000_000.0 {
                    format!("{:.2}M tokens", input_amount_display / 1_000_000.0)
                } else if input_amount_display >= 1_000.0 {
                    format!("{:.2}K tokens", input_amount_display / 1_000.0)
                } else {
                    format!("{:.4} tokens", input_amount_display)
                };
                let output_sol = quote.output_amount as f64 / 1_000_000_000.0;
                let output_fmt = format!("{:.6} SOL", output_sol);
                let price = if input_amount_display > 0.0 {
                    output_sol / input_amount_display
                } else {
                    0.0
                };
                (input_fmt, output_sol, output_fmt, price)
            };

            // Platform fee (0.5%) - calculated on SOL side
            let platform_fee_pct = 0.5;
            let platform_fee_sol = if direction == "buy" {
                input_amount_display * (platform_fee_pct / 100.0)
            } else {
                output_display * (platform_fee_pct / 100.0)
            };

            // Network fee estimate (approx 0.000005 SOL)
            let network_fee_sol = 0.000005;

            let response = QuotePreviewResponse {
                success: true,
                router: quote.router_name,
                direction: direction.to_string(),
                input_amount: input_amount_display,
                input_formatted,
                output_amount: output_display,
                output_formatted,
                price_per_token_sol: price_per_token,
                price_impact_pct: quote.price_impact_pct,
                platform_fee_pct,
                platform_fee_sol,
                network_fee_sol,
                route: quote.route_plan,
                slippage_bps: quote.slippage_bps,
                expires_in_secs: 30, // Quotes typically valid for ~30s
            };

            success_response(response)
        }
        Err(e) => error_response(
            StatusCode::BAD_GATEWAY,
            "QuoteFailed",
            &format!("Failed to fetch quote: {}", e),
            None,
        ),
    }
}
