// Three-tier caching system for OHLCV data
//
// INVARIANT: All cached data MUST be stored in ASC timestamp order.
// This ensures consistent behavior across cache hits, DB queries, and aggregations.

use crate::events::{record_ohlcv_event, Severity};
use crate::ohlcvs::types::{Candle, OhlcvError, OhlcvResult, Timeframe};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HOT_CACHE_MAX_TOKENS: usize = 100;
const HOT_CACHE_RETENTION_HOURS: i64 = 24;

#[derive(Clone)]
struct CacheEntry {
    data: Vec<Candle>,
    last_access: Instant,
    created_at: Instant,
}

impl CacheEntry {
    fn new(data: Vec<Candle>) -> Self {
        Self {
            data,
            last_access: Instant::now(),
            created_at: Instant::now(),
        }
    }

    fn access(&mut self) -> &Vec<Candle> {
        self.last_access = Instant::now();
        &self.data
    }

    fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }
}

type CacheKey = (String, Option<String>, Timeframe); // (mint, pool_address, timeframe)

pub struct OhlcvCache {
    hot_cache: Arc<Mutex<HashMap<CacheKey, CacheEntry>>>,
    access_order: Arc<Mutex<VecDeque<CacheKey>>>,
    hit_count: Arc<Mutex<u64>>,
    miss_count: Arc<Mutex<u64>>,
}

impl OhlcvCache {
    pub fn new() -> Self {
        Self {
            hot_cache: Arc::new(Mutex::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(VecDeque::new())),
            hit_count: Arc::new(Mutex::new(0)),
            miss_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Get data from cache
    pub fn get(
        &self,
        mint: &str,
        pool_address: Option<&str>,
        timeframe: Timeframe,
    ) -> OhlcvResult<Option<Vec<Candle>>> {
        let key = (
            mint.to_string(),
            pool_address.map(|s| s.to_string()),
            timeframe,
        );

        let mut cache = self
            .hot_cache
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        if let Some(entry) = cache.get_mut(&key) {
            // Check if expired
            if entry.is_expired(Duration::from_secs(
                3600 * (HOT_CACHE_RETENTION_HOURS as u64),
            )) {
                cache.remove(&key);
                self.record_miss();

                // DEBUG: Record cache expiration
                let mint = mint.to_string();
                let pool_address_owned = pool_address.map(|s| s.to_string());
                let timeframe_str = timeframe.to_string();
                tokio::spawn(async move {
                    record_ohlcv_event(
                        "cache_expired",
                        Severity::Debug,
                        Some(&mint),
                        pool_address_owned.as_deref(),
                        json!({
                            "mint": mint,
                            "pool_address": pool_address_owned,
                            "timeframe": timeframe_str,
                        }),
                    )
                    .await
                });

                return Ok(None);
            }

            // Update access order
            self.update_access_order(&key)?;

            // Record hit
            self.record_hit();

            // DEBUG: Record cache hit (only occasionally to avoid spam)
            if *self.hit_count.lock().unwrap_or_else(|e| e.into_inner()) % 100 == 0 {
                let mint = mint.to_string();
                let pool_address_owned = pool_address.map(|s| s.to_string());
                let timeframe_str = timeframe.to_string();
                let data_len = entry.data.len();
                tokio::spawn(async move {
                    record_ohlcv_event(
                        "cache_hit",
                        Severity::Debug,
                        Some(&mint),
                        pool_address_owned.as_deref(),
                        json!({
                            "mint": mint,
                            "pool_address": pool_address_owned,
                            "timeframe": timeframe_str,
                            "data_points": data_len,
                        }),
                    )
                    .await
                });
            }

            Ok(Some(entry.access().clone()))
        } else {
            self.record_miss();
            Ok(None)
        }
    }

    /// Put data into cache
    pub fn put(
        &self,
        mint: &str,
        pool_address: Option<&str>,
        timeframe: Timeframe,
        data: Vec<Candle>,
    ) -> OhlcvResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        let key = (
            mint.to_string(),
            pool_address.map(|s| s.to_string()),
            timeframe,
        );

        let mut cache = self
            .hot_cache
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        // Check if we need to evict
        if cache.len() >= HOT_CACHE_MAX_TOKENS && !cache.contains_key(&key) {
            self.evict_lru(&mut cache)?;
        }

        cache.insert(key.clone(), CacheEntry::new(data));
        self.update_access_order(&key)?;

        Ok(())
    }

    /// Invalidate cache for a specific token/pool
    pub fn invalidate(
        &self,
        mint: &str,
        pool_address: Option<&str>,
        timeframe: Option<Timeframe>,
    ) -> OhlcvResult<()> {
        let mut cache = self
            .hot_cache
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        let keys_to_remove: Vec<CacheKey> = cache
            .keys()
            .filter(|(m, p, tf)| {
                m == mint
                    && (pool_address.is_none() || pool_address == p.as_deref())
                    && (timeframe.is_none() || timeframe == Some(*tf))
            })
            .cloned()
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
            self.remove_from_access_order(&key)?;
        }

        Ok(())
    }

    /// Clear all cache
    pub fn clear(&self) -> OhlcvResult<()> {
        let mut cache = self
            .hot_cache
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        cache.clear();

        let mut access_order = self
            .access_order
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        access_order.clear();

        Ok(())
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hit_count.lock().unwrap_or_else(|e| e.into_inner());
        let misses = *self.miss_count.lock().unwrap_or_else(|e| e.into_inner());

        let total = hits + misses;
        if total == 0 {
            return 0.0;
        }

        (hits as f64) / (total as f64)
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.hot_cache.lock().map(|cache| cache.len()).unwrap_or(0)
    }

    /// Cleanup expired entries
    pub fn cleanup_expired(&self) -> OhlcvResult<usize> {
        let mut cache = self
            .hot_cache
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        let max_age = Duration::from_secs(3600 * (HOT_CACHE_RETENTION_HOURS as u64));

        let keys_to_remove: Vec<CacheKey> = cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(max_age))
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();

        for key in keys_to_remove {
            cache.remove(&key);
            self.remove_from_access_order(&key)?;
        }

        Ok(count)
    }

    // ==================== Private Methods ====================

    fn record_hit(&self) {
        if let Ok(mut hits) = self.hit_count.lock() {
            *hits += 1;
        }
    }

    fn record_miss(&self) {
        if let Ok(mut misses) = self.miss_count.lock() {
            *misses += 1;
        }
    }

    fn update_access_order(&self, key: &CacheKey) -> OhlcvResult<()> {
        let mut access_order = self
            .access_order
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        // Remove if exists
        access_order.retain(|k| k != key);

        // Add to end (most recently used)
        access_order.push_back(key.clone());

        Ok(())
    }

    fn remove_from_access_order(&self, key: &CacheKey) -> OhlcvResult<()> {
        let mut access_order = self
            .access_order
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        access_order.retain(|k| k != key);

        Ok(())
    }

    fn evict_lru(&self, cache: &mut HashMap<CacheKey, CacheEntry>) -> OhlcvResult<()> {
        let mut access_order = self
            .access_order
            .lock()
            .map_err(|e| OhlcvError::CacheError(format!("Lock error: {}", e)))?;

        while let Some(lru_key) = access_order.pop_front() {
            if cache.remove(&lru_key).is_some() {
                break;
            }
        }

        Ok(())
    }
}

impl Default for OhlcvCache {
    fn default() -> Self {
        Self::new()
    }
}
