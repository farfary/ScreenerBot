use axum::{
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::Serialize;
use std::sync::Arc;

use crate::{
    filtering,
    logger::{log, LogTag},
    webserver::state::AppState,
    webserver::utils::{error_response, success_response},
};

/// Filtering management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/filtering/refresh", post(trigger_refresh))
        .route("/filtering/stats", get(get_stats))
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct FilteringStatsResponse {
    total_tokens: usize,
    with_pool_price: usize,
    open_positions: usize,
    blacklisted: usize,
    with_ohlcv: usize,
    passed_filtering: usize,
    updated_at: String,
    timestamp: String,
}

/// GET /api/filtering/stats
/// Retrieve current filtering statistics including token counts and metrics
async fn get_stats() -> Response {
    match filtering::fetch_stats().await {
        Ok(stats) => success_response(FilteringStatsResponse {
            total_tokens: stats.total_tokens,
            with_pool_price: stats.with_pool_price,
            open_positions: stats.open_positions,
            blacklisted: stats.blacklisted,
            with_ohlcv: stats.with_ohlcv,
            passed_filtering: stats.passed_filtering,
            updated_at: stats.updated_at.to_rfc3339(),
            timestamp: Utc::now().to_rfc3339(),
        }),
        Err(err) => {
            log(
                LogTag::Filtering,
                "STATS_FETCH_FAILED",
                &format!("Failed to fetch filtering stats: {}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STATS_FETCH_FAILED",
                &format!("Failed to fetch filtering statistics: {}", err),
                None,
            )
        }
    }
}

/// POST /api/filtering/refresh
/// Force a synchronous rebuild of the filtering snapshot so downstream
/// consumers see the newly-saved configuration immediately.
async fn trigger_refresh() -> Response {
    match filtering::refresh().await {
        Ok(()) => {
            log(
                LogTag::Filtering,
                "REFRESH_TRIGGERED",
                "Filtering snapshot rebuilt via API request",
            );

            success_response(RefreshResponse {
                message: "Filtering snapshot rebuilt".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            log(
                LogTag::Filtering,
                "REFRESH_FAILED",
                &format!("Filtering refresh failed: {}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FILTERING_REFRESH_FAILED",
                &format!("Failed to rebuild filtering snapshot: {}", err),
                None,
            )
        }
    }
}
