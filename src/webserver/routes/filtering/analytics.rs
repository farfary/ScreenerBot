// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Filtering analytics route — serves filter rejection statistics and charts.

use axum::{
    extract::Query,
    response::{IntoResponse as _, Response},
};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    chains::ChainScope,
    filtering::{self, sources::rejection_text, FilteringStatsSnapshot, SnapshotState},
    i18n::ids,
    logger::{self, LogTag},
    tokens::{
        get_recent_rejections_async, get_rejection_stats_aggregated_async,
        get_rejection_stats_with_time_filter_async,
    },
    webserver::{
        api_error::{ApiError, ApiErrorCode},
        utils::success_response,
    },
};

use super::helpers::RejectionCategory;
use super::types::{
    AnalyticsQuery, AnalyticsResponse, CategoryBreakdown, CategoryReasonEntry, DataQualityMetric,
    RecentRejectionEntry, RejectionStatEntry, SourceBreakdown, TimeRangeInfo,
};

/// The headline cards. They always describe the current snapshot, in every time range, so
/// scanned, passed, rejected and both rates agree with each other. The selected range
/// drives only the breakdowns, which count rejection checks in that period.
struct Headline {
    total_tokens: Option<usize>,
    total_passed: Option<usize>,
    total_rejected: Option<usize>,
    pass_rate: Option<f64>,
    rejection_rate: Option<f64>,
}

impl Headline {
    /// Every figure is absent while the first snapshot is still building. A rate is only
    /// meaningful against a corpus size the snapshot has counted, so it stays absent too
    /// rather than resolving to a flat 0%.
    fn of(stats: Option<&FilteringStatsSnapshot>) -> Self {
        let Some(stats) = stats else {
            return Self {
                total_tokens: None,
                total_passed: None,
                total_rejected: None,
                pass_rate: None,
                rejection_rate: None,
            };
        };
        let total = stats.total_tokens;
        let passed = stats.passed_filtering;
        let pass_rate = if total > 0 {
            ((passed as f64 / total as f64) * 1000.0).round() / 10.0
        } else {
            0.0
        };
        Self {
            total_tokens: Some(total),
            total_passed: Some(passed),
            total_rejected: Some(total.saturating_sub(passed)),
            pass_rate: Some(pass_rate),
            rejection_rate: Some(((100.0 - pass_rate) * 10.0).round() / 10.0),
        }
    }
}

