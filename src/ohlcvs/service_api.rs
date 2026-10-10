// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV service — public API functions for querying candle data.
//!
//! A per-token call takes the token's chain and reaches that chain's runtime. An
//! aggregate takes a `ChainScope` and folds the per-chain values; with one chain in
//! scope it returns that chain's value unchanged.

use super::database::{ClearAllResult, DatabaseStats, DeleteResult, OhlcvTokenStatus};
use super::monitor::GAP_SUMMARY_LIMIT;
use super::priorities::ActivityType;
use super::service::{built_service, get_or_init_service, OhlcvServiceImpl};
use super::types::{
    Candle, ChartTail, MonitorStats, MonitorTelemetrySnapshot, OhlcvError, OhlcvMetrics,
    OhlcvResult, OhlcvStatus, PoolMetadata, Priority, Timeframe, TimeframeBundle,
};
use crate::chains::{AssetId, ChainId, ChainScope};
use std::collections::HashSet;
use std::sync::Arc;

// ==================== Public API Functions ====================

/// Fetch OHLCV candle data for a token with optional pool and time range filters.
pub async fn get_ohlcv_data(
    chain: ChainId,
    mint: &str,
    timeframe: Timeframe,
    pool_address: Option<&str>,
    limit: usize,
    from_timestamp: Option<i64>,
    to_timestamp: Option<i64>,
) -> OhlcvResult<Vec<Candle>> {
    let service = get_or_init_service(chain).await?;

    service
        .get_ohlcv_data(
            mint,
            timeframe,
            pool_address,
            limit,
            from_timestamp,
            to_timestamp,
        )
        .await
}

/// The newest `limit` candles (`0` for all) of a timeframe a chart is viewing. A timeframe
/// without native candles is read through from the Data Server first, within a bounded wait.
pub async fn get_chart_ohlcv(
    chain: ChainId,
    mint: &str,
    timeframe: Timeframe,
    limit: usize,
) -> OhlcvResult<Vec<Candle>> {
    let service = get_or_init_service(chain).await?;

    service.get_chart_ohlcv(mint, timeframe, limit).await
}

/// The stored tail of a timeframe a chart is viewing, from bucket `since` (unix secs) on,
/// read from storage only. See `ChartTail` for how a chart merges it.
pub async fn get_chart_tail(
    chain: ChainId,
    mint: &str,
    timeframe: Timeframe,
    since: i64,
) -> OhlcvResult<ChartTail> {
    let service = get_or_init_service(chain).await?;

    service.get_chart_tail(mint, timeframe, since).await
}

/// List available pools for a token that have OHLCV data.
pub async fn get_available_pools(chain: ChainId, mint: &str) -> OhlcvResult<Vec<PoolMetadata>> {
    let service = get_or_init_service(chain).await?;

    service.pool_manager.get_pool_metadata(mint).await
}

/// Get unfilled data gaps for a token's OHLCV timeline.
pub async fn get_data_gaps(
    chain: ChainId,
    mint: &str,
    timeframe: Timeframe,
) -> OhlcvResult<Vec<(i64, i64)>> {
    let service = get_or_init_service(chain).await?;

    let gaps = service
        .gap_manager
        .get_unfilled_gaps(mint, timeframe)
        .await?;

    Ok(gaps
        .into_iter()
        .map(|g| (g.start_timestamp, g.end_timestamp))
        .collect())
}

pub async fn request_refresh(chain: ChainId, mint: &str) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;

    // Priority bump only: `force_refresh` performs the single fetch this
    // request causes (and re-resolves the pools), so the activity's own
    // immediate fetch is skipped here.
    service
        .monitor
        .mark_activity(mint, ActivityType::DataRequested)
        .await?;

    service.monitor.force_refresh(mint).await
}

pub async fn add_token_monitoring(
    chain: ChainId,
    mint: &str,
    priority: Priority,
) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;

    service.monitor.add_token(mint.to_string(), priority).await
}

pub async fn remove_token_monitoring(chain: ChainId, mint: &str) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;

    service.monitor.remove_token(mint).await
}

