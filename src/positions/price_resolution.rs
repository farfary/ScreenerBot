// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Price resolution with API fallback.
//!
//! Resolves token prices for position operations using a priority cascade:
//! 1. Pool price (real-time on-chain calculation) — always preferred
//! 2. API price from Token database (DexScreener/GeckoTerminal) — if fresh enough
//! 3. Force fetch fresh API price if stale — immediate update and retry
//!
//! Entry decisions read [`entry_price`]: with live pool prices only the pool
//! price, never an API price; without them a recent API price both market
//! sources agree on. Synchronous readers (filtering, wallet worth, strategy
//! context) read [`cached_trading_price`], which never fetches.

use super::types::PriceSource;
use crate::chains::access::{self as chain_access, Capability};
use crate::chains::{chain_for_address, ChainId};
use crate::logger::{self, LogTag};
use crate::pools::{get_pool_price, PriceResult};
use crate::tokens::Token;
use std::collections::HashSet;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Maximum age (in seconds) for API price to be considered valid for trading
const API_PRICE_MAX_AGE_SECS: i64 = 60;

/// Maximum age (in seconds) of an API price an entry decision accepts on an
/// install without live pool prices. Tighter than the exit bound: an entry can
/// always wait for the next market refresh, an exit cannot.
const ENTRY_API_PRICE_MAX_AGE_SECS: i64 = 20;

