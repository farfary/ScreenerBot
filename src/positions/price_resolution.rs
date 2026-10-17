// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Price resolution with API fallback.
//!
//! Resolves token prices for position operations using a priority cascade:
//! 1. Pool price (real-time on-chain calculation) — always preferred
//! 2. API price from Token database (DexScreener/GeckoTerminal) — if fresh enough
//! 3. Force fetch fresh API price if stale — immediate update and retry

use super::types::PriceSource;
use crate::chains::chain_for_address;
use crate::logger::{self, LogTag};
use crate::pools::{get_pool_price, PriceResult};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Maximum age (in seconds) for API price to be considered valid for trading
const API_PRICE_MAX_AGE_SECS: i64 = 60;

/// Minimum interval between force-fetch attempts for the same mint. This is the
/// single throttle shared by every caller of the resolver (the 1s position price
/// updater plus the trading paths) — without it a token whose API fetch keeps
/// failing (e.g. delisted) would be force-fetched on every updater cycle.
const FORCE_FETCH_COOLDOWN_SECS: u64 = 30;

/// Bounded set of mints with a recent force-fetch attempt. Presence = in cooldown
/// (entries auto-expire after `FORCE_FETCH_COOLDOWN_SECS` via TTL).
static FORCE_FETCH_COOLDOWN: LazyLock<moka::sync::Cache<String, ()>> = LazyLock::new(|| {
    moka::sync::Cache::builder()
        .max_capacity(10_000)
        .time_to_live(Duration::from_secs(FORCE_FETCH_COOLDOWN_SECS))
        .build()
});

/// The live pool price of a position's token, on the chain its address
/// belongs to. A mint no enabled chain accepts has no pool price.
pub fn live_pool_price(mint: &str) -> Option<PriceResult> {
    chain_for_address(mint)
        .ok()
        .and_then(|chain| get_pool_price(chain, mint))
}

/// Get price with fallback to API when pool price unavailable
///
/// Priority:
/// 1. Pool price (real-time on-chain calculation) - always preferred
/// 2. API price from Token database (DexScreener/GeckoTerminal) - if fresh enough
/// 3. Force fetch fresh API price if stale and retry
///
/// An API price is used only when the two market sources do not contradict each other
/// (see [`sources_agree`]): a source can report a pair with fabricated liquidity as a
/// token's main market, and a position opened at its price carries a peak and an entry
/// that no trade ever paid.
///
/// This enables trading for tokens not yet in the pool service price list
pub async fn get_price_with_api_fallback(token_mint: &str) -> Option<(PriceResult, PriceSource)> {
    // The token's chain, resolved once for both the pool and the database read.
    // A mint no enabled chain accepts has neither a pool price nor a stored token.
    let chain = chain_for_address(token_mint).ok();

    // Priority 1: Try pool price (real-time on-chain data)
    if let Some(price_result) = chain.and_then(|chain| get_pool_price(chain, token_mint)) {
        if price_result.price_native > 0.0 && price_result.price_native.is_finite() {
            return Some((price_result, PriceSource::Pool));
        }
    }

    let (price_result, source) = api_price(chain, token_mint).await?;
    let chain = chain?;
    match crate::tokens::get_market_prices_sol_async(chain, token_mint).await {
        Ok((dexscreener, geckoterminal)) if !sources_agree(dexscreener, geckoterminal) => {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "No reliable price for {token_mint}: market sources disagree (DexScreener {dexscreener:?} SOL, GeckoTerminal {geckoterminal:?} SOL) and no pool price is known yet"
                ),
            );
            None
        }
        _ => Some((price_result, source)),
    }
}

/// Largest ratio between the two market sources' SOL prices at which an API price is
/// still trusted.
const MAX_API_SOURCE_DISAGREEMENT: f64 = 2.0;

