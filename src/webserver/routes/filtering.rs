use axum::{
    extract::Query,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::{
    filtering,
    logger::{self, LogTag},
    tokens::{
        get_recent_rejections_async, get_rejected_tokens_async,
        get_rejection_stats_aggregated_async, get_rejection_stats_async,
        get_rejection_stats_with_time_filter_async, get_token_info_batch_async,
    },
    webserver::state::AppState,
    webserver::utils::{error_response, success_response},
};

/// Filtering management routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/filtering/refresh", post(trigger_refresh))
        .route("/filtering/stats", get(get_stats))
        .route("/filtering/rejection-stats", get(get_rejection_stats))
        .route("/filtering/analytics", get(get_analytics))
        .route(
            "/filtering/rejected-tokens",
            get(get_rejected_tokens_handler),
        )
        .route(
            "/filtering/export-rejected-tokens",
            get(export_rejected_tokens),
        )
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
async fn trigger_refresh() -> Response {
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

/// Reason label to human-readable display mapping
fn get_rejection_display_label(reason: &str) -> String {
    match reason {
        "no_decimals" => "No decimals in database",
        "token_too_new" => "Token too new",
        "cooldown_filtered" => "Cooldown filtered",
        "dex_data_missing" => "DexScreener data missing",
        "gecko_data_missing" => "GeckoTerminal data missing",
        "rug_data_missing" => "Rugcheck data missing",
        "dex_empty_name" => "Empty name",
        "dex_empty_symbol" => "Empty symbol",
        "dex_empty_logo" => "Empty logo URL",
        "dex_empty_website" => "Empty website URL",
        "dex_txn_5m" => "Low 5m transactions",
        "dex_txn_1h" => "Low 1h transactions",
        "dex_zero_liq" => "Zero liquidity",
        "dex_liq_low" => "Liquidity too low",
        "dex_liq_high" => "Liquidity too high",
        "dex_mcap_low" => "Market cap too low",
        "dex_mcap_high" => "Market cap too high",
        "dex_vol_low" => "Volume too low",
        "dex_vol_missing" => "Volume missing",
        "dex_fdv_low" => "FDV too low",
        "dex_fdv_high" => "FDV too high",
        "dex_fdv_missing" => "FDV missing",
        "dex_vol5m_low" => "5m volume too low",
        "dex_vol5m_missing" => "5m volume missing",
        "dex_vol1h_low" => "1h volume too low",
        "dex_vol1h_missing" => "1h volume missing",
        "dex_vol6h_low" => "6h volume too low",
        "dex_vol6h_missing" => "6h volume missing",
        "dex_price_change_5m_low" => "5m price change too low",
        "dex_price_change_5m_high" => "5m price change too high",
        "dex_price_change_5m_missing" => "5m price change missing",
        "dex_price_change_low" => "Price change too low",
        "dex_price_change_high" => "Price change too high",
        "dex_price_change_missing" => "Price change missing",
        "dex_price_change_6h_low" => "6h price change too low",
        "dex_price_change_6h_high" => "6h price change too high",
        "dex_price_change_6h_missing" => "6h price change missing",
        "dex_price_change_24h_low" => "24h price change too low",
        "dex_price_change_24h_high" => "24h price change too high",
        "dex_price_change_24h_missing" => "24h price change missing",
        "gecko_liq_missing" => "Liquidity missing",
        "gecko_liq_low" => "Liquidity too low",
        "gecko_liq_high" => "Liquidity too high",
        "gecko_mcap_missing" => "Market cap missing",
        "gecko_mcap_low" => "Market cap too low",
        "gecko_mcap_high" => "Market cap too high",
        "gecko_vol5m_low" => "5m volume too low",
        "gecko_vol5m_missing" => "5m volume missing",
        "gecko_vol1h_low" => "1h volume too low",
        "gecko_vol1h_missing" => "1h volume missing",
        "gecko_vol24h_low" => "24h volume too low",
        "gecko_vol24h_missing" => "24h volume missing",
        "gecko_price_change_5m_low" => "5m price change too low",
        "gecko_price_change_5m_high" => "5m price change too high",
        "gecko_price_change_5m_missing" => "5m price change missing",
        "gecko_price_change_1h_low" => "1h price change too low",
        "gecko_price_change_1h_high" => "1h price change too high",
        "gecko_price_change_1h_missing" => "1h price change missing",
        "gecko_price_change_24h_low" => "24h price change too low",
        "gecko_price_change_24h_high" => "24h price change too high",
        "gecko_price_change_24h_missing" => "24h price change missing",
        "gecko_pool_count_low" => "Pool count too low",
        "gecko_pool_count_high" => "Pool count too high",
        "gecko_pool_count_missing" => "Pool count missing",
        "gecko_reserve_low" => "Reserve too low",
        "gecko_reserve_missing" => "Reserve missing",
        "rug_rugged" => "Rugged token",
        "rug_score" => "Risk score too high",
        "rug_level_danger" => "Danger risk level",
        "rug_mint_authority" => "Mint authority present",
        "rug_freeze_authority" => "Freeze authority present",
        "rug_top_holder" => "Top holder % too high",
        "rug_top3_holders" => "Top 3 holders % too high",
        "rug_min_holders" => "Not enough holders",
        "rug_insider_count" => "Too many insider holders",
        "rug_insider_pct" => "Insider % too high",
        "rug_creator_pct" => "Creator balance too high",
        "rug_transfer_fee_present" => "Transfer fee present",
        "rug_transfer_fee_high" => "Transfer fee too high",
        "rug_transfer_fee_missing" => "Transfer fee data missing",
        "rug_graph_insiders" => "Graph insiders too high",
        "rug_lp_providers_low" => "LP providers too low",
        "rug_lp_providers_missing" => "LP providers missing",
        "rug_lp_lock_low" => "LP lock too low",
        "rug_lp_lock_missing" => "LP lock missing",
        _ => reason, // Return original if not mapped
    }
    .to_string()
}

#[derive(Debug, Serialize)]
struct RejectionStatEntry {
    reason: String,
    display_label: String,
    source: String,
    count: i64,
    #[serde(default)]
    category: String,
    #[serde(default)]
    percentage: f64,
}

#[derive(Debug, Serialize)]
struct RejectionStatsResponse {
    stats: Vec<RejectionStatEntry>,
    by_source: HashMap<String, i64>,
    total_rejected: i64,
    timestamp: String,
}

/// GET /api/filtering/rejection-stats
/// Get counts of rejected tokens grouped by rejection reason
async fn get_rejection_stats() -> Response {
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

// ============================================================================
// ANALYTICS ENDPOINT - Advanced filtering analysis
// ============================================================================

/// Categorize rejection reason into high-level category
fn get_rejection_category(reason: &str) -> &'static str {
    if reason.starts_with("rug_") {
        if reason.contains("authority")
            || reason.contains("rugged")
            || reason.contains("level_danger")
        {
            "security"
        } else if reason.contains("holder")
            || reason.contains("insider")
            || reason.contains("creator")
        {
            "distribution"
        } else if reason.contains("lp_") {
            "liquidity_lock"
        } else if reason.contains("transfer_fee") {
            "fees"
        } else {
            "security"
        }
    } else if reason.starts_with("dex_") || reason.starts_with("gecko_") {
        if reason.contains("liq") || reason.contains("reserve") {
            "liquidity"
        } else if reason.contains("vol") {
            "volume"
        } else if reason.contains("mcap") || reason.contains("fdv") {
            "market_cap"
        } else if reason.contains("price_change") {
            "price_action"
        } else if reason.contains("txn") {
            "activity"
        } else if reason.contains("empty") || reason.contains("missing") {
            "data_quality"
        } else {
            "market"
        }
    } else if reason.contains("decimals") || reason.contains("data_missing") {
        "data_quality"
    } else if reason.contains("cooldown") || reason.contains("new") {
        "timing"
    } else {
        "other"
    }
}

/// Get human-readable category label
fn get_category_label(category: &str) -> &'static str {
    match category {
        "security" => "Security Issues",
        "distribution" => "Holder Distribution",
        "liquidity_lock" => "LP Lock Issues",
        "fees" => "Transfer Fees",
        "liquidity" => "Liquidity",
        "volume" => "Trading Volume",
        "market_cap" => "Market Cap/FDV",
        "price_action" => "Price Movement",
        "activity" => "Trading Activity",
        "data_quality" => "Missing Data",
        "timing" => "Timing Filters",
        "market" => "Market Data",
        _ => "Other",
    }
}

/// Get icon for category
fn get_category_icon(category: &str) -> &'static str {
    match category {
        "security" => "shield",
        "distribution" => "users",
        "liquidity_lock" => "lock",
        "fees" => "percent",
        "liquidity" => "droplet",
        "volume" => "chart-bar",
        "market_cap" => "dollar-sign",
        "price_action" => "trending-up",
        "activity" => "activity",
        "data_quality" => "circle-alert",
        "timing" => "clock",
        "market" => "trending-up",
        _ => "info",
    }
}