/// Confidence of a price read from the market APIs rather than a pool.
const API_PRICE_CONFIDENCE: f32 = 0.8;

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

    let chain = chain?;
    let (price_result, source) = api_price(chain, token_mint).await?;
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
async fn api_price(chain: ChainId, token_mint: &str) -> Option<(PriceResult, PriceSource)> {
    // Priority 2: Try API price from token database (DexScreener/GeckoTerminal).
    match crate::tokens::get_full_token_async(chain, token_mint).await {
        Ok(Some(token)) => {
            let age_secs = api_price_age_secs(&token, chrono::Utc::now());

            if age_secs <= API_PRICE_MAX_AGE_SECS && valid_price(token.price_sol) {
                logger::debug(
                    LogTag::Positions,
                    &format!(
                        "Using API price for {} (no pool price): {} SOL (age: {}s, source: {:?})",
                        token_mint, token.price_sol, age_secs, token.data_source
                    ),
                );
                return Some((api_price_result(chain, &token, "api"), PriceSource::Api));
            } else if age_secs > API_PRICE_MAX_AGE_SECS {
                // Price is stale - force fetch fresh data and retry
                logger::info(
                    LogTag::Positions,
                    &format!(
                        "API price stale for {} (age: {}s > max {}s), forcing fresh fetch...",
                        token_mint, age_secs, API_PRICE_MAX_AGE_SECS
                    ),
                );

                if let Some(fresh_token) = force_fetch_fresh_price(token_mint).await {
                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Using freshly fetched API price for {}: {} SOL (source: {:?})",
                            token_mint, fresh_token.price_sol, fresh_token.data_source
                        ),
                    );
                    return Some((
                        api_price_result(chain, &fresh_token, "api_fresh"),
                        PriceSource::Api,
                    ));
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
                logger::info(
                    LogTag::Positions,
                    &format!(
                        "Using newly fetched API price for {}: {} SOL",
                        token_mint, fresh_token.price_sol
                    ),
                );
                return Some((
                    api_price_result(chain, &fresh_token, "api_fresh"),
                    PriceSource::Api,
                ));
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

/// The price an entry decision is made on. With live pool prices: the pool
/// price only, never an API price. Without them: the stored API price, when it
/// is at most [`ENTRY_API_PRICE_MAX_AGE_SECS`] old and the two market sources
/// do not contradict each other. Never fetches.
pub async fn entry_price(chain: ChainId, mint: &str) -> Option<PriceResult> {
    if chain_access::has(chain, Capability::LivePoolPrices) {
        return get_pool_price(chain, mint).filter(|price| valid_price(price.price_native));
    }
    let token = crate::tokens::get_full_token_async(chain, mint)
        .await
        .ok()
        .flatten()?;
    let (dexscreener, geckoterminal) = crate::tokens::get_market_prices_sol_async(chain, mint)
        .await
        .ok()?;
    entry_api_price_usable(
        token.price_sol,
        api_price_age_secs(&token, chrono::Utc::now()),
        dexscreener,
        geckoterminal,
    )
    .then(|| api_price_result(chain, &token, "api"))
}

/// Whether an API price may decide an entry: a positive finite price no older
/// than [`ENTRY_API_PRICE_MAX_AGE_SECS`] that the market sources agree on.
fn entry_api_price_usable(
    price_native: f64,
    age_secs: i64,
    dexscreener: Option<f64>,
    geckoterminal: Option<f64>,
) -> bool {
    valid_price(price_native)
        && (0..=ENTRY_API_PRICE_MAX_AGE_SECS).contains(&age_secs)
        && sources_agree(dexscreener, geckoterminal)
}

/// The price trading reads synchronously, without fetching: the live pool
/// price, else, on an install without live pool prices, the cached token's API
/// price when it is at most [`API_PRICE_MAX_AGE_SECS`] old.
pub fn cached_trading_price(chain: ChainId, mint: &str) -> Option<PriceResult> {
    if let Some(price) = get_pool_price(chain, mint) {
        return valid_price(price.price_native).then_some(price);
    }
    if chain_access::has(chain, Capability::LivePoolPrices) {
        return None;
    }
    let token = crate::tokens::get_cached_token(chain, mint)?;
    let age_secs = api_price_age_secs(&token, chrono::Utc::now());
    ((0..=API_PRICE_MAX_AGE_SECS).contains(&age_secs) && valid_price(token.price_sol))
        .then(|| api_price_result(chain, &token, "api"))
}

/// The mints of `chain` with a live pool price, or `None` on an install
/// without live pool prices, where every candidate is priced per mint by
/// [`entry_price`].
pub fn live_priced_mints(chain: ChainId) -> Option<Vec<String>> {
    chain_access::has(chain, Capability::LivePoolPrices)
        .then(|| crate::pools::get_available_tokens(chain))
}

/// The mints among `mints` that have a trading price now: the live pool set
/// with live pool prices, else each mint with a [`cached_trading_price`].
pub fn priced_mints(chain: ChainId, mints: &[String]) -> HashSet<String> {
    match live_priced_mints(chain) {
        Some(pool_mints) => pool_mints.into_iter().collect(),
        None => mints
            .iter()
            .filter(|mint| cached_trading_price(chain, mint).is_some())
            .cloned()
            .collect(),
    }
}

/// The one [`PriceResult`] built from a token's API market data.
fn api_price_result(chain: ChainId, token: &Token, source: &str) -> PriceResult {
    PriceResult {
        mint: token.mint.clone(),
        price_native: token.price_sol,
        price_usd: token.price_usd,
        confidence: API_PRICE_CONFIDENCE,
        source_pool: Some(source.to_owned()),
        pool_address: token.pool_price_last_used_pool.clone().unwrap_or_default(),
        slot: 0,
        timestamp: Instant::now(),
        native_reserves: api_native_reserves(chain, token.liquidity_usd),
        token_reserves: 0.0,
    }
}

/// The native side of an API-priced token's liquidity. With live pool prices
/// an API price carries no reserves (liquidity rules read pool reserves only);
/// without them it is estimated from the API's USD liquidity.
fn api_native_reserves(chain: ChainId, liquidity_usd: Option<f64>) -> f64 {
    if chain_access::has(chain, Capability::LivePoolPrices) {
        return 0.0;
    }
    liquidity_estimate_native(
        liquidity_usd,
        crate::apis::native_price::get_native_price_for(chain),
    )
}

/// Half a pool's USD liquidity in the native coin: the native side of a
/// two-sided pool. Zero when either input is unknown.
fn liquidity_estimate_native(liquidity_usd: Option<f64>, native_usd: f64) -> f64 {
    match liquidity_usd {
        Some(liquidity) if valid_price(liquidity) && valid_price(native_usd) => {
            liquidity / 2.0 / native_usd
        }
        _ => 0.0,
    }
}

/// Seconds since the token's market data was fetched.
fn api_price_age_secs(token: &Token, now: chrono::DateTime<chrono::Utc>) -> i64 {
    now.signed_duration_since(token.market_data_last_fetched_at)
        .num_seconds()
}

/// A positive, finite amount.
fn valid_price(value: f64) -> bool {
    value > 0.0 && value.is_finite()
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

    // Request immediate market data update from tokens system. The monitor retries this
    // fallback on its own cadence and DexScreener answers alongside, so GeckoTerminal is
    // asked on the background lane: during a rate-limit penalty it is deferred instead of
    // holding the shared permit past the on-demand deadline.
    let update = match chain_for_address(token_mint) {
        Ok(chain) => crate::tokens::request_immediate_update(
            chain,
            token_mint,
            crate::apis::geckoterminal::RequestLane::Background,
        )
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
    fn an_entry_reads_only_a_recent_api_price_the_sources_agree_on() {
        assert!(entry_api_price_usable(0.002, 0, Some(0.002), Some(0.0021)));
        assert!(entry_api_price_usable(
            0.002,
            ENTRY_API_PRICE_MAX_AGE_SECS,
            None,
            None
        ));
        assert!(
            !entry_api_price_usable(0.002, ENTRY_API_PRICE_MAX_AGE_SECS + 1, None, None),
            "an API price older than the entry bound never decides an entry"
        );
        assert!(
            !entry_api_price_usable(0.002, -5, None, None),
            "a fetch time in the future is not trusted"
        );
        assert!(
            !entry_api_price_usable(0.002, 1, Some(14.197), Some(0.002)),
            "contradicting market sources block an entry"
        );
        assert!(!entry_api_price_usable(0.0, 1, None, None));
        assert!(!entry_api_price_usable(f64::NAN, 1, None, None));
        assert!(
            ENTRY_API_PRICE_MAX_AGE_SECS <= API_PRICE_MAX_AGE_SECS,
            "the entry bound never exceeds the exit bound"
        );
    }

    #[test]
    fn api_liquidity_is_half_the_usd_liquidity_in_the_native_coin() {
        assert_eq!(liquidity_estimate_native(Some(20_000.0), 200.0), 50.0);
        assert_eq!(liquidity_estimate_native(None, 200.0), 0.0);
        assert_eq!(liquidity_estimate_native(Some(20_000.0), 0.0), 0.0);
        assert_eq!(liquidity_estimate_native(Some(f64::INFINITY), 200.0), 0.0);
        assert_eq!(liquidity_estimate_native(Some(-1.0), 200.0), 0.0);
    }

    /// Every API price is built by one constructor.
    #[test]
    fn api_prices_have_one_constructor() {
        let source = include_str!("price_resolution.rs");
        let literal = concat!("PriceResult", " {");
        let return_type = concat!("-> PriceResult", " {");
        let constructions = source.matches(literal).count() - source.matches(return_type).count();
        assert_eq!(constructions, 1);
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
