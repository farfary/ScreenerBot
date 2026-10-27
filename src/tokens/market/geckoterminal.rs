// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! GeckoTerminal market data fetching and caching
//!
//! Flow: API -> Parse -> Database -> Store cache
//! Updates: on demand only (`tokens::request_immediate_update`); the scheduled market
//! update loops read DexScreener alone
//!
//! Architecture:
//! - Uses /networks/{net}/tokens/multi/{addresses} batch endpoint (up to 30 tokens per request)
//! - Returns aggregated token data (price, volume, market cap, etc.)
//! - No pool filtering logic (uses aggregated metrics from API)

use crate::apis::geckoterminal::RequestLane;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::market::persist;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{DataSource, GeckoTerminalData, TokenResult};
use crate::tokens::Error;
use chrono::Utc;
use std::collections::HashMap;

/// The answers of one batch fetch, and the error of every token whose answer could not
/// be stored.
type BatchOutcome = (
    HashMap<String, Option<GeckoTerminalData>>,
    HashMap<String, Error>,
);

/// Fetch GeckoTerminal data for up to 30 tokens in one call to
/// /networks/{net}/tokens/multi/{addresses}, which returns aggregated token data (price,
/// volume, market cap) without pool details. `lane` is the caller's [`RequestLane`].
///
/// Flow per token:
/// 1. Check cache (if fresh, skip API)
/// 2. Check database (if fresh < 60s, use it)
/// 3. Fetch from API (batch endpoint)
/// 4. Store in database + cache
///
/// A token the API answered without data is `None` (not listed). A token whose answer
/// could not be stored is left out of the answers and reported with its error, so it
/// counts as a failed fetch, never as "not listed".
async fn fetch_and_store(
    mints: &[String],
    db: &TokenDatabase,
    lane: RequestLane,
) -> TokenResult<BatchOutcome> {
    if mints.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    let mut results: HashMap<String, Option<GeckoTerminalData>> = HashMap::new();
    let mut store_failures: HashMap<String, Error> = HashMap::new();
    let mut to_fetch: Vec<String> = Vec::new();

    // Check cache and database for each token
    for mint in mints {
        // 1. Check in-memory cache
        if let Some(data) = store::get_cached_geckoterminal(db.chain(), mint) {
            results.insert(mint.clone(), Some(data));
            continue;
        }

        // 2. Check database (if fresh < 60s)
        if let Some(db_data) = db.get_geckoterminal_data(mint)? {
            let age_secs = Utc::now()
                .signed_duration_since(db_data.market_data_last_fetched_at)
                .num_seconds();

            if age_secs >= 0 && age_secs <= 60 {
                store::store_geckoterminal(db.chain(), mint, &db_data);
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

    // 3. Fetch from batch API endpoint
    let api_manager = crate::apis::manager::get_api_manager();
    let addresses_str = to_fetch.join(",");

    let tokens_response = api_manager
        .geckoterminal
        .fetch_tokens_multi(
            crate::chains::adapter_for(db.chain()).market_data_network(),
            &addresses_str,
            None,
            None,
            lane,
        )
        .await
        .map_err(|e| Error::Api {
            provider: "GeckoTerminal".to_owned(),
            message: format!("{:?}", e),
        })?;

    // Process each token from response
    for token_info in tokens_response.data {
        let mint = &token_info.attributes.address;
        let attrs = &token_info.attributes;

        let price_usd = attrs
            .price_usd
            .as_ref()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or_default();
        let native_price = crate::native_price::get_native_price();
        let price_sol = if native_price > 0.0 {
            price_usd / native_price
        } else {
            0.0
        };
        let volume_usd = attrs.volume_usd.as_ref();

        let data = GeckoTerminalData {
            price_usd,
            price_sol,
            price_native: attrs.price_usd.clone().unwrap_or_default(),
            price_change_5m: volume_usd.and_then(|v| v.m5.as_ref().and_then(|s| s.parse().ok())),
            price_change_1h: None,
            price_change_6h: None,
            price_change_24h: None,
            market_cap: attrs.market_cap_usd.as_ref().and_then(|s| s.parse().ok()),
            fdv: attrs.fdv_usd.as_ref().and_then(|s| s.parse().ok()),
            liquidity_usd: attrs
                .total_reserve_in_usd
                .as_ref()
                .and_then(|s| s.parse().ok()),
            volume_5m: volume_usd.and_then(|v| v.m5.as_ref().and_then(|s| s.parse().ok())),
            volume_1h: volume_usd.and_then(|v| v.h1.as_ref().and_then(|s| s.parse().ok())),
            volume_6h: volume_usd.and_then(|v| v.h6.as_ref().and_then(|s| s.parse().ok())),
            volume_24h: volume_usd.and_then(|v| v.h24.as_ref().and_then(|s| s.parse().ok())),
            pool_count: None,
            top_pool_address: None,
            reserve_in_usd: attrs
                .total_reserve_in_usd
                .as_ref()
                .and_then(|s| s.parse().ok()),
            image_url: attrs.image_url.clone(),
            market_data_last_fetched_at: Utc::now(),
            market_data_first_fetched_at: Utc::now(),
        };

        // Store the token row (with symbol, name and decimals) and its market data.
        // Decimals are written only while none is known: an upsert replaces the stored
        // value, and a chain-resolved value must never be overwritten by an API value.
        let decimals = attrs
            .decimals
            .filter(|d| crate::tokens::decimals::is_valid(*d))
            .filter(|_| crate::tokens::decimals::get_cached(db.chain(), mint).is_none());
        if let Err(e) = persist::store_market_answer(
            db,
            "GECKOTERMINAL",
            mint,
            persist::AnsweredMetadata {
                symbol: Some(attrs.symbol.as_str()).filter(|s| !s.is_empty()),
                name: Some(attrs.name.as_str()).filter(|s| !s.is_empty()),
                decimals,
            },
            || db.upsert_geckoterminal_data(mint, &data),
        ) {
            store_failures.insert(mint.clone(), e);
            continue;
        }

        // Cache it
        store::store_geckoterminal(db.chain(), mint, &data);
        store::refresh_token_snapshot(db.chain(), mint).await.ok();

        results.insert(mint.clone(), Some(data));
    }

    // The API answered without data for the rest: not listed
    for mint in &to_fetch {
        if results.contains_key(mint) || store_failures.contains_key(mint) {
            continue;
        }
        store::record_not_listed(db.chain(), DataSource::GeckoTerminal, mint);
        results.insert(mint.clone(), None);
    }

    Ok((results, store_failures))
}

/// Fetch GeckoTerminal data for a single token (wrapper around batch endpoint)
///
/// Flow:
/// 1. Check cache (if fresh, return immediately)
/// 2. Check database (if fresh, cache + return)
/// 3. Fetch from batch API with single token
///
/// # Arguments
/// * `mint` - Token mint address
/// * `db` - Database instance
/// * `lane` - The caller's request lane: interactive waits out a rate-limit penalty,
///   background is deferred by it
///
/// # Returns
/// GeckoTerminalData if found, None if token not listed; `Err` when the lookup failed or
/// the answer could not be stored
pub async fn fetch_geckoterminal_data(
    mint: &str,
    db: &TokenDatabase,
    lane: RequestLane,
) -> TokenResult<Option<GeckoTerminalData>> {
    // Use batch endpoint with single token
    let (mut answers, mut store_failures) = fetch_and_store(&[mint.to_string()], db, lane).await?;
    if let Some(error) = store_failures.remove(mint) {
        return Err(error);
    }

    Ok(answers.remove(mint).flatten())
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    store::geckoterminal_cache_metrics()
}

/// Return current cache size
pub fn get_cache_size() -> usize {
    store::geckoterminal_cache_size()
}
