// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading configuration utilities.

use crate::config::{with_config, DcaLevel};

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

/// The DCA ladder: one step per automatic add, in the order the adds fire
pub fn get_dca_levels() -> Vec<DcaLevel> {
    with_config(|cfg| cfg.trader.dca_levels())
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
