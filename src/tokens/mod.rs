/// New unified token data system
///
/// Architecture:
/// - api/: Pure API clients for external data sources
/// - cache/: Unified caching with configurable TTLs
/// - storage/: Database layer with separate tables per source
/// - provider/: High-level data access interface
/// - types.rs: Core domain types
pub mod api;
pub mod blacklist;
pub mod cache;
pub mod decimals;
pub mod discovery;
pub mod events;
pub mod pools;
pub mod priorities;
pub mod provider;
pub mod service;
pub mod storage;
pub mod store;
pub mod types;

// Re-export main types for convenience
pub use provider::{CacheStrategy, TokenDataProvider};

// Domain types from types.rs
pub use types::{
    ApiError, DataSource, PoolSummary, RugcheckHolder, RugcheckRisk, SocialLink, Token,
    TokenMetadata, WebsiteLink,
};

// API parsing types from api modules
pub use api::dexscreener_types::DexScreenerPool;
pub use api::geckoterminal_types::GeckoTerminalPool;
pub use api::rugcheck_types::RugcheckInfo;

// Re-export common types from new modules
pub use events::{subscribe as subscribe_events, TokenEvent};
pub use priorities::Priority;
pub use store::{
    all_snapshots, count_tokens, filter_blacklisted, get_by_min_liquidity, get_by_priority,
    get_recently_updated, get_snapshot, list_mints, search_tokens, set_priority, BestPoolSummary,
    Snapshot as TokenSnapshot,
};

// Re-export decimals API
pub use decimals::{get as get_decimals, get_cached as get_cached_decimals};
