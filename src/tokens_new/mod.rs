/// New unified token data system
///
/// Architecture:
/// - api/: Pure API clients for external data sources
/// - cache/: Unified caching with configurable TTLs
/// - storage/: Database layer with separate tables per source
/// - provider/: High-level data access interface
/// - types.rs: Core domain types

pub mod api;
pub mod cache;
pub mod provider;
pub mod storage;
pub mod types;
pub mod decimals;
pub mod blacklist;
pub mod store;
pub mod pools;
pub mod priorities;
pub mod events;
pub mod discovery;
pub mod service;

// Re-export main types for convenience
pub use provider::{CacheStrategy, TokenDataProvider};
pub use types::{
    ApiError, DataSource, DexScreenerPool, GeckoTerminalPool, RugcheckHolder,
    RugcheckInfo, RugcheckRisk, SocialLink, TokenMetadata, WebsiteLink,
};

// Re-export common types from new modules
pub use priorities::Priority;
pub use store::{BestPoolSummary, Snapshot as TokenSnapshot, get_snapshot, set_priority, list_mints, all_snapshots};
pub use events::{TokenEvent, subscribe as subscribe_events};
