// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! DexScreener market data fetching and caching
//!
//! Flow: API -> Parse -> Database -> Store cache
//! Updates: Every 30 seconds for active tokens
//!
//! Architecture:
//! - Uses /tokens/v1 batch endpoint (up to 30 tokens per request)
//! - Returns ONE best pool per token (DexScreener picks most liquid)
//! - No pool filtering logic (trust DexScreener API)

use crate::apis::dexscreener::DexScreenerPool;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::persist;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{DataSource, DexScreenerData, TokenResult};
use crate::tokens::Error;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Convert API pool data to our DexScreenerData type
fn convert_pool_to_data(pool: &DexScreenerPool, is_native_pair: bool) -> DexScreenerData {
    fn parse_f64(value: &str) -> Option<f64> {
        value.parse::<f64>().ok()
    }

    fn combine_txns(buys: Option<i64>, sells: Option<i64>) -> Option<(u32, u32)> {
        match (buys, sells) {
            (Some(b), Some(s)) if b >= 0 && s >= 0 => Some((b as u32, s as u32)),
            _ => None,
        }
    }

    let price_usd = parse_f64(&pool.price_usd).unwrap_or_default();

    // Calculate price_sol based on pool type
    let price_sol = if is_native_pair {
        // For SOL-paired pools, priceNative IS the SOL price
        parse_f64(&pool.price_native).unwrap_or_default()
    } else {
        // For non-SOL pairs, derive SOL price from USD: price_sol = price_usd / sol_usd_price.
        // priceNative here is denominated in the quote token (e.g. USDC), NOT SOL — using it as
        // a fallback would emit a USDC value mislabelled as SOL and corrupt P&L/quotes. When the
        // SOL price is unavailable we leave price_sol at 0.0 so downstream treats it as unpriced.
        let native_price = crate::native_price::get_native_price();
        if native_price > 0.0 {
            price_usd / native_price
        } else {
            0.0
        }
    };

    DexScreenerData {
        price_usd,
        price_sol,
        price_native: pool.price_native.clone(),
        price_change_5m: pool.price_change_m5,
        price_change_1h: pool.price_change_h1,
        price_change_6h: pool.price_change_h6,
        price_change_24h: pool.price_change_h24,
        market_cap: pool.market_cap,
        fdv: pool.fdv,
        liquidity_usd: pool.liquidity_usd,
        volume_5m: pool.volume_m5,
        volume_1h: pool.volume_h1,
        volume_6h: pool.volume_h6,
        volume_24h: pool.volume_h24,
        txns_5m: combine_txns(pool.txns_m5_buys, pool.txns_m5_sells),
        txns_1h: combine_txns(pool.txns_h1_buys, pool.txns_h1_sells),
        txns_6h: combine_txns(pool.txns_h6_buys, pool.txns_h6_sells),
        txns_24h: combine_txns(pool.txns_h24_buys, pool.txns_h24_sells),
        pair_address: if pool.pair_address.is_empty() {
            None
        } else {
            Some(pool.pair_address.clone())
        },
        chain_id: if pool.chain_id.is_empty() {
            None
        } else {
            Some(pool.chain_id.clone())
        },
        dex_id: if pool.dex_id.is_empty() {
            None
        } else {
            Some(pool.dex_id.clone())
        },
        url: pool.url.clone(),
        image_url: pool.info_image_url.clone(),
        header_image_url: pool.info_header.clone(),
        pair_blockchain_created_at: pool
            .pair_created_at
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        market_data_last_fetched_at: Utc::now(),
        market_data_first_fetched_at: Utc::now(),
    }
}

/// Fetch DexScreener data for multiple tokens in a single batch API call
///
/// Uses /tokens/v1 endpoint which returns ONE best pool per token.
/// DexScreener API automatically picks the most liquid pool.
///
/// Flow per token:
/// 1. Check cache (if fresh, skip API)
/// 2. Check database (if fresh < 30s, use it)
/// 3. Fetch from API (batch endpoint)
/// 4. Store in database + cache
///
/// # Arguments
/// * `mints` - Token mint addresses (up to 30)
/// * `db` - Database instance
///
/// # Returns
/// HashMap mapping mint -> Option<DexScreenerData>
/// - Some(data) if token has market data
/// - None if the API answered and listed no pair for the token
///
/// # Errors
/// `Error::Api` when the endpoint is unhealthy (no request is made) or the batch
/// request fails. No mint is reported as `None` in that case, so a failure can
/// never be mistaken for "not listed".
///
/// A token whose answer could not be stored is left out of the map, so the caller
/// counts it as a failed fetch, never as "not listed".
pub async fn fetch_dexscreener_data_batch(
    mints: &[String],
    db: &TokenDatabase,
) -> TokenResult<HashMap<String, Option<DexScreenerData>>> {
    fetch_and_store(mints, db).await.map(|(answers, _)| answers)
}

/// The answers of one batch fetch, and the error of every token whose answer could not
/// be stored.
type BatchOutcome = (
    HashMap<String, Option<DexScreenerData>>,
    HashMap<String, Error>,
);

