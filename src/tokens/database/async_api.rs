//! Token database async API — async wrappers around synchronous SQLite operations.

use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::filtering::RejectedToken;
use crate::tokens::store;
use crate::tokens::types::{
    DataSource, DexScreenerData, GeckoTerminalData, Priority, RugcheckData, Token, TokenError,
    TokenMetadata, TokenPoolsSnapshot, TokenResult, UpdateTrackingInfo,
};

use super::{get_global_database, TokenBlacklistRecord};

/// Fetch token metadata asynchronously via spawn_blocking
pub async fn get_token_async(mint: &str) -> TokenResult<Option<TokenMetadata>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    let mint = mint.to_string();
    tokio::task::spawn_blocking(move || db.get_token(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_token_images_batch (returns HashMap<mint, image_url>)
/// Fetches image URLs for multiple tokens in a single query - use for batch operations
pub async fn get_token_images_batch_async(
    mints: Vec<String>,
) -> TokenResult<HashMap<String, String>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.get_token_images_batch(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_token_decimals_batch (returns HashMap<mint, decimals>)
pub async fn get_token_decimals_batch_async(
    mints: Vec<String>,
) -> TokenResult<HashMap<String, u8>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.get_token_decimals_batch(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_token_info_batch (returns HashMap<mint, (symbol, name, image_url)>)
/// Fetches basic token info for multiple tokens in a single query - use for display purposes
pub async fn get_token_info_batch_async(
    mints: Vec<String>,
) -> TokenResult<HashMap<String, (Option<String>, Option<String>, Option<String>)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.get_token_info_batch(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_full_token (returns complete Token)
pub async fn get_full_token_async(mint: &str) -> TokenResult<Option<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    let mint = mint.to_string();
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_full_token(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_full_token_for_source (returns complete Token with specific source)
pub async fn get_full_token_for_source_async(
    mint: &str,
    source: DataSource,
) -> TokenResult<Option<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    let mint = mint.to_string();
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_full_token_for_source(&mint, source))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_token_pools (returns aggregated pool snapshot)
pub async fn get_token_pools_async(mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    let mint = mint.to_string();
    let db_clone = db.clone();
    tokio::task::spawn_blocking(move || db_clone.get_token_pools(&mint))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for replace_token_pools (persist aggregated pool snapshot)
pub async fn replace_token_pools_async(snapshot: TokenPoolsSnapshot) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    let mint = snapshot.mint.clone();

    tokio::task::spawn_blocking(move || db.replace_token_pools(&snapshot))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))??;

    // Invalidate cache after successful pool replacement
    store::invalidate_token_snapshot(&mint);

    Ok(())
}

/// Async wrapper for list_tokens (returns Vec<TokenMetadata>)
pub async fn list_tokens_async(limit: usize) -> TokenResult<Vec<TokenMetadata>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.list_tokens(limit))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for listing all token blacklist entries
pub async fn list_blacklisted_tokens_async() -> TokenResult<Vec<TokenBlacklistRecord>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.list_blacklisted_tokens())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper to count total tokens in database (fast, no data loading)
pub async fn count_tokens_async() -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || {
        let conn = db
            .conn
            .lock()
            .map_err(|e| TokenError::Database(format!("Lock failed: {e}")))?;

        let count: usize = conn
            .query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))
            .map_err(|e| TokenError::Database(format!("Count query failed: {e}")))?;

        Ok(count)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async wrapper for get_all_tokens_optional_market (returns Vec<Token> with optional market data)
