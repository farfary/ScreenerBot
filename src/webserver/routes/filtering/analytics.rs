use axum::{extract::Query, http::StatusCode, response::Response};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    filtering,
    logger::{self, LogTag},
    tokens::{
        get_recent_rejections_async, get_rejection_stats_aggregated_async,
        get_rejection_stats_with_time_filter_async,
    },
    webserver::utils::{error_response, success_response},
};

use super::helpers::{
    get_category_icon, get_category_label, get_rejection_category, get_rejection_display_label,
};
use super::types::{
    AnalyticsQuery, AnalyticsResponse, CategoryBreakdown, CategoryReasonEntry, DataQualityMetric,
    RecentRejectionEntry, RejectionStatEntry, SourceBreakdown, TimeRangeInfo,
};

/// GET /api/filtering/analytics
/// Comprehensive filtering analytics with detailed breakdowns
/// Supports optional time range filtering via start_time and end_time query params
pub async fn get_analytics(Query(query): Query<AnalyticsQuery>) -> Response {
    // Fetch stats and rejection data
    let stats_result = filtering::fetch_stats().await;

    // Choose correct data source based on whether we want "Current State" or "Historical Data"
    let rejection_result = if query.start_time.is_some() || query.end_time.is_some() {
        // Time range specified -> Use history table (rejection_stats)
        // This gives us the cumulative volume of rejections over time
        get_rejection_stats_aggregated_async(query.start_time, query.end_time).await
    } else {
        // No time range -> Use current state snapshot (update_tracking)
        // This gives us the current snapshot of rejected tokens (one per token)
        get_rejection_stats_with_time_filter_async(None, None).await
    };

    let recent_result = get_recent_rejections_async(20).await;

    match (stats_result, rejection_result, recent_result) {
        (Ok(stats), Ok(raw_stats), Ok(recent_raw)) => {
            let total_tokens = stats.total_tokens;
            let total_passed = stats.passed_filtering;

            // Calculate totals and build category/source maps
            let mut by_category_map: HashMap<String, Vec<(String, String, i64)>> = HashMap::new();
            let mut by_source_map: HashMap<String, Vec<(String, String, i64)>> = HashMap::new();
            let mut total_rejected: i64 = 0;

            // Data quality specific counts
            let mut data_quality_counts: HashMap<String, i64> = HashMap::new();

            for (reason, source, count) in &raw_stats {
                total_rejected += count;

                let category = get_rejection_category(reason).to_string();
                by_category_map.entry(category.clone()).or_default().push((
                    reason.clone(),
                    get_rejection_display_label(reason),
                    *count,
                ));

                by_source_map.entry(source.clone()).or_default().push((
                    reason.clone(),
                    get_rejection_display_label(reason),
                    *count,
                ));

                // Track data quality issues specifically
                if reason.contains("missing")
                    || reason.contains("no_decimals")
                    || reason == "dex_data_missing"
                    || reason == "gecko_data_missing"
                    || reason == "rug_data_missing"
                {
                    *data_quality_counts.entry(reason.clone()).or_insert(0) += count;
                }
            }

            // Build category breakdown
            let mut by_category: Vec<CategoryBreakdown> = by_category_map
                .into_iter()
                .map(|(category, reasons)| {
                    let cat_count: i64 = reasons.iter().map(|(_, _, c)| c).sum();
                    let cat_pct = if total_rejected > 0 {
                        (cat_count as f64 / total_rejected as f64) * 100.0
                    } else {
                        0.0
                    };

                    let mut reason_entries: Vec<CategoryReasonEntry> = reasons
                        .into_iter()
                        .map(|(reason, display_label, count)| {
                            let pct = if cat_count > 0 {
                                (count as f64 / cat_count as f64) * 100.0
                            } else {
                                0.0
                            };
                            CategoryReasonEntry {
                                reason,
                                display_label,
                                count,
                                percentage: (pct * 10.0).round() / 10.0,
                            }
                        })
                        .collect();

                    reason_entries.sort_by(|a, b| b.count.cmp(&a.count));

                    CategoryBreakdown {
                        label: get_category_label(&category).to_string(),
                        icon: get_category_icon(&category).to_string(),
                        category,
                        count: cat_count,
                        percentage: (cat_pct * 10.0).round() / 10.0,
                        reasons: reason_entries,
                    }
                })
                .collect();

            by_category.sort_by(|a, b| b.count.cmp(&a.count));

            // Build source breakdown
            let mut by_source: Vec<SourceBreakdown> = by_source_map
                .into_iter()
                .map(|(source, reasons)| {
                    let src_count: i64 = reasons.iter().map(|(_, _, c)| c).sum();
                    let src_pct = if total_rejected > 0 {
                        (src_count as f64 / total_rejected as f64) * 100.0
                    } else {
                        0.0
                    };

                    let mut top_reasons: Vec<RejectionStatEntry> = reasons
                        .into_iter()
                        .map(|(reason, display_label, count)| {
                            let pct = if src_count > 0 {
                                (count as f64 / src_count as f64) * 100.0
                            } else {
                                0.0
                            };
                            RejectionStatEntry {
                                category: get_rejection_category(&reason).to_string(),
                                reason,
                                display_label,
                                source: source.clone(),
                                count,
                                percentage: (pct * 10.0).round() / 10.0,
                            }
                        })
                        .collect();

                    top_reasons.sort_by(|a, b| b.count.cmp(&a.count));
                    top_reasons.truncate(5); // Top 5 per source

                    SourceBreakdown {
                        source,
                        count: src_count,
                        percentage: (src_pct * 10.0).round() / 10.0,
                        top_reasons,
                    }
                })
                .collect();

            by_source.sort_by(|a, b| b.count.cmp(&a.count));

            // Build data quality metrics
            let data_quality: Vec<DataQualityMetric> = data_quality_counts
                .into_iter()
                .map(|(metric, count)| {
                    let pct = if total_rejected > 0 {
                        (count as f64 / total_rejected as f64) * 100.0
                    } else {
                        0.0
                    };
                    let severity = if pct > 20.0 {
                        "critical"
                    } else if pct > 5.0 {
                        "warning"
                    } else {
                        "info"
                    };
                    DataQualityMetric {
                        label: get_rejection_display_label(&metric),
                        metric,
                        count,
                        percentage: (pct * 10.0).round() / 10.0,
                        severity: severity.to_string(),
                    }
                })
                .collect();

            // Build top reasons list
            let mut top_reasons: Vec<RejectionStatEntry> = raw_stats
                .into_iter()
                .map(|(reason, source, count)| {
                    let pct = if total_rejected > 0 {
                        (count as f64 / total_rejected as f64) * 100.0
                    } else {
                        0.0
                    };
                    RejectionStatEntry {
                        display_label: get_rejection_display_label(&reason),
                        category: get_rejection_category(&reason).to_string(),
                        reason,
                        source,
                        count,
                        percentage: (pct * 10.0).round() / 10.0,
                    }
                })
                .collect();

            top_reasons.sort_by(|a, b| b.count.cmp(&a.count));

            // Build recent rejections list
            let recent_rejections: Vec<RecentRejectionEntry> = recent_raw
                .into_iter()
                .map(|(mint, reason, source, ts, symbol)| RecentRejectionEntry {
                    mint,
                    symbol,
                    display_label: get_rejection_display_label(&reason),
                    reason,
                    source,
                    rejected_at: DateTime::from_timestamp(ts, 0)
                        .unwrap_or_else(|| Utc::now())
                        .to_rfc3339(),
                })
                .collect();

            // Calculate rates
            let pass_rate = if total_tokens > 0 {
                (total_passed as f64 / total_tokens as f64) * 100.0
            } else {
                0.0
            };

            let rejection_rate = 100.0 - pass_rate;

            // Build time range info if filtering was applied
            let time_range = if query.start_time.is_some() || query.end_time.is_some() {
                Some(TimeRangeInfo {
                    start_time: query.start_time,
                    end_time: query.end_time,
                    preset: query.preset.clone(),
                })
            } else {
                None
            };

            success_response(AnalyticsResponse {
                total_tokens,
                total_rejected,
                total_passed,
                pass_rate: (pass_rate * 10.0).round() / 10.0,
                rejection_rate: (rejection_rate * 10.0).round() / 10.0,
                by_category,
                by_source,
                data_quality,
                top_reasons,
                recent_rejections,
                time_range,
                last_updated: stats.updated_at.to_rfc3339(),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        (Err(err), _, _) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch filtering stats for analytics: {err}"),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ANALYTICS_FAILED",
                &format!("Failed to fetch analytics: {err}"),
                None,
            )
        }
        (_, Err(err), _) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejection stats for analytics: {:?}", err),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ANALYTICS_FAILED",
                &format!("Failed to fetch analytics: {:?}", err),
                None,
            )
        }
        (_, _, Err(err)) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch recent rejections for analytics: {:?}", err),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ANALYTICS_FAILED",
                &format!("Failed to fetch analytics: {:?}", err),
                None,
            )
        }
    }
}
