/// GeckoTerminal market data fetching and caching
///
/// Flow: API -> Parse -> Database -> Store cache
/// Updates: Every 60 seconds for active tokens
///
/// Architecture:
/// - Uses /networks/{net}/tokens/multi/{addresses} batch endpoint (up to 30 tokens per request)
/// - Returns aggregated token data (price, volume, market cap, etc.)
/// - No pool filtering logic (uses aggregated metrics from API)
use crate::apis::geckoterminal::{GeckoTerminalPool, GeckoTerminalTokenInfoResponse};
use crate::tokens::database::TokenDatabase;
use crate::tokens::store::{self, CacheMetrics};
use crate::tokens::types::{GeckoTerminalData, TokenError, TokenResult};
use chrono::Utc;
use std::collections::HashMap;

/// Fetch GeckoTerminal data for multiple tokens in a single batch API call
///
/// Uses /networks/{net}/tokens/multi/{addresses} endpoint which returns
/// aggregated token data (price, volume, market cap) without pool details.
///
/// Flow per token:
/// 1. Check cache (if fresh, skip API)
/// 2. Check database (if fresh < 60s, use it)
/// 3. Fetch from API (batch endpoint)
/// 4. Store in database + cache
///
/// # Arguments
/// * `mints` - Token mint addresses (up to 30)
/// * `db` - Database instance
///
/// # Returns
/// HashMap mapping mint -> Option<GeckoTerminalData>
/// - Some(data) if token has market data
/// - None if token not listed on any DEX
pub async fn fetch_geckoterminal_data_batch(
    mints: &[String],
    db: &TokenDatabase,
) -> TokenResult<HashMap<String, Option<GeckoTerminalData>>> {
    if mints.is_empty() {
        return Ok(HashMap::new());
    }

    let mut results: HashMap<String, Option<GeckoTerminalData>> = HashMap::new();
    let mut to_fetch: Vec<String> = Vec::new();

    // Check cache and database for each token
    for mint in mints {
        // 1. Check in-memory cache
        if let Some(data) = store::get_cached_geckoterminal(mint) {
            results.insert(mint.clone(), Some(data));
            continue;
        }

        // 2. Check database (if fresh < 60s)
        if let Some(db_data) = db.get_geckoterminal_data(mint)? {
            let age = Utc::now()
                .signed_duration_since(db_data.fetched_at)
                .num_seconds();

            if age < 60 {
                store::store_geckoterminal(mint, &db_data);
                if let Err(err) = store::refresh_token_snapshot(mint).await {
                    eprintln!(
                        "[TOKENS][STORE] Failed to refresh token snapshot after DB hit mint={} err={:?}",
                        mint, err
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
    let addresses_str = to_fetch.join(",");
    
    let tokens_response = api_manager
        .geckoterminal
        .fetch_tokens_multi("solana", &addresses_str, None, None)
        .await
        .map_err(|e| TokenError::Api {
            source: "GeckoTerminal".to_string(),
            message: format!("{:?}", e),
        })?;

    // Process each token from response
    for token_info in tokens_response.data {
        let mint = &token_info.attributes.address;
        let attrs = &token_info.attributes;
        
        let price_usd = attrs.price_usd.as_ref().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
        let sol_price = crate::sol_price::get_sol_price();
        let price_sol = if sol_price > 0.0 { price_usd / sol_price } else { 0.0 };
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
            liquidity_usd: attrs.total_reserve_in_usd.as_ref().and_then(|s| s.parse().ok()),
            volume_5m: volume_usd.and_then(|v| v.m5.as_ref().and_then(|s| s.parse().ok())),
            volume_1h: volume_usd.and_then(|v| v.h1.as_ref().and_then(|s| s.parse().ok())),
            volume_6h: volume_usd.and_then(|v| v.h6.as_ref().and_then(|s| s.parse().ok())),
            volume_24h: volume_usd.and_then(|v| v.h24.as_ref().and_then(|s| s.parse().ok())),
            pool_count: None,
            top_pool_address: None,
            reserve_in_usd: attrs.total_reserve_in_usd.as_ref().and_then(|s| s.parse().ok()),
            image_url: None,
            fetched_at: Utc::now(),
        };

        db.upsert_geckoterminal_data(mint, &data).ok();
        store::store_geckoterminal(mint, &data);
        store::refresh_token_snapshot(mint).await.ok();

        results.insert(mint.clone(), Some(data));
    }

    // Mark tokens with no data as None (not listed)
    for mint in &to_fetch {
        results.entry(mint.clone()).or_insert(None);
    }

    Ok(results)
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
///
/// # Returns
/// GeckoTerminalData if found, None if token not listed
pub async fn fetch_geckoterminal_data(
    mint: &str,
    db: &TokenDatabase,
) -> TokenResult<Option<GeckoTerminalData>> {
    // Use batch endpoint with single token
    let mut batch_results = fetch_geckoterminal_data_batch(&[mint.to_string()], db).await?;
    
    Ok(batch_results.remove(mint).flatten())
}

/// Get cache metrics for monitoring
pub fn get_cache_metrics() -> CacheMetrics {
    store::geckoterminal_cache_metrics()
}

/// Return current cache size
pub fn get_cache_size() -> usize {
    store::geckoterminal_cache_size()
}
