/// GeckoTerminal market data fetching and caching
/// 
/// Flow: API -> Parse -> Database -> Cache
/// Updates: Every 60 seconds for active tokens

use crate::apis::geckoterminal::GeckoTerminalPool;
use crate::cache::{CacheConfig, CacheManager};
use crate::tokens::database::TokenDatabase;
use crate::tokens::types::{GeckoTerminalData, TokenError, TokenResult};
use chrono::Utc;
use once_cell::sync::OnceCell;
use std::sync::Arc;

// Global cache instance (TTL = 60s)
static GECKOTERMINAL_CACHE: OnceCell<Arc<CacheManager<String, GeckoTerminalData>>> =
    OnceCell::new();

/// Get or initialize GeckoTerminal cache
fn get_cache() -> Arc<CacheManager<String, GeckoTerminalData>> {
    GECKOTERMINAL_CACHE
        .get_or_init(|| {
            let config = CacheConfig::market_geckoterminal(); // 60s TTL
            Arc::new(CacheManager::new(config))
        })
        .clone()
}

/// Convert API pool data to our GeckoTerminalData type
fn convert_pool_to_data(pool: &GeckoTerminalPool) -> GeckoTerminalData {
    GeckoTerminalData {
        mint: pool.mint.clone(),
        pool_address: pool.pool_address.clone(),
        pool_name: pool.pool_name.clone(),
        dex_id: pool.dex_id.clone(),
        
        base_token_id: pool.base_token_id.clone(),
        quote_token_id: pool.quote_token_id.clone(),
        
        base_token_price_usd: pool.base_token_price_usd.clone(),
        base_token_price_native: pool.base_token_price_native.clone(),
        base_token_price_quote: pool.base_token_price_quote.clone(),
        quote_token_price_usd: pool.quote_token_price_usd.clone(),
        quote_token_price_native: pool.quote_token_price_native.clone(),
        quote_token_price_base: pool.quote_token_price_base.clone(),
        token_price_usd: pool.token_price_usd.clone(),
        
        fdv_usd: pool.fdv_usd,
        market_cap_usd: pool.market_cap_usd,
        reserve_usd: pool.reserve_usd,
        
        volume_m5: pool.volume_m5,
        volume_m15: pool.volume_m15,
        volume_m30: pool.volume_m30,
        volume_h1: pool.volume_h1,
        volume_h6: pool.volume_h6,
        volume_h24: pool.volume_h24,
        
        price_change_m5: pool.price_change_m5,
        price_change_m15: pool.price_change_m15,
        price_change_m30: pool.price_change_m30,
        price_change_h1: pool.price_change_h1,
        price_change_h6: pool.price_change_h6,
        price_change_h24: pool.price_change_h24,
        
        txns_m5_buys: pool.txns_m5_buys,
        txns_m5_sells: pool.txns_m5_sells,
        txns_m5_buyers: pool.txns_m5_buyers,
        txns_m5_sellers: pool.txns_m5_sellers,
        txns_m15_buys: pool.txns_m15_buys,
        txns_m15_sells: pool.txns_m15_sells,
        txns_m15_buyers: pool.txns_m15_buyers,
        txns_m15_sellers: pool.txns_m15_sellers,
        txns_m30_buys: pool.txns_m30_buys,
        txns_m30_sells: pool.txns_m30_sells,
        txns_m30_buyers: pool.txns_m30_buyers,
        txns_m30_sellers: pool.txns_m30_sellers,
        txns_h1_buys: pool.txns_h1_buys,
        txns_h1_sells: pool.txns_h1_sells,
        txns_h1_buyers: pool.txns_h1_buyers,
        txns_h1_sellers: pool.txns_h1_sellers,
        txns_h6_buys: pool.txns_h6_buys,
        txns_h6_sells: pool.txns_h6_sells,
        txns_h6_buyers: pool.txns_h6_buyers,
        txns_h6_sellers: pool.txns_h6_sellers,
        txns_h24_buys: pool.txns_h24_buys,
        txns_h24_sells: pool.txns_h24_sells,
        txns_h24_buyers: pool.txns_h24_buyers,
        txns_h24_sellers: pool.txns_h24_sellers,
        
        pool_created_at: pool.pool_created_at.clone(),
        
        updated_at: Utc::now(),
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
    let cache = get_cache();
    
    // 1. Check cache
    if let Some(data) = cache.get(&mint.to_string()) {
        return Ok(Some(data));
    }
    
    // 2. Check database (if recently updated, use it)
    if let Some(db_data) = db.get_geckoterminal_data(mint)? {
        // If data is fresh (< 60s old), use it
        let age = Utc::now()
            .signed_duration_since(db_data.updated_at)
            .num_seconds();
        
        if age < 60 {
            cache.insert(mint.to_string(), db_data.clone());
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
        
        // Cache it
        cache.insert(mint.to_string(), data.clone());
        
        Ok(Some(data))
    } else {
        // No pools found
        Ok(None)
    }
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> String {
    let cache = get_cache();
    let metrics = cache.metrics();
    format!(
        "GeckoTerminal cache: {} entries, {:.2}% hit rate ({} hits, {} misses)",
        cache.len(),
        metrics.hit_rate() * 100.0,
        metrics.hits,
        metrics.misses
    )
}
