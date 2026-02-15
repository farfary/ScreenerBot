use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// RESPONSE TYPES
// ============================================================================

#[derive(Debug, Serialize)]
pub struct RefreshResponse {
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct FilteringStatsResponse {
    pub total_tokens: usize,
    pub with_pool_price: usize,
    pub open_positions: usize,
    pub blacklisted: usize,
    pub with_ohlcv: usize,
    pub passed_filtering: usize,
    pub updated_at: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct RejectionStatEntry {
    pub reason: String,
    pub display_label: String,
    pub source: String,
    pub count: i64,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct RejectionStatsResponse {
    pub stats: Vec<RejectionStatEntry>,
    pub by_source: HashMap<String, i64>,
    pub total_rejected: i64,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryBreakdown {
    pub category: String,
    pub label: String,
    pub icon: String,
    pub count: i64,
    pub percentage: f64,
    pub reasons: Vec<CategoryReasonEntry>,
}

#[derive(Debug, Serialize)]
pub struct CategoryReasonEntry {
    pub reason: String,
    pub display_label: String,
    pub count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct SourceBreakdown {
    pub source: String,
    pub count: i64,
    pub percentage: f64,
    pub top_reasons: Vec<RejectionStatEntry>,
}

#[derive(Debug, Serialize)]
pub struct DataQualityMetric {
    pub metric: String,
    pub label: String,
    pub count: i64,
    pub percentage: f64,
    pub severity: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    // Overview
    pub total_tokens: usize,
    pub total_rejected: i64,
    pub total_passed: usize,
    pub pass_rate: f64,
    pub rejection_rate: f64,

    // Category breakdown
    pub by_category: Vec<CategoryBreakdown>,

    // Source breakdown
    pub by_source: Vec<SourceBreakdown>,

    // Data quality
    pub data_quality: Vec<DataQualityMetric>,

    // Top rejection reasons (detailed)
    pub top_reasons: Vec<RejectionStatEntry>,

    // Recent rejections
    pub recent_rejections: Vec<RecentRejectionEntry>,

    // Time range filter info
    pub time_range: Option<TimeRangeInfo>,

    // Metadata
    pub last_updated: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct TimeRangeInfo {
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub preset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RecentRejectionEntry {
    pub mint: String,
    pub symbol: Option<String>,
    pub reason: String,
    pub display_label: String,
    pub source: String,
    pub rejected_at: String,
}

#[derive(Debug, Serialize)]
pub struct RejectedTokenEntry {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub image_url: Option<String>,
    pub reason: String,
    pub display_label: String,
    pub source: String,
    pub rejected_at: String,
}

// ============================================================================
// QUERY TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    /// Start time as Unix timestamp (seconds)
    pub start_time: Option<i64>,
    /// End time as Unix timestamp (seconds)
    pub end_time: Option<i64>,
    /// Preset name for reference (1h, 6h, 24h, 7d, all)
    pub preset: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectedTokensQuery {
    pub reason: Option<String>,
    pub source: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
