/// New unified token data system
///
/// Architecture:
/// - cache/: Unified caching with configurable TTLs
/// - storage/: Database layer with separate tables per source
/// - provider/: High-level data access interface
/// - types.rs: Core domain types
///
/// Note: API clients moved to crate::apis module
pub mod blacklist;
pub mod cache;
pub mod decimals;
pub mod discovery;
pub mod events;
pub mod priorities;
pub mod provider;
pub mod security;
pub mod service;
pub mod storage;
pub mod store;
pub mod types;

// Re-export main types for convenience
pub use provider::{CacheStrategy, TokenDataProvider};

// Domain types from types.rs
pub use types::{
    ApiError, DataSource, SecurityRisk, SocialLink, Token, TokenHolder, TokenMetadata, WebsiteLink,
};

// API parsing types from api modules (now in crate::apis)
pub use crate::apis::dexscreener_types::DexScreenerPool;
pub use crate::apis::geckoterminal_types::GeckoTerminalPool;
pub use crate::apis::rugcheck_types::RugcheckInfo;

// Re-export common types from new modules
pub use events::{subscribe as subscribe_events, TokenEvent};
pub use priorities::Priority;
pub use store::{
    all_tokens, count_tokens, filter_blacklisted, get_by_priority, get_by_source,
    get_recently_updated, get_token, list_mints, search_tokens, set_priority, token_exists,
};

// Re-export decimals API
pub use decimals::{get as get_decimals, get_cached as get_cached_decimals};
