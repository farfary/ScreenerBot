//! Trading configuration utilities

use crate::config::with_config;

/// Get the maximum number of open positions allowed
pub fn get_max_open_positions() -> usize {
    with_config(|cfg| cfg.trader.max_open_positions)
}

/// Get the default trade size in SOL
pub fn get_trade_size_sol() -> f64 {
    with_config(|cfg| cfg.trader.trade_size_sol)
}

/// Get the entry check concurrency limit
pub fn get_entry_check_concurrency() -> usize {
    with_config(|cfg| cfg.trader.entry_check_concurrency)
}

/// Check if trader is enabled
pub fn is_trader_enabled() -> bool {
    with_config(|cfg| cfg.trader.enabled)
}

/// Check if DCA is enabled
pub fn is_dca_enabled() -> bool {
    with_config(|cfg| cfg.trader.dca_enabled)
}

/// Get DCA threshold percentage
pub fn get_dca_threshold_pct() -> f64 {
    with_config(|cfg| cfg.trader.dca_threshold_pct)
}

/// Get maximum DCA count per position
pub fn get_dca_max_count() -> usize {
    with_config(|cfg| cfg.trader.dca_max_count)
}

/// Get DCA size as percentage of initial position
pub fn get_dca_size_percentage() -> f64 {
    with_config(|cfg| cfg.trader.dca_size_percentage)
}

/// Get DCA cooldown in minutes
pub fn get_dca_cooldown_minutes() -> i64 {
    with_config(|cfg| cfg.trader.dca_cooldown_minutes)
}

/// Check if trailing stop is enabled
pub fn is_trailing_stop_enabled() -> bool {
    with_config(|cfg| cfg.positions.trailing_stop_enabled)
}

/// Get trailing stop activation percentage
pub fn get_trailing_stop_activation_pct() -> f64 {
    with_config(|cfg| cfg.positions.trailing_stop_activation_pct)
}

/// Get trailing stop distance percentage
pub fn get_trailing_stop_distance_pct() -> f64 {
    with_config(|cfg| cfg.positions.trailing_stop_distance_pct)
}

/// Check if partial exits are enabled
pub fn is_partial_exit_enabled() -> bool {
    with_config(|cfg| cfg.positions.partial_exit_enabled)
}

/// Get default partial exit percentage
pub fn get_partial_exit_default_pct() -> f64 {
    with_config(|cfg| cfg.positions.partial_exit_default_pct)
}

/// Check if ROI-based exit is enabled
pub fn is_roi_exit_enabled() -> bool {
    with_config(|cfg| cfg.trader.min_profit_threshold_enabled)
}

/// Get target profit percentage for ROI exit
pub fn get_target_profit_pct() -> f64 {
    with_config(|cfg| cfg.trader.min_profit_threshold_percent)
}

/// Get time override loss threshold percentage
pub fn get_time_override_loss_threshold_pct() -> f64 {
    with_config(|cfg| cfg.trader.time_override_loss_threshold_percent)
}

/// Get time override duration in hours
pub fn get_time_override_duration_hours() -> f64 {
    with_config(|cfg| cfg.trader.time_override_duration_hours)
}

/// Get position close cooldown in minutes
pub fn get_position_close_cooldown_minutes() -> u64 {
    with_config(|cfg| cfg.trader.position_close_cooldown_minutes as u64)
}