#[derive(Debug, Serialize)]
struct CategoryBreakdown {
    category: String,
    label: String,
    icon: String,
    count: i64,
    percentage: f64,
    reasons: Vec<CategoryReasonEntry>,
}

#[derive(Debug, Serialize)]
struct CategoryReasonEntry {
    reason: String,
    display_label: String,
    count: i64,
    percentage: f64,
}

#[derive(Debug, Serialize)]
struct SourceBreakdown {
    source: String,
    count: i64,
    percentage: f64,
    top_reasons: Vec<RejectionStatEntry>,
}

#[derive(Debug, Serialize)]
struct DataQualityMetric {
    metric: String,
    label: String,
    count: i64,
    percentage: f64,
    severity: String,
}

#[derive(Debug, Serialize)]
struct AnalyticsResponse {
    // Overview
    total_tokens: usize,
    total_rejected: i64,
    total_passed: usize,
    pass_rate: f64,
    rejection_rate: f64,

    // Category breakdown
    by_category: Vec<CategoryBreakdown>,

    // Source breakdown
    by_source: Vec<SourceBreakdown>,

    // Data quality
    data_quality: Vec<DataQualityMetric>,

    // Top rejection reasons (detailed)
    top_reasons: Vec<RejectionStatEntry>,

    // Recent rejections
    recent_rejections: Vec<RecentRejectionEntry>,

