use serde::Serialize;

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
}

#[derive(Debug, Serialize)]
pub struct ProfitTargets {
    pub base_min_profit_percent: f64,
    pub min_profit_threshold_enabled: bool,
    pub profit_extra_needed_sol: f64,
}
