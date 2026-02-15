use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;

use crate::rpc::{RpcMinuteBucket, RpcSessionSnapshot, RpcStats};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct RpcMetricsSummary {
    pub total_calls: u64,
    pub total_errors: u64,
    pub success_rate: f32,
    pub recent_calls_per_minute: f64,
}

impl From<&RpcStats> for RpcMetricsSummary {
    fn from(stats: &RpcStats) -> Self {
        Self {
            total_calls: stats.total_calls(),
            total_errors: stats.total_errors(),
            success_rate: stats.success_rate(),
            recent_calls_per_minute: stats.calls_per_minute_recent(5),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub trading_enabled: bool,
    pub trader_mode: String,
    pub trader_running: bool,
    pub open_positions: usize,
    pub closed_positions_today: usize,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub services: ServiceStatusSnapshot,
    pub metrics: SystemMetricsSnapshot,
    pub rpc_stats: Option<RpcStatsSnapshot>,
    pub wallet: Option<WalletStatusSnapshot>,
    pub ohlcv_stats: Option<OhlcvStatsSnapshot>,
    pub pools: Option<PoolServiceStatusSnapshot>,
    pub discovery: Option<TokenDiscoveryStatusSnapshot>,
    pub events: Option<EventsStatusSnapshot>,
    pub transactions: Option<TransactionsStatusSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dexscreener: Option<DexscreenerStatusSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geckoterminal: Option<GeckoTerminalStatusSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatusSnapshot {
    pub tokens_system: ServiceStateSnapshot,
    pub positions_system: ServiceStateSnapshot,
    pub pool_service: ServiceStateSnapshot,
    pub transactions_system: ServiceStateSnapshot,
    pub all_ready: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStateSnapshot {
    pub ready: bool,
    pub status: String,
    pub last_check: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ServiceStateSnapshot {
    pub(super) fn new(ready: bool, last_check: DateTime<Utc>, error: Option<String>) -> Self {
        let status = if ready { "healthy" } else { "starting" }.to_string();
        let error = if ready { None } else { error };
        Self {
            ready,
            status,
            last_check,
            error,
        }
    }
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct SystemMetricsSnapshot {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub system_memory_used_mb: u64,
    pub system_memory_total_mb: u64,
    pub process_memory_mb: u64,
    pub cpu_system_percent: f32,
    pub cpu_process_percent: f32,
    pub active_threads: usize,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub rpc_success_rate: f32,
    pub rpc_calls_per_minute_recent: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletStatusSnapshot {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub usdc_balance: f64,
    pub total_tokens_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_time: Option<DateTime<Utc>>,
    pub token_balances: Vec<WalletTokenBalanceSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletTokenBalanceSnapshot {
    pub mint: String,
    pub balance: u64,
    pub balance_ui: f64,
    pub decimals: u8,
    pub is_token_2022: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct OhlcvStatsSnapshot {
    pub total_tokens: usize,
    pub critical_tokens: usize,
    pub high_tokens: usize,
    pub medium_tokens: usize,
    pub low_tokens: usize,
    pub cache_hit_rate: f64,
    pub api_calls_per_minute: f64,
    pub queue_size: usize,
    pub telemetry: OhlcvTelemetrySnapshot,
    pub backfills_in_progress: usize,
    pub open_gap_tokens: usize,
    pub open_gap_total: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub top_open_gaps: Vec<OhlcvGapSummarySnapshot>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct OhlcvTelemetrySnapshot {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_cycle_started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_cycle_completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_cycle_duration_ms: Option<u64>,
    pub monitor_cycle_tokens_processed: usize,
    pub monitor_cycle_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_cycle_started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_cycle_completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_cycle_duration_ms: Option<u64>,
    pub gap_cycle_tokens_processed: usize,
    pub gap_cycle_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_rate_limit_at: Option<DateTime<Utc>>,
    pub rate_limit_events: u64,
    pub total_backfills_scheduled: u64,
    pub total_backfills_completed: u64,
    pub total_backfills_failed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backfill_started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backfill_completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backfill_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backfill_points: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backfill_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OhlcvGapSummarySnapshot {
    pub mint: String,
    pub open_gaps: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub largest_gap_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_gap_end: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolServiceStatusSnapshot {
    pub running: bool,
    pub system_ready: bool,
    pub single_pool_mode: bool,
    pub monitored_tokens: usize,
    pub monitored_capacity: usize,
    pub price_subscribers: usize,
    pub cache: PoolCacheSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyzer: Option<PoolAnalyzerSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetcher: Option<PoolFetcherSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery: Option<PoolDiscoverySnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolCacheSnapshot {
    pub total_prices: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolAnalyzerSnapshot {
    pub total_pools: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub program_distribution: Vec<PoolProgramCount>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolProgramCount {
    pub program: String,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolFetcherSnapshot {
    pub total_bundles: usize,
    pub bundles_with_data: usize,
    pub total_accounts_tracked: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct PoolDiscoverySnapshot {
    pub sources_enabled: Vec<String>,
    pub debug_override_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_override_count: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenDiscoveryStatusSnapshot {
    pub running: bool,
    pub total_cycles: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cycle_started: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_cycle_completed: Option<String>,
    pub last_processed: usize,
    pub last_added: usize,
    pub last_deduplicated_removed: usize,
    pub last_blacklist_removed: usize,
    pub total_processed: u64,
    pub total_added: u64,
    pub sources: DiscoverySourceSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiscoverySourceSnapshot {
    pub profiles: usize,
    pub boosted: usize,
    pub top_boosts: usize,
    pub rug_new: usize,
    pub rug_viewed: usize,
    pub rug_trending: usize,
    pub rug_verified: usize,
    pub gecko_updated: usize,
    pub gecko_trending: usize,
    pub jupiter_tokens: usize,
    pub jupiter_top_organic: usize,
    pub jupiter_top_traded: usize,
    pub jupiter_top_trending: usize,
    pub coingecko_markets: usize,
    pub defillama_protocols: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DexscreenerStatusSnapshot {
    pub enabled: bool,
    pub initialized: bool,
    pub rate_limit_per_minute: usize,
    pub discovery_rate_limit_per_minute: usize,
    pub max_tokens_per_call: usize,
    pub token_cache_entries: usize,
    pub token_cache_fresh: usize,
    pub pool_cache_entries: usize,
    pub pool_cache_fresh: usize,
    pub price_cache_ttl_secs: i64,
    pub pool_cache_ttl_secs: i64,
    pub api_total_requests: u64,
    pub api_successful_requests: u64,
    pub api_failed_requests: u64,
    pub api_success_rate: f64,
    pub api_cache_hits: u64,
    pub api_cache_misses: u64,
    pub api_cache_hit_rate: f64,
    pub api_average_response_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_request_time: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GeckoTerminalStatusSnapshot {
    pub enabled: bool,
    pub initialized: bool,
    pub rate_limit_per_minute: usize,
    pub max_tokens_per_batch: usize,
    pub cache_entries: usize,
    pub cache_fresh: usize,
    pub cache_ttl_secs: i64,
    pub api_total_requests: u64,
    pub api_successful_requests: u64,
    pub api_failed_requests: u64,
    pub api_success_rate: f64,
    pub api_cache_hits: u64,
    pub api_cache_misses: u64,
    pub api_cache_hit_rate: f64,
    pub api_average_response_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_request_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_success_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_time: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_message: Option<String>,
    pub current_rate_limit_calls: usize,
    pub current_rate_limit_max: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_resets_in_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventsStatusSnapshot {
    pub running: bool,
    pub total_events: i64,
    pub events_24h: i64,
    pub db_size_bytes: i64,
    pub category_counts: HashMap<String, u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recent_events: Vec<EventSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventSnapshot {
    pub id: i64,
    pub event_time: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionsStatusSnapshot {
    pub running: bool,
    pub system_ready: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_signature_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newest_known_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_known_signature: Option<String>,
    pub stats: crate::transactions::TransactionStats,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub queue: TransactionQueueSnapshot,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<TransactionDatabaseSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<TransactionBootstrapSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionQueueSnapshot {
    pub pending_local: u64,
    pub pending_global: u64,
    pub deferred_retries: u64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<TransactionPendingSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oldest_age_seconds: Option<i64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionPendingSnapshot {
    pub signature: String,
    pub age_seconds: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionDatabaseSnapshot {
    pub raw_transactions: u64,
    pub processed_transactions: u64,
    pub known_signatures: u64,
    pub pending_records: u64,
    pub deferred_retry_records: u64,
    pub size_bytes: u64,
    pub schema_version: u32,
    pub last_updated: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionBootstrapSnapshot {
    pub full_history_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backfill_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct RpcStatsSnapshot {
    pub total_calls: u64,
    pub total_errors: u64,
    pub success_rate: f32,
    pub calls_per_second: f64,
    pub average_response_time_ms: f64,
    pub calls_per_url: HashMap<String, u64>,
    pub errors_per_url: HashMap<String, u64>,
    pub calls_per_method: HashMap<String, u64>,
    pub errors_per_method: HashMap<String, u64>,
    pub uptime_seconds: i64,
    pub session_id: String,
    pub session_started_at: DateTime<Utc>,
    pub recent_calls_per_minute: f64,
    pub minute_buckets: Vec<RpcMinuteBucket>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_session: Option<RpcSessionSnapshot>,
}