pub async fn get_all_tokens_optional_market_async(
    limit: usize,
    offset: usize,
    sort_by: Option<String>,
    sort_direction: Option<String>,
) -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || {
        db.get_all_tokens_optional_market(
            limit,
            offset,
            sort_by.as_deref(),
            sort_direction.as_deref(),
            false, // Load all tokens including those without market data
        )
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: Load tokens for filtering with market data optimization.
/// PERF: Uses efficient JOINs to avoid N+1 query problem.
/// PERF: Only loads tokens with DexScreener OR GeckoTerminal data (reduces 144k -> ~56k tokens).
/// Returns tokens with market data and security fields needed for filtering.
pub async fn get_all_tokens_for_filtering_async() -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || {
        // PERF: require_market_data=true reduces initial load by ~60%
        // Tokens without market data are immediately rejected anyway (dex_data_missing)
        db.get_all_tokens_optional_market(0, 0, None, None, true)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: count tokens with no market
pub async fn count_tokens_no_market_async() -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.count_tokens_no_market())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get tokens with no market
pub async fn get_tokens_no_market_async(
    limit: usize,
    offset: usize,
    sort_by: Option<String>,
    sort_direction: Option<String>,
) -> TokenResult<Vec<Token>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || {
        db.get_tokens_no_market(limit, offset, sort_by.as_deref(), sort_direction.as_deref())
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: update token priority
pub async fn update_token_priority_async(mint: &str, priority: i32) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.update_priority(&mint_owned, priority))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Check if a token's market data exceeds the staleness threshold
#[allow(dead_code)]
pub async fn is_market_data_stale_async(mint: &str, threshold_seconds: i64) -> TokenResult<bool> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.is_market_data_stale(&mint_owned, threshold_seconds))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: count tokens with permanent market data failure
pub async fn count_permanent_market_failures_async() -> TokenResult<u64> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.count_permanent_market_failures())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: update token rejection status
pub async fn update_rejection_status_async(
    mint: &str,
    reason: &str,
    source: &str,
    rejected_at: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let mint_owned = mint.to_string();
    let reason_owned = reason.to_string();
    let source_owned = source.to_string();
    tokio::task::spawn_blocking(move || {
        db.update_rejection_status(&mint_owned, &reason_owned, &source_owned, rejected_at)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: clear token rejection status (when token passes)
pub async fn clear_rejection_status_async(mint: &str) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let mint_owned = mint.to_string();
    tokio::task::spawn_blocking(move || db.clear_rejection_status(&mint_owned))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: batch clear rejection status for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
pub async fn batch_clear_rejection_status_async(mints: Vec<String>) -> TokenResult<usize> {
    if mints.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.batch_clear_rejection_status(&mints))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: batch update priority for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
pub async fn batch_update_priority_async(mints: Vec<String>, priority: i32) -> TokenResult<usize> {
    if mints.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.batch_update_priority(&mints, priority))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: batch update rejection status for multiple tokens (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
/// updates: Vec of (mint, reason, source, rejected_at)
pub async fn batch_update_rejection_status_async(
    updates: Vec<(String, String, String, i64)>,
) -> TokenResult<usize> {
    if updates.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.batch_update_rejection_status(&updates))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: batch upsert rejection stats (PERF optimization)
/// Reduces 130k+ tokio::spawn calls to a single blocking task with transaction
/// stats: Vec of (reason, source, timestamp)
pub async fn batch_upsert_rejection_stats_async(
    stats: Vec<(String, String, i64)>,
) -> TokenResult<usize> {
    if stats.is_empty() {
        return Ok(0);
    }
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.batch_upsert_rejection_stats(&stats))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get rejection statistics grouped by reason
pub async fn get_rejection_stats_async() -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats())
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get rejection statistics with optional time filter
/// Counts UNIQUE tokens rejected in the time range (not cumulative events)
pub async fn get_rejection_stats_with_time_filter_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || {
        db.get_rejection_stats_with_time_filter(start_time, end_time)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get rejected tokens list
pub async fn get_recent_rejections_async(
    limit: usize,
) -> TokenResult<Vec<(String, String, String, i64, Option<String>, Option<String>, Option<String>)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || db.get_recent_rejections(limit))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Fetch rejected tokens with optional filters asynchronously
pub async fn get_rejected_tokens_async(
    reason_filter: Option<String>,
    source_filter: Option<String>,
    search_filter: Option<String>,
    limit: usize,
    offset: usize,
) -> TokenResult<Vec<(String, String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;

    tokio::task::spawn_blocking(move || {
        db.get_rejected_tokens(reason_filter, source_filter, search_filter, limit, offset)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: insert rejection history event (for time-range analytics)
pub async fn insert_rejection_history_async(
    mint: &str,
    reason: &str,
    source: &str,
    rejected_at: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let mint_owned = mint.to_string();
    let reason_owned = reason.to_string();
    let source_owned = source.to_string();
    tokio::task::spawn_blocking(move || {
        db.insert_rejection_history(&mint_owned, &reason_owned, &source_owned, rejected_at)
    })
    .await
    .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get rejection statistics for a specific time range
pub async fn get_rejection_stats_for_range_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats_for_range(start_time, end_time))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: cleanup old rejection history entries (keep last N hours)
pub async fn cleanup_rejection_history_async(hours_to_keep: i64) -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.cleanup_rejection_history(hours_to_keep))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: upsert rejection stat into aggregated hourly bucket
pub async fn upsert_rejection_stat_async(
    reason: &str,
    source: &str,
    timestamp: i64,
) -> TokenResult<()> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    let reason = reason.to_string();
    let source = source.to_string();
    tokio::task::spawn_blocking(move || db.upsert_rejection_stat(&reason, &source, timestamp))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: get rejection statistics from aggregated table
pub async fn get_rejection_stats_aggregated_async(
    start_time: Option<i64>,
    end_time: Option<i64>,
) -> TokenResult<Vec<(String, String, i64)>> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.get_rejection_stats_aggregated(start_time, end_time))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}

/// Async: cleanup old aggregated rejection stats
pub async fn cleanup_rejection_stats_async(hours_to_keep: i64) -> TokenResult<usize> {
    let db = get_global_database()
        .ok_or_else(|| TokenError::Database("Global database not initialized".to_owned()))?;
    tokio::task::spawn_blocking(move || db.cleanup_rejection_stats(hours_to_keep))
        .await
        .map_err(|e| TokenError::Database(format!("Join error: {e}")))?
}
