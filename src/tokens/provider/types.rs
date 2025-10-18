// Provider module types: High-level data types for token information

use crate::apis::dexscreener_types::DexScreenerPool;
use crate::apis::geckoterminal_types::GeckoTerminalPool;
use crate::apis::rugcheck_types::RugcheckInfo;
use crate::tokens::types::{DataSource, TokenMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete token data from all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTokenData {
    pub mint: String,
    pub metadata: TokenMetadata,
    pub rugcheck_info: Option<RugcheckInfo>,
    pub sources_used: Vec<DataSource>,
    pub fetch_timestamp: DateTime<Utc>,
    pub cache_hits: Vec<DataSource>,
    pub cache_misses: Vec<DataSource>,
}

/// Options for fetching token data
#[derive(Debug, Clone)]
pub struct FetchOptions {
    /// Which data sources to query
    pub sources: Vec<DataSource>,
    /// Cache strategy to use
    pub cache_strategy: CacheStrategy,
    /// Maximum acceptable data age (None = use default TTLs)
    pub max_age_seconds: Option<u64>,
    /// Whether to save fetched data to database
    pub persist: bool,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            sources: vec![
                DataSource::DexScreener,
                DataSource::GeckoTerminal,
                DataSource::Rugcheck,
            ],
            cache_strategy: CacheStrategy::CacheFirst,
            max_age_seconds: None,
            persist: true,
        }
    }
}

/// Cache strategy for data fetching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Try cache first, fetch if miss or expired
    CacheFirst,
    /// Always fetch fresh data, update cache
    NetworkFirst,
    /// Only use cache, never fetch
    CacheOnly,
    /// Only fetch from network, ignore cache
    NetworkOnly,
}

/// Result of a fetch operation
#[derive(Debug)]
pub struct FetchResult<T> {
    pub data: T,
    pub source: DataSource,
    pub from_cache: bool,
    pub fetch_duration_ms: u64,
}

/// Statistics for provider operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderStats {
    pub total_fetches: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub api_calls: u64,
    pub database_saves: u64,
    pub errors: u64,
}

impl ProviderStats {
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_fetches == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / self.total_fetches as f64) * 100.0
        }
    }
}
