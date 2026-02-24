//! Updates orchestrator - State-based priority updates
//!
//! Coordinates fetching from all sources (DexScreener, GeckoTerminal, Rugcheck)
//! with rate limiting and state-based priority scheduling.
//!
//! Priority levels (named by token state):
//! - OpenPosition (100): Tokens with active trading positions → Update every 5s
//! - PoolTracked (75): Tokens tracked by Pool Service → Update every 7s
//! - FilterPassed (60): Tokens that passed filtering criteria → Update every 8s
//! - Uninitialized (55): New tokens without market data yet → Update every 10s (immediate seeding)
//! - Stale (40): Tokens with outdated market data → Update every 15s
//! - Standard (25): Regular tokens with fresh data → Update every 20s
//! - Background (10): Oldest tokens being refreshed in background → Update every 30s
//!
//! Security data (Rugcheck) is fetched in a separate loop, one token per interval (configurable, default 60s),
//! and only for tokens that don't have security data yet.

use super::core::{
    update_security_data, update_token, update_tokens_batch, PoolPriorityManager, UpdateResult,
};
use super::helpers::{filter_dashboard_active_token, handle_market_failure, should_skip_for_tools};
use super::rate_limiter::RateLimitCoordinator;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::pools;
use crate::tokens::database::TokenDatabase;
use crate::tokens::priorities::Priority;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Start all update loops (main entry point)
///
/// Spawns multiple tokio tasks for different priority levels and security data.
/// Each loop runs on its own schedule based on the priority level.
///
/// # Returns
/// Vec<JoinHandle<()>> - Handles for all spawned loops
pub fn start_update_loop(
    db: Arc<TokenDatabase>,
    shutdown: Arc<Notify>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Vec<JoinHandle<()>> {
    let mut handles = Vec::new();

    // Security data loop (one-time fetch for tokens without Rugcheck data)
    let db_security = db.clone();
    let coord_security = coordinator.clone();
    let shutdown_security = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (0s delay)
        update_security_data(&db_security, &coord_security).await;
        loop {
            tokio::select! {
                _ = shutdown_security.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.security_seconds))) => {
                    update_security_data(&db_security, &coord_security).await;
                }
            }
        }
    }));

    // Immediate seeding loop for tokens that have no market data yet
    let db_seed = db.clone();
    let coord_seed = coordinator.clone();
    let shutdown_seed = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (2s delay)
        sleep(Duration::from_secs(2)).await;
        update_uninitialized_tokens(&db_seed, &coord_seed).await;
        loop {
            tokio::select! {
                _ = shutdown_seed.notified() => break,
                _ = sleep(Duration::from_secs(10)) => {
                    update_uninitialized_tokens(&db_seed, &coord_seed).await;
                }
            }
        }
    }));

    // Pool priority sync loop (every 5s)
    let pool_priority_manager = Arc::new(PoolPriorityManager::new(Duration::from_secs(60)));
    let manager_sync = pool_priority_manager.clone();
    let db_pool_state = db.clone();
    let shutdown_pool_sync = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (4s delay)
        sleep(Duration::from_secs(4)).await;
        manager_sync.sync(db_pool_state.as_ref()).await;
        loop {
            tokio::select! {
                _ = shutdown_pool_sync.notified() => break,
                _ = sleep(Duration::from_secs(5)) => {
                    manager_sync.sync(db_pool_state.as_ref()).await;
                }
            }
        }
    }));

    // Pool-tracked tokens loop (configurable)
    let db_pool_update = db.clone();
    let coord_pool = coordinator.clone();
    let shutdown_pool_update = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (6s delay)
        sleep(Duration::from_secs(6)).await;
        update_pool_tracked_tokens(&db_pool_update, &coord_pool).await;
        loop {
            tokio::select! {
                _ = shutdown_pool_update.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.pool_tracked_seconds))) => {
                    update_pool_tracked_tokens(&db_pool_update, &coord_pool).await;
                }
            }
        }
    }));

    // Open position tokens loop (configurable)
    let db_open_pos = db.clone();
    let coord_open_pos = coordinator.clone();
    let shutdown_open_pos = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (8s delay)
        sleep(Duration::from_secs(8)).await;
        loop {
            tokio::select! {
                _ = shutdown_open_pos.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.open_position_seconds))) => {
                    update_open_position_tokens(&db_open_pos, &coord_open_pos).await;
                }
            }
        }
    }));

    // Filter-passed tokens loop (configurable)
    let db_filter_passed = db.clone();
    let coord_filter_passed = coordinator.clone();
    let shutdown_filter_passed = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (10s delay)
        sleep(Duration::from_secs(10)).await;
        loop {
            tokio::select! {
                _ = shutdown_filter_passed.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.filter_passed_seconds))) => {
                    update_filter_passed_tokens(&db_filter_passed, &coord_filter_passed).await;
                }
            }
        }
    }));

    // Background tokens loop (configurable)
    let db_background = db.clone();
    let coord_background = coordinator.clone();
    let shutdown_background = shutdown.clone();
    handles.push(tokio::spawn(async move {
        // Stagger loop start to avoid thundering herd (12s delay)
        sleep(Duration::from_secs(12)).await;
        loop {
            tokio::select! {
                _ = shutdown_background.notified() => break,
                _ = sleep(Duration::from_secs(with_config(|cfg| cfg.tokens.update_intervals.background_seconds))) => {
                    update_background_tokens(&db_background, &coord_background).await;
                }
            }
        }
    }));

    handles
}

