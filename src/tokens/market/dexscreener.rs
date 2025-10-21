/// DexScreener market data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 30 seconds for active tokens
use crate::apis::dexscreener::DexScreenerPool;
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{DexScreenerData, TokenError, TokenResult};
use chrono::{DateTime, Utc};

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

/// Fetch DexScreener data for a token (with cache + database)
///
/// Flow:
/// 1. Check cache (if fresh, return immediately)
/// 2. Check database (if fresh, cache + return)
/// 3. Fetch from API (store in database + cache + return)
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
    // 1. Check in-memory store cache
    if let Some(data) = store::get_cached_dexscreener(mint) {
        return Ok(Some(data));
    }

    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_dexscreener_data(mint)? {
        // If data is fresh (< 30s old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.fetched_at)
            .num_seconds();

        if age < 30 {
            store::store_dexscreener(mint, &db_data);
            if let Err(err) = store::refresh_token_snapshot(mint).await {
                eprintln!(
                    "[TOKENS][STORE] Failed to refresh token snapshot after DexScreener DB hit mint={} err={:?}",
                    mint,
                    err
                );
            }
            return Ok(Some(db_data));
        }
    }

    // 3. Fetch from API
    let api_manager = crate::apis::manager::get_api_manager();
    let pools = api_manager
        .dexscreener
        .fetch_token_pools(mint, None)
        .await
        .map_err(|e| TokenError::Api {
            source: "DexScreener".to_string(),
            message: format!("{:?}", e),
        })?;

    // Find best SOL-paired pool (highest liquidity)
    // SOL mint address (native SOL and wrapped SOL are the same address)
    const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

    // CRITICAL: Only consider pools where our token is the BASE token!
    // The API returns pools where token can be either base OR quote.
    // We want the price OF the token, not the price IN the token.
    let valid_pools: Vec<&DexScreenerPool> = pools
        .iter()
        .filter(|p| {
            p.liquidity_usd.is_some() && p.base_token_address == mint // Token must be base, not quote!
        })
        .collect();

    // Among valid pools, prefer SOL-paired pools
    let sol_pools: Vec<&DexScreenerPool> = valid_pools
        .iter()
        .filter(|p| p.quote_token_address == SOL_MINT)
        .copied()
        .collect();

    // Select best pool: prefer SOL-paired, fallback to highest liquidity
    let best_pool = if !sol_pools.is_empty() {
        // Use best SOL-paired pool
        sol_pools
            .iter()
            .max_by(|a, b| {
                a.liquidity_usd
                    .unwrap_or(0.0)
                    .partial_cmp(&b.liquidity_usd.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    } else if !valid_pools.is_empty() {
        // Fallback: use highest liquidity pool (any quote token, but token is base)
        valid_pools
            .iter()
            .max_by(|a, b| {
                a.liquidity_usd
                    .unwrap_or(0.0)
                    .partial_cmp(&b.liquidity_usd.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    } else {
        // No valid pools found
        None
    };

    if let Some(pool) = best_pool {
        const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
        let is_sol_pair = pool.quote_token_address == SOL_MINT;
        let data = convert_pool_to_data(pool, is_sol_pair);

        // Store in database
        db.upsert_dexscreener_data(mint, &data)?;

        // Cache it in store and refresh token snapshot
        store::store_dexscreener(mint, &data);
        if let Err(err) = store::refresh_token_snapshot(mint).await {
            eprintln!(
                "[TOKENS][STORE] Failed to refresh token snapshot after DexScreener API mint={} err={:?}",
                mint,
                err
            );
        }

        Ok(Some(data))
    } else {
        // No pools found
        Ok(None)
    }
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    store::dexscreener_cache_metrics()
}

/// Return current cache size (for monitoring)
pub fn get_cache_size() -> usize {
    store::dexscreener_cache_size()
}
