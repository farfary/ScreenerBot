//! Safety systems for trading

mod limits;
mod risk;

pub use limits::{check_position_limits, has_open_position, is_in_reentry_cooldown};
pub use risk::check_risk_limits;

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Initialize the safety system
pub async fn init_safety_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing safety system...");
    // Blacklist is now managed by tokens/filtering modules - no init needed
    logger::info(LogTag::Trader, "Safety system initialized");
    Ok(())
}

/// Check if a token is blacklisted (sync - direct read from tokens module)
pub fn is_blacklisted(mint: &str) -> bool {
    let blacklisted_tokens = crate::tokens::get_blacklisted_tokens();
    blacklisted_tokens.contains(&mint.to_string())
}

/// Check if a position should be exited due to blacklist (sync)
///
/// Returns an immediate exit decision if the position's token is blacklisted.
/// This is a critical safety check that overrides all other exit conditions.
///
/// Uses tokens module as single source of truth (no cache layer).
pub fn check_blacklist_exit(position: &Position, current_price: f64) -> Option<TradeDecision> {
    // Direct check against tokens module (no cache)
    let blacklisted_tokens = crate::tokens::get_blacklisted_tokens();

    if blacklisted_tokens.contains(&position.mint) {
        logger::warning(
            LogTag::Trader,
            &format!(
                "⛔ BLACKLISTED: {} (mint={}) - Triggering emergency exit at {:.9} SOL",
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