pub async fn update_token_priority(
    chain: ChainId,
    mint: &str,
    priority: Priority,
) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;

    service.monitor.update_priority(mint, priority).await
}

pub async fn record_activity(
    chain: ChainId,
    mint: &str,
    activity_type: ActivityType,
) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;

    service.monitor.record_activity(mint, activity_type).await
}

/// System metrics over every already-built chain runtime in `scope`; never builds one.
pub async fn get_metrics(scope: ChainScope) -> OhlcvMetrics {
    let mut total: Option<ChainMetrics> = None;
    for chain in scope.chains() {
        if let Some(service) = built_service(chain) {
            let metrics = get_metrics_impl(service.as_ref()).await;
            total = Some(combine_metrics(total, metrics));
        }
    }
    total.map(|total| total.metrics).unwrap_or_default()
}

/// Monitor statistics over every already-built chain runtime in `scope`; `None` when
/// no chain in scope has a runtime yet. Never builds one.
pub async fn get_monitor_stats(scope: ChainScope) -> Option<MonitorStats> {
    let mut total: Option<ChainMonitorStats> = None;
    for chain in scope.chains() {
        if let Some(service) = built_service(chain) {
            let stats = ChainMonitorStats {
                stats: service.monitor.get_stats().await,
                lookups: service.cache.lookups(),
            };
            total = Some(combine_monitor_stats(total, stats));
        }
    }
    total.map(|total| total.stats)
}

