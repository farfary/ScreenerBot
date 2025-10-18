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
pub use types::{
    ApiError, DataSource, DexScreenerPool, GeckoTerminalPool, RugcheckHolder, RugcheckInfo,
    RugcheckRisk, SocialLink, TokenMetadata, WebsiteLink,
};

// Re-export common types from new modules
pub use events::{subscribe as subscribe_events, TokenEvent};
pub use priorities::Priority;
pub use store::{
    all_snapshots, get_snapshot, list_mints, set_priority, BestPoolSummary,
    Snapshot as TokenSnapshot,
};
