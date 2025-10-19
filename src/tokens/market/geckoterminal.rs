/// GeckoTerminal market data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 60 seconds for active tokens
use crate::apis::geckoterminal::GeckoTerminalPool;
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{GeckoTerminalData, TokenError, TokenResult};
use chrono::Utc;

/// Convert API pool data to our GeckoTerminalData type
fn convert_pool_to_data(pool: &GeckoTerminalPool) -> GeckoTerminalData {
    fn parse_f64(value: &str) -> Option<f64> {
        value.parse::<f64>().ok()
    }

    GeckoTerminalData {
        price_usd: parse_f64(&pool.token_price_usd).unwrap_or(0.0),
        price_sol: parse_f64(&pool.base_token_price_native).unwrap_or(0.0),
        price_native: pool.base_token_price_native.clone(),
        price_change_5m: pool.price_change_m5,
        price_change_1h: pool.price_change_h1,
        price_change_6h: pool.price_change_h6,
        price_change_24h: pool.price_change_h24,
        market_cap: pool.market_cap_usd,
        fdv: pool.fdv_usd,
        liquidity_usd: pool.reserve_usd,
        volume_5m: pool.volume_m5,
        volume_1h: pool.volume_h1,
        volume_6h: pool.volume_h6,
        volume_24h: pool.volume_h24,
        pool_count: None,
        top_pool_address: if pool.pool_address.is_empty() {
            None
        } else {
            Some(pool.pool_address.clone())
        },
        reserve_in_usd: pool.reserve_usd,
        fetched_at: Utc::now(),
    }
}

/// Fetch GeckoTerminal data for a token (with cache + database)
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
/// GeckoTerminalData if found, None if token not listed
pub async fn fetch_geckoterminal_data(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<GeckoTerminalData>> {
    // 1. Check in-memory store cache
    if let Some(data) = store::get_cached_geckoterminal(mint) {
        return Ok(Some(data));
    }

    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_geckoterminal_data(mint)? {
        // If data is fresh (< 60s old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.fetched_at)
            .num_seconds();

        if age < 60 {
            store::store_geckoterminal(mint, &db_data);
            if let Err(err) = store::refresh_token_snapshot(mint).await {
                eprintln!(
                    "[TOKENS][STORE] Failed to refresh token snapshot after GeckoTerminal DB hit mint={} err={:?}",
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
        .geckoterminal
        .fetch_pools(mint)
        .await
        .map_err(|e| TokenError::Api {
            source: "GeckoTerminal".to_string(),
            message: format!("{:?}", e),
        })?;

    // Find best pool (highest reserve_usd)
    let best_pool = pools
        .iter()
        .filter(|p| p.reserve_usd.is_some())
        .max_by(|a, b| {
            a.reserve_usd
                .unwrap_or(0.0)
                .partial_cmp(&b.reserve_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let Some(pool) = best_pool {
        let data = convert_pool_to_data(pool);

        // Store in database
        db.upsert_geckoterminal_data(mint, &data)?;

        // Cache it in store and refresh token snapshot
        store::store_geckoterminal(mint, &data);
        if let Err(err) = store::refresh_token_snapshot(mint).await {
            eprintln!(
                "[TOKENS][STORE] Failed to refresh token snapshot after GeckoTerminal API mint={} err={:?}",
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
    store::geckoterminal_cache_metrics()
}

/// Return current cache size
pub fn get_cache_size() -> usize {
    store::geckoterminal_cache_size()
}
