//! Fixed stop loss exit implementation
//!
//! Triggers exit when position loss exceeds configured threshold from entry price.
//! Unlike trailing stop (which tracks from peak), this is measured from entry.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::Position;
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

/// Check if a position should be exited based on fixed stop loss
///
/// Stop loss triggers when:
/// 1. Stop loss is enabled in config
/// 2. Position has been held for min_hold_seconds (if configured)
/// 3. Current loss percentage exceeds threshold
///
/// Loss calculation: ((entry_price - current_price) / entry_price) * 100
/// Example: Entry at 0.001, current at 0.0005 = 50% loss
pub async fn check_stop_loss(
    position: &Position,
    current_price: f64,
) -> Result<Option<TradeDecision>, String> {
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!(
            "Invalid current_price for stop loss: {}",
            current_price
        ));
    }

    // Check if stop loss is enabled
    if !is_stop_loss_enabled() {
        return Ok(None);
    }

    // Get configuration
    let threshold_pct = get_stop_loss_threshold_pct();
    let min_hold_seconds = get_stop_loss_min_hold_seconds();

    // Validate entry price
    let entry_price = position.average_entry_price;
    if entry_price <= 0.0 || !entry_price.is_finite() {
        return Err(format!(
            "Invalid entry_price for stop loss: {} (mint={})",
            entry_price, position.mint
        ));
    }

    // Check minimum hold time if configured
    if min_hold_seconds > 0 {
        let position_age_seconds = (Utc::now() - position.entry_time).num_seconds();
        if position_age_seconds < min_hold_seconds as i64 {
            // Position hasn't been held long enough
            return Ok(None);
        }
    }

    // Calculate loss percentage from entry price
    // Positive value = loss, negative value = profit
    let loss_pct = ((entry_price - current_price) / entry_price) * 100.0;

    // Check if loss exceeds threshold
    // loss_pct >= threshold means we've lost at least threshold% from entry
    if loss_pct >= threshold_pct {
        // Determine exit size based on partial exit config
        let allow_partial = get_stop_loss_allow_partial();
        let size_sol = if allow_partial {
            // Use partial exit percentage from positions config
            Some(with_config(|cfg| cfg.positions.partial_exit_default_pct))
        } else {
            None // Full exit
        };

        logger::info(
            LogTag::Trader,
            &format!(
                "Stop loss triggered for {} ({}): entry_price={:.12} SOL, current_price={:.12} SOL, loss={:.2}%, threshold={:.1}%, mint={}",
                position.symbol,
                if allow_partial { "partial exit" } else { "full exit" },
                entry_price,
                current_price,
                loss_pct,
                threshold_pct,
                position.mint
            ),
        );

        return Ok(Some(TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::StopLoss,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::High, // High priority for stop loss
            price_sol: Some(current_price),
            size_sol,
        }));
    }

    Ok(None)
}
