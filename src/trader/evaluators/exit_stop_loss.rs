// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Fixed stop loss exit implementation
//!
//! Triggers exit when position loss exceeds configured threshold from entry price.
//! Unlike trailing stop (which tracks from peak), this is measured from entry.

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::policy::StopLossPolicy;

/// The stop-loss rule on plain numbers, shared by the live evaluator and the
/// paper copy book. Returns `Some(exit_percentage)` when it fires, where the
/// inner `None` is a full exit.
///
/// Loss is measured from entry: ((entry_price - current_price) / entry_price) * 100.
///
/// A partial stop-loss may be taken ONCE. Selling moves neither the entry price nor
/// the current price, so once a position is below the stop this fires again on every
/// cycle: it sold the configured partial share, then that share of the remainder, and so
/// on, bleeding the position out in a geometric series and paying fees plus slippage
/// on every leg instead of exiting. If the price is STILL under the stop after
/// exposure was already cut once, the thesis is done -- exit fully.
pub fn stop_loss_exit(
    entry_price: f64,
    current_price: f64,
    held_seconds: i64,
    already_partially_exited: bool,
    policy: &StopLossPolicy,
) -> Option<Option<f64>> {
    if !policy.enabled || !entry_price.is_finite() || entry_price <= 0.0 {
        return None;
    }
    if policy.min_hold_seconds > 0 && held_seconds < policy.min_hold_seconds as i64 {
        return None;
    }
    let loss_pct = ((entry_price - current_price) / entry_price) * 100.0;
    if loss_pct < policy.threshold_pct {
        return None;
    }
    Some(
        (policy.allow_partial && !already_partially_exited)
            .then_some(policy.partial_exit_default_pct),
    )
}

/// The log line of a fired stop loss, with the figures it fired on.
pub(crate) fn log_stop_loss_triggered(
    position: &Position,
    current_price: f64,
    exit_percentage: Option<f64>,
    policy: &StopLossPolicy,
) {
    let entry_price = position.average_entry_price;
    logger::info(
        LogTag::Trader,
        &format!(
            "Stop loss triggered for {} ({}): entry_price={:.12} SOL, current_price={:.12} SOL, loss={:.2}%, threshold={:.1}%, mint={}",
            position.symbol,
            if exit_percentage.is_some() { "partial exit" } else { "full exit" },
            entry_price,
            current_price,
            ((entry_price - current_price) / entry_price) * 100.0,
            policy.threshold_pct,
            position.mint
        ),
    );
}
