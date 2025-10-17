/// Unified cache manager for all token data sources
use super::config::CacheConfig;
use super::types::{CacheEntry, CacheKey, CacheStats};
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Thread-safe cache manager
pub struct CacheManager {
    entries: Arc<DashMap<CacheKey, CacheEntry>>,
    config: CacheConfig,
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            config,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        }
    }

    /// Get cached data if available and not expired
    pub fn get<T>(&self, key: &CacheKey) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let entry = self.entries.get(key)?;

        if entry.is_expired() {
            drop(entry);
            self.entries.remove(key);
            self.misses.fetch_add(1, Ordering::Relaxed);
            self.evictions.fetch_add(1, Ordering::Relaxed);
            return None;
        }

        match serde_json::from_value(entry.data.clone()) {
            Ok(data) => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(data)
            }
            Err(_) => {
                drop(entry);
                self.entries.remove(key);
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Store data in cache
    pub fn set<T>(&self, key: CacheKey, data: &T) -> Result<(), String>
    where
        T: Serialize,
    {
        let json_data = serde_json::to_value(data)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        let ttl = self.config.get_ttl(key.source);
        let entry = CacheEntry::new(json_data, ttl, key.source);

        self.entries.insert(key, entry);
        Ok(())
    }

    /// Remove entry from cache
    pub fn remove(&self, key: &CacheKey) {
        if self.entries.remove(key).is_some() {
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let count = self.entries.len();
        self.entries.clear();
        self.evictions.fetch_add(count as u64, Ordering::Relaxed);
    }

    /// Clear expired entries
    pub fn clear_expired(&self) {
        let mut expired_keys = Vec::new();

        for entry in self.entries.iter() {
            if entry.value().is_expired() {
                expired_keys.push(entry.key().clone());
            }
        }

        for key in expired_keys {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let mut expired_count = 0;
        for entry in self.entries.iter() {
            if entry.value().is_expired() {
                expired_count += 1;
            }
        }

        CacheStats {
            total_entries: self.entries.len(),
            expired_entries: expired_count,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Check if key exists and is valid
    pub fn contains(&self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}