/// Seed market data for tokens that have never been updated
async fn update_uninitialized_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (uninitialized) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    const MAX_INITIAL_BATCH: usize = 30;

    let tokens = match db.get_tokens_without_market_data(MAX_INITIAL_BATCH) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to load uninitialized tokens: {e}"),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!(
            "Seeding market data for {} newly discovered tokens",
            tokens.len()
        ),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = tokens
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        handle_market_failure(db, &result.mint, &result.failures);
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure while seeding {}: {:?}",
                                result.mint, result.failures
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error during seeding: {e}"),
                );
            }
        }
    }
}

/// Update open position tokens (tokens with active trading positions)
async fn update_open_position_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (open positions) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::OpenPosition.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get open position tokens: {e}"),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} open position tokens", tokens.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = tokens
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        logger::error(
                            LogTag::Tokens,
                            &format!("Total failure for {}: {:?}", result.mint, result.failures),
                        );
                        handle_market_failure(db, &result.mint, &result.failures);
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for open position tokens: {e}"),
                );
            }
        }
    }
}

/// Update pool-tracked tokens (Pool Service tracked tokens)
async fn update_pool_tracked_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (pool tracked) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::PoolTracked.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get pool-tracked tokens: {e}"),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    let batch = &tokens[..tokens.len().min(90)];
    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} pool-tracked tokens", batch.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = batch
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        handle_market_failure(db, &result.mint, &result.failures);
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    } else if result.is_success() {
                        // Success: Check if token should keep PoolTracked priority
                        // Bug #23 fix: Don't demote if current priority is PoolTracked
                        // This prevents priority churn for tokens actively tracked by pool service
                        let current_priority =
                            db.get_priority(&result.mint).unwrap_or(Priority::Standard);
                        if current_priority != Priority::PoolTracked {
                            // Demote from higher priorities to Stale (40) after fresh update
                            // Token returns to normal priority rotation
                            if let Err(e) =
                                db.update_priority(&result.mint, Priority::Stale.to_value())
                            {
                                logger::warning(
                                    LogTag::Tokens,
                                    &format!(
                                        "Failed to demote {} to Stale priority: {}",
                                        result.mint, e
                                    ),
                                );
                            }
                        }
                        // If PoolTracked, keep it - pool service maintains this priority
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for pool priority tokens: {e}"),
                );
            }
        }
    }
}

/// Update filter-passed tokens (tokens that passed filtering criteria)
async fn update_filter_passed_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (filter passed) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    let tokens = match db.get_tokens_by_priority(Priority::FilterPassed.to_value(), 200) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get filter-passed tokens: {e}"),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    // Limit to 60 tokens total, process in batches of 30
    let batch = &tokens[..tokens.len().min(60)];
    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} filter-passed tokens", batch.len()),
    );

    // Process all chunks concurrently to maximize rate limit utilization
    let chunk_futures: Vec<_> = batch
        .chunks(30)
        .map(|chunk| {
            let chunk_vec = chunk.to_vec();
            let db_clone = db.clone();
            let coord_clone = coordinator.clone();
            async move { update_tokens_batch(&chunk_vec, &db_clone, &coord_clone).await }
        })
        .collect();

    let all_results = join_all(chunk_futures).await;

    for batch_result in all_results {
        match batch_result {
            Ok(results) => {
                for result in results {
                    if result.is_total_failure() {
                        handle_market_failure(db, &result.mint, &result.failures);
                    } else if result.is_partial_failure() {
                        logger::warning(
                            LogTag::Tokens,
                            &format!(
                                "Partial failure for {}: {} succeeded, {} failed",
                                result.mint,
                                result.successes.len(),
                                result.failures.len()
                            ),
                        );
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Tokens,
                    &format!("Batch error for passed priority tokens: {e}"),
                );
            }
        }
    }
}

/// Update background tokens (oldest non-blacklisted tokens)
async fn update_background_tokens(db: &TokenDatabase, coordinator: &RateLimitCoordinator) {
    if should_skip_for_tools() {
        logger::debug(
            LogTag::Tokens,
            "Token update (background) skipped - tools active (reducing RPC contention)",
        );
        return;
    }

    // Get oldest 30 non-blacklisted tokens (batch size)
    let tokens = match db.get_oldest_non_blacklisted(30) {
        Ok(tokens) => tokens,
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Failed to get background tokens: {e}"),
            );
            return;
        }
    };

    // Skip dashboard-active token (getting priority updates via UI)
    let tokens = filter_dashboard_active_token(tokens);

    if tokens.is_empty() {
        return;
    }

    logger::debug(
        LogTag::Tokens,
        &format!("Updating {} background tokens", tokens.len()),
    );

    // Process all in one batch (already limited to 30)
    match update_tokens_batch(&tokens, db, coordinator).await {
        Ok(results) => {
            for result in results {
                if result.is_total_failure() {
                    handle_market_failure(db, &result.mint, &result.failures);
                } else if result.is_partial_failure() {
                    logger::warning(
                        LogTag::Tokens,
                        &format!(
                            "Partial failure for {}: {} succeeded, {} failed",
                            result.mint,
                            result.successes.len(),
                            result.failures.len()
                        ),
                    );
                }
            }
        }
        Err(e) => {
            logger::error(
                LogTag::Tokens,
                &format!("Batch error for low priority tokens: {e}"),
            );
        }
    }
}
