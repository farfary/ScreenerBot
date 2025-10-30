/// Token pools submodule - centralized pool management
///
/// This submodule owns ALL token pool operations across the system.
///
/// Architecture:
/// - api.rs: Fetching pool data from external APIs (DexScreener, GeckoTerminal)
/// - conversion.rs: Converting API responses to TokenPoolInfo
/// - operations.rs: Merging, deduplication, canonical selection
/// - cache.rs: Caching layer with TTL and stale fallback
/// - utils.rs: Helper functions (parsing, metrics, extraction)
///
/// Public API:
/// - get_snapshot(): Get fresh pool snapshot (60s cache)
/// - get_snapshot_allow_stale(): Get snapshot with stale fallback
/// - prefetch(): Background prefetch for multiple tokens
/// - clear_cache(): Clear pool cache
/// - Utility functions for pool metrics and selection
mod api;
mod cache;
mod conversion;
mod operations;
mod utils;

// Re-export public API
pub use cache::{
    clear_cache, get_snapshot, get_snapshot_allow_stale, metrics as cache_metrics, prefetch,
};

pub use operations::{choose_canonical_pool, merge_pool_info, sort_pools_for_snapshot};

pub use utils::{calculate_pool_metric, extract_dex_label, extract_pool_liquidity, parse_f64};
