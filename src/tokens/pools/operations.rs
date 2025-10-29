/// Pool operations - merging, deduplication, canonical selection
use crate::tokens::types::{TokenPoolInfo, TokenPoolSources};
use std::cmp::Ordering;
use std::collections::{hash_map::Entry, HashMap};

use super::utils::calculate_pool_metric;

/// Merge pool sources (combines DexScreener + GeckoTerminal data)
pub fn merge_pool_sources(target: &mut TokenPoolSources, incoming: TokenPoolSources) {
    if incoming.dexscreener.is_some() {
        target.dexscreener = incoming.dexscreener;
    }
    if incoming.geckoterminal.is_some() {
        target.geckoterminal = incoming.geckoterminal;
    }
}

/// Merge pool info (combines data from multiple sources)
pub fn merge_pool_info(target: &mut TokenPoolInfo, incoming: TokenPoolInfo) {
    if target.dex.is_none() {
        target.dex = incoming.dex.clone();
    }

    if !target.is_sol_pair && incoming.is_sol_pair {
        target.is_sol_pair = true;
    }

    if let Some(liquidity_usd) = incoming.liquidity_usd {
        target.liquidity_usd = Some(match target.liquidity_usd {
            Some(existing) => existing.max(liquidity_usd),
            None => liquidity_usd,
        });
    }

    if let Some(liquidity_token) = incoming.liquidity_token {
        target.liquidity_token = Some(match target.liquidity_token {
            Some(existing) => existing.max(liquidity_token),
            None => liquidity_token,
        });
    }

    if let Some(liquidity_sol) = incoming.liquidity_sol {
        target.liquidity_sol = Some(match target.liquidity_sol {
            Some(existing) => existing.max(liquidity_sol),
            None => liquidity_sol,
        });
    }

    if let Some(volume) = incoming.volume_h24 {
        target.volume_h24 = Some(match target.volume_h24 {
            Some(existing) => existing.max(volume),
            None => volume,
        });
    }

    if let Some(price_usd) = incoming.price_usd {
        target.price_usd = Some(price_usd);
    }

    if let Some(price_sol) = incoming.price_sol {
        target.price_sol = Some(price_sol);
    }

    if incoming.price_native.is_some() {
        target.price_native = incoming.price_native.clone();
    }

    target.fetched_at = target.fetched_at.max(incoming.fetched_at);
    merge_pool_sources(&mut target.sources, incoming.sources);
}

/// Ingest pool entry into map (merges if exists, inserts if new)
pub fn ingest_pool_entry(map: &mut HashMap<String, TokenPoolInfo>, info: TokenPoolInfo) {
    if info.pool_address.is_empty() {
        return;
    }

    match map.entry(info.pool_address.clone()) {
        Entry::Vacant(slot) => {
            slot.insert(info);
        }
        Entry::Occupied(mut slot) => {
            merge_pool_info(slot.get_mut(), info);
        }
    }
}

/// Choose canonical pool address (highest liquidity SOL pair)
pub fn choose_canonical_pool(pools: &[TokenPoolInfo]) -> Option<String> {
    pools
        .iter()
        .filter(|pool| pool.is_sol_pair)
        .max_by(|a, b| {
            let metric_a = calculate_pool_metric(a);
            let metric_b = calculate_pool_metric(b);
            match metric_a.partial_cmp(&metric_b).unwrap_or(Ordering::Equal) {
                Ordering::Equal => {
                    let vol_a = a.volume_h24.unwrap_or(0.0);
                    let vol_b = b.volume_h24.unwrap_or(0.0);
                    vol_a.partial_cmp(&vol_b).unwrap_or(Ordering::Equal)
                }
                ordering => ordering,
            }
        })
        .map(|pool| pool.pool_address.clone())
}

/// Sort pools for snapshot (SOL pairs first, then by liquidity)
pub fn sort_pools_for_snapshot(pools: &mut [TokenPoolInfo]) {
    pools.sort_by(|a, b| match (b.is_sol_pair, a.is_sol_pair) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => match calculate_pool_metric(b)
            .partial_cmp(&calculate_pool_metric(a))
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => a.pool_address.cmp(&b.pool_address),
            ordering => ordering,
        },
    });
}