/// True unless both market sources price the token and their prices differ by more than
/// [`MAX_API_SOURCE_DISAGREEMENT`]. One source alone cannot be contradicted.
pub fn sources_agree(dexscreener: Option<f64>, geckoterminal: Option<f64>) -> bool {
    match (dexscreener, geckoterminal) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => {
            a.max(b) / a.min(b) <= MAX_API_SOURCE_DISAGREEMENT
        }
        _ => true,
    }
}

/// The stored API price of `token_mint`, refreshed when stale or missing.
async fn api_price(
    chain: Option<crate::chains::ChainId>,
    token_mint: &str,
) -> Option<(PriceResult, PriceSource)> {
    // Priority 2: Try API price from token database (DexScreener/GeckoTerminal).
    let stored = match chain {
        Some(chain) => crate::tokens::get_full_token_async(chain, token_mint).await,
        None => Ok(None),
    };
    match stored {
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
                    price_native: token.price_sol,
                    price_usd: token.price_usd,
                    confidence: 0.8, // Lower confidence for API price vs on-chain
                    source_pool: Some("api".to_owned()),
                    pool_address: token.pool_price_last_used_pool.clone().unwrap_or_default(),
                    slot: 0,
                    timestamp: Instant::now(),
                    native_reserves: 0.0,
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
                        price_native: fresh_token.price_sol,
                        price_usd: fresh_token.price_usd,
                        confidence: 0.8,
                        source_pool: Some("api_fresh".to_owned()),
                        pool_address: fresh_token
                            .pool_price_last_used_pool
                            .clone()
                            .unwrap_or_default(),
                        slot: 0,
                        timestamp: Instant::now(),
                        native_reserves: 0.0,
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
                    price_native: fresh_token.price_sol,
                    price_usd: fresh_token.price_usd,
                    confidence: 0.8,
                    source_pool: Some("api_fresh".to_owned()),
                    pool_address: fresh_token
                        .pool_price_last_used_pool
                        .clone()
                        .unwrap_or_default(),
                    slot: 0,
                    timestamp: Instant::now(),
                    native_reserves: 0.0,
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
    // Throttle: skip if we force-fetched this mint within the cooldown window.
    // request_immediate_update has no dedup of its own, so this is the only guard
    // against hammering the API for a perpetually-stale (e.g. delisted) token.
    if FORCE_FETCH_COOLDOWN.get(token_mint).is_some() {
        return None;
    }
    FORCE_FETCH_COOLDOWN.insert(token_mint.to_string(), ());

    // Request immediate market data update from tokens system
    let update = match chain_for_address(token_mint) {
        Ok(chain) => crate::tokens::request_immediate_update(chain, token_mint)
            .await
            .map(|result| (chain, result)),
        Err(error) => Err(crate::tokens::Error::from(error)),
    };
    match update {
        Ok((chain, result)) => {
            if result.is_success() {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Force fetch succeeded for {}: {:?}",
                        token_mint, result.successes
                    ),
                );

                // Re-fetch token data after update
                match crate::tokens::get_full_token_async(chain, token_mint).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_prices_are_trusted_unless_the_two_sources_contradict_each_other() {
        // A fabricated JUP/MET pair priced JUP at 14.197 SOL; the real market was 0.00288.
        assert!(!sources_agree(Some(14.197_378), Some(0.002_883)));
        assert!(!sources_agree(Some(0.002_883), Some(14.197_378)));
        assert!(sources_agree(Some(0.002_90), Some(0.002_88)));
        assert!(
            sources_agree(Some(2.0), Some(1.0)),
            "a factor of two is within tolerance"
        );
        assert!(
            sources_agree(Some(14.197_378), None),
            "one source cannot be contradicted"
        );
        assert!(sources_agree(None, None));
    }

    #[test]
    fn entry_swap_cost_does_not_scale_later_pool_prices() {
        let removed_correction = concat!("apply_pool_bias_", "correction");
        let source = include_str!("price_resolution.rs");
        assert!(!source.contains(removed_correction));
        assert!(!include_str!("price_updater.rs").contains(removed_correction));
        assert!(!include_str!("../trader/evaluators/exit.rs").contains(removed_correction));
    }
}
