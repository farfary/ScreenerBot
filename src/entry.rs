/// Pool-based entry logic for ScreenerBot
///
/// This module provides pool price-based entry decisions with -10% drop detection.
/// Uses real-time blockchain pool data for trading decisions while API data is used only for validation.

use crate::tokens::Token;
use crate::tokens::pool::get_pool_service;
use crate::tokens::is_token_excluded_from_trading;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;

/// Pool-based entry decision function with -10% drop detection
/// Returns true if the token should be bought based on pool price movement
pub async fn should_buy(token: &Token) -> bool {
    // 0. ABSOLUTE FIRST: Check blacklist and exclusion status
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Token is blacklisted or excluded", token.symbol)
            );
        }
        return false;
    }

    // Get pool service for real-time price data
    let pool_service = get_pool_service();

    // Check if pool price is available for this token
    if !pool_service.check_token_availability(&token.mint).await {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: No pool price available", token.symbol)
            );
        }
        return false;
    }

    // Get current pool price
    let current_pool_price = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => price,
                _ => {
                    if is_debug_trader_enabled() {
                        log(
                            LogTag::Trader,
                            "ENTRY_REJECT",
                            &format!("❌ {} rejected: Invalid pool price", token.symbol)
                        );
                    }
                    return false;
                }
            }
        }
        None => {
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "ENTRY_REJECT",
                    &format!("❌ {} rejected: Pool price calculation failed", token.symbol)
                );
            }
            return false;
        }
    };

    // Get recent price history for -10% drop detection
    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    if price_history.len() < 2 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!(
                    "❌ {} rejected: Insufficient price history for drop detection",
                    token.symbol
                )
            );
        }
        return false;
    }

    // Find recent high (highest price in last 5 data points)
    let recent_high = price_history
        .iter()
        .rev()
        .take(5)
        .map(|(_, price)| *price)
        .fold(0.0f64, f64::max);

    if recent_high <= 0.0 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Invalid recent high price", token.symbol)
            );
        }
        return false;
    }

    // Calculate price drop percentage
    let price_drop_percent = ((recent_high - current_pool_price) / recent_high) * 100.0;

    // Check for -10% drop trigger
    if price_drop_percent >= 10.0 {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_ACCEPT",
                &format!(
                    "✅ {} accepted: -{:.1}% drop detected (high: {:.12}, current: {:.12} SOL)",
                    token.symbol,
                    price_drop_percent,
                    recent_high,
                    current_pool_price
                )
            );
        }
        return true;
    } else {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!(
                    "❌ {} rejected: Only -{:.1}% drop (need -10%+, high: {:.12}, current: {:.12} SOL)",
                    token.symbol,
                    price_drop_percent,
                    recent_high,
                    current_pool_price
                )
            );
        }
        return false;
    }
}

/// Get profit target range based on pool liquidity
/// Returns (min_profit%, max_profit%)
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    // Get pool service for real-time liquidity data
    let pool_service = get_pool_service();

    // Try to get current pool data for accurate liquidity
    let liquidity_usd = if
        let Some(pool_result) = pool_service.get_pool_price(&token.mint, None).await
    {
        pool_result.liquidity_usd
    } else {
        // Fallback to API liquidity data
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    };

    // Simple tiered profit targets based on liquidity
    match liquidity_usd {
        x if x >= 1_000_000.0 => (5.0, 15.0), // Large tokens: 5-15%
        x if x >= 100_000.0 => (10.0, 30.0), // Medium tokens: 10-30%
        x if x >= 10_000.0 => (20.0, 50.0), // Small tokens: 20-50%
        _ => (30.0, 100.0), // Micro tokens: 30-100%
    }
}
