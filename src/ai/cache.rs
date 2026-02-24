//! AI response cache — memoizes LLM responses to reduce API calls and latency.

use crate::ai::types::{AiDecision, Priority};
use std::time::Duration;

/// Cached AI decision entry
#[derive(Clone)]
struct CachedEntry {
    decision: AiDecision,
    cached_at: std::time::Instant,
}

/// AI response cache with TTL and priority support — bounded moka cache (max 5K entries).
pub struct AiCache {
    cache: moka::sync::Cache<String, CachedEntry>,
    ttl: Duration,
}

impl AiCache {
    pub fn new(ttl_seconds: u64) -> Self {
        let ttl = Duration::from_secs(ttl_seconds);
        Self {
            cache: moka::sync::Cache::builder()
                .max_capacity(5_000)
                .time_to_live(ttl)
                .build(),
            ttl,
        }
    }

    /// Get cached decision if fresh and priority allows
    pub fn get(&self, mint: &str, evaluation_type: &str, priority: Priority) -> Option<AiDecision> {
        // HIGH priority always bypasses cache
        if priority == Priority::High {
            return None;
        }

        let cache_key = format!("{evaluation_type}:{mint}");
        let entry = self.cache.get(&cache_key)?;
        if entry.cached_at.elapsed() > self.ttl {
            self.cache.invalidate(&cache_key);
            return None;
        }

        Some(entry.decision.clone())
    }

    /// Insert decision into cache
    pub fn insert(&self, mint: &str, evaluation_type: &str, decision: AiDecision) {
        let cache_key = format!("{evaluation_type}:{mint}");
        self.cache.insert(
            cache_key,
            CachedEntry {
                decision,
                cached_at: std::time::Instant::now(),
            },
        );
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get cache stats
    pub fn stats(&self) -> (usize, usize) {
        let total = self.cache.entry_count() as usize;
        // With moka TTL, all entries should be fresh (TTL handles eviction)
        (total, total)
    }
}
