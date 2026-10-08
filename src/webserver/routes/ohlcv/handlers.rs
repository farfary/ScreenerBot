// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! OHLCV route handlers — endpoint implementations for candlestick data.

use super::types::*;
use crate::chains::{ChainId, ChainScope};
use crate::i18n::{ids, MessageId};
use crate::logger::{self, LogTag};
use crate::ohlcvs::{
    add_token_monitoring, clear_all_ohlcv_data, delete_inactive_tokens, delete_token_data,
    get_all_tokens_with_status, get_available_pools, get_data_gaps, get_database_stats,
    get_metrics, get_ohlcv_data, record_activity, remove_token_monitoring, request_refresh,
    ActivityType, Priority, Timeframe,
};
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::utils::success_response;
use axum::{
    extract::{Path, Query},
    response::{IntoResponse as _, Json, Response},
};

/// The chain a mint route's address belongs to. An address no single enabled chain
/// accepts is refused as invalid input under the route's own failure message.
fn chain_of_mint(mint: &str, failure: MessageId) -> Result<ChainId, Response> {
    crate::chains::chain_for_address(mint).map_err(|e| {
        logger::warning(
            LogTag::Webserver,
            &format!("OHLCV route refused mint={mint}: {e}"),
        );
        ApiError::new(ApiErrorCode::InvalidInput, failure)
            .details(e.to_string())
            .into_response()
    })
}

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
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_FETCH_FAILED)?;

    // Fetch data
    match get_ohlcv_data(
        chain,
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
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_FETCH_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

pub(super) async fn get_pools_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_POOLS_FAILED)?;
    match get_available_pools(chain, &mint).await {
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
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_POOLS_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
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

    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_GAPS_FAILED)?;
    match get_data_gaps(chain, &mint, timeframe).await {
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
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_GAPS_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

pub(super) async fn get_status_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_FETCH_FAILED)?;
    // Check if we have data for this token
    let has_data = get_ohlcv_data(chain, &mint, Timeframe::Minute1, None, 1, None, None)
        .await
        .map(|d| !d.is_empty())
        .unwrap_or_default();

    // Check which timeframes have data
    let mut timeframes_available = Vec::new();
    for tf in Timeframe::all() {
        if let Ok(data) = get_ohlcv_data(chain, &mint, tf, None, 1, None, None).await {
            if !data.is_empty() {
                timeframes_available.push(tf.as_str().to_owned());
            }
        }
    }

    // Get latest timestamp
    let latest_timestamp = get_ohlcv_data(chain, &mint, Timeframe::Minute1, None, 1, None, None)
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
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_REFRESH_FAILED)?;
    match request_refresh(chain, &mint).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "mint": mint
        }))),
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_REFRESH_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

pub(super) async fn get_metrics_handler() -> Result<Response, Response> {
    let metrics = get_metrics(ChainScope::All).await;

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

    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_MONITOR_START_FAILED)?;
    match add_token_monitoring(chain, &mint, priority).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "mint": mint,
            "priority": priority.as_str()
        }))),
        Err(e) => Err(ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_OHLCV_MONITOR_START_FAILED,
        )
        .details(e.to_string())
        .into_response()),
    }
}

pub(super) async fn remove_monitoring_handler(
    Path(mint): Path<String>,
) -> Result<Response, Response> {
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_MONITOR_STOP_FAILED)?;
    match remove_token_monitoring(chain, &mint).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "mint": mint
        }))),
        Err(e) => Err(ApiError::new(
            ApiErrorCode::Internal,
            ids::ERRORS_OHLCV_MONITOR_STOP_FAILED,
        )
        .details(e.to_string())
        .into_response()),
    }
}

pub(super) async fn record_view_handler(Path(mint): Path<String>) -> Result<Response, Response> {
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_ACTIVITY_FAILED)?;
    match record_activity(chain, &mint, ActivityType::ChartViewed).await {
        Ok(_) => Ok(success_response(serde_json::json!({
            "mint": mint
        }))),
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_ACTIVITY_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

/// List all OHLCV tokens with their monitoring status
pub(super) async fn get_all_tokens_handler() -> Result<Response, Response> {
    match get_all_tokens_with_status(ChainScope::All).await {
        Ok(tokens) => {
            // Get database stats for the response
            let stats = get_database_stats(ChainScope::All)
                .await
                .unwrap_or_default();

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
                        chain: t.chain,
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
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_LIST_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

/// Get OHLCV database statistics
pub(super) async fn get_stats_handler() -> Result<Response, Response> {
    let stats = get_database_stats(ChainScope::All)
        .await
        .unwrap_or_default();

    let active_tokens = match get_all_tokens_with_status(ChainScope::All).await {
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
    let chain = chain_of_mint(&mint, ids::ERRORS_OHLCV_DELETE_FAILED)?;
    match delete_token_data(chain, &mint).await {
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
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_DELETE_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

/// Clear ALL cached OHLCV candle data (manual "Clear OHLCV Cache" action).
/// Wipes candles + gaps and resets backfill progress so monitored tokens
/// re-fetch from scratch; pools and the monitoring list are preserved.
pub(super) async fn clear_all_handler() -> Result<Response, Response> {
    match clear_all_ohlcv_data(ChainScope::All).await {
        Ok(result) => Ok(success_response(ClearAllResponse {
            candles_deleted: result.candles_deleted,
            gaps_deleted: result.gaps_deleted,
            tokens_reset: result.tokens_reset,
        })),
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_CLEAR_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}

/// Clean up data for inactive tokens
pub(super) async fn cleanup_inactive_handler(
    Json(body): Json<CleanupRequest>,
) -> Result<Response, Response> {
    let inactive_hours = body.inactive_hours.unwrap_or(24); // Default: 24 hours

    match delete_inactive_tokens(ChainScope::All, inactive_hours).await {
        Ok(deleted) => {
            let response = CleanupResponse {
                deleted_count: deleted.len(),
                deleted,
            };

            Ok(success_response(response))
        }
        Err(e) => Err(
            ApiError::new(ApiErrorCode::Internal, ids::ERRORS_OHLCV_CLEANUP_FAILED)
                .details(e.to_string())
                .into_response(),
        ),
    }
}
