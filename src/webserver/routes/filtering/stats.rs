use axum::{http::StatusCode, response::Response};
use chrono::Utc;
use std::collections::HashMap;

use crate::{
    filtering,
    logger::{self, LogTag},
    tokens::get_rejection_stats_async,
    webserver::utils::{error_response, success_response},
};

use super::helpers::{get_rejection_category, get_rejection_display_label};
use super::types::{FilteringStatsResponse, RefreshResponse, RejectionStatEntry, RejectionStatsResponse};

/// GET /api/filtering/stats
/// Retrieve current filtering statistics including token counts and metrics
pub async fn get_stats() -> Response {
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
            logger::info(
                LogTag::Filtering,
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
pub async fn trigger_refresh() -> Response {
    match filtering::refresh().await {
        Ok(()) => {
            logger::info(
                LogTag::Filtering,
                "Filtering snapshot rebuilt via API request",
            );

            success_response(RefreshResponse {
                message: "Filtering snapshot rebuilt".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            logger::info(
                LogTag::Filtering,
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

/// GET /api/filtering/rejection-stats
/// Get counts of rejected tokens grouped by rejection reason
pub async fn get_rejection_stats() -> Response {
    match get_rejection_stats_async().await {
        Ok(raw_stats) => {
            let mut by_source: HashMap<String, i64> = HashMap::new();
            let mut total_rejected: i64 = 0;

            let stats: Vec<RejectionStatEntry> = raw_stats
                .into_iter()
                .map(|(reason, source, count)| {
                    total_rejected += count;
                    *by_source.entry(source.clone()).or_insert(0) += count;
                    RejectionStatEntry {
                        display_label: get_rejection_display_label(&reason).to_string(),
                        category: get_rejection_category(&reason).to_string(),
                        reason,
                        source,
                        count,
                        percentage: 0.0, // Calculated on frontend for this view
                    }
                })
                .collect();

            success_response(RejectionStatsResponse {
                stats,
                by_source,
                total_rejected,
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejection stats: {:?}", err),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "REJECTION_STATS_FAILED",
                &format!("Failed to fetch rejection statistics: {:?}", err),
                None,
            )
        }
    }
}
