//! Data types for wallet balance monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Wallet balance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub id: Option<i64>,
    pub wallet_address: String,
    pub snapshot_time: DateTime<Utc>,
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub total_nfts_count: u32,
    pub token_balances: Vec<SnapshotTokenBalance>,
    pub nft_balances: Vec<NftBalance>,
}

/// Token balance record (fungible tokens only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotTokenBalance {
    pub id: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub mint: String,
    pub balance: u64,    // Raw token amount
    pub balance_ui: f64, // UI amount (adjusted for decimals)
    pub decimals: u8,
    pub is_token_2022: bool,
}

/// NFT balance record (non-fungible tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftBalance {
    pub id: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub mint: String,
    pub account_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub image_url: Option<String>,
    pub is_token_2022: bool,
}

/// Wallet monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMonitorStats {
    pub total_snapshots: u64,
    pub latest_snapshot_time: Option<DateTime<Utc>>,
    pub wallet_address: String,
    pub current_sol_balance: Option<f64>,
    pub current_tokens_count: Option<u32>,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

/// High level wallet summary for dashboard consumers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSummarySnapshot {
    pub window_hours: i64,
    pub current_sol_balance: f64,
    pub previous_sol_balance: Option<f64>,
    pub sol_change: f64,
    pub sol_change_percent: Option<f64>,
    pub token_count: u32,
    pub last_snapshot_time: Option<String>,
}

/// Aggregated wallet flow metrics calculated from transactions module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowMetrics {
    pub window_hours: i64,
    pub inflow_sol: f64,
    pub outflow_sol: f64,
    pub net_sol: f64,
    pub transactions_analyzed: usize,
}

/// Data point for SOL balance trend chart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletBalancePoint {
    pub timestamp: i64,
    pub sol_balance: f64,
}

/// Daily flow data point for time-series chart
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyFlowPoint {
    pub date: String,    // YYYY-MM-DD
    pub timestamp: i64,  // Unix timestamp for charting
    pub inflow: f64,     // SOL inflow that day
    pub outflow: f64,    // SOL outflow that day
    pub net: f64,        // inflow - outflow
    pub tx_count: usize, // Number of transactions
}

/// Token row with enriched metadata for wallet table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTokenOverview {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub image_url: Option<String>,
    pub balance_ui: f64,
    pub balance_raw: u64,
    pub decimals: u8,
    pub is_token_2022: bool,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub value_sol: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub last_updated: Option<String>,
    pub dex_id: Option<String>,
}

/// NFT overview for wallet display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletNftOverview {
    pub mint: String,
    pub account_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub image_url: Option<String>,
    pub is_token_2022: bool,
}

/// Complete dashboard payload for wallet UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletDashboardData {
    pub summary: WalletSummarySnapshot,
    pub flows: WalletFlowMetrics,
    pub balance_trend: Vec<WalletBalancePoint>,
    pub daily_flows: Vec<DailyFlowPoint>,
    pub tokens: Vec<WalletTokenOverview>,
    pub nfts: Vec<WalletNftOverview>,
    pub last_updated: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_metadata: Option<DashboardCacheMetadata>,
}

/// Flow cache stats for diagnostics/UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowCacheStats {
    pub rows: u64,
    pub max_timestamp: Option<String>,
}

/// Cached readiness data for lightweight webserver checks
#[derive(Debug, Clone)]
pub struct WalletSnapshotStatus {
    pub is_ready: bool,
    pub last_updated: Option<DateTime<Utc>>,
}

// =============================================================================
// DASHBOARD CACHE TYPES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardCacheFreshness {
    Fresh,
    Aging,
    Stale,
    Realtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardDataSource {
    Memory,
    Database,
    Realtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardCacheMetadata {
    pub window_key: Option<String>,
    pub cached_at: Option<String>,
    pub valid_until: Option<String>,
    pub age_seconds: Option<u64>,
    pub next_update_in_seconds: Option<u64>,
    pub freshness: DashboardCacheFreshness,
    pub source: DashboardDataSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub computation_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_count: Option<usize>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CachePerformanceMetrics {
    pub(crate) total_requests: u64,
    pub(crate) memory_hits: u64,
    pub(crate) database_hits: u64,
    pub(crate) realtime_computations: u64,
    pub(crate) stale_responses: u64,
    pub(crate) total_latency_ms: u128,
    pub(crate) last_source: Option<DashboardDataSource>,
}

// =============================================================================
// INTERNAL CACHE TYPES
// =============================================================================

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub(super) struct DashboardRequestKey {
    pub(super) window_hours: i64,
    pub(super) snapshot_limit: usize,
    pub(super) max_tokens: usize,
}

#[derive(Debug, Clone)]
pub(super) struct CachedDashboardResponse {
    pub(super) data: WalletDashboardData,
    pub(super) cached_at: std::time::Instant,
}
