// tokens_new/pools.rs
// Best-pool selection helper built on top of provider data

use chrono::Utc;

use crate::tokens_new::provider::TokenDataProvider;
use crate::tokens_new::store::{upsert_snapshot, BestPoolSummary, Snapshot};

pub async fn refresh_for(provider: &TokenDataProvider, mint: &str) -> Result<(), String> {
    let data = provider.fetch_complete_data(mint, None).await?;

    // Select best SOL pool by highest liquidity from dexscreener first, then gecko
    let best = data
        .dexscreener_pools
        .iter()
        .filter(|p| p.quote_token_symbol.to_uppercase().contains("SOL")
            || p.base_token_symbol.to_uppercase().contains("SOL"))
        .max_by(|a, b| a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal))
        .map(|p| BestPoolSummary {
            program_id: Some(p.dex_id.clone()),
            pool_address: Some(p.pair_address.clone()),
            dex: Some("dexscreener".to_string()),
            liquidity_sol: None, // convert if needed later
        })
        .or_else(|| {
            data.geckoterminal_pools
                .iter()
                .max_by(|a, b| a.reserve_usd.partial_cmp(&b.reserve_usd).unwrap_or(std::cmp::Ordering::Equal))
                .map(|p| BestPoolSummary {
                    program_id: Some(p.dex_id.clone()),
                    pool_address: Some(p.pool_address.clone()),
                    dex: Some("geckoterminal".to_string()),
                    liquidity_sol: None,
                })
        });

    let snapshot = Snapshot {
        mint: mint.to_string(),
        symbol: data.metadata.symbol.clone(),
        name: data.metadata.name.clone(),
        decimals: data.metadata.decimals,
        is_blacklisted: false,
        best_pool: best,
        sources: data.sources_used.clone(),
        priority: crate::tokens_new::priorities::Priority::Medium,
        fetched_at: Some(data.fetch_timestamp),
        updated_at: Utc::now(),
    };

    upsert_snapshot(snapshot);
    Ok(())
}
