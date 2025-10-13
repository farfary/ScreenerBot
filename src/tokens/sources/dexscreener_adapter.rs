use super::{DataSource, SourcedPool, SourcedPrice, UnifiedTokenInfo};
use crate::tokens::dexscreener::{
    get_cached_pools_for_token, get_global_dexscreener_api, get_token_pairs_from_api,
};
use crate::tokens::types::Token;
use chrono::Utc;

/// Build a unified token snapshot using a prefetched DexScreener token response.
pub async fn unify_prefetched_token(token: &Token) -> Result<UnifiedTokenInfo, String> {
    let mut prices = Vec::new();
    if let Some(price_sol) = token.price_dexscreener_sol {
        prices.push(SourcedPrice {
            source: DataSource::DexScreener,
            price_sol,
            price_usd: token.price_dexscreener_usd,
            pool_address: token.pair_address.clone(),
            liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
            fetched_at: Utc::now(),
        });
    }

    let mut pools: Vec<SourcedPool> = Vec::new();
    let pairs = match get_cached_pools_for_token(&token.mint).await {
        Some(cached) => cached,
        None => get_token_pairs_from_api(&token.mint).await?,
    };

    for p in pairs {
        pools.push(SourcedPool {
            source: DataSource::DexScreener,
            pool_address: p.pair_address.clone(),
            dex_id: p.dex_id.clone(),
            base_token: p.base_token.address.clone(),
            quote_token: p.quote_token.address.clone(),
            liquidity_usd: p.liquidity.as_ref().map(|l| l.usd),
            volume_24h_usd: p.volume.h24,
            price_sol: p.price_native.parse::<f64>().ok(),
        });
    }

    Ok(UnifiedTokenInfo {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        prices,
        consensus_price_sol: None,
        price_confidence: 0.0,
        pools,
        primary_pool: None,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h_usd: token.volume.as_ref().and_then(|v| v.h24),
        market_cap: token.market_cap,
        last_updated: Utc::now(),
        sources: vec![DataSource::DexScreener],
        fetch_timestamp: Utc::now(),
    })
}

/// Fetch DexScreener data over HTTP and convert to the unified representation.
pub async fn fetch_token_info_from_dexscreener(mint: &str) -> Result<UnifiedTokenInfo, String> {
    let api = get_global_dexscreener_api().await?;
    let mut api = api.lock().await;
    let token_opt = api
        .get_token_data(mint)
        .await
        .map_err(|e| format!("DexScreener token fetch failed: {}", e))?;
    let token = token_opt.ok_or_else(|| "Token not found on DexScreener".to_string())?;

    unify_prefetched_token(&token).await
}
