//! Filtering stats route — computes and returns token filtering pass/fail rates.

use axum::{http::StatusCode, response::Response};
use chrono::Utc;
use std::collections::HashMap;

use crate::{
    filtering::{self, SnapshotState},
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
/// finished held the request for the whole timeout and the tab appeared frozen.
///
/// While that build is running the counts are reported as `snapshot_state: "building"` with
/// NULL values, never as zeros: the tab must show that it is waiting, not claim an empty
/// corpus and a refresh that just happened.
pub async fn get_stats() -> Response {
    let stats = filtering::try_fetch_stats().await;

    success_response(FilteringStatsResponse {
        snapshot_state: SnapshotState::of(&stats),
        total_tokens: stats.as_ref().map(|s| s.total_tokens),
        with_pool_price: stats.as_ref().map(|s| s.with_pool_price),
        open_positions: stats.as_ref().map(|s| s.open_positions),
        blacklisted: stats.as_ref().map(|s| s.blacklisted),
        with_ohlcv: stats.as_ref().map(|s| s.with_ohlcv),
        passed_filtering: stats.as_ref().map(|s| s.passed_filtering),
        updated_at: stats.as_ref().map(|s| s.updated_at.to_rfc3339()),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A count this endpoint has not got is NULL, never 0, and the state says which.
    ///
    /// The distinction is the whole point of the field: `0` is a measurement — it says the
    /// pipeline evaluated the corpus and nothing passed — while the freshly-launched app
    /// simply has not finished counting. The dashboard's formatters render null as `—`, so
    /// getting this wrong does not fail loudly, it just makes the tab state something false
    /// and confident for a few seconds on every launch.
    ///
    /// Asserted as an invariant over whichever state the process happens to be in, because
    /// the filtering store is global and another test in the same binary may have built a
    /// snapshot: state and values must agree, in both directions.
    #[tokio::test]
    async fn absent_counts_are_null_and_declared_building() {
        let response = get_stats().await;
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read stats body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("stats json");

        let state = json["snapshot_state"]
            .as_str()
            .expect("responses always carry a snapshot state");

        let counters = [
            "total_tokens",
            "with_pool_price",
            "open_positions",
            "blacklisted",
            "with_ohlcv",
            "passed_filtering",
        ];

        match state {
            "building" => {
                for key in counters {
                    assert!(
                        json[key].is_null(),
                        "{key} came back as {} while the snapshot is still building — a \
                         number here claims a count that has not been taken",
                        json[key],
                    );
                }
                assert!(
                    json["updated_at"].is_null(),
                    "updated_at reported a refresh time while no snapshot exists",
                );
            }
            "ready" => {
                for key in counters {
                    assert!(
                        json[key].is_u64(),
                        "{key} is {} with a snapshot present — a ready snapshot has every \
                         count",
                        json[key],
                    );
                }
                assert!(
                    json["updated_at"].is_string(),
                    "a ready snapshot must report when it was built",
                );
            }
            other => panic!("unknown snapshot state {other:?}"),
        }
    }
}