    // Time range filter info
    time_range: Option<TimeRangeInfo>,

    // Metadata
    last_updated: String,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct TimeRangeInfo {
    start_time: Option<i64>,
    end_time: Option<i64>,
    preset: Option<String>,
}

#[derive(Debug, Serialize)]
struct RecentRejectionEntry {
    mint: String,
    symbol: Option<String>,
    reason: String,
    display_label: String,
    source: String,
    rejected_at: String,
}

#[derive(Debug, Deserialize)]
struct AnalyticsQuery {
    /// Start time as Unix timestamp (seconds)
    start_time: Option<i64>,
    /// End time as Unix timestamp (seconds)
    end_time: Option<i64>,
    /// Preset name for reference (1h, 6h, 24h, 7d, all)
    preset: Option<String>,
}

/// GET /api/filtering/analytics
/// Comprehensive filtering analytics with detailed breakdowns
/// Supports optional time range filtering via start_time and end_time query params
async fn get_analytics(Query(query): Query<AnalyticsQuery>) -> Response {
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
                &format!("Failed to fetch filtering stats for analytics: {}", err),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "ANALYTICS_FAILED",
                &format!("Failed to fetch analytics: {}", err),
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

#[derive(Debug, Deserialize)]
struct RejectedTokensQuery {
    reason: Option<String>,
    source: Option<String>,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[derive(Debug, Serialize)]
struct RejectedTokenEntry {
    mint: String,
    symbol: Option<String>,
    name: Option<String>,
    image_url: Option<String>,
    reason: String,
    display_label: String,
    source: String,
    rejected_at: String,
}

/// GET /api/filtering/rejected-tokens
/// Get list of rejected tokens with pagination and filtering
async fn get_rejected_tokens_handler(Query(params): Query<RejectedTokensQuery>) -> Response {
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 per page
    let offset = params.offset.unwrap_or(0);

    match get_rejected_tokens_async(params.reason, params.source, params.search, limit, offset)
        .await
    {
        Ok(tokens) => {
            // Collect mints for batch token info lookup
            let mints: Vec<String> = tokens.iter().map(|(mint, _, _, _)| mint.clone()).collect();

            // Fetch token info (symbol, name, image) in a single batch query
            let token_info = get_token_info_batch_async(mints).await.unwrap_or_default();

            let entries: Vec<RejectedTokenEntry> = tokens
                .into_iter()
                .map(|(mint, reason, source, ts)| {
                    let (symbol, name, image_url) =
                        token_info.get(&mint).cloned().unwrap_or((None, None, None));

                    RejectedTokenEntry {
                        mint,
                        symbol,
                        name,
                        image_url,
                        display_label: get_rejection_display_label(&reason),
                        reason,
                        source,
                        rejected_at: DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(|| Utc::now())
                            .to_rfc3339(),
                    }
                })
                .collect();

            success_response(entries)
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejected tokens: {:?}", err),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FETCH_FAILED",
                &format!("Failed to fetch rejected tokens: {:?}", err),
                None,
            )
        }
    }
}

/// GET /api/filtering/export-rejected-tokens
/// Export rejected tokens to CSV
async fn export_rejected_tokens(Query(params): Query<RejectedTokensQuery>) -> Response {
    // Fetch up to 100,000 tokens for export
    let limit = 100000;
    let offset = 0;

    match get_rejected_tokens_async(params.reason, params.source, params.search, limit, offset)
        .await
    {
        Ok(tokens) => {
            let mut wtr = csv::Writer::from_writer(vec![]);
            // Write header
            if let Err(e) =
                wtr.write_record(&["Mint", "Reason", "Display Label", "Source", "Rejected At"])
            {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CSV_ERROR",
                    &format!("Failed to write CSV header: {}", e),
                    None,
                );
            }

            // Write records
            for (mint, reason, source, ts) in tokens {
                let dt = DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(|| Utc::now())
                    .to_rfc3339();
                let display_label = get_rejection_display_label(&reason);

                if let Err(e) = wtr.write_record(&[mint, reason, display_label, source, dt]) {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "CSV_ERROR",
                        &format!("Failed to write CSV record: {}", e),
                        None,
                    );
                }
            }

            match wtr.into_inner() {
                Ok(data) => {
                    let filename =
                        format!("rejected_tokens_{}.csv", Utc::now().format("%Y%m%d_%H%M%S"));

                    Response::builder()
                        .header("Content-Type", "text/csv")
                        .header(
                            "Content-Disposition",
                            format!("attachment; filename=\"{}\"", filename),
                        )
                        .body(axum::body::Body::from(data))
                        .unwrap_or_else(|_| {
                            error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "RESPONSE_ERROR",
                                "Failed to build response",
                                None,
                            )
                        })
                }
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CSV_ERROR",
                    &format!("Failed to finalize CSV: {}", e),
                    None,
                ),
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to fetch rejected tokens for export: {:?}", err),
            );
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "FETCH_FAILED",
                &format!("Failed to fetch rejected tokens: {:?}", err),
                None,
            )
        }
    }
}
