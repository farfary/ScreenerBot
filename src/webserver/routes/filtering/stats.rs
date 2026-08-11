//! Filtering stats route — computes and returns token filtering pass/fail rates.

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
use super::types::{
    FilteringStatsResponse, RefreshResponse, RejectionStatEntry, RejectionStatsResponse,
};

/// GET /api/filtering/stats
/// Retrieve current filtering statistics including token counts and metrics
///
/// Reads the snapshot only if one already exists. The blocking variant waits up to 30
/// seconds for the FIRST snapshot to be built, which is what made the filtering tab the
/// slowest surface in the app on a fresh launch — opening it before the initial build
/// finished held the request for the whole timeout and the tab appeared frozen. Zeroed
/// counts for one poll are the right trade; the next poll has the real ones.
pub async fn get_stats() -> Response {
    let stats = filtering::try_fetch_stats().await;
    let updated_at = stats
        .as_ref()
        .map(|s| s.updated_at.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    success_response(FilteringStatsResponse {
        total_tokens: stats.as_ref().map(|s| s.total_tokens).unwrap_or_default(),
        with_pool_price: stats
            .as_ref()
            .map(|s| s.with_pool_price)
            .unwrap_or_default(),
        open_positions: stats.as_ref().map(|s| s.open_positions).unwrap_or_default(),
        blacklisted: stats.as_ref().map(|s| s.blacklisted).unwrap_or_default(),
        with_ohlcv: stats.as_ref().map(|s| s.with_ohlcv).unwrap_or_default(),
        passed_filtering: stats
            .as_ref()
            .map(|s| s.passed_filtering)
            .unwrap_or_default(),
        updated_at,
        timestamp: Utc::now().to_rfc3339(),
    })
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
                message: "Filtering snapshot rebuilt".to_owned(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        Err(err) => {
            logger::info(
                LogTag::Filtering,
                &format!("Filtering refresh failed: {err}"),
            );

            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FILTERING_REFRESH_FAILED",
                &format!("Failed to rebuild filtering snapshot: {err}"),
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
                    *by_source.entry(source.clone()).or_default() += count;
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
