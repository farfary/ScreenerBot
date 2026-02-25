//! OHLCV route handlers — endpoint implementations for candlestick data.

use super::types::*;
use crate::ohlcvs::{
    add_token_monitoring, delete_inactive_tokens, delete_token_data, get_all_tokens_with_status,
    get_available_pools, get_data_gaps, get_database_stats, get_metrics, get_ohlcv_data,
    record_activity, remove_token_monitoring, request_refresh, ActivityType, DatabaseStats,
    Priority, Timeframe,
};
use crate::webserver::utils::{error_response, success_response};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
    response::Response,
};

pub(super) async fn get_ohlcv_data_handler(
    Path(mint): Path<String>,
    Query(params): Query<OhlcvQuery>,
) -> Result<Response, Response> {
    // Parse timeframe
    let timeframe = params
        .timeframe
        .as_deref()
        .and_then(Timeframe::from_str)
        .unwrap_or(Timeframe::Minute1);

    let limit = params.limit.unwrap_or(100).min(1000); // Cap at 1000

    // Fetch data
    match get_ohlcv_data(
        &mint,
        timeframe,
        params.pool.as_deref(),
        limit,
        params.from,
        params.to,
    )
    .await
    {
        Ok(data) => {
            let response = OhlcvDataResponse {
                mint: mint.clone(),
                pool_address: params.pool,
                timeframe: timeframe.as_str().to_owned(),
                count: data.len(),
                data,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_fetch_failed",
            &format!("Failed to fetch OHLCV data: {e}"),
            None,
        )),
    }
}

pub(super) async fn get_pools_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    match get_available_pools(&mint).await {
        Ok(pools) => {
            let default_pool = pools
                .iter()
                .find(|p| p.is_default)
                .map(|p| p.address.clone());

            let response = PoolsResponse {
                mint,
                pools,
                default_pool,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_pools_failed",
            &format!("Failed to fetch pools: {e}"),
            None,
        )),
    }
}

pub(super) async fn get_gaps_handler(
    Path(mint): Path<String>,
    Query(params): Query<GapsQuery>,
) -> Result<Response, Response> {
    let timeframe = params
        .timeframe
        .as_deref()
        .and_then(Timeframe::from_str)
        .unwrap_or(Timeframe::Minute1);

    match get_data_gaps(&mint, timeframe).await {
        Ok(gap_tuples) => {
            let gaps: Vec<GapInfo> = gap_tuples
                .iter()
                .map(|(start, end)| GapInfo {
                    start_timestamp: *start,
                    end_timestamp: *end,
                    duration_seconds: end - start,
                })
                .collect();

            let response = GapsResponse {
                mint,
                timeframe: timeframe.as_str().to_owned(),
                total_gaps: gaps.len(),
                gaps,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_gaps_failed",
            &format!("Failed to fetch gaps: {e}"),
            None,
        )),
    }
}

pub(super) async fn get_status_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    // Check if we have data for this token
    let has_data = get_ohlcv_data(&mint, Timeframe::Minute1, None, 1, None, None)
        .await
        .map(|d| !d.is_empty())
        .unwrap_or_default();

    // Check which timeframes have data
    let mut timeframes_available = Vec::new();
    for tf in Timeframe::all() {
        if let Ok(data) = get_ohlcv_data(&mint, tf, None, 1, None, None).await {
            if !data.is_empty() {
                timeframes_available.push(tf.as_str().to_owned());
            }
        }
    }

    // Get latest timestamp
    let latest_timestamp = get_ohlcv_data(&mint, Timeframe::Minute1, None, 1, None, None)
        .await
        .ok()
        .and_then(|d| d.first().map(|p| p.timestamp));

    // Simple quality assessment
    let data_quality = if has_data {
        if timeframes_available.len() >= 5 {
            "excellent"
        } else if timeframes_available.len() >= 3 {
            "good"
        } else {
            "partial"
        }
    } else {
        "no_data"
    };

    let response = DataStatusResponse {
        mint,
        has_data,
        timeframes_available,
        latest_timestamp,
        data_quality: data_quality.to_string(),
    };

    Ok(success_response(response))
}

pub(super) async fn refresh_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    match request_refresh(&mint).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "message": "Refresh requested",
            "mint": mint
        }))),
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_refresh_failed",
            &format!("Failed to refresh: {e}"),
            None,
        )),
    }
}

pub(super) async fn get_metrics_handler() -> Result<Response, Response> {
    let metrics = get_metrics().await;

    let response = MetricsResponse {
        tokens_monitored: metrics.tokens_monitored,
        pools_tracked: metrics.pools_tracked,
        api_calls_per_minute: metrics.api_calls_per_minute,
        cache_hit_rate_percent: metrics.cache_hit_rate * 100.0,
        average_fetch_latency_ms: metrics.average_fetch_latency_ms,
        gaps_detected: metrics.gaps_detected,
        gaps_filled: metrics.gaps_filled,
        data_points_stored: metrics.data_points_stored,
        database_size_mb: metrics.database_size_mb,
    };

    Ok(success_response(response))
}

pub(super) async fn add_monitoring_handler(
    Path(mint): Path<String>,
    Json(body): Json<MonitorRequest>,
) -> Result<Response, Response> {
    let priority = body
        .priority
        .as_deref()
        .and_then(Priority::from_str)
        .unwrap_or(Priority::Medium);

    match add_token_monitoring(&mint, priority).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "message": "Monitoring started",
            "mint": mint,
            "priority": priority.as_str()
        }))),
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_monitor_start_failed",
            &format!("Failed to start monitoring: {e}"),
            None,
        )),
    }
}

