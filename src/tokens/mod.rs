/// New unified token data system with clean architecture
///
/// Architecture:
/// - database.rs: Unified database operations (all SQL in one place)
/// - schema.rs: Database schema definition (6 tables, 12 indexes)
/// - market/: Market data fetchers (DexScreener, GeckoTerminal)
/// - security/: Security data fetchers (Rugcheck)
/// - updates.rs: Priority-based background updates with rate limiting
/// - cleanup.rs: Automatic blacklist management
/// - filtered_store.rs: Centralized storage for filtered token lists
/// - service_new.rs: ServiceManager integration
/// - decimals.rs: Decimals lookup with caching
/// - types.rs: Core domain types
///
/// Note: API clients in crate::apis module
pub mod cleanup;
pub mod database;
pub mod decimals;
pub mod discovery;
pub mod events;
pub mod favorites;
pub mod filtered;
pub mod market;
pub mod pools;
pub mod priorities;
pub mod schema;
pub mod search;
pub mod security;
pub mod service;
pub mod store;
pub mod types;
pub mod updates;

// Re-export main types for convenience
pub use database::{
    // Async wrappers for external code
    count_tokens_async,
    count_tokens_no_market_async,
    get_all_tokens_for_filtering_async,
    get_all_tokens_optional_market_async,
    get_full_token_async,
    get_full_token_for_source_async,
    get_global_database,
    get_token_async,
    get_tokens_no_market_async,
    init_global_database,
    is_market_data_stale_async,
    list_blacklisted_tokens_async,
    list_tokens_async,
    update_token_priority_async,
    TokenBlacklistRecord,
    TokenDatabase,
};
pub use filtered::{
    get_blacklisted_tokens, get_counts as get_filtered_counts, get_filtered_lists,
    get_last_update_time as get_filtered_last_update, get_passed_tokens, get_rejected_tokens,
    get_tokens_with_open_positions, get_tokens_with_pool_price, store_filtered_results,
    FilteredListCounts, FilteredTokenLists,
};
pub use market::{dexscreener, geckoterminal};
pub use security::rugcheck;

// Domain types from types.rs
pub use types::{
    ApiError, DataSource, DexScreenerData, GeckoTerminalData, MarketDataBundle, RugcheckData,
    SecurityBundle, SecurityLevel, SecurityRisk, SecurityScore, SocialLink, Token, TokenError,
    TokenHolder, TokenMetadata, TokenResult, UpdateTrackingInfo, WebsiteLink,
};

// API parsing types from api modules (now in crate::apis)
pub use crate::apis::dexscreener::types::DexScreenerPool;
pub use crate::apis::geckoterminal::types::GeckoTerminalPool;
pub use crate::apis::rugcheck::types::RugcheckInfo;

// Re-export common types from new modules
pub use events::{subscribe as subscribe_events, TokenEvent};
pub use priorities::Priority;

// Re-export store APIs
pub use store::{
    dexscreener_cache_metrics, dexscreener_cache_size, geckoterminal_cache_metrics,
    geckoterminal_cache_size, get_cached_token, invalidate_token_snapshot, refresh_token_snapshot,
    rugcheck_cache_metrics, rugcheck_cache_size, store_token_snapshot, CacheMetrics,
};

// Re-export pools APIs
pub use pools::{
    cache_metrics as pool_cache_metrics, calculate_pool_metric, choose_canonical_pool,
    clear_cache as clear_pool_cache, extract_dex_label, extract_pool_liquidity,
    fetch_immediate as fetch_token_pools_immediate, get_snapshot as get_token_pools_snapshot,
    get_snapshot_allow_stale as get_token_pools_snapshot_allow_stale, merge_pool_info,
    prefetch as prefetch_token_pools,
};

// Re-export decimals API
pub use decimals::{
    cache as cache_decimals, clear_all_cache as clear_all_decimals_cache,
    clear_cache as clear_decimals_cache, get as get_decimals, get_cached as get_cached_decimals,
    get_token_decimals_from_chain, SOL_DECIMALS, SOL_MINT,
};

// Re-export updates API
pub use updates::UpdateResult;

// Re-export search API
pub use search::{search_tokens, SearchResults, TokenSearchResult};

// Re-export favorites API
pub use favorites::{
    add_favorite_async, get_favorite_async, get_favorites_async, get_favorites_count_async,
    is_favorite_async, remove_favorite_async, update_favorite_async, AddFavoriteRequest,
    FavoriteToken, UpdateFavoriteRequest,
};

// ============================================================================
// PUBLIC FORCE UPDATE API
// ============================================================================

/// Request immediate update for a token (bypasses normal scheduling)
///
/// This function provides on-demand token data refresh for use cases like
/// viewing token details where user expects fresh data immediately.
///
/// Fetches from ALL sources in parallel:
/// - DexScreener (market data)
/// - GeckoTerminal (market data)
/// - Rugcheck (security data)
///
/// # Arguments
/// * `mint` - Token address to update
///
/// # Returns
/// UpdateResult with success/failure details from each data source
///
/// # Example
/// ```no_run
/// match request_immediate_update("TokenMintAddress").await {
///     Ok(result) if result.is_success() => println!("Updated successfully"),
///     Ok(result) => println!("Update failed: {:?}", result.failures),
///     Err(e) => println!("Error: {}", e),
/// }
/// ```
pub async fn request_immediate_update(mint: &str) -> TokenResult<UpdateResult> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Token database not initialized".to_string()))?;

    let coordinator = service::get_rate_coordinator()
        .ok_or_else(|| TokenError::Database("Rate limit coordinator not available".to_string()))?;

    updates::force_update_token(mint, db, coordinator).await
}
