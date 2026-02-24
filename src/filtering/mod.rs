//! Token filtering engine — evaluates tokens against user-defined filter criteria.
pub mod background;
mod engine;
pub mod sources;
mod store;
pub mod types;

pub use types::{
    BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringSnapshot,
    FilteringStatsSnapshot, FilteringView, PassedToken, RejectedToken, SortDirection, TokenSortKey,
};

/// Obtain filtered token mint list for trading and pool services
pub async fn get_filtered_token_mints() -> Result<Vec<String>, String> {
    store::get_filtered_mints().await
}

/// Obtain full passed tokens list
pub async fn get_passed_tokens() -> Result<Vec<PassedToken>, String> {
    store::get_passed_tokens().await
}

/// Rebuild the cached filtering snapshot synchronously (used by services)
pub async fn refresh() -> Result<(), String> {
    store::refresh_snapshot().await
}

/// Query token listings according to filtering parameters
pub async fn query_tokens(query: FilteringQuery) -> Result<FilteringQueryResult, String> {
    store::execute_query(query).await
}

/// Snapshot statistics for dashboard metrics
pub async fn fetch_stats() -> Result<FilteringStatsSnapshot, String> {
    store::get_stats().await
}

/// Access to the global filtering store (primarily for services)
pub fn global_store() -> std::sync::Arc<store::FilteringStore> {
    store::global_store()
}