pub(super) async fn remove_monitoring_handler(
    Path(mint): Path<String>,
) -> Result<Response, Response> {
    match remove_token_monitoring(&mint).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "message": "Monitoring stopped",
            "mint": mint
        }))),
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_monitor_stop_failed",
            &format!("Failed to stop monitoring: {e}"),
            None,
        )),
    }
}

pub(super) async fn record_view_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    match record_activity(&mint, ActivityType::ChartViewed).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "message": "Activity recorded",
            "mint": mint
        }))),
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_activity_failed",
            &format!("Failed to record activity: {e}"),
            None,
        )),
    }
}

/// List all OHLCV tokens with their monitoring status
pub(super) async fn get_all_tokens_handler() -> Result<Response, Response> {
    match get_all_tokens_with_status().await {
        Ok(tokens) => {
            // Get database stats for the response
            let stats = get_database_stats().await.unwrap_or(DatabaseStats {
                total_candles: 0,
                total_gaps: 0,
                total_pools: 0,
                total_configs: 0,
                active_configs: 0,
                database_size_bytes: 0,
            });

            // Convert to response format
            let token_items: Vec<OhlcvTokenItem> = tokens
                .iter()
                .map(|t| {
                    // Build backfill status from individual fields
                    let timeframes = BackfillTimeframes {
                        m1: t.backfill_1m,
                        m5: t.backfill_5m,
                        m15: t.backfill_15m,
                        h1: t.backfill_1h,
                        h4: t.backfill_4h,
                        h12: t.backfill_12h,
                        d1: t.backfill_1d,
                    };

                    let completed = [
                        t.backfill_1m,
                        t.backfill_5m,
                        t.backfill_15m,
                        t.backfill_1h,
                        t.backfill_4h,
                        t.backfill_12h,
                        t.backfill_1d,
                    ]
                    .iter()
                    .filter(|&&v| v)
                    .count() as u8;

                    let total = 7u8;
                    let percent = (completed as f64 / total as f64) * 100.0;

                    let backfill = BackfillProgress {
                        completed,
                        total,
                        percent,
                        timeframes,
                    };

                    // Calculate data span in hours
                    let data_span_hours = if t.latest_timestamp > 0 && t.earliest_timestamp > 0 {
                        (t.latest_timestamp - t.earliest_timestamp) as f64 / 3600.0
                    } else {
                        0.0
                    };

                    OhlcvTokenItem {
                        mint: t.mint.clone(),
                        priority: t.priority.clone(),
                        status: if t.is_active { "active" } else { "inactive" }.to_string(),
                        is_active: t.is_active,
                        fetch_interval_seconds: t.fetch_interval_seconds,
                        last_fetch: t.last_fetch.clone(),
                        last_activity: t.last_activity.clone(),
                        consecutive_empty_fetches: t.consecutive_empty_fetches,
                        consecutive_pool_failures: t.consecutive_pool_failures,
                        backfill_progress: backfill,
                        candle_count: t.candle_count,
                        earliest_timestamp: t.earliest_timestamp,
                        latest_timestamp: t.latest_timestamp,
                        data_span_hours,
                        open_gaps: t.open_gaps,
                        pool_count: t.pool_count,
                        created_at: t.created_at.clone(),
                        updated_at: t.updated_at.clone(),
                    }
                })
                .collect();

            let total_count = token_items.len();
            let active_count = token_items.iter().filter(|t| t.is_active).count();

            let response = OhlcvTokenListResponse {
                tokens: token_items,
                total_count,
                stats: OhlcvStatsResponse {
                    total_tokens: total_count,
                    active_tokens: active_count,
                    total_candles: stats.total_candles,
                    total_gaps: stats.total_gaps,
                    total_pools: stats.total_pools,
                    database_size_mb: stats.database_size_bytes as f64 / 1_048_576.0,
                },
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_list_failed",
            &format!("Failed to list OHLCV tokens: {e}"),
            None,
        )),
    }
}

/// Get OHLCV database statistics
pub(super) async fn get_stats_handler() -> Result<Response, Response> {
    let stats = get_database_stats().await.unwrap_or(DatabaseStats {
        total_candles: 0,
        total_gaps: 0,
        total_pools: 0,
        total_configs: 0,
        active_configs: 0,
        database_size_bytes: 0,
    });

    let active_tokens = match get_all_tokens_with_status().await {
        Ok(tokens) => tokens.iter().filter(|t| t.is_active).count(),
        Err(_) => 0,
    };

    let response = OhlcvStatsResponse {
        total_tokens: stats.total_configs,
        active_tokens,
        total_candles: stats.total_candles,
        total_gaps: stats.total_gaps,
        total_pools: stats.total_pools,
        database_size_mb: stats.database_size_bytes as f64 / 1_048_576.0,
    };

    Ok(success_response(response))
}

/// Delete all OHLCV data for a specific token
pub(super) async fn delete_token_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    match delete_token_data(&mint).await {
        Ok(result) => {
            let response = DeleteTokenResponse {
                mint,
                candles_deleted: result.candles_deleted,
                gaps_deleted: result.gaps_deleted,
                pools_deleted: result.pools_deleted,
                config_deleted: result.config_deleted,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_delete_failed",
            &format!("Failed to delete token data: {e}"),
            None,
        )),
    }
}

/// Clean up data for inactive tokens
pub(super) async fn cleanup_inactive_handler(
    Json(body): Json<CleanupRequest>,
) -> Result<Response, Response> {
    let inactive_hours = body.inactive_hours.unwrap_or(24); // Default: 24 hours

    match delete_inactive_tokens(inactive_hours).await {
        Ok(deleted_mints) => {
            let response = CleanupResponse {
                deleted_count: deleted_mints.len(),
                deleted_mints,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ohlcv_cleanup_failed",
            &format!("Failed to cleanup inactive tokens: {e}"),
            None,
        )),
    }
}
