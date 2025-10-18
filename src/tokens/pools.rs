// tokens/pools.rs
// Best-pool selection helper built on top of provider data

use chrono::Utc;

use crate::constants::SOL_MINT;

use crate::tokens::provider::TokenDataProvider;
use crate::tokens::store::{upsert_snapshot, Snapshot};
use crate::tokens::api::dexscreener_types::DexScreenerPool;
use crate::tokens::api::geckoterminal_types::GeckoTerminalPool;

pub async fn refresh_for(provider: &TokenDataProvider, mint: &str) -> Result<(), String> {
    let data = provider.fetch_complete_data(mint, None).await?;

    // Note: Pool information is available through the pools module via get_pool_price()
    // Snapshots only store lightweight metadata, not pool details

    let snapshot = Snapshot {
        mint: mint.to_string(),
        symbol: data.metadata.symbol.clone(),
        name: data.metadata.name.clone(),
        decimals: data.metadata.decimals,
        is_blacklisted: false,
        sources: data.sources_used.clone(),
        priority: crate::tokens::priorities::Priority::Medium,
        fetched_at: Some(data.fetch_timestamp),
        updated_at: Utc::now(),
    };

    // Update both memory and database through unified store
    upsert_snapshot(snapshot)?;
    Ok(())
}

fn compute_liquidity_sol_from_dex(pool: &DexScreenerPool) -> Option<f64> {
    let quote_is_sol = pool.quote_token_address.eq_ignore_ascii_case(SOL_MINT)
        || pool.quote_token_symbol.eq_ignore_ascii_case("SOL");
    if quote_is_sol {
        return pool.liquidity_quote;
    }

    let base_is_sol = pool.base_token_address.eq_ignore_ascii_case(SOL_MINT)
        || pool.base_token_symbol.eq_ignore_ascii_case("SOL");
    if base_is_sol {
        return pool.liquidity_base;
    }

    None
}

fn compute_liquidity_sol_from_gecko(pool: &GeckoTerminalPool) -> Option<f64> {
    let base_is_sol = is_gecko_solana_id(&pool.base_token_id);
    if base_is_sol {
        // GeckoTerminal reports base token prices relative to quote.
        // When base token is SOL, reserve_usd approximates total USD liquidity.
        // Half of that belongs to SOL side.
        if let Some(reserve) = pool.reserve_usd {
            if let Ok(sol_price) = pool.base_token_price_usd.parse::<f64>() {
                if sol_price > 0.0 {
                    return Some((reserve / 2.0) / sol_price);
                }
            }
        }
    }

    let quote_is_sol = is_gecko_solana_id(&pool.quote_token_id);
    if quote_is_sol {
        if let Some(reserve) = pool.reserve_usd {
            if let Ok(sol_price) = pool.quote_token_price_usd.parse::<f64>() {
                if sol_price > 0.0 {
                    return Some((reserve / 2.0) / sol_price);
                }
            }
        }
    }

    None
}

fn is_gecko_solana_id(id: &str) -> bool {
    let normalized = id
        .strip_prefix("solana_")
        .or_else(|| id.strip_prefix("solana:"))
        .unwrap_or(id);
    normalized.eq_ignore_ascii_case(SOL_MINT)
}
