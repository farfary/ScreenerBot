//! Blacklist-based safety checks
//!
//! Uses tokens module as single source of truth.
//! No caching - direct synchronous calls for simplicity and correctness.
//! The tokens module maintains the blacklist, this module just checks it.

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a token is blacklisted (sync - direct read from tokens module)
///
/// Returns true if the token is in the blacklist maintained by the tokens module.
/// No caching layer - tokens module is the single source of truth.
pub fn is_blacklisted(mint: &str) -> bool {
    crate::tokens::get_blacklisted_tokens().contains(&mint.to_string())
}

/// Check if a position should be exited due to blacklist (sync)
///
/// Returns an immediate exit decision if the position's token is blacklisted.
/// This is a critical safety check that overrides all other exit conditions.
///
/// Uses tokens module as single source of truth (no cache layer).
/// Priority: Emergency (highest)
pub fn check_blacklist_exit(position: &Position, current_price: f64) -> Option<TradeDecision> {
    if is_blacklisted(&position.mint) {
        logger::warning(
            LogTag::Trader,
            &format!(
                "⛔ BLACKLISTED: {} (mint={}) - Emergency exit at {:.9} SOL",
                position.symbol, position.mint, current_price
            ),
        );

        return Some(TradeDecision {
            position_id: position.id.map(|id| id.to_string()),
            mint: position.mint.clone(),
            action: TradeAction::Sell,
            reason: TradeReason::Blacklisted,
            strategy_id: None,
            timestamp: Utc::now(),
            priority: TradePriority::Emergency,
            price_sol: Some(current_price),
            size_sol: None, // Sell entire position
        });
    }

    None
}
