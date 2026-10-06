// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token filtering engine — evaluates tokens against user-defined filter criteria.
pub mod background;
mod engine;
mod error;
pub mod sources;
mod store;
mod store_helpers;
#[cfg(test)]
mod store_helpers_tests;
pub mod types;

use crate::chains::{ChainId, ChainScope};

pub use engine::apply_all_filters as evaluate_token;
pub use error::{Error, Result};
pub use types::{
    BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringSnapshot,
    FilteringStatsSnapshot, FilteringView, PassedToken, RejectedToken, SnapshotState,
    SortDirection, TokenSortKey,
};

pub use store::{store, FilteringStore};

/// Obtain one chain's filtered token mint list for trading and pool services
pub async fn get_filtered_token_mints(chain: ChainId) -> Result<Vec<String>> {
    store(chain).get_filtered_mints().await
}

/// Obtain the passed tokens of every chain in `scope`
pub async fn get_passed_tokens(scope: ChainScope) -> Result<Vec<PassedToken>> {
    let mut passed = Vec::new();
    for chain in scope.chains() {
        passed.extend(store(chain).get_passed_tokens().await?);
    }
    Ok(passed)
}

/// Rebuild the cached filtering snapshot of every chain in `scope` (used by services)
pub async fn refresh(scope: ChainScope) -> Result<()> {
    for chain in scope.chains() {
        store(chain).refresh().await?;
    }
    Ok(())
}

/// Query one chain's token listings according to filtering parameters
pub async fn query_tokens(chain: ChainId, query: FilteringQuery) -> Result<FilteringQueryResult> {
    store(chain).execute_query(query).await
}

/// Snapshot statistics for dashboard metrics, summed over every chain in `scope`.
///
/// Waits for the first snapshot if none exists yet. Callers on a first-paint path want
/// [`try_fetch_stats`] instead.
pub async fn fetch_stats(scope: ChainScope) -> Result<FilteringStatsSnapshot> {
    let mut total: Option<FilteringStatsSnapshot> = None;
    for chain in scope.chains() {
        let stats = store(chain).get_stats().await?;
        total = Some(combine_stats(total, stats));
    }
    // A scope that covers no enabled chain has no statistics to report.
    total.ok_or_else(|| {
        Error::Chain(match scope {
            ChainScope::One(chain) => crate::chains::Error::ChainNotEnabled { chain },
            ChainScope::All => crate::chains::Error::AmbiguousChain {
                candidates: Vec::new(),
            },
        })
    })
}

/// Snapshot statistics summed over every chain in `scope`, if each of those chains already
/// has a snapshot; `None` while any of them is still building its first one.
pub async fn try_fetch_stats(scope: ChainScope) -> Option<FilteringStatsSnapshot> {
    let mut total: Option<FilteringStatsSnapshot> = None;
    for chain in scope.chains() {
        let stats = store(chain).stats_if_ready().await?;
        total = Some(combine_stats(total, stats));
    }
    total
}

/// Adds every count and keeps the oldest `updated_at`, so a sum is never fresher than
/// its stalest chain.
fn combine_stats(
    total: Option<FilteringStatsSnapshot>,
    stats: FilteringStatsSnapshot,
) -> FilteringStatsSnapshot {
    let Some(total) = total else {
        return stats;
    };
    FilteringStatsSnapshot {
        total_tokens_in_database: total.total_tokens_in_database + stats.total_tokens_in_database,
        total_tokens: total.total_tokens + stats.total_tokens,
        with_pool_price: total.with_pool_price + stats.with_pool_price,
        open_positions: total.open_positions + stats.open_positions,
        blacklisted: total.blacklisted + stats.blacklisted,
        with_ohlcv: total.with_ohlcv + stats.with_ohlcv,
        passed_filtering: total.passed_filtering + stats.passed_filtering,
        updated_at: total.updated_at.min(stats.updated_at),
    }
}
