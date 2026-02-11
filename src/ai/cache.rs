use crate::ai::types::{AiDecision, Priority};
use dashmap::DashMap;
use std::time::{Duration, Instant};

/// Cached AI decision entry
struct CachedEntry {
    decision: AiDecision,
    cached_at: Instant,
}

/// AI response cache with TTL and priority support
pub struct AiCache {
    cache: DashMap<String, CachedEntry>,
    ttl: Duration,
}

impl AiCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached decision if fresh and priority allows
    pub fn get(&self, mint: &str, evaluation_type: &str, priority: Priority) -> Option<AiDecision> {
        // HIGH priority always bypasses cache
        if priority == Priority::High {
            return None;
        }

        let cache_key = format!("{}:{}", evaluation_type, mint);
        let entry = self.cache.get(&cache_key)?;
        if entry.cached_at.elapsed() > self.ttl {
            drop(entry); // Release read lock before removing
            self.cache.remove(&cache_key);
            return None;
        }

        Some(entry.decision.clone())
    }

    /// Insert decision into cache
    pub fn insert(&self, mint: &str, evaluation_type: &str, decision: AiDecision) {
        let cache_key = format!("{}:{}", evaluation_type, mint);
        self.cache.insert(
            cache_key,
            CachedEntry {
                decision,
                cached_at: Instant::now(),
            },
        );
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache stats
    pub fn stats(&self) -> (usize, usize) {
        let total = self.cache.len();
        let fresh = self
            .cache
            .iter()
            .filter(|e| e.cached_at.elapsed() <= self.ttl)
            .count();
        (total, fresh)
    }
}
