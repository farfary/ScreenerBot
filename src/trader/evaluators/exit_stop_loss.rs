//! Fixed stop loss exit implementation
//!
//! Triggers exit when position loss exceeds configured threshold from entry price.
//! Unlike trailing stop (which tracks from peak), this is measured from entry.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::policy::StopLossPolicy;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

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

/// The stop-loss rule on plain numbers, shared by the live evaluator and the
/// paper copy book. Returns `Some(exit_percentage)` when it fires, where the
/// inner `None` is a full exit.
///
/// Loss is measured from entry: ((entry_price - current_price) / entry_price) * 100.
///
/// A partial stop-loss may be taken ONCE. Selling moves neither the entry price nor
/// the current price, so once a position is below the stop this fires again on every
/// cycle: it sold partial_exit_default_pct, then that share of the remainder, and so
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

/// Check if a position should be exited based on fixed stop loss
pub async fn check_stop_loss(
    position: &Position,
    current_price: f64,
    policy: &StopLossPolicy,
) -> crate::trader::Result<Option<TradeDecision>> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: current_price,
        }
        .into());
    }
    if !policy.enabled {
        return Ok(None);
    }
    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Err(crate::positions::Error::InvalidPrice {
            mint: position.mint.clone(),
            price: entry_price,
        }
        .into());
    }
    let held_seconds = (Utc::now() - position.entry_time).num_seconds();
    let Some(exit_percentage) = stop_loss_exit(
        entry_price,
        current_price,
        held_seconds,
        position.partial_exit_count > 0,
        policy,
    ) else {
        return Ok(None);
    };

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

    Ok(Some(TradeDecision {
        position_id: position.id.map(|id| id.to_string()),
        mint: position.mint.clone(),
        action: TradeAction::Sell,
        reason: TradeReason::StopLoss,
        strategy_id: None,
        timestamp: Utc::now(),
        priority: TradePriority::High,
        price_sol: Some(current_price),
        size_sol: None,
        exit_percentage,
        // Auto-trader slippage always follows config.
        slippage_pct: None,
    }))
}
