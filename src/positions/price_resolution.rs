//! Price resolution with API fallback.
//!
//! Resolves token prices for position operations using a priority cascade:
//! 1. Pool price (real-time on-chain calculation) — always preferred
//! 2. API price from Token database (DexScreener/GeckoTerminal) — if fresh enough
//! 3. Force fetch fresh API price if stale — immediate update and retry

use super::types::PriceSource;
use crate::logger::{self, LogTag};
use crate::pools::{get_pool_price, PriceResult};
use std::time::Instant;

/// Maximum age (in seconds) for API price to be considered valid for trading
const API_PRICE_MAX_AGE_SECS: i64 = 60;

/// Get price with fallback to API when pool price unavailable
///
/// Priority:
/// 1. Pool price (real-time on-chain calculation) - always preferred
/// 2. API price from Token database (DexScreener/GeckoTerminal) - if fresh enough
/// 3. Force fetch fresh API price if stale and retry
///
/// This enables trading for tokens not yet in the pool service price list
pub async fn get_price_with_api_fallback(token_mint: &str) -> Option<(PriceResult, PriceSource)> {
    // Priority 1: Try pool price (real-time on-chain data)
    if let Some(price_result) = get_pool_price(token_mint) {
        if price_result.price_sol > 0.0 && price_result.price_sol.is_finite() {
            return Some((price_result, PriceSource::Pool));
        }
    }

    // Priority 2: Try API price from token database (DexScreener/GeckoTerminal)
    match crate::tokens::get_full_token_async(token_mint).await {
        Ok(Some(token)) => {
            // Check if API price is fresh enough (within max age)
            let now = chrono::Utc::now();
            let age_secs = now
                .signed_duration_since(token.market_data_last_fetched_at)
                .num_seconds();

            if age_secs <= API_PRICE_MAX_AGE_SECS
                && token.price_sol > 0.0
                && token.price_sol.is_finite()
            {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Using API price for {} (no pool price): {} SOL (age: {}s, source: {:?})",
                        token_mint, token.price_sol, age_secs, token.data_source
                    ),
                );

                // Convert Token API price to PriceResult
                let price_result = PriceResult {
                    mint: token_mint.to_string(),
                    price_sol: token.price_sol,
                    price_usd: token.price_usd,
                    confidence: 0.8, // Lower confidence for API price vs on-chain
                    source_pool: Some("api".to_owned()),
                    pool_address: token.pool_price_last_used_pool.clone().unwrap_or_default(),
                    slot: 0,
                    timestamp: Instant::now(),
                    sol_reserves: 0.0,
                    token_reserves: 0.0,
                };

                return Some((price_result, PriceSource::Api));
            } else if age_secs > API_PRICE_MAX_AGE_SECS {
                // Price is stale - force fetch fresh data and retry
                logger::info(
                    LogTag::Positions,
                    &format!(
                        "API price stale for {} (age: {}s > max {}s), forcing fresh fetch...",
                        token_mint, age_secs, API_PRICE_MAX_AGE_SECS
                    ),
                );

                // Force immediate market data update
                if let Some(fresh_token) = force_fetch_fresh_price(token_mint).await {
                    let price_result = PriceResult {
                        mint: token_mint.to_string(),
                        price_sol: fresh_token.price_sol,
                        price_usd: fresh_token.price_usd,
                        confidence: 0.8,
                        source_pool: Some("api_fresh".to_owned()),
                        pool_address: fresh_token
                            .pool_price_last_used_pool
                            .clone()
                            .unwrap_or_default(),
                        slot: 0,
                        timestamp: Instant::now(),
                        sol_reserves: 0.0,
                        token_reserves: 0.0,
                    };

                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Using freshly fetched API price for {}: {} SOL (source: {:?})",
                            token_mint, fresh_token.price_sol, fresh_token.data_source
                        ),
                    );

                    return Some((price_result, PriceSource::Api));
                }
            }
        }
        Ok(None) => {
            // Token not in database - try to force fetch it
            logger::info(
                LogTag::Positions,
                &format!(
                    "No token data found for {}, attempting to fetch...",
                    token_mint
                ),
            );

            if let Some(fresh_token) = force_fetch_fresh_price(token_mint).await {
                let price_result = PriceResult {
                    mint: token_mint.to_string(),
                    price_sol: fresh_token.price_sol,
                    price_usd: fresh_token.price_usd,
                    confidence: 0.8,
                    source_pool: Some("api_fresh".to_owned()),
                    pool_address: fresh_token
                        .pool_price_last_used_pool
                        .clone()
                        .unwrap_or_default(),
                    slot: 0,
                    timestamp: Instant::now(),
                    sol_reserves: 0.0,
                    token_reserves: 0.0,
                };

                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Using newly fetched API price for {}: {} SOL",
                        token_mint, fresh_token.price_sol
                    ),
                );

                return Some((price_result, PriceSource::Api));
            }
        }
        Err(e) => {
            logger::debug(
                LogTag::Positions,
                &format!("Failed to get token data for {token_mint}: {e}"),
            );
        }
    }

    None
}

/// Force fetch fresh price data from API and return updated token
async fn force_fetch_fresh_price(token_mint: &str) -> Option<crate::tokens::Token> {
    // Request immediate market data update from tokens system
    match crate::tokens::request_immediate_update(token_mint).await {
        Ok(result) => {
            if result.is_success() {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Force fetch succeeded for {}: {:?}",
                        token_mint, result.successes
                    ),
                );

                // Re-fetch token data after update
                match crate::tokens::get_full_token_async(token_mint).await {
                    Ok(Some(token)) => {
                        // Verify the price is valid
                        if token.price_sol > 0.0 && token.price_sol.is_finite() {
                            return Some(token);
                        } else {
                            logger::warning(
                                LogTag::Positions,
                                &format!(
                                    "Force fetched token {} has invalid price: {}",
                                    token_mint, token.price_sol
                                ),
                            );
                        }
                    }
                    Ok(None) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!("Token {token_mint} not found after force fetch"),
                        );
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!(
                                "Failed to get token after force fetch for {}: {}",
                                token_mint, e
                            ),
                        );
                    }
                }
            } else {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Force fetch failed for {}: {:?}",
                        token_mint, result.failures
                    ),
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Positions,
                &format!("Force fetch error for {token_mint}: {e}"),
            );
        }
    }

    None
}
