// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The single pool each token is priced from.
//!
//! Discovery ranks a token's pools (`crate::tokens::calculate_pool_metric`) and
//! sends its choice, after the blacklist fallback, to the analyzer. The analyzer
//! records that choice here once the pool is analyzed; it is the only writer.
//! The calculator publishes a price only for the selected pool of its token, and
//! `get_canonical_pool` answers from this map, so there is exactly one ranking.
//! A pool replaced by a newer selection leaves the pool directory, which stops
//! its account fetching (the fetcher derives its work from the directory).

use crate::pools::types::PoolDescriptor;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Token mint -> the pool that token is priced from.
pub type SelectedPools = Arc<RwLock<HashMap<String, Pubkey>>>;

/// Create an empty selection map.
pub fn new_selected_pools() -> SelectedPools {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Record `pool_id` as the pool `mint` is priced from.
///
/// When this replaces a different pool, the replaced pool is removed from
/// `pool_directory` and returned so the caller can release its fetched
/// account data. Returns `None` when the selection is new or unchanged.
pub(crate) fn select_pool(
    selected: &SelectedPools,
    pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    mint: &str,
    pool_id: Pubkey,
) -> Option<Pubkey> {
    let previous = {
        let mut selected = selected.write().unwrap();
        selected.insert(mint.to_owned(), pool_id)
    };

    match previous {
        Some(old) if old != pool_id => {
            pool_directory.write().unwrap().remove(&old);
            Some(old)
        }
        _ => None,
    }
}

/// Whether a price computed from `pool_id` may be published as `mint`'s price.
pub(crate) fn is_selected_pool(
    selected: &HashMap<String, Pubkey>,
    mint: &str,
    pool_id: &Pubkey,
) -> bool {
    selected.get(mint) == Some(pool_id)
}

/// Descriptor of the pool `mint` is priced from, if it is selected and analyzed.
pub(crate) fn selected_descriptor(
    selected: &SelectedPools,
    pool_directory: &Arc<RwLock<HashMap<Pubkey, PoolDescriptor>>>,
    mint: &str,
) -> Option<PoolDescriptor> {
    let pool_id = *selected.read().ok()?.get(mint)?;
    pool_directory.read().ok()?.get(&pool_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::{AssetId, ChainId, PoolId};
    use crate::pools::types::ProtocolId;
    use std::time::Instant;

    fn descriptor(pool_id: &Pubkey) -> PoolDescriptor {
        PoolDescriptor {
            pool_id: PoolId::new(ChainId::Solana, pool_id.to_string()).unwrap(),
            program_kind: ProtocolId::new("RAYDIUM CPMM"),
            base_mint: AssetId::new(ChainId::Solana, "TokenA").unwrap(),
            quote_mint: AssetId::new(ChainId::Solana, crate::chains::solana::constants::SOL_MINT)
                .unwrap(),
            reserve_accounts: Vec::new(),
            liquidity_usd: 0.0,
            volume_h24_usd: 0.0,
            last_updated: Instant::now(),
        }
    }

    #[test]
    fn first_selection_evicts_nothing() {
        let selected = new_selected_pools();
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let pool = Pubkey::new_unique();
        directory.write().unwrap().insert(pool, descriptor(&pool));

        assert_eq!(select_pool(&selected, &directory, "TokenA", pool), None);
        assert_eq!(select_pool(&selected, &directory, "TokenA", pool), None);
        assert!(directory.read().unwrap().contains_key(&pool));
    }

    #[test]
    fn a_new_selection_evicts_the_superseded_pool_from_the_directory() {
        let selected = new_selected_pools();
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let old = Pubkey::new_unique();
        let new = Pubkey::new_unique();
        directory.write().unwrap().insert(old, descriptor(&old));
        directory.write().unwrap().insert(new, descriptor(&new));

        select_pool(&selected, &directory, "TokenA", old);
        assert_eq!(select_pool(&selected, &directory, "TokenA", new), Some(old));

        let directory = directory.read().unwrap();
        assert!(!directory.contains_key(&old));
        assert!(directory.contains_key(&new));
    }

    #[test]
    fn only_the_selected_pool_may_publish_a_price() {
        let selected = new_selected_pools();
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let chosen = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        select_pool(&selected, &directory, "TokenA", chosen);

        let map = selected.read().unwrap();
        assert!(is_selected_pool(&map, "TokenA", &chosen));
        assert!(!is_selected_pool(&map, "TokenA", &other));
        assert!(!is_selected_pool(&map, "TokenB", &chosen));
    }

    #[test]
    fn selected_descriptor_requires_an_analyzed_pool() {
        let selected = new_selected_pools();
        let directory = Arc::new(RwLock::new(HashMap::new()));
        let pool = Pubkey::new_unique();
        select_pool(&selected, &directory, "TokenA", pool);
        assert!(selected_descriptor(&selected, &directory, "TokenA").is_none());

        directory.write().unwrap().insert(pool, descriptor(&pool));
        let found = selected_descriptor(&selected, &directory, "TokenA").expect("selected");
        assert_eq!(found.pool_id.address(), pool.to_string());
        assert!(selected_descriptor(&selected, &directory, "TokenB").is_none());
    }
}
