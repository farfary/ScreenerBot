/// Cache entry types
use crate::tokens_new::types::DataSource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cache key for identifying cached data
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub source: DataSource,
    pub data_type: CacheDataType,
    pub identifier: String,
}

impl CacheKey {
    pub fn new(source: DataSource, data_type: CacheDataType, identifier: String) -> Self {
        Self {
            source,
            data_type,
            identifier,
        }
    }

    pub fn dexscreener_pools(mint: &str) -> Self {
        Self {
            source: DataSource::DexScreener,
            data_type: CacheDataType::Pools,
            identifier: mint.to_string(),
        }
    }

    pub fn geckoterminal_pools(mint: &str) -> Self {
        Self {
            source: DataSource::GeckoTerminal,
            data_type: CacheDataType::Pools,
            identifier: mint.to_string(),
        }
    }

    pub fn rugcheck_info(mint: &str) -> Self {
        Self {
            source: DataSource::Rugcheck,
            data_type: CacheDataType::Info,
            identifier: mint.to_string(),
        }
    }
}

/// Type of cached data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheDataType {
    Pools,
    Info,
}

/// Cache entry with data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub data: serde_json::Value,
    pub cached_at: DateTime<Utc>,
    pub ttl: Duration,
    pub source: DataSource,
}

impl CacheEntry {
    pub fn new(data: serde_json::Value, ttl: Duration, source: DataSource) -> Self {
        Self {
            data,
            cached_at: Utc::now(),
            ttl,
            source,
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() as u64 >= self.ttl.as_secs()
    }

    pub fn age(&self) -> Duration {
        let age = Utc::now() - self.cached_at;
        Duration::from_secs(age.num_seconds().max(0) as u64)
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64 / total as f64) * 100.0
        }
    }
}
