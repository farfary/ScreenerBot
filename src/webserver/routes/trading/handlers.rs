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
            time_override_duration_hours: {
                use crate::config::TimeUnit;
                let unit =
                    TimeUnit::from_str(&cfg.trader.time_override_unit).unwrap_or(TimeUnit::Hours);
                unit.to_seconds(cfg.trader.time_override_duration) / 3600.0
            },
        },
        profit_targets: ProfitTargets {
            base_min_profit_percent: cfg.trader.roi_target_percent,
            min_profit_threshold_enabled: cfg.trader.roi_exit_enabled,
            profit_extra_needed_sol: cfg.positions.profit_extra_needed_sol,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    });

    success_response(response)
}
