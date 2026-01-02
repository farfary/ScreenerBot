//! Pool Selector for Trade Watcher
//!
//! Searches for all pools of a token from both GeckoTerminal and DexScreener APIs.

use super::types::{PoolInfo, PoolSource};
use crate::apis::manager::get_api_manager;
use crate::logger::{self, LogTag};

/// Search for all pools of a token from both GeckoTerminal and DexScreener
///
/// Returns pools sorted by liquidity (highest first).
///
/// # Arguments
/// * `token_mint` - The token mint address to search for
///
/// # Returns
/// Vec of PoolInfo from both sources, deduplicated and sorted by liquidity
pub async fn search_pools(token_mint: &str) -> Result<Vec<PoolInfo>, String> {
    let mut pools = Vec::new();
    let api_manager = get_api_manager();

    logger::debug(
        LogTag::Tools,
        &format!("[TRADE_WATCHER] Searching pools for token: {}", token_mint),
    );

    // Search GeckoTerminal
    match api_manager.geckoterminal.fetch_pools(token_mint).await {
        Ok(gt_pools) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] GeckoTerminal returned {} pools for {}",
                    gt_pools.len(),
                    token_mint
                ),
            );
            for pool in gt_pools {
                pools.push(PoolInfo {
                    address: pool.pool_address.clone(),
                    source: PoolSource::GeckoTerminal,
                    dex: pool.dex_id.clone(),
                    base_token: pool.base_token_id.clone(),
                    base_symbol: pool
                        .pool_name
                        .split('/')
                        .next()
                        .unwrap_or("???")
                        .to_string(),
                    quote_token: pool.quote_token_id.clone(),
                    quote_symbol: pool
                        .pool_name
                        .split('/')
                        .last()
                        .unwrap_or("SOL")
                        .to_string(),
                    liquidity_usd: pool.reserve_usd.unwrap_or(0.0),
                    volume_24h: pool.volume_h24.unwrap_or(0.0),
                    price_usd: pool.base_token_price_usd.parse().unwrap_or(0.0),
                });
            }
        }
        Err(e) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] GeckoTerminal pool search failed for {}: {}",
                    token_mint, e
                ),
            );
        }
    }

    // Search DexScreener
    match api_manager
        .dexscreener
        .fetch_token_pools(token_mint, None)
        .await
    {
        Ok(ds_pools) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] DexScreener returned {} pools for {}",
                    ds_pools.len(),
                    token_mint
                ),
            );
            for pool in ds_pools {
                // Avoid duplicates by checking address
                let pool_addr = pool.pair_address.clone();
                if !pools.iter().any(|p| p.address == pool_addr) {
                    pools.push(PoolInfo {
                        address: pool_addr,
                        source: PoolSource::DexScreener,
                        dex: pool.dex_id.clone(),
                        base_token: pool.base_token_address.clone(),
                        base_symbol: pool.base_token_symbol.clone(),
                        quote_token: pool.quote_token_address.clone(),
                        quote_symbol: pool.quote_token_symbol.clone(),
                        liquidity_usd: pool.liquidity_usd.unwrap_or(0.0),
                        volume_24h: pool.volume_h24.unwrap_or(0.0),
                        price_usd: pool.price_usd.parse().unwrap_or(0.0),
                    });
                }
            }
        }
        Err(e) => {
            logger::debug(
                LogTag::Tools,
                &format!(
                    "[TRADE_WATCHER] DexScreener pool search failed for {}: {}",
                    token_mint, e
                ),
            );
        }
    }

    // Sort by liquidity descending
    pools.sort_by(|a, b| {
        b.liquidity_usd
            .partial_cmp(&a.liquidity_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    logger::debug(
        LogTag::Tools,
        &format!(
            "[TRADE_WATCHER] Total pools found for {}: {}",
            token_mint,
            pools.len()
        ),
    );

    Ok(pools)
}

/// Get the best pool for a token (highest liquidity)
pub async fn get_best_pool(token_mint: &str) -> Result<Option<PoolInfo>, String> {
    let pools = search_pools(token_mint).await?;
    Ok(pools.into_iter().next())
}

/// Search pools and filter by minimum liquidity
pub async fn search_pools_with_min_liquidity(
    token_mint: &str,
    min_liquidity_usd: f64,
) -> Result<Vec<PoolInfo>, String> {
    let pools = search_pools(token_mint).await?;
    Ok(pools
        .into_iter()
        .filter(|p| p.liquidity_usd >= min_liquidity_usd)
        .collect())
}

/// Search pools from a specific source only
pub async fn search_pools_by_source(
    token_mint: &str,
    source: PoolSource,
) -> Result<Vec<PoolInfo>, String> {
    let mut pools = Vec::new();
    let api_manager = get_api_manager();

    match source {
        PoolSource::GeckoTerminal => {
            match api_manager.geckoterminal.fetch_pools(token_mint).await {
                Ok(gt_pools) => {
                    for pool in gt_pools {
                        pools.push(PoolInfo {
                            address: pool.pool_address.clone(),
                            source: PoolSource::GeckoTerminal,
                            dex: pool.dex_id.clone(),
                            base_token: pool.base_token_id.clone(),
                            base_symbol: pool
                                .pool_name
                                .split('/')
                                .next()
                                .unwrap_or("???")
                                .to_string(),
                            quote_token: pool.quote_token_id.clone(),
                            quote_symbol: pool
                                .pool_name
                                .split('/')
                                .last()
                                .unwrap_or("SOL")
                                .to_string(),
                            liquidity_usd: pool.reserve_usd.unwrap_or(0.0),
                            volume_24h: pool.volume_h24.unwrap_or(0.0),
                            price_usd: pool.base_token_price_usd.parse().unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => return Err(format!("GeckoTerminal search failed: {}", e)),
            }
        }
        PoolSource::DexScreener => {
            match api_manager
                .dexscreener
                .fetch_token_pools(token_mint, None)
                .await
            {
                Ok(ds_pools) => {
                    for pool in ds_pools {
                        pools.push(PoolInfo {
                            address: pool.pair_address.clone(),
                            source: PoolSource::DexScreener,
                            dex: pool.dex_id.clone(),
                            base_token: pool.base_token_address.clone(),
                            base_symbol: pool.base_token_symbol.clone(),
                            quote_token: pool.quote_token_address.clone(),
                            quote_symbol: pool.quote_token_symbol.clone(),
                            liquidity_usd: pool.liquidity_usd.unwrap_or(0.0),
                            volume_24h: pool.volume_h24.unwrap_or(0.0),
                            price_usd: pool.price_usd.parse().unwrap_or(0.0),
                        });
                    }
                }
                Err(e) => return Err(format!("DexScreener search failed: {}", e)),
            }
        }
    }

    // Sort by liquidity descending
    pools.sort_by(|a, b| {
        b.liquidity_usd
            .partial_cmp(&a.liquidity_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(pools)
}
