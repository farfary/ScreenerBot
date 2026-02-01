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
    pub fn get(&self, mint: &str, priority: Priority) -> Option<AiDecision> {
        // HIGH priority always bypasses cache
        if priority == Priority::High {
            return None;
        }

        let entry = self.cache.get(mint)?;
        if entry.cached_at.elapsed() > self.ttl {
            self.cache.remove(mint);
            return None;
        }

        Some(entry.decision.clone())
    }

    /// Insert decision into cache
    pub fn insert(&self, mint: &str, decision: AiDecision) {
        self.cache.insert(
            mint.to_string(),
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