/// GET /api/filtering/analytics
/// Comprehensive filtering analytics with detailed breakdowns
/// Supports optional time range filtering via start_time and end_time query params; the
/// range changes the breakdowns only, never the headline figures ([`Headline`])
pub async fn get_analytics(Query(query): Query<AnalyticsQuery>) -> Response {
    // Fetch stats and rejection data. Non-blocking: the analytics panel must not hold the
    // request for up to 30 seconds waiting on the first snapshot build (the rejection
    // breakdowns below come from the database and are useful on their own meanwhile).
    let stats_result = filtering::try_fetch_stats(ChainScope::All).await;

    // Choose correct data source based on whether we want "Current State" or "Historical Data"
    // The rejection tables are per chain; a route without a chain reads the only one.
    let chain = ChainScope::All.sole_chain();
    let rejection_result = match chain {
        Err(ref error) => Err(error.clone().into()),
        Ok(chain) if query.start_time.is_some() || query.end_time.is_some() => {
            // Time range specified -> Use history table (rejection_stats)
            // This gives us the cumulative volume of rejections over time
            get_rejection_stats_aggregated_async(chain, query.start_time, query.end_time).await
        }
        Ok(chain) => {
            // No time range -> Use current state snapshot (update_tracking)
            // This gives us the current snapshot of rejected tokens (one per token)
            get_rejection_stats_with_time_filter_async(chain, None, None).await
        }
    };

    let recent_result = match chain {
        Ok(chain) => get_recent_rejections_async(chain, 20).await,
        Err(error) => Err(error.into()),
    };

    match (stats_result, rejection_result, recent_result) {
        (stats, Ok(raw_stats), Ok(recent_raw)) => {
            // Absent while the first snapshot is still building; the rejection breakdowns
            // below are database-backed and render regardless.
            let headline = Headline::of(stats.as_ref());

            // Calculate totals and build category/source maps
            let mut by_category_map: HashMap<
                &'static str,
                (RejectionCategory, Vec<(String, i64)>),
            > = HashMap::new();
            let mut by_source_map: HashMap<String, Vec<(String, i64)>> = HashMap::new();
            let mut breakdown_total: i64 = 0;

            // Data quality specific counts
            let mut data_quality_counts: HashMap<String, i64> = HashMap::new();

            for (reason, source, count) in &raw_stats {
                breakdown_total += count;

                let category = RejectionCategory::of_code(reason);
                by_category_map
                    .entry(category.id())
                    .or_insert_with(|| (category, Vec::new()))
                    .1
                    .push((reason.clone(), *count));

                by_source_map
                    .entry(source.clone())
                    .or_default()
                    .push((reason.clone(), *count));

                // Track data quality issues specifically
                if reason.contains("missing")
                    || reason.contains("no_decimals")
                    || reason == "dex_data_missing"
                    || reason == "gecko_data_missing"
                    || reason == "rug_data_missing"
                {
                    *data_quality_counts.entry(reason.clone()).or_default() += count;
                }
            }

            // Build category breakdown
            let mut by_category: Vec<CategoryBreakdown> = by_category_map
                .into_iter()
                .map(|(_, (category, reasons))| {
                    let cat_count: i64 = reasons.iter().map(|(_, c)| c).sum();
                    let cat_pct = if breakdown_total > 0 {
                        (cat_count as f64 / breakdown_total as f64) * 100.0
                    } else {
                        0.0
                    };

                    let mut reason_entries: Vec<CategoryReasonEntry> = reasons
                        .into_iter()
                        .map(|(reason, count)| {
                            let pct = if cat_count > 0 {
                                (count as f64 / cat_count as f64) * 100.0
                            } else {
                                0.0
                            };
                            CategoryReasonEntry {
                                reason_text: rejection_text(&reason),
                                reason,
                                count,
                                percentage: (pct * 10.0).round() / 10.0,
                            }
                        })
                        .collect();

                    reason_entries.sort_by(|a, b| b.count.cmp(&a.count));

                    CategoryBreakdown {
                        category_text: category.text(),
                        icon: category.icon().to_string(),
                        category: category.id().to_string(),
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
                    let src_count: i64 = reasons.iter().map(|(_, c)| c).sum();
                    let src_pct = if breakdown_total > 0 {
                        (src_count as f64 / breakdown_total as f64) * 100.0
                    } else {
                        0.0
                    };

                    let mut top_reasons: Vec<RejectionStatEntry> = reasons
                        .into_iter()
                        .map(|(reason, count)| {
                            let pct = if src_count > 0 {
                                (count as f64 / src_count as f64) * 100.0
                            } else {
                                0.0
                            };
                            let category = RejectionCategory::of_code(&reason);
                            RejectionStatEntry {
                                category: category.id().to_string(),
                                category_text: category.text(),
                                reason_text: rejection_text(&reason),
                                reason,
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
                    let pct = if breakdown_total > 0 {
                        (count as f64 / breakdown_total as f64) * 100.0
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
                        reason_text: rejection_text(&metric),
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
                    let pct = if breakdown_total > 0 {
                        (count as f64 / breakdown_total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let category = RejectionCategory::of_code(&reason);
                    RejectionStatEntry {
                        reason_text: rejection_text(&reason),
                        category: category.id().to_string(),
                        category_text: category.text(),
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
                .map(
                    |(mint, reason, source, ts, symbol, name, image_url)| RecentRejectionEntry {
                        mint,
                        symbol,
                        name,
                        image_url,
                        reason_text: rejection_text(&reason),
                        reason,
                        source,
                        rejected_at: DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(|| Utc::now())
                            .to_rfc3339(),
                    },
                )
                .collect();

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
                snapshot_state: SnapshotState::of(&stats),
                total_tokens: headline.total_tokens,
                total_rejected: headline.total_rejected,
                total_passed: headline.total_passed,
                pass_rate: headline.pass_rate,
                rejection_rate: headline.rejection_rate,
                breakdown_total,
                by_category,
                by_source,
                data_quality,
                top_reasons,
                recent_rejections,
                time_range,
                last_updated: stats.as_ref().map(|s| s.updated_at.to_rfc3339()),
                timestamp: Utc::now().to_rfc3339(),
            })
        }
        (_, Err(err), _) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejection stats for analytics: {:?}", err),
            );
            ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_FILTERING_ANALYTICS_FAILED,
            )
            .details(format!("{err:?}"))
            .into_response()
        }
        (_, _, Err(err)) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch recent rejections for analytics: {:?}", err),
            );
            ApiError::new(
                ApiErrorCode::Internal,
                ids::ERRORS_FILTERING_ANALYTICS_FAILED,
            )
            .details(format!("{err:?}"))
            .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(total_tokens: usize, passed_filtering: usize) -> FilteringStatsSnapshot {
        FilteringStatsSnapshot {
            total_tokens_in_database: total_tokens * 10,
            total_tokens,
            with_pool_price: 0,
            open_positions: 0,
            blacklisted: 0,
            with_ohlcv: 0,
            passed_filtering,
            updated_at: Utc::now(),
        }
    }

    /// Scanned, passed and rejected come from the one snapshot, so they add up and the two
    /// rates are complements. The history table counts every re-evaluation, so a figure
    /// taken from it next to the snapshot's corpus size read 720,000 rejected of 30,000
    /// scanned; the headline takes no range input at all.
    #[test]
    fn headline_figures_come_from_the_snapshot_and_agree() {
        let headline = Headline::of(Some(&snapshot(30_000, 1_200)));

        assert_eq!(headline.total_tokens, Some(30_000));
        assert_eq!(headline.total_passed, Some(1_200));
        assert_eq!(headline.total_rejected, Some(28_800));
        assert_eq!(headline.pass_rate, Some(4.0));
        assert_eq!(headline.rejection_rate, Some(96.0));
    }

    /// An empty corpus is a measurement of zero, with a 0% pass rate.
    #[test]
    fn headline_of_an_empty_snapshot_is_zero_not_absent() {
        let headline = Headline::of(Some(&snapshot(0, 0)));

        assert_eq!(headline.total_tokens, Some(0));
        assert_eq!(headline.total_rejected, Some(0));
        assert_eq!(headline.pass_rate, Some(0.0));
        assert_eq!(headline.rejection_rate, Some(100.0));
    }

    /// Before the first snapshot exists every figure is absent, never 0.
    #[test]
    fn headline_is_absent_while_the_snapshot_is_building() {
        let headline = Headline::of(None);

        assert_eq!(headline.total_tokens, None);
        assert_eq!(headline.total_passed, None);
        assert_eq!(headline.total_rejected, None);
        assert_eq!(headline.pass_rate, None);
        assert_eq!(headline.rejection_rate, None);
    }
}
