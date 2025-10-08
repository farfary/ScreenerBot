use axum::{response::Response, routing::get, Router};
use serde::Serialize;
use std::sync::Arc;

use crate::config::with_config;
use crate::profit::STOP_LOSS_PERCENT;
use crate::webserver::{state::AppState, utils::success_response};

#[derive(Debug, Serialize)]
pub struct TradingConfigResponse {
    pub trading_limits: TradingLimits,
    pub risk_management: RiskManagement,
    pub profit_targets: ProfitTargets,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct TradingLimits {
    pub max_open_positions: usize,
    pub trade_size_sol: f64,
    pub entry_monitor_interval_secs: u64,
    pub position_monitor_interval_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct RiskManagement {
    pub stop_loss_percent: f64,
    pub time_override_loss_threshold_percent: f64,
    pub time_override_duration_hours: f64,
    pub debug_force_sell_mode: bool,
    pub debug_force_buy_mode: bool,
}

#[derive(Debug, Serialize)]
pub struct ProfitTargets {
    pub base_min_profit_percent: f64,
    pub min_profit_threshold_enabled: bool,
    pub profit_extra_needed_sol: f64,
}

/// Create trading routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/config", get(get_trading_config))
}

/// GET /api/trading/config - summarized trading configuration for dashboard
async fn get_trading_config() -> Response {
    let response = with_config(|cfg| TradingConfigResponse {
        trading_limits: TradingLimits {
            max_open_positions: cfg.trader.max_open_positions,
            trade_size_sol: cfg.trader.trade_size_sol,
            entry_monitor_interval_secs: cfg.trader.entry_monitor_interval_secs,
            position_monitor_interval_secs: cfg.trader.position_monitor_interval_secs,
        },
        risk_management: RiskManagement {
            stop_loss_percent: STOP_LOSS_PERCENT,
            time_override_loss_threshold_percent: cfg.trader.time_override_loss_threshold_percent,
            time_override_duration_hours: cfg.trader.time_override_duration_hours,
            debug_force_sell_mode: cfg.trader.debug_force_sell_mode,
            debug_force_buy_mode: cfg.trader.debug_force_buy_mode,
        },
        profit_targets: ProfitTargets {
            base_min_profit_percent: cfg.trader.min_profit_threshold_percent,
            min_profit_threshold_enabled: cfg.trader.min_profit_threshold_enabled,
            profit_extra_needed_sol: cfg.trader.profit_extra_needed_sol,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(response)
}
