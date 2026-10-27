// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading configuration utilities.

use crate::config::{with_config, DcaLevel, DcaMeasureFrom, TakeProfitLevel};

/// Get the maximum number of open positions allowed
pub fn get_max_open_positions() -> usize {
    with_config(|cfg| cfg.trader.max_open_positions)
}

/// Get the default trade size in SOL
pub fn get_trade_size_native() -> f64 {
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

/// The DCA ladder: one step per automatic add, in the order the adds fire
pub fn get_dca_levels() -> Vec<DcaLevel> {
    with_config(|cfg| cfg.trader.dca_levels())
}

/// Which entry price the DCA drop is measured from
pub fn get_dca_measure_from() -> DcaMeasureFrom {
    with_config(|cfg| cfg.trader.dca_measure_from)
}

/// Minutes a position is held after its entry before its first automatic add
pub fn get_dca_min_hold_minutes() -> i64 {
    with_config(|cfg| cfg.trader.dca_min_hold_minutes)
}

/// Get DCA cooldown in minutes
pub fn get_dca_cooldown_minutes() -> i64 {
    with_config(|cfg| cfg.trader.dca_cooldown_minutes)
}

/// Check if stop loss is enabled
pub fn is_stop_loss_enabled() -> bool {
    with_config(|cfg| cfg.trader.stop_loss_enabled)
}

/// Get stop loss threshold percentage
pub fn get_stop_loss_threshold_pct() -> f64 {
    with_config(|cfg| cfg.trader.stop_loss_threshold_pct)
}

/// Get stop loss minimum hold seconds
pub fn get_stop_loss_min_hold_seconds() -> u64 {
    with_config(|cfg| cfg.trader.stop_loss_min_hold_seconds)
}

/// Check if partial exits are allowed for stop loss
pub fn get_stop_loss_allow_partial() -> bool {
    with_config(|cfg| cfg.trader.stop_loss_allow_partial)
}

/// Get the share of the position a partial stop loss sells, in percent
pub fn get_stop_loss_partial_pct() -> f64 {
    with_config(|cfg| cfg.trader.stop_loss_partial_pct)
}

/// Check if trailing stop is enabled
pub fn is_trailing_stop_enabled() -> bool {
    with_config(|cfg| cfg.trader.trailing_stop_enabled)
}

/// Get trailing stop activation percentage
pub fn get_trailing_stop_activation_pct() -> f64 {
    with_config(|cfg| cfg.trader.trailing_stop_activation_pct)
}

/// Get trailing stop distance percentage
pub fn get_trailing_stop_distance_pct() -> f64 {
    with_config(|cfg| cfg.trader.trailing_stop_distance_pct)
}

/// Check if the break-even stop is enabled
pub fn is_break_even_enabled() -> bool {
    with_config(|cfg| cfg.trader.break_even_enabled)
}

/// Get the profit from the average entry that arms the break-even stop, in percent
pub fn get_break_even_trigger_pct() -> f64 {
    with_config(|cfg| cfg.trader.break_even_trigger_pct)
}

/// Get the margin above the average entry the break-even stop keeps, in percent
pub fn get_break_even_offset_pct() -> f64 {
    with_config(|cfg| cfg.trader.break_even_offset_pct)
}

/// Check if the take-profit ladder is enabled
pub fn is_take_profit_enabled() -> bool {
    with_config(|cfg| cfg.trader.take_profit_enabled)
}

/// Get the take-profit ladder, one step per configured level
pub fn get_take_profit_levels() -> Vec<TakeProfitLevel> {
    with_config(|cfg| cfg.trader.take_profit_levels())
}

/// Check if time override is enabled
pub fn is_time_override_enabled() -> bool {
    with_config(|cfg| cfg.trader.time_override_enabled)
}

/// Get time override duration in seconds (converted from configured unit)
pub fn get_time_override_duration_seconds() -> f64 {
    with_config(|cfg| {
        cfg.trader
            .time_override_unit
            .to_seconds(cfg.trader.time_override_duration)
    })
}

/// Check if the stagnation exit is enabled
pub fn is_time_exit_stagnation_enabled() -> bool {
    with_config(|cfg| cfg.trader.time_exit_stagnation_enabled)
}

/// Get how long a position stays flat before the stagnation exit closes it, in minutes
pub fn get_time_exit_stagnation_minutes() -> f64 {
    with_config(|cfg| cfg.trader.time_exit_stagnation_minutes)
}

/// Get the profit-or-loss band the stagnation exit counts as flat, in percent
pub fn get_time_exit_stagnation_band_pct() -> f64 {
    with_config(|cfg| cfg.trader.time_exit_stagnation_band_pct)
}

/// Get time override loss threshold percentage
pub fn get_time_override_loss_threshold_pct() -> f64 {
    with_config(|cfg| cfg.trader.time_override_loss_threshold_percent)
}

/// Get position close cooldown in minutes
pub fn get_position_close_cooldown_minutes() -> u64 {
    with_config(|cfg| cfg.trader.position_close_cooldown_minutes as u64)
}

/// Get sell (exit) concurrency limit
pub fn get_sell_concurrency() -> usize {
    with_config(|cfg| cfg.trader.sell_concurrency)
}

// ==================== ENTRY/EXIT MONITOR CONTROL ====================

/// Check if entry monitor is enabled
/// Combines master switch (trader.enabled) AND entry-specific flag
pub fn is_entry_monitor_enabled() -> bool {
    with_config(|cfg| cfg.trader.enabled && cfg.trader.entry_monitor_enabled)
}

/// Check if exit monitor is enabled
/// Combines master switch (trader.enabled) AND exit-specific flag
pub fn is_exit_monitor_enabled() -> bool {
    with_config(|cfg| cfg.trader.enabled && cfg.trader.exit_monitor_enabled)
}

/// Check if only entry monitor is enabled (for UI display)
pub fn is_entry_monitor_enabled_standalone() -> bool {
    with_config(|cfg| cfg.trader.entry_monitor_enabled)
}

/// Check if only exit monitor is enabled (for UI display)
pub fn is_exit_monitor_enabled_standalone() -> bool {
    with_config(|cfg| cfg.trader.exit_monitor_enabled)
}

// ==================== LOSS LIMIT CONFIGURATION ====================

/// Check if loss limit protection is enabled
pub fn is_loss_limit_enabled() -> bool {
    with_config(|cfg| cfg.trader.loss_limit_enabled)
}

/// Get loss limit threshold in SOL
pub fn get_loss_limit_native() -> f64 {
    with_config(|cfg| cfg.trader.loss_limit_sol)
}

/// Get loss limit period in hours
pub fn get_loss_limit_period_hours() -> u64 {
    with_config(|cfg| cfg.trader.loss_limit_period_hours)
}

/// Check if loss limit should auto-resume after period reset
pub fn is_loss_limit_auto_resume() -> bool {
    with_config(|cfg| cfg.trader.loss_limit_auto_resume)
}
