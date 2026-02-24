//! Background workers for filtering service — periodic refresh and cleanup.
//!
//! This module contains the core business logic for:
//! - Periodic filter snapshot refresh with new token detection
//! - Rejection history and stats cleanup to prevent unbounded database growth

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::filtering;
use crate::logger::{self, LogTag};
use crate::telegram::pagination::PAGINATION_MANAGER;
use crate::telegram::{queue_notification, Notification};
use crate::tokens::{cleanup_rejection_history_async, cleanup_rejection_stats_async};

// Timing constants
const FILTER_CACHE_TTL_SECS: u64 = 30;
// Cleanup rejection history every 10 minutes to prevent unbounded growth
const REJECTION_HISTORY_CLEANUP_INTERVAL_SECS: u64 = 600;
// Keep only 24 hours of rejection history (data grows ~5GB/day otherwise)
const REJECTION_HISTORY_HOURS_TO_KEEP: i64 = 24;

/// Get the configured refresh interval in seconds
pub fn refresh_interval_secs() -> u64 {
    FILTER_CACHE_TTL_SECS
}

/// Get the snapshot staleness threshold (4x refresh interval)
pub fn snapshot_stale_limit_secs() -> u64 {
    refresh_interval_secs() * 4
}

/// Main filtering refresh loop - periodically refreshes filter snapshots and detects new tokens.
///
/// This loop:
/// 1. Does an immediate refresh on start (async, doesn't block other services)
/// 2. Then continues with periodic refreshes every `refresh_interval_secs()`
/// 3. Detects newly passed tokens by comparing snapshots
/// 4. Sends Telegram notifications for new tokens
pub async fn run_refresh_loop(
    shutdown: Arc<Notify>,
    operations: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
) {
    // Do first refresh immediately on start (async, doesn't block other services)
    logger::info(LogTag::Filtering, "Starting initial filtering refresh...");
    match filtering::refresh().await {
        Ok(_) => {
            operations.fetch_add(1, Ordering::Relaxed);
            logger::info(LogTag::Filtering, "Initial filtering refresh complete");
        }
        Err(err) => {
            errors.fetch_add(1, Ordering::Relaxed);
            logger::warning(
                LogTag::Filtering,
                &format!("Initial refresh failed: {}", err),
            );
        }
    }

    // Then continue with periodic refresh loop
    loop {
        let interval_secs = refresh_interval_secs();
        let sleep_duration = Duration::from_secs(interval_secs);

        tokio::select! {
            _ = shutdown.notified() => break,
            _ = tokio::time::sleep(sleep_duration) => {}
        }

        // Capture state before refresh
        let prev_passed = filtering::get_passed_tokens().await.unwrap_or_default();
        let prev_mints: HashSet<String> = prev_passed.iter().map(|t| t.mint.clone()).collect();

        match filtering::refresh().await {
            Ok(_) => {
                operations.fetch_add(1, Ordering::Relaxed);

                // Check for new tokens
                if let Ok(current_passed) = filtering::get_passed_tokens().await {
                    let new_tokens: Vec<_> = current_passed
                        .into_iter()
                        .filter(|t| !prev_mints.contains(&t.mint))
                        .collect();

                    if !new_tokens.is_empty() {
                        logger::info(
                            LogTag::Filtering,
                            &format!("Found {} new passed tokens", new_tokens.len()),
                        );

                        let session_id =
                            PAGINATION_MANAGER.create_session(new_tokens.clone(), None);
                        queue_notification(Notification::new_tokens_found(
                            session_id,
                            new_tokens.len(),
                        ));
                    }
                }
            }
            Err(err) => {
                errors.fetch_add(1, Ordering::Relaxed);
                logger::warning(LogTag::Filtering, &err);
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

/// Rejection history and stats cleanup loop - prevents unbounded database growth.
///
/// This loop:
/// 1. Does an immediate cleanup on start
/// 2. Then continues with periodic cleanup every `REJECTION_HISTORY_CLEANUP_INTERVAL_SECS`
/// 3. Removes rejection_history entries older than `REJECTION_HISTORY_HOURS_TO_KEEP`
/// 4. Removes rejection_stats buckets older than `REJECTION_HISTORY_HOURS_TO_KEEP`
pub async fn run_cleanup_loop(shutdown: Arc<Notify>) {
    // Do initial cleanup immediately on start
    logger::info(
        LogTag::Filtering,
        "Running initial rejection data cleanup...",
    );

    // Cleanup old rejection_history entries
    match cleanup_rejection_history_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
        Ok(deleted) => {
            if deleted > 0 {
                logger::info(
                    LogTag::Filtering,
                    &format!(
                        "Initial cleanup: removed {} old rejection history entries",
                        deleted
                    ),
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Initial rejection history cleanup failed: {e}"),
            );
        }
    }

    // Cleanup old rejection_stats entries (aggregated hourly buckets)
    match cleanup_rejection_stats_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
        Ok(deleted) => {
            if deleted > 0 {
                logger::info(
                    LogTag::Filtering,
                    &format!(
                        "Initial cleanup: removed {} old rejection stats buckets",
                        deleted
                    ),
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Initial rejection stats cleanup failed: {e}"),
            );
        }
    }

    // Then continue with periodic cleanup loop
    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = tokio::time::sleep(Duration::from_secs(REJECTION_HISTORY_CLEANUP_INTERVAL_SECS)) => {}
        }

        // Cleanup rejection_history
        match cleanup_rejection_history_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
            Ok(deleted) => {
                if deleted > 0 {
                    logger::debug(
                        LogTag::Filtering,
                        &format!(
                            "Rejection history cleanup: removed {} entries older than {}h",
                            deleted, REJECTION_HISTORY_HOURS_TO_KEEP
                        ),
                    );
                }
            }
            Err(e) => {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Rejection history cleanup failed: {e}"),
                );
            }
        }

        // Cleanup rejection_stats (aggregated hourly buckets)
        match cleanup_rejection_stats_async(REJECTION_HISTORY_HOURS_TO_KEEP).await {
            Ok(deleted) => {
                if deleted > 0 {
                    logger::debug(
                        LogTag::Filtering,
                        &format!(
                            "Rejection stats cleanup: removed {} buckets older than {}h",
                            deleted, REJECTION_HISTORY_HOURS_TO_KEEP
                        ),
                    );
                }
            }
            Err(e) => {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Rejection stats cleanup failed: {e}"),
                );
            }
        }
    }
}
