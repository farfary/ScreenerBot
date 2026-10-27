// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pool operations - merging, deduplication, canonical selection.

use crate::apis::dexscreener::TOKEN_PAIRS_CAP;
use crate::chains::{adapter_for, ChainId};
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

    if !target.is_native_pair && incoming.is_native_pair {
        target.is_native_pair = true;
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

    if let Some(liquidity_native) = incoming.liquidity_native {
        target.liquidity_native = Some(match target.liquidity_native {
            Some(existing) => existing.max(liquidity_native),
            None => liquidity_native,
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

    target.pool_data_last_fetched_at = target
        .pool_data_last_fetched_at
        .max(incoming.pool_data_last_fetched_at);
    target.pool_data_first_seen_at = target
        .pool_data_first_seen_at
        .min(incoming.pool_data_first_seen_at);
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

/// Whether a mint's DexScreener list must be completed from GeckoTerminal. `token-pairs/v1`
/// stops at `TOKEN_PAIRS_CAP` pairs and can omit the mint's deepest pools against the chain's
/// native coin, so a list holding no native pair, or one cut at the cap, is incomplete.
/// `listed` holds the converted rows and `pair_count` the pairs DexScreener returned.
pub fn dexscreener_list_is_incomplete(listed: &[TokenPoolInfo], pair_count: usize) -> bool {
    pair_count >= TOKEN_PAIRS_CAP || !listed.iter().any(|pool| pool.is_native_pair)
}

/// The GeckoTerminal pools that complete `known` for `mint`: rows oriented to `mint` (its
/// `base_mint`) whose counter asset is the chain's native coin or one of its stablecoins, and
/// whose address `known` does not hold, so the row already listed wins a pool both providers
/// report. A row for another mint is dropped.
pub fn reliable_completion_pools(
    chain: ChainId,
    mint: &str,
    known: &HashMap<String, TokenPoolInfo>,
    extra: Vec<TokenPoolInfo>,
) -> Vec<TokenPoolInfo> {
    let adapter = adapter_for(chain);
    extra
        .into_iter()
        .filter(|pool| {
            pool.base_mint == mint
                && adapter.is_quote_asset(&pool.quote_mint)
                && !known.contains_key(&pool.pool_address)
        })
        .collect()
}

/// Share of the best pool's liquidity metric an incumbent canonical pool keeps and still
/// wins. Two venues of one token report liquidity within a few percent of each other and swap
/// the lead on every refresh; a canonical that flips moves the pricing pool, and with it a
/// position's price, P&L and exit triggers.
pub const CANONICAL_KEEP_RATIO: f64 = 0.8;

/// Choose canonical pool address: the highest-liquidity SOL pair (metric, then 24h volume),
/// except that a native-pair `previous` still in `pools` keeps the role while its metric is at
/// least `CANONICAL_KEEP_RATIO` of the best metric.
pub fn choose_canonical_pool(
    chain: ChainId,
    pools: &[TokenPoolInfo],
    previous: Option<&str>,
) -> Option<String> {
    let best = pools
        .iter()
        .filter(|pool| pool.is_native_pair)
        .max_by(|a, b| {
            let metric_a = calculate_pool_metric(chain, a);
            let metric_b = calculate_pool_metric(chain, b);
            match metric_a.partial_cmp(&metric_b).unwrap_or(Ordering::Equal) {
                Ordering::Equal => {
                    let vol_a = a.volume_h24.unwrap_or_default();
                    let vol_b = b.volume_h24.unwrap_or_default();
                    vol_a.partial_cmp(&vol_b).unwrap_or(Ordering::Equal)
                }
                ordering => ordering,
            }
        })?;

    let incumbent = previous.and_then(|address| {
        pools
            .iter()
            .find(|pool| pool.is_native_pair && pool.pool_address == address)
    });
    if let Some(incumbent) = incumbent {
        if calculate_pool_metric(chain, incumbent)
            >= CANONICAL_KEEP_RATIO * calculate_pool_metric(chain, best)
        {
            return Some(incumbent.pool_address.clone());
        }
    }
    Some(best.pool_address.clone())
}

/// Sort pools for snapshot (SOL pairs first, then by liquidity)
pub fn sort_pools_for_snapshot(chain: ChainId, pools: &mut [TokenPoolInfo]) {
    pools.sort_by(|a, b| match (b.is_native_pair, a.is_native_pair) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => match calculate_pool_metric(chain, b)
            .partial_cmp(&calculate_pool_metric(chain, a))
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => a.pool_address.cmp(&b.pool_address),
            ordering => ordering,
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const MEME: &str = "MemeMint1111111111111111111111111111111111";
    const OTHER: &str = "OtherMint111111111111111111111111111111111";

    fn pair(address: &str, base: &str, quote: &str) -> TokenPoolInfo {
        TokenPoolInfo {
            pool_address: address.to_owned(),
            base_mint: base.to_owned(),
            quote_mint: quote.to_owned(),
            is_native_pair: base == SOL || quote == SOL,
            ..TokenPoolInfo::default()
        }
    }

    #[test]
    fn a_dexscreener_list_without_a_native_pair_is_incomplete() {
        let listed = [pair("usdc", MEME, USDC)];
        assert!(dexscreener_list_is_incomplete(&listed, listed.len()));
        assert!(dexscreener_list_is_incomplete(&[], 0));
    }

    #[test]
    fn a_native_pair_under_the_cap_is_complete_and_a_list_at_the_cap_is_not() {
        let listed = [pair("sol", MEME, SOL), pair("usdc", MEME, USDC)];
        assert!(!dexscreener_list_is_incomplete(
            &listed,
            TOKEN_PAIRS_CAP - 1
        ));
        assert!(dexscreener_list_is_incomplete(&listed, TOKEN_PAIRS_CAP));
    }

    #[test]
    fn only_new_native_or_stable_countered_pools_complete_the_list() {
        let mut listed_row = pair("shared", MEME, SOL);
        listed_row.liquidity_usd = Some(1.0);
        let known = HashMap::from([("shared".to_owned(), listed_row)]);
        let mut duplicate = pair("shared", MEME, SOL);
        duplicate.liquidity_usd = Some(999.0);
        let extra = vec![
            duplicate,
            pair("deep-sol", MEME, SOL),
            pair("usdc", MEME, USDC),
            pair("other", MEME, OTHER),
            pair("unrelated", SOL, USDC),
        ];
        let merged: Vec<String> = reliable_completion_pools(ChainId::Solana, MEME, &known, extra)
            .into_iter()
            .map(|pool| pool.pool_address)
            .collect();
        assert_eq!(merged, ["deep-sol", "usdc"]);
        assert_eq!(known["shared"].liquidity_usd, Some(1.0));
    }

    fn native_pool(address: &str, liquidity_native: f64) -> TokenPoolInfo {
        TokenPoolInfo {
            pool_address: address.to_owned(),
            is_native_pair: true,
            liquidity_native: Some(liquidity_native),
            ..TokenPoolInfo::default()
        }
    }

    #[test]
    fn an_incumbent_within_the_keep_ratio_stays_canonical() {
        let pools = [native_pool("pump", 166.2), native_pool("dlmm", 174.0)];
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, Some("pump")).as_deref(),
            Some("pump")
        );
        let pools = [native_pool("pump", 175.8), native_pool("dlmm", 174.0)];
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, Some("dlmm")).as_deref(),
            Some("dlmm")
        );
    }

    #[test]
    fn a_rival_far_above_the_incumbent_wins() {
        let pools = [native_pool("pump", 166.2), native_pool("dlmm", 166.2 * 1.5)];
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, Some("pump")).as_deref(),
            Some("dlmm")
        );
    }

    #[test]
    fn a_missing_or_non_native_incumbent_yields_the_best_pool() {
        let mut pools = vec![native_pool("pump", 166.2), native_pool("dlmm", 174.0)];
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, Some("gone")).as_deref(),
            Some("dlmm")
        );
        pools[0].is_native_pair = false;
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, Some("pump")).as_deref(),
            Some("dlmm")
        );
    }

    #[test]
    fn no_previous_choice_yields_the_best_pool() {
        let pools = [native_pool("pump", 175.8), native_pool("dlmm", 174.0)];
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &pools, None).as_deref(),
            Some("pump")
        );
        assert_eq!(choose_canonical_pool(ChainId::Solana, &[], None), None);
    }

    #[test]
    fn unknown_best_liquidity_keeps_a_valid_incumbent() {
        let mut unknown = native_pool("dlmm", 0.0);
        unknown.liquidity_native = None;
        let mut incumbent = unknown.clone();
        incumbent.pool_address = "pump".to_owned();
        assert_eq!(
            choose_canonical_pool(ChainId::Solana, &[unknown, incumbent], Some("pump")).as_deref(),
            Some("pump")
        );
    }
}
