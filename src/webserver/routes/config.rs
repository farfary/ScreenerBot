/// Trading Config API routes
///
/// Provides read-only access to trading configuration constants

use axum::{ extract::State, response::Json, routing::get, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::webserver::state::AppState;
use crate::trader;
use crate::profit;
use crate::entry;

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingConfigResponse {
    pub trading_limits: TradingLimits,
    pub risk_management: RiskManagement,
    pub profit_targets: ProfitTargets,
    pub timing: TimingConfig,
    pub slippage: SlippageConfig,
    pub monitoring: MonitoringConfig,
    pub debug_modes: DebugModes,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingLimits {
    pub max_open_positions: usize,
    pub trade_size_sol: f64,
    pub entry_check_concurrency: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskManagement {
    pub stop_loss_percent: f64,
    pub extreme_loss_percent: f64,
    pub min_profit_threshold_percent: f64,
    pub min_profit_threshold_enabled: bool,
    pub time_override_duration_hours: f64,
    pub time_override_loss_threshold_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfitTargets {
    pub base_min_profit_percent: f64,
    pub instant_exit_level_1: f64,
    pub instant_exit_level_2: f64,
    pub default_target_max_percent: f64,
    pub trail_min_gap: f64,
    pub trail_max_gap: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimingConfig {
    pub max_hold_minutes: f64,
    pub position_close_cooldown_minutes: i64,
    pub entry_monitor_interval_secs: u64,
    pub position_monitor_interval_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlippageConfig {
    pub quote_default_pct: f64,
    pub exit_profit_shortfall_pct: f64,
    pub exit_loss_shortfall_pct: f64,
    pub exit_retry_steps_pct: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub entry_check_interval_secs: u64,
    pub position_monitor_interval_secs: u64,
    pub token_check_task_timeout_secs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugModes {
    pub force_sell_mode: bool,
    pub force_sell_timeout_secs: f64,
    pub force_buy_mode: bool,
    pub force_buy_drop_threshold_percent: f64,
}

/// Create trading config routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/trading/config", get(get_trading_config))
}

/// Get current trading configuration
async fn get_trading_config() -> Json<TradingConfigResponse> {
    Json(TradingConfigResponse {
        trading_limits: TradingLimits {
            max_open_positions: trader::MAX_OPEN_POSITIONS,
            trade_size_sol: trader::TRADE_SIZE_SOL,
            entry_check_concurrency: trader::ENTRY_CHECK_CONCURRENCY,
        },
        risk_management: RiskManagement {
            stop_loss_percent: profit::STOP_LOSS_PERCENT,
            extreme_loss_percent: profit::EXTREME_LOSS_PERCENT,
            min_profit_threshold_percent: trader::MIN_PROFIT_THRESHOLD_PERCENT,
            min_profit_threshold_enabled: trader::MIN_PROFIT_THRESHOLD_ENABLED,
            time_override_duration_hours: trader::TIME_OVERRIDE_DURATION_HOURS,
            time_override_loss_threshold_percent: trader::TIME_OVERRIDE_LOSS_THRESHOLD_PERCENT,
        },
        profit_targets: ProfitTargets {
            base_min_profit_percent: profit::BASE_MIN_PROFIT_PERCENT,
            instant_exit_level_1: profit::INSTANT_EXIT_LEVEL_1,
            instant_exit_level_2: profit::INSTANT_EXIT_LEVEL_2,
            default_target_max_percent: profit::DEFAULT_TARGET_MAX_PERCENT,
            trail_min_gap: profit::TRAIL_MIN_GAP,
            trail_max_gap: profit::TRAIL_MAX_GAP,
        },
        timing: TimingConfig {
            max_hold_minutes: profit::MAX_HOLD_MINUTES,
            position_close_cooldown_minutes: trader::POSITION_CLOSE_COOLDOWN_MINUTES,
            entry_monitor_interval_secs: trader::ENTRY_MONITOR_INTERVAL_SECS,
            position_monitor_interval_secs: trader::POSITION_MONITOR_INTERVAL_SECS,
        },
        slippage: SlippageConfig {
            quote_default_pct: trader::SLIPPAGE_QUOTE_DEFAULT_PCT,
            exit_profit_shortfall_pct: trader::SLIPPAGE_EXIT_PROFIT_SHORTFALL_PCT,
            exit_loss_shortfall_pct: trader::SLIPPAGE_EXIT_LOSS_SHORTFALL_PCT,
            exit_retry_steps_pct: trader::SLIPPAGE_EXIT_RETRY_STEPS_PCT.to_vec(),
        },
        monitoring: MonitoringConfig {
            entry_check_interval_secs: trader::ENTRY_MONITOR_INTERVAL_SECS,
            position_monitor_interval_secs: trader::POSITION_MONITOR_INTERVAL_SECS,
            token_check_task_timeout_secs: trader::TOKEN_CHECK_TASK_TIMEOUT_SECS,
        },
        debug_modes: DebugModes {
            force_sell_mode: trader::DEBUG_FORCE_SELL_MODE,
            force_sell_timeout_secs: trader::DEBUG_FORCE_SELL_TIMEOUT_SECS,
            force_buy_mode: trader::DEBUG_FORCE_BUY_MODE,
            force_buy_drop_threshold_percent: trader::DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
