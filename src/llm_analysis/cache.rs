//! LLM analysis response cache — memoizes model decisions to cut provider calls and latency.

use crate::llm_analysis::types::{AnalysisDecision, Priority};
use std::time::Duration;

/// Cached analysis decision entry
#[derive(Clone)]
struct CachedEntry {
    decision: AnalysisDecision,
    cached_at: std::time::Instant,
}

/// Analysis response cache with TTL and priority support — bounded moka cache (max 5K entries).
pub struct AnalysisCache {
    cache: moka::sync::Cache<String, CachedEntry>,
    ttl: Duration,
}

impl AnalysisCache {
    /// Create a new analysis cache with the given time-to-live in seconds
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
    pub fn get(
        &self,
        mint: &str,
        evaluation_type: &str,
        priority: Priority,
    ) -> Option<AnalysisDecision> {
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
    pub fn insert(&self, mint: &str, evaluation_type: &str, decision: AnalysisDecision) {
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
