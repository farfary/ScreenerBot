/// Centralized storage for filtered token lists
///
/// This module stores the results from the filtering system for consumption by other services.
/// It provides a single source of truth for which tokens have passed filtering, been rejected,
/// or are blacklisted.
///
/// Architecture:
/// - Filtering engine computes snapshot and stores results here
/// - Pool service gets passed tokens from here
/// - Dashboard gets stats from here
/// - Trader gets available tokens from here
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
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
static FILTERED_LISTS: Lazy<RwLock<FilteredTokenLists>> =
    Lazy::new(|| RwLock::new(FilteredTokenLists::default()));

/// Store filtered results from filtering system
///
/// Called by filtering engine after computing snapshot.
/// Updates the centralized filtered lists for all consumers.
pub fn store_filtered_results(lists: FilteredTokenLists) {
    let mut guard = FILTERED_LISTS.write().expect("filtered lists poisoned");
    *guard = lists;
}

/// Get tokens that passed all filters (for pool service)
///
/// Returns list of token mints that are currently available for trading.
pub fn get_passed_tokens() -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.passed.clone()
}

/// Get rejected tokens
///
/// Returns list of token mints that failed one or more filters.
pub fn get_rejected_tokens() -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.rejected.clone()
}

/// Get blacklisted tokens
///
/// Returns list of token mints that are permanently blacklisted.
pub fn get_blacklisted_tokens() -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.blacklisted.clone()
}

/// Mark a token as blacklisted in the in-memory snapshot
///
/// Updates the filtered lists so downstream consumers observe the change
/// immediately, even before the next filtering refresh persists new state.
pub fn mark_token_blacklisted(mint: &str) {
    let mut guard = FILTERED_LISTS.write().expect("filtered lists poisoned");
    let needle = mint.to_string();

    if !guard.blacklisted.iter().any(|entry| entry == &needle) {
        guard.blacklisted.push(needle.clone());
    }

    guard.passed.retain(|entry| entry != &needle);
    guard.with_pool_price.retain(|entry| entry != &needle);

    guard.updated_at = Utc::now();
}

/// Get tokens with pool price
///
/// Returns list of token mints that have pricing data from pools.
pub fn get_tokens_with_pool_price() -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.with_pool_price.clone()
}

/// Get tokens with open positions
///
/// Returns list of token mints that have active trading positions.
pub fn get_tokens_with_open_positions() -> Vec<String> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.open_positions.clone()
}

/// Get last update time
///
/// Returns timestamp of when filtered lists were last updated.
pub fn get_last_update_time() -> DateTime<Utc> {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.updated_at
}

/// Get full snapshot of filtered lists
///
/// Returns complete filtered lists with all categories.
pub fn get_filtered_lists() -> FilteredTokenLists {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    guard.clone()
}

/// Get counts for each category (useful for stats)
pub fn get_counts() -> FilteredListCounts {
    let guard = FILTERED_LISTS.read().expect("filtered lists poisoned");
    FilteredListCounts {
        passed: guard.passed.len(),
        rejected: guard.rejected.len(),
        blacklisted: guard.blacklisted.len(),
        with_pool_price: guard.with_pool_price.len(),
        open_positions: guard.open_positions.len(),
        updated_at: guard.updated_at,
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
