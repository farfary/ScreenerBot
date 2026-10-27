// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Serialization types for the trading-config route, defining
//! TradingConfigResponse with its TradingLimits, RiskManagement and
//! ProfitTargets sections.

use serde::Serialize;

use crate::config::TakeProfitLevel;

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
    pub take_profit_enabled: bool,
    /// The take-profit ladder: each step's profit level and the share of the original
    /// position it sells.
    pub take_profit_levels: Vec<TakeProfitLevel>,
    pub profit_extra_needed_sol: f64,
}
