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
pub mod filtered_store;
pub mod market;
pub mod priorities;
pub mod schema;
pub mod security;
pub mod service_new;
pub mod store;
pub mod types;
pub mod updates;

// Re-export main types for convenience
pub use database::{
    get_global_database,
    // Async wrappers for external code
    get_token_async,
    init_global_database,
    list_tokens_async,
    TokenDatabase,
};
pub use filtered_store::{
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
    geckoterminal_cache_size, get_cached_token, get_full_token_async, invalidate_token_snapshot,
    refresh_token_snapshot, rugcheck_cache_metrics, rugcheck_cache_size, store_token_snapshot,
    CacheMetrics,
};

// Re-export decimals API
pub use decimals::{
    cache as cache_decimals, clear_all_cache as clear_all_decimals_cache,
    clear_cache as clear_decimals_cache, get as get_decimals, get_cached as get_cached_decimals,
    get_token_decimals_from_chain, SOL_DECIMALS, SOL_MINT, WSOL_MINT,
};