pub async fn has_data(chain: ChainId, mint: &str) -> OhlcvResult<bool> {
    let service = get_or_init_service(chain).await?;
    let service_clone = service.clone();
    let mint_owned = mint.to_string();

    // Wrap sync DB call in spawn_blocking to prevent blocking async runtime
    tokio::task::spawn_blocking(move || service_clone.has_data(&mint_owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

pub async fn get_status(
    chain: ChainId,
    mint: &str,
    range: Option<(i64, i64)>,
) -> OhlcvResult<OhlcvStatus> {
    let service = get_or_init_service(chain).await?;
    service.get_status(mint, range).await
}

pub async fn get_mints_with_data(chain: ChainId, mints: &[String]) -> OhlcvResult<HashSet<String>> {
    if mints.is_empty() {
        return Ok(HashSet::new());
    }

    let service = get_or_init_service(chain).await?;
    let service_clone = service.clone();
    let owned = mints.to_vec();

    tokio::task::spawn_blocking(move || service_clone.get_mints_with_data(&owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

async fn get_metrics_impl(service: &OhlcvServiceImpl) -> ChainMetrics {
    let stats = service.monitor.get_stats().await;

    let tokens_monitored = stats.total_tokens;
    // Offload synchronous DB calls to blocking threads to avoid stalling async runtime
    let db = Arc::clone(&service.db);
    let (pools_tracked, data_points_stored, gaps_detected, gaps_filled) =
        tokio::task::spawn_blocking(move || {
            let pools = db.get_pool_count().unwrap_or_default();
            let points = db.get_data_point_count().unwrap_or_default();
            let gaps_det = db.get_gap_count(false).unwrap_or_default();
            let gaps_fill = db.get_gap_count(true).unwrap_or_default();
            (pools, points, gaps_det, gaps_fill)
        })
        .await
        .unwrap_or((0, 0, 0, 0));

    // Calculate database size (rough estimate)
    let database_size_bytes = (data_points_stored as u128).saturating_mul(64);
    let database_size_mb = (database_size_bytes as f64) / (1024.0 * 1024.0); // ~64 bytes per point

    ChainMetrics {
        metrics: OhlcvMetrics {
            tokens_monitored,
            pools_tracked,
            api_calls_per_minute: service.fetcher.calls_per_minute(),
            cache_hit_rate: service.cache.hit_rate(),
            average_fetch_latency_ms: service.fetcher.average_latency_ms(),
            gaps_detected,
            gaps_filled,
            data_points_stored,
            database_size_mb,
            oldest_data_timestamp: None, // Could query DB for this if needed
        },
        lookups: service.cache.lookups(),
        calls: service.fetcher.calls_recorded(),
    }
}

// ==================== Phase 2: Bundle Cache API ====================

/// Get timeframe bundle from cache for strategy evaluation (non-blocking)
/// Returns None if bundle is stale or missing - background worker will prepare it
pub async fn get_timeframe_bundle(
    chain: ChainId,
    mint: &str,
) -> OhlcvResult<Option<TimeframeBundle>> {
    let service = get_or_init_service(chain).await?;
    service.get_timeframe_bundle(mint).await
}

/// Build complete timeframe bundle (used by background worker and on-demand)
pub async fn build_timeframe_bundle(chain: ChainId, mint: &str) -> OhlcvResult<TimeframeBundle> {
    let service = get_or_init_service(chain).await?;
    service.build_timeframe_bundle(mint).await
}

/// Store bundle in cache with LRU eviction
/// Takes bundle by value to avoid unnecessary cloning
pub async fn store_bundle(
    chain: ChainId,
    mint: String,
    bundle: TimeframeBundle,
) -> OhlcvResult<()> {
    let service = get_or_init_service(chain).await?;
    service.store_bundle(mint, bundle).await
}

// ==================== OHLCV Listing and Management API ====================

/// Every OHLCV token of every enabled chain in `scope`, each row naming its chain.
pub async fn get_all_tokens_with_status(scope: ChainScope) -> OhlcvResult<Vec<OhlcvTokenStatus>> {
    let mut tokens = Vec::new();
    for chain in scope.chains() {
        let service = get_or_init_service(chain).await?;
        let db = Arc::clone(&service.db);
        let rows = tokio::task::spawn_blocking(move || db.get_all_tokens_with_status())
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;
        tokens.extend(rows);
    }
    Ok(tokens)
}

/// Delete all OHLCV data for a specific token
pub async fn delete_token_data(chain: ChainId, mint: &str) -> OhlcvResult<DeleteResult> {
    let service = get_or_init_service(chain).await?;
    let db = Arc::clone(&service.db);
    let mint_owned = mint.to_string();

    // Also remove from monitoring
    let _ = service.monitor.remove_token(&mint_owned).await;

    tokio::task::spawn_blocking(move || db.delete_token_data(&mint_owned))
        .await
        .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))?
}

/// Delete OHLCV data for tokens inactive for `inactive_hours` on every enabled chain
/// in `scope`, returning each deleted token with its chain.
pub async fn delete_inactive_tokens(
    scope: ChainScope,
    inactive_hours: i64,
) -> OhlcvResult<Vec<AssetId>> {
    let mut deleted = Vec::new();
    for chain in scope.chains() {
        let service = get_or_init_service(chain).await?;
        let db = Arc::clone(&service.db);
        let mints = tokio::task::spawn_blocking(move || db.delete_inactive_tokens(inactive_hours))
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;
        for mint in mints {
            deleted.push(AssetId::new(chain, mint)?);
        }
    }
    Ok(deleted)
}

/// Clear ALL cached OHLCV candle data (candles + gaps) and reset backfill
/// progress on every enabled chain in `scope`, so every monitored token
/// re-fetches from scratch. Pools and the monitoring list are preserved.
pub async fn clear_all_ohlcv_data(scope: ChainScope) -> OhlcvResult<ClearAllResult> {
    let mut total: Option<ClearAllResult> = None;
    for chain in scope.chains() {
        let service = get_or_init_service(chain).await?;
        // Wipe the DB AND the in-memory caches together, otherwise stale candles keep
        // being served from the hot/bundle caches after the DB is cleared.
        let result = service.clear_all_data().await?;
        total = Some(combine_clear_results(total, result));
    }
    Ok(total.unwrap_or_default())
}

/// Database statistics summed over every enabled chain in `scope`.
pub async fn get_database_stats(scope: ChainScope) -> OhlcvResult<DatabaseStats> {
    let mut total: Option<DatabaseStats> = None;
    for chain in scope.chains() {
        let service = get_or_init_service(chain).await?;
        let db = Arc::clone(&service.db);
        let stats = tokio::task::spawn_blocking(move || db.get_database_stats())
            .await
            .map_err(|e| OhlcvError::DatabaseError(format!("Task join error: {e}")))??;
        total = Some(combine_database_stats(total, stats));
    }
    Ok(total.unwrap_or_default())
}

// ==================== Aggregation across chains ====================

/// One chain's metrics with the denominators of its two ratios, so a sum over
/// chains weights each ratio by the chain's own traffic.
struct ChainMetrics {
    metrics: OhlcvMetrics,
    lookups: u64,
    calls: u64,
}

/// One chain's monitor statistics with the denominator of its cache hit rate.
struct ChainMonitorStats {
    stats: MonitorStats,
    lookups: u64,
}

/// `(rate_a * weight_a + rate_b * weight_b) / (weight_a + weight_b)`; 0 with no weight.
fn weighted_rate(rate_a: f64, weight_a: u64, rate_b: f64, weight_b: u64) -> f64 {
    let weight = weight_a + weight_b;
    if weight == 0 {
        return 0.0;
    }
    (rate_a * weight_a as f64 + rate_b * weight_b as f64) / weight as f64
}

/// The earlier of two optional values, ignoring an absent one.
fn earliest<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

/// The later of two optional values, ignoring an absent one.
fn latest<T: Ord>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

/// Sums counts, weights the cache hit rate by lookups and the fetch latency by
/// recorded calls, and keeps the oldest data timestamp.
fn combine_metrics(total: Option<ChainMetrics>, next: ChainMetrics) -> ChainMetrics {
    let Some(total) = total else {
        return next;
    };
    let (a, b) = (&total.metrics, &next.metrics);
    ChainMetrics {
        metrics: OhlcvMetrics {
            tokens_monitored: a.tokens_monitored + b.tokens_monitored,
            pools_tracked: a.pools_tracked + b.pools_tracked,
            api_calls_per_minute: a.api_calls_per_minute + b.api_calls_per_minute,
            cache_hit_rate: weighted_rate(
                a.cache_hit_rate,
                total.lookups,
                b.cache_hit_rate,
                next.lookups,
            ),
            average_fetch_latency_ms: weighted_rate(
                a.average_fetch_latency_ms,
                total.calls,
                b.average_fetch_latency_ms,
                next.calls,
            ),
            gaps_detected: a.gaps_detected + b.gaps_detected,
            gaps_filled: a.gaps_filled + b.gaps_filled,
            data_points_stored: a.data_points_stored + b.data_points_stored,
            database_size_mb: a.database_size_mb + b.database_size_mb,
            oldest_data_timestamp: earliest(a.oldest_data_timestamp, b.oldest_data_timestamp),
        },
        lookups: total.lookups + next.lookups,
        calls: total.calls + next.calls,
    }
}

/// Sums counts, weights the cache hit rate by lookups, merges telemetry and keeps
/// the largest open gaps across chains in the monitor's own ranking.
fn combine_monitor_stats(
    total: Option<ChainMonitorStats>,
    next: ChainMonitorStats,
) -> ChainMonitorStats {
    let Some(total) = total else {
        return next;
    };
    let (a, b) = (total.stats, next.stats);
    let mut top_open_gaps = a.top_open_gaps;
    top_open_gaps.extend(b.top_open_gaps);
    // Same order as the per-chain gap summary query: largest gap, then latest end.
    top_open_gaps.sort_by(|x, y| {
        y.largest_gap_seconds
            .cmp(&x.largest_gap_seconds)
            .then(y.latest_gap_end.cmp(&x.latest_gap_end))
    });
    top_open_gaps.truncate(GAP_SUMMARY_LIMIT);
    ChainMonitorStats {
        stats: MonitorStats {
            total_tokens: a.total_tokens + b.total_tokens,
            critical_tokens: a.critical_tokens + b.critical_tokens,
            high_tokens: a.high_tokens + b.high_tokens,
            medium_tokens: a.medium_tokens + b.medium_tokens,
            low_tokens: a.low_tokens + b.low_tokens,
            cache_hit_rate: weighted_rate(
                a.cache_hit_rate,
                total.lookups,
                b.cache_hit_rate,
                next.lookups,
            ),
            api_calls_per_minute: a.api_calls_per_minute + b.api_calls_per_minute,
            queue_size: a.queue_size + b.queue_size,
            telemetry: combine_telemetry(a.telemetry, b.telemetry),
            backfills_in_progress: a.backfills_in_progress + b.backfills_in_progress,
            open_gap_tokens: a.open_gap_tokens + b.open_gap_tokens,
            open_gap_total: a.open_gap_total + b.open_gap_total,
            top_open_gaps,
        },
        lookups: total.lookups + next.lookups,
    }
}

/// Sums counters; each timestamp takes the latest value, and the duration, point
/// count and error recorded with a completion come from the chain that supplied it.
fn combine_telemetry(
    a: MonitorTelemetrySnapshot,
    b: MonitorTelemetrySnapshot,
) -> MonitorTelemetrySnapshot {
    let monitor_cycle_from_b = b.monitor_cycle_completed_at > a.monitor_cycle_completed_at;
    let gap_cycle_from_b = b.gap_cycle_completed_at > a.gap_cycle_completed_at;
    let backfill_from_b = b.last_backfill_completed_at > a.last_backfill_completed_at;
    MonitorTelemetrySnapshot {
        monitor_cycle_started_at: latest(a.monitor_cycle_started_at, b.monitor_cycle_started_at),
        monitor_cycle_completed_at: latest(
            a.monitor_cycle_completed_at,
            b.monitor_cycle_completed_at,
        ),
        monitor_cycle_duration_ms: if monitor_cycle_from_b {
            b.monitor_cycle_duration_ms
        } else {
            a.monitor_cycle_duration_ms
        },
        monitor_cycle_tokens_processed: a.monitor_cycle_tokens_processed
            + b.monitor_cycle_tokens_processed,
        monitor_cycle_total: a.monitor_cycle_total + b.monitor_cycle_total,
        gap_cycle_started_at: latest(a.gap_cycle_started_at, b.gap_cycle_started_at),
        gap_cycle_completed_at: latest(a.gap_cycle_completed_at, b.gap_cycle_completed_at),
        gap_cycle_duration_ms: if gap_cycle_from_b {
            b.gap_cycle_duration_ms
        } else {
            a.gap_cycle_duration_ms
        },
        gap_cycle_tokens_processed: a.gap_cycle_tokens_processed + b.gap_cycle_tokens_processed,
        gap_cycle_total: a.gap_cycle_total + b.gap_cycle_total,
        last_rate_limit_at: latest(a.last_rate_limit_at, b.last_rate_limit_at),
        rate_limit_events: a.rate_limit_events + b.rate_limit_events,
        total_backfills_scheduled: a.total_backfills_scheduled + b.total_backfills_scheduled,
        total_backfills_completed: a.total_backfills_completed + b.total_backfills_completed,
        total_backfills_failed: a.total_backfills_failed + b.total_backfills_failed,
        last_backfill_started_at: latest(a.last_backfill_started_at, b.last_backfill_started_at),
        last_backfill_completed_at: latest(
            a.last_backfill_completed_at,
            b.last_backfill_completed_at,
        ),
        last_backfill_duration_ms: if backfill_from_b {
            b.last_backfill_duration_ms
        } else {
            a.last_backfill_duration_ms
        },
        last_backfill_points: if backfill_from_b {
            b.last_backfill_points
        } else {
            a.last_backfill_points
        },
        last_backfill_error: if backfill_from_b {
            b.last_backfill_error
        } else {
            a.last_backfill_error
        },
    }
}

/// Field-wise sum.
fn combine_database_stats(total: Option<DatabaseStats>, next: DatabaseStats) -> DatabaseStats {
    let Some(total) = total else {
        return next;
    };
    DatabaseStats {
        total_candles: total.total_candles + next.total_candles,
        total_gaps: total.total_gaps + next.total_gaps,
        total_pools: total.total_pools + next.total_pools,
        total_configs: total.total_configs + next.total_configs,
        active_configs: total.active_configs + next.active_configs,
        database_size_bytes: total.database_size_bytes + next.database_size_bytes,
    }
}

/// Field-wise sum.
fn combine_clear_results(total: Option<ClearAllResult>, next: ClearAllResult) -> ClearAllResult {
    let Some(total) = total else {
        return next;
    };
    ClearAllResult {
        candles_deleted: total.candles_deleted + next.candles_deleted,
        gaps_deleted: total.gaps_deleted + next.gaps_deleted,
        tokens_reset: total.tokens_reset + next.tokens_reset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ohlcvs::types::MintGapAggregate;
    use chrono::{TimeZone, Utc};

    fn at(secs: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).unwrap()
    }

    fn metrics(seed: usize, hit_rate: f64, latency: f64, oldest: Option<i64>) -> OhlcvMetrics {
        OhlcvMetrics {
            tokens_monitored: seed,
            pools_tracked: seed + 1,
            api_calls_per_minute: seed as f64 + 0.5,
            cache_hit_rate: hit_rate,
            average_fetch_latency_ms: latency,
            gaps_detected: seed + 2,
            gaps_filled: seed + 3,
            data_points_stored: seed + 4,
            database_size_mb: seed as f64 + 0.25,
            oldest_data_timestamp: oldest.map(at),
        }
    }

    fn gap(chain: ChainId, mint: &str, largest: i64, latest_end: i64) -> MintGapAggregate {
        MintGapAggregate {
            chain,
            mint: mint.to_string(),
            open_gaps: 1,
            largest_gap_seconds: Some(largest),
            latest_gap_end: Some(latest_end),
        }
    }

    fn telemetry(completed: i64, duration: u64, error: &str) -> MonitorTelemetrySnapshot {
        MonitorTelemetrySnapshot {
            monitor_cycle_started_at: Some(at(completed - 10)),
            monitor_cycle_completed_at: Some(at(completed)),
            monitor_cycle_duration_ms: Some(duration),
            monitor_cycle_tokens_processed: 3,
            monitor_cycle_total: 7,
            gap_cycle_started_at: Some(at(completed - 5)),
            gap_cycle_completed_at: Some(at(completed)),
            gap_cycle_duration_ms: Some(duration + 1),
            gap_cycle_tokens_processed: 2,
            gap_cycle_total: 4,
            last_rate_limit_at: Some(at(completed - 1)),
            rate_limit_events: 1,
            total_backfills_scheduled: 5,
            total_backfills_completed: 4,
            total_backfills_failed: 1,
            last_backfill_started_at: Some(at(completed - 20)),
            last_backfill_completed_at: Some(at(completed)),
            last_backfill_duration_ms: Some(duration + 2),
            last_backfill_points: Some(duration as usize),
            last_backfill_error: Some(error.to_string()),
        }
    }

    fn monitor_stats(seed: usize, hit_rate: f64, gaps: Vec<MintGapAggregate>) -> MonitorStats {
        MonitorStats {
            total_tokens: seed,
            critical_tokens: seed + 1,
            high_tokens: seed + 2,
            medium_tokens: seed + 3,
            low_tokens: seed + 4,
            cache_hit_rate: hit_rate,
            api_calls_per_minute: seed as f64,
            queue_size: seed + 5,
            telemetry: telemetry(1_000 + seed as i64, seed as u64, "e"),
            backfills_in_progress: seed + 6,
            open_gap_tokens: seed + 7,
            open_gap_total: seed + 8,
            top_open_gaps: gaps,
        }
    }

    #[test]
    fn a_single_chain_metrics_value_is_returned_unchanged() {
        let one = combine_metrics(
            None,
            ChainMetrics {
                metrics: metrics(3, 0.4, 120.0, Some(50)),
                lookups: 10,
                calls: 4,
            },
        );
        let m = one.metrics;
        assert_eq!(m.tokens_monitored, 3);
        assert_eq!(m.pools_tracked, 4);
        assert_eq!(m.api_calls_per_minute, 3.5);
        assert_eq!(m.cache_hit_rate, 0.4);
        assert_eq!(m.average_fetch_latency_ms, 120.0);
        assert_eq!(m.gaps_detected, 5);
        assert_eq!(m.gaps_filled, 6);
        assert_eq!(m.data_points_stored, 7);
        assert_eq!(m.database_size_mb, 3.25);
        assert_eq!(m.oldest_data_timestamp, Some(at(50)));
        assert_eq!((one.lookups, one.calls), (10, 4));
    }

    #[test]
    fn two_chain_metrics_sum_counts_and_weight_ratios_by_their_own_denominators() {
        let first = ChainMetrics {
            metrics: metrics(1, 1.0, 100.0, Some(90)),
            lookups: 30,
            calls: 1,
        };
        let second = ChainMetrics {
            metrics: metrics(2, 0.0, 400.0, None),
            lookups: 10,
            calls: 3,
        };
        let m = combine_metrics(Some(first), second).metrics;
        assert_eq!(m.tokens_monitored, 3);
        assert_eq!(m.pools_tracked, 5);
        assert_eq!(m.data_points_stored, 11);
        assert!(
            (m.cache_hit_rate - 0.75).abs() < 1e-12,
            "{}",
            m.cache_hit_rate
        );
        assert!((m.average_fetch_latency_ms - 325.0).abs() < 1e-9);
        assert_eq!(m.oldest_data_timestamp, Some(at(90)));
    }

    #[test]
    fn the_oldest_data_timestamp_is_the_minimum_present() {
        let first = ChainMetrics {
            metrics: metrics(1, 0.0, 0.0, Some(90)),
            lookups: 0,
            calls: 0,
        };
        let second = ChainMetrics {
            metrics: metrics(1, 0.0, 0.0, Some(20)),
            lookups: 0,
            calls: 0,
        };
        let m = combine_metrics(Some(first), second).metrics;
        assert_eq!(m.oldest_data_timestamp, Some(at(20)));
        assert_eq!(m.cache_hit_rate, 0.0);
    }

    #[test]
    fn a_single_chain_monitor_stats_value_is_returned_unchanged() {
        let gaps = vec![
            gap(ChainId::Solana, "a", 10, 5),
            gap(ChainId::Solana, "b", 50, 1),
        ];
        let one = combine_monitor_stats(
            None,
            ChainMonitorStats {
                stats: monitor_stats(2, 0.3, gaps),
                lookups: 9,
            },
        );
        let s = one.stats;
        assert_eq!(
            (
                s.total_tokens,
                s.critical_tokens,
                s.high_tokens,
                s.medium_tokens,
                s.low_tokens
            ),
            (2, 3, 4, 5, 6)
        );
        assert_eq!(s.cache_hit_rate, 0.3);
        assert_eq!(s.api_calls_per_minute, 2.0);
        assert_eq!(s.queue_size, 7);
        assert_eq!(s.backfills_in_progress, 8);
        assert_eq!((s.open_gap_tokens, s.open_gap_total), (9, 10));
        // A single chain keeps the order its own query produced.
        let mints: Vec<&str> = s.top_open_gaps.iter().map(|g| g.mint.as_str()).collect();
        assert_eq!(mints, ["a", "b"]);
        assert_eq!(s.telemetry.monitor_cycle_duration_ms, Some(2));
        assert_eq!(s.telemetry.last_backfill_error.as_deref(), Some("e"));
        assert_eq!(one.lookups, 9);
    }

    #[test]
    fn two_chain_monitor_stats_sum_counts_and_weight_the_hit_rate() {
        let first = ChainMonitorStats {
            stats: monitor_stats(1, 0.5, Vec::new()),
            lookups: 2,
        };
        let second = ChainMonitorStats {
            stats: monitor_stats(3, 1.0, Vec::new()),
            lookups: 6,
        };
        let s = combine_monitor_stats(Some(first), second).stats;
        assert_eq!(s.total_tokens, 4);
        assert_eq!(s.low_tokens, 12);
        assert_eq!(s.queue_size, 14);
        assert_eq!(s.open_gap_total, 20);
        assert!((s.cache_hit_rate - 0.875).abs() < 1e-12);
        assert_eq!(s.telemetry.monitor_cycle_total, 14);
        assert_eq!(s.telemetry.rate_limit_events, 2);
        assert_eq!(s.telemetry.total_backfills_scheduled, 10);
    }

    #[test]
    fn the_latest_completion_wins_together_with_its_duration_points_and_error() {
        let earlier = telemetry(1_000, 11, "earlier");
        let later = telemetry(2_000, 22, "later");
        for (a, b) in [
            (earlier.clone(), later.clone()),
            (later.clone(), earlier.clone()),
        ] {
            let t = combine_telemetry(a, b);
            assert_eq!(t.monitor_cycle_completed_at, Some(at(2_000)));
            assert_eq!(t.monitor_cycle_duration_ms, Some(22));
            assert_eq!(t.gap_cycle_duration_ms, Some(23));
            assert_eq!(t.last_backfill_completed_at, Some(at(2_000)));
            assert_eq!(t.last_backfill_duration_ms, Some(24));
            assert_eq!(t.last_backfill_points, Some(22));
            assert_eq!(t.last_backfill_error.as_deref(), Some("later"));
            assert_eq!(t.monitor_cycle_started_at, Some(at(1_990)));
            assert_eq!(t.last_rate_limit_at, Some(at(1_999)));
        }
    }

    #[test]
    fn open_gaps_are_re_ranked_and_truncated_across_chains_keeping_their_chain() {
        let first: Vec<_> = (0..GAP_SUMMARY_LIMIT as i64)
            .map(|i| gap(ChainId::Solana, &format!("s{i}"), 10 + i, 0))
            .collect();
        let second = vec![gap(ChainId::Solana, "big", 1_000, 0)];
        let s = combine_monitor_stats(
            Some(ChainMonitorStats {
                stats: monitor_stats(1, 0.0, first),
                lookups: 0,
            }),
            ChainMonitorStats {
                stats: monitor_stats(1, 0.0, second),
                lookups: 0,
            },
        )
        .stats;
        assert_eq!(s.top_open_gaps.len(), GAP_SUMMARY_LIMIT);
        assert_eq!(s.top_open_gaps[0].mint, "big");
        assert_eq!(s.top_open_gaps[0].chain, ChainId::Solana);
        let largest: Vec<_> = s
            .top_open_gaps
            .iter()
            .map(|g| g.largest_gap_seconds)
            .collect();
        let mut sorted = largest.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(largest, sorted);
    }

    #[test]
    fn database_stats_and_clear_results_sum_field_by_field() {
        let stats = |n: usize| DatabaseStats {
            total_candles: n,
            total_gaps: n + 1,
            total_pools: n + 2,
            total_configs: n + 3,
            active_configs: n + 4,
            database_size_bytes: n as u64 + 5,
        };
        let single = combine_database_stats(None, stats(1));
        assert_eq!(
            (
                single.total_candles,
                single.total_gaps,
                single.total_pools,
                single.total_configs,
                single.active_configs,
                single.database_size_bytes
            ),
            (1, 2, 3, 4, 5, 6)
        );
        let sum = combine_database_stats(Some(stats(1)), stats(10));
        assert_eq!(
            (
                sum.total_candles,
                sum.total_gaps,
                sum.total_pools,
                sum.total_configs,
                sum.active_configs,
                sum.database_size_bytes
            ),
            (11, 13, 15, 17, 19, 21)
        );

        let clear = |n: usize| ClearAllResult {
            candles_deleted: n,
            gaps_deleted: n + 1,
            tokens_reset: n + 2,
        };
        let single = combine_clear_results(None, clear(2));
        assert_eq!(
            (
                single.candles_deleted,
                single.gaps_deleted,
                single.tokens_reset
            ),
            (2, 3, 4)
        );
        let sum = combine_clear_results(Some(clear(2)), clear(5));
        assert_eq!(
            (sum.candles_deleted, sum.gaps_deleted, sum.tokens_reset),
            (7, 9, 11)
        );
    }
}
