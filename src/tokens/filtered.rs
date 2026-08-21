//! Centralized storage for filtered token lists
//!
//! This module stores the results from the filtering system for consumption by other services.
//! It provides a single source of truth for which tokens have passed filtering, been rejected,
//! or are blacklisted.
//!
//! Architecture:
//! - Filtering engine computes snapshot and stores results here
//! - Pool service gets passed tokens from here
//! - Dashboard gets stats from here
//! - Trader gets available tokens from here

use crate::chains::ChainId;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;

/// Filtered token lists with metadata
#[derive(Clone, Debug)]
pub struct FilteredTokenLists {
    /// Tokens that passed all filters and are available for trading
    pub passed: Vec<String>,
    /// Tokens that were rejected by one or more filters
    pub rejected: Vec<String>,
    /// Tokens that are permanently blacklisted
    pub blacklisted: Vec<String>,
    /// Tokens that have pool price data available
    pub with_pool_price: Vec<String>,
    /// Tokens with open positions
    pub open_positions: Vec<String>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Default for FilteredTokenLists {
    fn default() -> Self {
        Self {
            passed: Vec::new(),
            rejected: Vec::new(),
            blacklisted: Vec::new(),
            with_pool_price: Vec::new(),
            open_positions: Vec::new(),
            updated_at: Utc::now(),
        }
    }
}

/// Global storage for filtered lists
static FILTERED_LISTS: LazyLock<RwLock<HashMap<ChainId, FilteredTokenLists>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Store filtered results from filtering system
///
/// Called by filtering engine after computing snapshot.
/// Updates the centralized filtered lists for all consumers.
pub fn store_filtered_results(chain: ChainId, lists: FilteredTokenLists) {
    let mut guard = FILTERED_LISTS.write().expect("filtered lists poisoned");
    guard.insert(chain, lists);
}

/// Remove the cached filtering result for one chain.
pub fn clear_filtered_results(chain: ChainId) {
    let mut guard = FILTERED_LISTS.write().expect("filtered lists poisoned");
    guard.remove(&chain);
}

/// Get tokens that passed all filters (for pool service)
///
/// Returns list of token mints that are currently available for trading.
pub fn get_passed_tokens(chain: ChainId) -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.passed.clone())
        .unwrap_or_default()
}

/// Get rejected tokens
///
/// Returns list of token mints that failed one or more filters.
pub fn get_rejected_tokens(chain: ChainId) -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.rejected.clone())
        .unwrap_or_default()
}

/// Get blacklisted tokens
///
/// Returns list of token mints that are permanently blacklisted.
pub fn get_blacklisted_tokens(chain: ChainId) -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.blacklisted.clone())
        .unwrap_or_default()
}

/// Get tokens with pool price
///
/// Returns list of token mints that have pricing data from pools.
pub fn get_tokens_with_pool_price(chain: ChainId) -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.with_pool_price.clone())
        .unwrap_or_default()
}

/// Get tokens with open positions
///
/// Returns list of token mints that have active trading positions.
pub fn get_tokens_with_open_positions(chain: ChainId) -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.open_positions.clone())
        .unwrap_or_default()
}

/// Get last update time
///
/// Returns timestamp of when filtered lists were last updated.
pub fn get_last_update_time(chain: ChainId) -> DateTime<Utc> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard
        .get(&chain)
        .map(|lists| lists.updated_at)
        .unwrap_or_else(Utc::now)
}

/// Get full snapshot of filtered lists
///
/// Returns complete filtered lists with all categories.
pub fn get_filtered_lists(chain: ChainId) -> FilteredTokenLists {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.get(&chain).cloned().unwrap_or_default()
}

/// Get counts for each category (useful for stats)
pub fn get_counts(chain: ChainId) -> FilteredListCounts {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    let lists = guard.get(&chain).cloned().unwrap_or_default();
    FilteredListCounts {
        passed: lists.passed.len(),
        rejected: lists.rejected.len(),
        blacklisted: lists.blacklisted.len(),
        with_pool_price: lists.with_pool_price.len(),
        open_positions: lists.open_positions.len(),
        updated_at: lists.updated_at,
    }
}

/// Counts for each filtered list category
#[derive(Clone, Debug)]
pub struct FilteredListCounts {
    pub passed: usize,
    pub rejected: usize,
    pub blacklisted: usize,
    pub with_pool_price: usize,
    pub open_positions: usize,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_lists_require_a_chain_scope_for_store_read_and_invalidation() {
        let chain = ChainId::Solana;
        clear_filtered_results(chain);
        store_filtered_results(
            chain,
            FilteredTokenLists {
                passed: vec!["mint".to_owned()],
                ..Default::default()
            },
        );

        assert_eq!(get_passed_tokens(chain), ["mint"]);
        clear_filtered_results(chain);
        assert!(get_passed_tokens(chain).is_empty());
    }
}
