// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard handler serving the summarized trading configuration - trade size limits, risk management and profit targets.

use axum::response::Response;

use crate::{config::with_config, webserver::utils::success_response};

use super::types::*;

/// GET /api/trading/config - summarized trading configuration for dashboard
pub(super) async fn get_trading_config() -> Response {
    let response = with_config(|cfg| TradingConfigResponse {
        trading_limits: TradingLimits {
            max_open_positions: cfg.trader.max_open_positions,
            trade_size_sol: cfg.trader.trade_size_sol,
            entry_monitor_interval_secs: crate::trader::ENTRY_MONITOR_INTERVAL_SECS,
            position_monitor_interval_secs: crate::trader::POSITION_MONITOR_INTERVAL_SECS,
        },
        risk_management: RiskManagement {
            stop_loss_percent: if cfg.trader.stop_loss_enabled {
                cfg.trader.stop_loss_threshold_pct
            } else {
                0.0
            },
            time_override_loss_threshold_percent: cfg.trader.time_override_loss_threshold_percent,
            time_override_duration_hours: cfg
                .trader
                .time_override_unit
                .to_seconds(cfg.trader.time_override_duration)
                / 3600.0,
            break_even_trigger_percent: cfg
                .trader
                .break_even_enabled
                .then_some(cfg.trader.break_even_trigger_pct),
            break_even_offset_percent: cfg
                .trader
                .break_even_enabled
                .then_some(cfg.trader.break_even_offset_pct),
            time_stagnation_duration_hours: cfg
                .trader
                .time_exit_stagnation_enabled
                .then_some(cfg.trader.time_exit_stagnation_minutes / 60.0),
            time_stagnation_band_percent: cfg
                .trader
                .time_exit_stagnation_enabled
                .then_some(cfg.trader.time_exit_stagnation_band_pct),
        },
        profit_targets: ProfitTargets {
            take_profit_enabled: cfg.trader.take_profit_enabled,
            take_profit_levels: cfg.trader.take_profit_levels(),
            profit_extra_needed_sol: cfg.positions.profit_extra_needed_sol,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(response)
}
