//! Blacklist-based safety checks
//!
//! Uses tokens module as single source of truth.
//! No caching - direct synchronous calls for simplicity and correctness.
//! The tokens module maintains the blacklist, this module just checks it.

use crate::logger::{self, LogTag};
use crate::positions::Position;
use crate::trader::types::{TradeAction, TradeDecision, TradePriority, TradeReason};
use chrono::Utc;

/// Check if a token is blacklisted at the TOKEN level (async - DB check)
///
/// Only checks the token blacklist table (explicit bad tokens: scam, rug, mint/freeze authority).
/// Does NOT include pool-level or account-level blacklists — those are handled by the pool
/// service (fallback to alternative pools) and should not block entry for tokens that have
/// valid alternative pools with working prices.
///
/// See BUG-30: Pool blacklist was propagating to token-level via filtering engine,
/// blocking entry for tokens with valid alternative pools.
pub async fn is_blacklisted(mint: &str) -> bool {
    // Check token-level blacklist in DB (the authoritative source for "bad token")
    match crate::tokens::get_global_database() {
        Some(db) => db.is_blacklisted(mint).unwrap_or(false),
        None => {
            // DB not initialized yet — fall back to in-memory filtered list
            crate::tokens::get_blacklisted_tokens(crate::chains::ChainId::Solana)
                .contains(&mint.to_string())
        }
    }
}

/// Check if a position should be exited due to blacklist (sync)
///
/// Returns an immediate exit decision if the position's token is blacklisted.
/// This is a critical safety check that overrides all other exit conditions.
///
/// Uses tokens module as single source of truth (no cache layer).
/// Priority: Emergency (highest)
pub async fn check_blacklist_exit(
    position: &Position,
    current_price: f64,
) -> Option<TradeDecision> {
    if is_blacklisted(&position.mint).await {
        logger::warning(
            LogTag::Trader,
            &format!(
                "BLACKLISTED: {} (mint={}) - Emergency exit at {:.9} SOL",
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
            exit_percentage: None,
            // Auto-trader slippage always follows config.
            slippage_pct: None,
        });
    }

    None
}
