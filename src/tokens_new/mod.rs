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

// Re-export main types for convenience
pub use provider::{CacheStrategy, TokenDataProvider};
pub use types::{
    ApiError, DataSource, DexScreenerPool, GeckoTerminalPool, RugcheckHolder,
    RugcheckInfo, RugcheckRisk, SocialLink, TokenMetadata, WebsiteLink,
};
