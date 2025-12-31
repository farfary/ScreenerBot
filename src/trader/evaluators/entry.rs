//! Entry evaluation logic with integrated safety checks
//!
//! Evaluates whether an entry should be made for a token by checking:
//! 1. Connectivity health
//! 2. Position limits
//! 3. Existing position check
//! 4. Re-entry cooldown
//! 5. Blacklist status
//! 6. Strategy signals

use crate::pools::PriceResult;
use crate::trader::types::TradeDecision;
use crate::trader::{evaluators, safety};

/// Evaluate entry opportunity for a token
///
/// Performs all safety checks before strategy evaluation:
/// - Connectivity check (RPC, DexScreener, RugCheck must be healthy)
/// - Position limits (can't exceed max open positions)
/// - Existing position check (no duplicate entries)
/// - Re-entry cooldown (prevents immediate re-entry after exit)
/// - Blacklist check (token not blacklisted)
/// - Strategy evaluation (signals from configured strategies)
///
/// Returns:
/// - Ok(Some(TradeDecision)) if entry should be made
/// - Ok(None) if no entry signal or safety check failed
/// - Err(String) if evaluation failed due to connectivity or other errors
pub async fn evaluate_entry_for_token(
    token_mint: &str,
    price_info: &PriceResult,
) -> Result<Option<TradeDecision>, String> {
    // Early exit: Force stop is active
    if crate::global::is_force_stopped() {
        return Ok(None);
    }

    // Early exit: Loss limit reached
    if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
        return Ok(None);
    }

    // 1. Connectivity check - critical endpoints must be healthy
    if let Some(unhealthy) =
        crate::connectivity::check_endpoints_healthy(&["rpc", "dexscreener", "rugcheck"]).await
    {
        return Err(format!("Unhealthy endpoints: {}", unhealthy));
    }

    // 2. Position limits - check if we can open more positions
    if !safety::check_position_limits().await? {
        return Ok(None); // Hit position limit
    }

    // 3. Existing position check - prevent duplicate entries
    if safety::has_open_position(token_mint).await? {
        return Ok(None); // Already have position
    }

    // 4. Re-entry cooldown - prevent immediate re-entry after exit
    if safety::is_in_reentry_cooldown(token_mint).await? {
        return Ok(None); // Still in cooldown
    }

    // 5. Blacklist check - sync check, no caching needed
    if safety::is_blacklisted(token_mint) {
        return Ok(None); // Token is blacklisted
    }

    // 6. Strategy evaluation - check configured entry strategies
    evaluators::StrategyEvaluator::check_entry_strategies(token_mint, price_info).await
}
