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

pub use engine::apply_all_filters as evaluate_token;
pub use error::{Error, Result};
pub use types::{
    BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringSnapshot,
    FilteringStatsSnapshot, FilteringView, PassedToken, RejectedToken, SnapshotState,
    SortDirection, TokenSortKey,
};

/// Obtain filtered token mint list for trading and pool services
pub async fn get_filtered_token_mints() -> Result<Vec<String>> {
    store::get_filtered_mints().await
}

/// Obtain full passed tokens list
pub async fn get_passed_tokens() -> Result<Vec<PassedToken>> {
    store::get_passed_tokens().await
}

/// Rebuild the cached filtering snapshot synchronously (used by services)
pub async fn refresh() -> Result<()> {
    store::refresh_snapshot().await
}

/// Query token listings according to filtering parameters
pub async fn query_tokens(query: FilteringQuery) -> Result<FilteringQueryResult> {
    store::execute_query(query).await
}

/// Snapshot statistics for dashboard metrics.
///
/// Waits for the first snapshot if none exists yet. Callers on a first-paint path want
/// [`try_fetch_stats`] instead.
pub async fn fetch_stats() -> Result<FilteringStatsSnapshot> {
    store::get_stats().await
}

/// Snapshot statistics if a snapshot already exists, `None` while the first one is building.
pub async fn try_fetch_stats() -> Option<FilteringStatsSnapshot> {
    store::stats_if_ready().await
}

/// Access to the global filtering store (primarily for services)
pub fn global_store() -> std::sync::Arc<store::FilteringStore> {
    store::global_store()
}
