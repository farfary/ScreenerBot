/// DexScreener market data fetching and caching
/// 
/// Flow: API -> Parse -> Database -> Cache
/// Updates: Every 30 seconds for active tokens

use crate::apis::dexscreener::DexScreenerPool;
use crate::cache::{CacheConfig, CacheManager};
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
    DexScreenerData {
        mint: pool.mint.clone(),
        pair_address: pool.pair_address.clone(),
        chain_id: pool.chain_id.clone(),
        dex_id: pool.dex_id.clone(),
        url: pool.url.clone(),
        
        base_token_address: pool.base_token_address.clone(),
        base_token_name: pool.base_token_name.clone(),
        base_token_symbol: pool.base_token_symbol.clone(),
        quote_token_address: pool.quote_token_address.clone(),
        quote_token_name: pool.quote_token_name.clone(),
        quote_token_symbol: pool.quote_token_symbol.clone(),
        
        price_native: pool.price_native.clone(),
        price_usd: pool.price_usd.clone(),
        
        liquidity_usd: pool.liquidity_usd,
        liquidity_base: pool.liquidity_base,
        liquidity_quote: pool.liquidity_quote,
        
        volume_m5: pool.volume_m5,
        volume_h1: pool.volume_h1,
        volume_h6: pool.volume_h6,
        volume_h24: pool.volume_h24,
        
        txns_m5_buys: pool.txns_m5_buys,
        txns_m5_sells: pool.txns_m5_sells,
        txns_h1_buys: pool.txns_h1_buys,
        txns_h1_sells: pool.txns_h1_sells,
        txns_h6_buys: pool.txns_h6_buys,
        txns_h6_sells: pool.txns_h6_sells,
        txns_h24_buys: pool.txns_h24_buys,
        txns_h24_sells: pool.txns_h24_sells,
        
        price_change_m5: pool.price_change_m5,
        price_change_h1: pool.price_change_h1,
        price_change_h6: pool.price_change_h6,
        price_change_h24: pool.price_change_h24,
        
        fdv: pool.fdv,
        market_cap: pool.market_cap,
        
        pair_created_at: pool.pair_created_at,
        labels: pool.labels.clone(),
        
        info_image_url: pool.info_image_url.clone(),
        info_header: pool.info_header.clone(),
        info_open_graph: pool.info_open_graph.clone(),
        info_websites: pool.info_websites.clone(),
        info_socials: pool.info_socials.clone(),
        
        updated_at: Utc::now(),
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
            .signed_duration_since(db_data.updated_at)
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
pub fn get_cache_metrics() -> String {
    let cache = get_cache();
    let metrics = cache.metrics();
    format!(
        "DexScreener cache: {} entries, {:.2}% hit rate ({} hits, {} misses)",
        cache.len(),
        metrics.hit_rate() * 100.0,
        metrics.hits,
        metrics.misses
    )
}
