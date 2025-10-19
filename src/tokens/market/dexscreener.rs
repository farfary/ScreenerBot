/// DexScreener market data fetching and caching
/// 
/// Flow: API -> Parse -> Database -> Cache
/// Updates: Every 30 seconds for active tokens

use crate::apis::dexscreener::DexScreenerPool;
use crate::cache::{CacheConfig, CacheManager};
use crate::cache::manager::CacheMetrics;
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{DexScreenerData, TokenError, TokenResult};
use chrono::Utc;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// Global cache instance (TTL = 30s)
static DEXSCREENER_CACHE: OnceCell<Arc<CacheManager<String, DexScreenerData>>> = OnceCell::new();

/// Get or initialize DexScreener cache
fn get_cache() -> Arc<CacheManager<String, DexScreenerData>> {
    DEXSCREENER_CACHE
        .get_or_init(|| {
            let config = CacheConfig::market_dexscreener(); // 30s TTL
            Arc::new(CacheManager::new(config))
        })
        .clone()
}

/// Convert API pool data to our DexScreenerData type
fn convert_pool_to_data(pool: &DexScreenerPool) -> DexScreenerData {
    fn parse_f64(value: &str) -> Option<f64> {
        value.parse::<f64>().ok()
    }

    fn combine_txns(buys: Option<i64>, sells: Option<i64>) -> Option<(u32, u32)> {
        match (buys, sells) {
            (Some(b), Some(s)) if b >= 0 && s >= 0 => Some((b as u32, s as u32)),
            _ => None,
        }
    }

    DexScreenerData {
        price_usd: parse_f64(&pool.price_usd).unwrap_or(0.0),
        price_sol: parse_f64(&pool.price_native).unwrap_or(0.0),
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
    let cache = get_cache();
    
    // 1. Check cache
    if let Some(data) = cache.get(&mint.to_string()) {
        return Ok(Some(data));
    }
    
    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_dexscreener_data(mint)? {
        // If data is fresh (< 30s old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.fetched_at)
            .num_seconds();
        
        if age < 30 {
            cache.insert(mint.to_string(), db_data.clone());
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
    
    // Find best pool (highest liquidity)
    let best_pool = pools
        .iter()
        .filter(|p| p.liquidity_usd.is_some())
        .max_by(|a, b| {
            a.liquidity_usd
                .unwrap_or(0.0)
                .partial_cmp(&b.liquidity_usd.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    
    if let Some(pool) = best_pool {
        let data = convert_pool_to_data(pool);
        
        // Store in database
    db.upsert_dexscreener_data(mint, &data)?;
        
    // Cache it
    cache.insert(mint.to_string(), data.clone());
        
        Ok(Some(data))
    } else {
        // No pools found
        Ok(None)
    }
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    get_cache().metrics()
}

/// Return current cache size (for monitoring)
pub fn get_cache_size() -> usize {
    get_cache().len()
}