async fn fetch_and_store(mints: &[String], db: &TokenDatabase) -> TokenResult<BatchOutcome> {
    // Check connectivity before API call - skip the request only when the
    // endpoint is EXPLICITLY Unhealthy. An `Unknown` state (the health monitor
    // hasn't completed its first probe yet — every endpoint starts Unknown at
    // boot) must NOT suppress fetches: otherwise, right after the app launches,
    // a user opening a token dialog gets all DexScreener fetches skipped until
    // the first health check lands, making token details look empty/"stuck".
    // (Previously used `check_endpoints_healthy`, which maps Unknown→unhealthy.)
    let connectivity_ok = !crate::connectivity::get_endpoint_health("dexscreener")
        .await
        .map(|h| h.is_unhealthy())
        .unwrap_or(false);

    if !connectivity_ok {
        logger::debug(
            LogTag::Tokens,
            "DexScreener endpoint unhealthy - only cached/DB data can be served",
        );
    }

    if mints.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    let mut results: HashMap<String, Option<DexScreenerData>> = HashMap::new();
    let mut store_failures: HashMap<String, Error> = HashMap::new();
    let mut to_fetch: Vec<String> = Vec::new();

    // Check cache and database for each token
    for mint in mints {
        // 1. Check in-memory cache
        if let Some(data) = store::get_cached_dexscreener(db.chain(), mint) {
            results.insert(mint.clone(), Some(data));
            continue;
        }

        // 2. Check database (if fresh < 30s)
        if let Some(db_data) = db.get_dexscreener_data(mint)? {
            let age_secs = Utc::now()
                .signed_duration_since(db_data.market_data_last_fetched_at)
                .num_seconds();

            // age_secs < 0 means future timestamp (clock skew) - treat as stale
            if age_secs >= 0 && age_secs <= 30 {
                store::store_dexscreener(db.chain(), mint, &db_data);
                if let Err(err) = store::refresh_token_snapshot(db.chain(), mint).await {
                    logger::error(
                        LogTag::Tokens,
                        &format!(
                            "[TOKENS][STORE] Failed to refresh token snapshot after DB hit mint={} err={:?}",
                            mint, err
                        ),
                    );
                }
                results.insert(mint.clone(), Some(db_data));
                continue;
            }
        }

        // Need to fetch from API
        to_fetch.push(mint.clone());
    }

    // If all tokens were cached, return early
    if to_fetch.is_empty() {
        return Ok((results, store_failures));
    }

    // Connectivity is down: the API is not asked, so nothing is known about these
    // tokens. This is a failure, never a "not listed" answer.
    if !connectivity_ok {
        return Err(Error::Api {
            provider: "DexScreener".to_owned(),
            message: format!(
                "endpoint unhealthy, batch of {} tokens not requested",
                to_fetch.len()
            ),
        });
    }

    // 3. Fetch from batch API endpoint
    let api_manager = crate::apis::manager::get_api_manager();
    let pools = api_manager
        .dexscreener
        .fetch_token_batch(&to_fetch, None)
        .await
        .map_err(|e| Error::Api {
            provider: "DexScreener".to_owned(),
            message: format!("batch of {} tokens failed: {:?}", to_fetch.len(), e),
        })?;

    // Process pools - DexScreener batch returns ONE best pool per token
    for pool in pools {
        let mint = &pool.base_token_address;

        // Skip if not in our request list
        if !to_fetch.contains(mint) {
            continue;
        }

        let is_native_pair =
            crate::chains::adapter_for(db.chain()).is_native_asset(&pool.quote_token_address);
        let data = convert_pool_to_data(&pool, is_native_pair);

        // Store the token row (with the pair's symbol and name) and its market data
        let symbol = Some(pool.base_token_symbol.as_str()).filter(|s| !s.is_empty());
        let name = Some(pool.base_token_name.as_str()).filter(|s| !s.is_empty());
        if let Err(e) = persist::store_market_answer(
            db,
            "DEXSCREENER",
            mint,
            persist::AnsweredMetadata {
                symbol,
                name,
                decimals: None,
            },
            || db.upsert_dexscreener_data(mint, &data),
        ) {
            store_failures.insert(mint.clone(), e);
            continue;
        }

        // Cache it
        store::store_dexscreener(db.chain(), mint, &data);
        if let Err(err) = store::refresh_token_snapshot(db.chain(), mint).await {
            logger::error(
                LogTag::Tokens,
                &format!(
                    "[TOKENS][STORE] Failed to refresh token snapshot after API mint={} err={:?}",
                    mint, err
                ),
            );
        }

        results.insert(mint.clone(), Some(data));
    }

    // The API answered without a pair for the rest: not listed
    for mint in &to_fetch {
        if results.contains_key(mint) || store_failures.contains_key(mint) {
            continue;
        }
        store::record_not_listed(db.chain(), DataSource::DexScreener, mint);
        results.insert(mint.clone(), None);
    }

    Ok((results, store_failures))
}

/// Fetch DexScreener data for a single token (wrapper around batch endpoint)
///
/// Flow:
/// 1. Check cache (if fresh, return immediately)
/// 2. Check database (if fresh, cache + return)
/// 3. Fetch from batch API with single token
///
/// # Arguments
/// * `mint` - Token mint address
/// * `db` - Database instance
///
/// # Returns
/// DexScreenerData if found, None if token not listed; `Err` when the lookup failed or
/// the answer could not be stored
pub async fn fetch_dexscreener_data(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<DexScreenerData>> {
    // Use batch endpoint with single token
    let (mut answers, mut store_failures) = fetch_and_store(&[mint.to_string()], db).await?;
    if let Some(error) = store_failures.remove(mint) {
        return Err(error);
    }

    Ok(answers.remove(mint).flatten())
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    store::dexscreener_cache_metrics()
}

/// Return current cache size (for monitoring)
pub fn get_cache_size() -> usize {
    store::dexscreener_cache_size()
}
