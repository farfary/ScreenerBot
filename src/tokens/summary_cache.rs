use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::tokens::summary::TokenSummary;

/// In-memory cache of the most recent `TokenSummary` per mint.
///
/// The token monitor emits realtime summaries whenever tokens are fetched
/// or refreshed. We store a copy of each summary here so that snapshot
/// requests (e.g. WebSocket subscriptions) can be served without having to
/// rebuild heavy context (security/positions/ohlcv) from scratch.
static SUMMARY_CACHE: Lazy<DashMap<String, TokenSummary>> = Lazy::new(DashMap::new);

/// Tracks the Unix timestamp (seconds) of the most recent cache update.
static LAST_UPDATED: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

/// Store or update the cached summary for a mint.
pub fn store(summary: TokenSummary) {
    LAST_UPDATED.store(current_unix_ts(), Ordering::Relaxed);
    SUMMARY_CACHE.insert(summary.mint.clone(), summary);
}

/// Remove a token from the cache when it is deleted.
pub fn remove(mint: &str) {
    SUMMARY_CACHE.remove(mint);
    LAST_UPDATED.store(current_unix_ts(), Ordering::Relaxed);
}

/// Get a cached summary for a single mint.
pub fn get(mint: &str) -> Option<TokenSummary> {
    SUMMARY_CACHE.get(mint).map(|entry| entry.clone())
}

/// Get cached summaries for a list of mints, returning any that were missing.
pub fn get_for_mints(mints: &[String]) -> (Vec<TokenSummary>, Vec<String>) {
    let mut summaries = Vec::with_capacity(mints.len());
    let mut missing = Vec::new();

    for mint in mints {
        match SUMMARY_CACHE.get(mint) {
            Some(entry) => summaries.push(entry.clone()),
            None => missing.push(mint.clone()),
        }
    }

    (summaries, missing)
}

/// Return all cached summaries.
pub fn all() -> Vec<TokenSummary> {
    SUMMARY_CACHE.iter().map(|entry| entry.clone()).collect()
}

/// Cache statistics for observability.
#[derive(Debug, Clone)]
pub struct SummaryCacheStats {
    pub entries: usize,
    pub last_updated_unix: u64,
}

/// Get current cache stats.
pub fn stats() -> SummaryCacheStats {
    SummaryCacheStats {
        entries: SUMMARY_CACHE.len(),
        last_updated_unix: LAST_UPDATED.load(Ordering::Relaxed),
    }
}

fn current_unix_ts() -> u64 {
    chrono::Utc::now().timestamp() as u64
}

/// Pre-warm cache by loading all tokens from database at startup.
///
/// This ensures the cache is fully populated before the webserver accepts
/// connections, eliminating cold-start cache misses and fallback paths.
pub async fn prewarm_from_database() -> Result<usize, String> {
    use crate::tokens::{
        cache::TokenDatabase,
        summary::{token_to_summary, TokenSummaryContext},
    };

    let db = TokenDatabase::new().map_err(|e| format!("DB init failed: {}", e))?;

    // Get all tokens from database
    let all_tokens = db.get_all_tokens().await?;
    if all_tokens.is_empty() {
        return Ok(0);
    }

    // Build context ONCE for entire batch (efficient)
    let mints: Vec<String> = all_tokens.iter().map(|t| t.mint.clone()).collect();
    let context = TokenSummaryContext::build(&mints).await;

    // Store all summaries in cache
    let mut count = 0;
    for token in all_tokens {
        let summary = token_to_summary(&token, &context);
        store(summary);
        count += 1;
    }

    Ok(count)
}
