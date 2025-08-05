/// Simple entry logic for ScreenerBot
///
/// This module provides a basic entry decision function that can be called from the trader.
/// Replaces the complex smart_entry.rs with a straightforward approach.

use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;

/// Simple entry decision function
/// Returns true if the token should be bought based on basic criteria
pub fn should_buy_simple(token: &Token) -> bool {
    // Basic safety checks
    if token.price_dexscreener_sol.unwrap_or(0.0) <= 0.0 {
        return false;
    }

    // Check minimum liquidity requirement ($1000 USD)
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    if liquidity_usd < 1000.0 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Low liquidity ${:.0}", token.symbol, liquidity_usd)
            );
        }
        return false;
    }

    // Check if price is not at recent highs (avoid buying at ATH)
    // Use simple heuristic: if 24h change is less than +50%, it's probably not at ATH
    let h24_change = token.price_change
        .as_ref()
        .and_then(|pc| pc.h24)
        .unwrap_or(0.0);

    if h24_change > 50.0 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Too high 24h gain {:.1}%", token.symbol, h24_change)
            );
        }
        return false;
    }

    // Check that 5-minute trend is not strongly bearish
    let m5_change = token.price_change
        .as_ref()
        .and_then(|pc| pc.m5)
        .unwrap_or(0.0);

    if m5_change < -10.0 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Strong 5m drop {:.1}%", token.symbol, m5_change)
            );
        }
        return false;
    }

    if is_debug_trader_enabled() {
        log(
            LogTag::Trader,
            "ENTRY_ACCEPT",
            &format!(
                "✅ {} accepted: Liquidity ${:.0}, 24h {:.1}%, 5m {:.1}%",
                token.symbol,
                liquidity_usd,
                h24_change,
                m5_change
            )
        );
    }

    true
}

/// Get simple profit target range based on liquidity
/// Returns (min_profit%, max_profit%)
pub fn get_profit_target(token: &Token) -> (f64, f64) {
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    // Simple tiered profit targets based on liquidity
    match liquidity_usd {
        x if x >= 1_000_000.0 => (5.0, 15.0), // Large tokens: 5-15%
        x if x >= 100_000.0 => (10.0, 30.0), // Medium tokens: 10-30%
        x if x >= 10_000.0 => (20.0, 50.0), // Small tokens: 20-50%
        _ => (30.0, 100.0), // Micro tokens: 30-100%
    }
}
