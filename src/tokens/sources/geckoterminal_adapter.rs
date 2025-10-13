use super::{DataSource, SourcedPool, SourcedPrice, UnifiedTokenInfo};
use crate::tokens::geckoterminal::{get_token_pools_from_geckoterminal, GeckoTerminalPool};
use chrono::Utc;

pub async fn fetch_token_info_from_geckoterminal(mint: &str) -> Result<UnifiedTokenInfo, String> {
    // GeckoTerminal primary for pools; token symbol/name not directly provided consistently
    let pools = get_token_pools_from_geckoterminal(mint)
        .await
        .map_err(|e| format!("GeckoTerminal pool fetch failed: {}", e))?;

    let mut unified_pools: Vec<SourcedPool> = Vec::new();
    let mut prices: Vec<SourcedPrice> = Vec::new();

    for p in pools.iter() {
        unified_pools.push(SourcedPool {
            source: DataSource::GeckoTerminal,
            pool_address: p.pool_address.clone(),
            dex_id: p.dex_id.clone(),
            base_token: p.base_token.clone(),
            quote_token: p.quote_token.clone(),
            liquidity_usd: Some(p.liquidity_usd),
            volume_24h_usd: Some(p.volume_24h),
            price_sol: Some(p.price_native),
        });

        prices.push(SourcedPrice {
            source: DataSource::GeckoTerminal,
            price_sol: p.price_native,
            price_usd: Some(p.price_usd),
            pool_address: Some(p.pool_address.clone()),
            liquidity_usd: Some(p.liquidity_usd),
            fetched_at: Utc::now(),
        });
    }

    Ok(UnifiedTokenInfo {
        mint: mint.to_string(),
        symbol: String::new(),
        name: String::new(),
        prices,
        consensus_price_sol: None,
        price_confidence: 0.0,
        pools: unified_pools,
        primary_pool: None,
        liquidity_usd: None,
        volume_24h_usd: None,
        market_cap: None,
        last_updated: Utc::now(),
        sources: vec![DataSource::GeckoTerminal],
        fetch_timestamp: Utc::now(),
    })
}
