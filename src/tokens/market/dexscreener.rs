/// DexScreener market data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 30 seconds for active tokens
///
/// Architecture:
/// - Uses /tokens/v1 batch endpoint (up to 30 tokens per request)
/// - Returns ONE best pool per token (DexScreener picks most liquid)
/// - No pool filtering logic (trust DexScreener API)
use crate::apis::dexscreener::DexScreenerPool;
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{DexScreenerData, TokenError, TokenResult};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Convert API pool data to our DexScreenerData type
fn convert_pool_to_data(pool: &DexScreenerPool, is_sol_pair: bool) -> DexScreenerData {
    fn parse_f64(value: &str) -> Option<f64> {
        value.parse::<f64>().ok()
    }

    fn combine_txns(buys: Option<i64>, sells: Option<i64>) -> Option<(u32, u32)> {
        match (buys, sells) {
            (Some(b), Some(s)) if b >= 0 && s >= 0 => Some((b as u32, s as u32)),
            _ => None,
        }
    }

    let price_usd = parse_f64(&pool.price_usd).unwrap_or(0.0);

    // Calculate price_sol based on pool type
    let price_sol = if is_sol_pair {
        // For SOL-paired pools, priceNative IS the SOL price
        parse_f64(&pool.price_native).unwrap_or(0.0)
    } else {
        // For non-SOL pairs, calculate: price_sol = price_usd / sol_usd_price
        let sol_price = crate::sol_price::get_sol_price();
        if sol_price > 0.0 {
            price_usd / sol_price
        } else {
            // Fallback: use priceNative as-is (may be wrong but better than 0)
            // This happens when SOL price service isn't running (e.g., in debug tools)
            parse_f64(&pool.price_native).unwrap_or(0.0)
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
        pair_created_at: pool
            .pair_created_at
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        fetched_at: Utc::now(),
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
/// - None if token not listed on any DEX
pub async fn fetch_dexscreener_data_batch(
    mints: &[String],
    db: &TokenDatabase,
) -> TokenResult<HashMap<String, Option<DexScreenerData>>> {
    if mints.is_empty() {
        return Ok(HashMap::new());
    }

    let mut results: HashMap<String, Option<DexScreenerData>> = HashMap::new();
    let mut to_fetch: Vec<String> = Vec::new();

    // Check cache and database for each token
    for mint in mints {
        // 1. Check in-memory cache
        if let Some(data) = store::get_cached_dexscreener(mint) {
            results.insert(mint.clone(), Some(data));
            continue;
        }

        // 2. Check database (if fresh < 30s)
        if let Some(db_data) = db.get_dexscreener_data(mint)? {
            let age = Utc::now()
                .signed_duration_since(db_data.fetched_at)
                .num_seconds();

            if age < 30 {
                store::store_dexscreener(mint, &db_data);
                if let Err(err) = store::refresh_token_snapshot(mint).await {
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
        return Ok(results);
    }

    // 3. Fetch from batch API endpoint
    let api_manager = crate::apis::manager::get_api_manager();
    let pools = api_manager
        .dexscreener
        .fetch_token_batch(&to_fetch, None)
        .await
        .map_err(|e| TokenError::Api {
            source: "DexScreener".to_string(),
            message: format!("{:?}", e),
        })?;

    // Process pools - DexScreener batch returns ONE best pool per token
    use crate::constants::SOL_MINT;

    for pool in pools {
        let mint = &pool.base_token_address;

        // Skip if not in our request list
        if !to_fetch.contains(mint) {
            continue;
        }

        let is_sol_pair = pool.quote_token_address == SOL_MINT;
        let data = convert_pool_to_data(&pool, is_sol_pair);

        // Store in database
        if let Err(e) = db.upsert_dexscreener_data(mint, &data) {
            logger::error(
                LogTag::Tokens,
                &format!(
                    "[TOKENS][DEXSCREENER] Failed to store data for {}: {}",
                    mint, e
                ),
            );
        }

        // Cache it
        store::store_dexscreener(mint, &data);
        if let Err(err) = store::refresh_token_snapshot(mint).await {
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

    // Mark tokens with no pools as None (not listed)
    for mint in &to_fetch {
        results.entry(mint.clone()).or_insert(None);
    }

    Ok(results)
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
/// DexScreenerData if found, None if token not listed
pub async fn fetch_dexscreener_data(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<DexScreenerData>> {
    // Use batch endpoint with single token
    let mut batch_results = fetch_dexscreener_data_batch(&[mint.to_string()], db).await?;

    Ok(batch_results.remove(mint).flatten())
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    store::dexscreener_cache_metrics()
}

/// Return current cache size (for monitoring)
pub fn get_cache_size() -> usize {
    store::dexscreener_cache_size()
}
