use chrono::{DateTime, Duration as ChronoDuration, Utc};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use futures::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
/// Wallet Balance Monitoring Module
///
/// This module provides wallet balance monitoring with historical snapshots stored in SQLite database.
/// It monitors both SOL balance and token balances for the configured wallet address.
///
/// Features:
/// - Background service that checks wallet balance every minute
/// - Delayed RPC calls to avoid overwhelming the global RPC client
/// - Historical snapshots stored in data/wallet.db
/// - Tracks both SOL and token balances
/// - Integration with existing RPC infrastructure
/// - Pure wallet monitoring without position management interference
use std::path::Path;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, RwLock};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::nfts::fetch_nft_metadata_batch;
use crate::rpc::{get_rpc_client, RpcClientMethods, TokenAccountInfo};
// Use tokens::store accessors directly when needed
use crate::transactions::get_transaction_database;
use crate::utils::get_wallet_address;

// Database schema version
const WALLET_SCHEMA_VERSION: u32 = 3;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_WALLET_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address TEXT NOT NULL,
    snapshot_time TEXT NOT NULL,
    sol_balance REAL NOT NULL,
    sol_balance_lamports INTEGER NOT NULL,
    total_tokens_count INTEGER NOT NULL DEFAULT 0,
    total_nfts_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_TOKEN_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 0,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_NFT_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS nft_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    account_address TEXT NOT NULL,
    name TEXT,
    symbol TEXT,
    image_url TEXT,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_WALLET_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Cache table for pre-aggregated SOL flows (one row per processed transaction)
const SCHEMA_SOL_FLOW_CACHE: &str = r#"
CREATE TABLE IF NOT EXISTS sol_flow_cache (
    signature TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    sol_delta REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_WALLET_DASHBOARD_METRICS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_dashboard_metrics (
    window_key TEXT PRIMARY KEY,
    window_hours INTEGER NOT NULL,
    snapshot_limit INTEGER NOT NULL,
    token_limit INTEGER NOT NULL,
    payload_blob BLOB NOT NULL,
    payload_format TEXT NOT NULL DEFAULT 'json-gzip',
    computed_at TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    computation_duration_ms INTEGER,
    snapshot_count INTEGER NOT NULL DEFAULT 0,
    flow_cache_rows INTEGER NOT NULL DEFAULT 0,
    last_processed_timestamp TEXT,
    last_processed_signature TEXT,
    window_start TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Indexes for fast range aggregation on cache
const FLOW_CACHE_INDEXES: &[&str] =
    &["CREATE INDEX IF NOT EXISTS idx_flow_cache_timestamp ON sol_flow_cache(timestamp DESC);"];

const DASHBOARD_METRICS_INDEXES: &[&str] = &["CREATE INDEX IF NOT EXISTS idx_dashboard_metrics_valid_until ON wallet_dashboard_metrics(valid_until DESC);"];

// Performance indexes
const WALLET_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_address ON wallet_snapshots(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_time ON wallet_snapshots(snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_id ON token_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON token_balances(mint);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_mint ON token_balances(snapshot_id, mint);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_snapshot_id ON nft_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_mint ON nft_balances(mint);",
];

const DEFAULT_PRECOMPUTED_SNAPSHOT_LIMIT: usize = 600;
const DEFAULT_PRECOMPUTED_TOKEN_LIMIT: usize = 250;
const MAX_API_CACHE_ENTRIES: usize = 128;
const CIRCUIT_BREAKER_THRESHOLD: u32 = 3;
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 300;
const TOKEN_METADATA_CONCURRENCY: usize = 20;

// =============================================================================
// DATA STRUCTURES
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
    pub token_balances: Vec<TokenBalance>,
    pub nft_balances: Vec<NftBalance>,
}

/// Token balance record (fungible tokens only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
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

#[derive(Default)]
struct WalletSnapshotStatusCache {
    ready: std::sync::atomic::AtomicBool,
    last_updated: StdMutex<Option<DateTime<Utc>>>,
}

impl WalletSnapshotStatusCache {
    fn mark_ready(&self, timestamp: DateTime<Utc>) {
        if let Ok(mut guard) = self.last_updated.lock() {
            *guard = Some(timestamp);
        }
        self.ready.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn reset(&self) {
        if let Ok(mut guard) = self.last_updated.lock() {
            *guard = None;
        }
        self.ready.store(false, std::sync::atomic::Ordering::SeqCst);
    }

    fn set(&self, timestamp: Option<DateTime<Utc>>) {
        if let Some(ts) = timestamp {
            self.mark_ready(ts);
        } else {
            self.reset();
        }
    }

    fn snapshot(&self) -> WalletSnapshotStatus {
        let ready = self.ready.load(std::sync::atomic::Ordering::SeqCst);
        let last_updated = self
            .last_updated
            .lock()
            .ok()
            .and_then(|guard| guard.clone());

        WalletSnapshotStatus {
            is_ready: ready && last_updated.is_some(),
            last_updated,
        }
    }
}

static WALLET_SNAPSHOT_STATUS: Lazy<WalletSnapshotStatusCache> =
    Lazy::new(WalletSnapshotStatusCache::default);

fn update_wallet_snapshot_status(timestamp: DateTime<Utc>) {
    WALLET_SNAPSHOT_STATUS.mark_ready(timestamp);
}

fn hydrate_wallet_snapshot_status(timestamp: Option<DateTime<Utc>>) {
    WALLET_SNAPSHOT_STATUS.set(timestamp);
}

pub fn get_cached_wallet_snapshot_status() -> WalletSnapshotStatus {
    WALLET_SNAPSHOT_STATUS.snapshot()
}

#[derive(Debug, Clone)]
struct CachedDashboardMetrics {
    window_key: String,
    window_hours: i64,
    snapshot_limit: usize,
    token_limit: usize,
    payload: Vec<u8>,
    payload_format: String,
    computed_at: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    computation_duration_ms: Option<i64>,
    snapshot_count: usize,
    flow_cache_rows: usize,
    last_processed_timestamp: Option<DateTime<Utc>>,
    last_processed_signature: Option<String>,
    window_start: Option<DateTime<Utc>>,
}

static API_RESPONSE_CACHE: Lazy<
    Arc<RwLock<HashMap<DashboardRequestKey, CachedDashboardResponse>>>,
> = Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

static CACHE_METRICS: Lazy<Arc<RwLock<CachePerformanceMetrics>>> =
    Lazy::new(|| Arc::new(RwLock::new(CachePerformanceMetrics::default())));

static COMPUTATION_FAILURES: Lazy<Arc<RwLock<HashMap<String, (u32, Instant)>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

fn compress_bytes(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder
        .write_all(raw)
        .map_err(|e| format!("Failed to write compressed payload: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finalize compression: {}", e))
}

fn decompress_bytes(raw: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(raw);
    let mut buffer = Vec::new();
    decoder
        .read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to decompress payload: {}", e))?;
    Ok(buffer)
}

fn serialize_dashboard_payload(payload: &WalletDashboardData) -> Result<Vec<u8>, String> {
    let mut sanitized = payload.clone();
    sanitized.cache_metadata = None;
    let json = serde_json::to_vec(&sanitized)
        .map_err(|e| format!("Failed to serialize dashboard payload: {}", e))?;
    compress_bytes(&json)
}

fn deserialize_dashboard_payload(raw: &[u8]) -> Result<WalletDashboardData, String> {
    let json_bytes = decompress_bytes(raw)?;
    serde_json::from_slice::<WalletDashboardData>(&json_bytes)
        .map_err(|e| format!("Failed to deserialize dashboard payload: {}", e))
}

fn canonical_window(window_hours: i64) -> Option<(&'static str, i64)> {
    match window_hours {
        24 => Some(("24h", 24)),
        168 => Some(("7d", 168)),
        720 => Some(("30d", 720)),
        0 => Some(("all_time", 0)),
        _ => None,
    }
}

fn ttl_for_window(window_key: &str) -> u64 {
    with_config(|cfg| match window_key {
        "24h" => cfg.wallet.dashboard_metrics_24h_interval_secs,
        "7d" => cfg.wallet.dashboard_metrics_7d_interval_secs,
        "30d" => cfg.wallet.dashboard_metrics_30d_interval_secs,
        "all_time" => cfg.wallet.dashboard_metrics_alltime_interval_secs,
        _ => cfg.wallet.dashboard_metrics_24h_interval_secs,
    })
}

async fn record_cache_metrics(source: DashboardDataSource, latency_ms: u128, stale: bool) {
    let mut guard = CACHE_METRICS.write().await;
    guard.total_requests = guard.total_requests.saturating_add(1);
    guard.total_latency_ms = guard.total_latency_ms.saturating_add(latency_ms);
    guard.last_source = Some(source.clone());

    match source {
        DashboardDataSource::Memory => guard.memory_hits = guard.memory_hits.saturating_add(1),
        DashboardDataSource::Database => {
            guard.database_hits = guard.database_hits.saturating_add(1)
        }
        DashboardDataSource::Realtime => {
            guard.realtime_computations = guard.realtime_computations.saturating_add(1)
        }
    }

    if stale {
        guard.stale_responses = guard.stale_responses.saturating_add(1);
    }
}

async fn circuit_should_skip(window_key: &str) -> bool {
    let guard = COMPUTATION_FAILURES.read().await;
    if let Some((count, last_failure)) = guard.get(window_key) {
        if *count >= CIRCUIT_BREAKER_THRESHOLD
            && last_failure.elapsed().as_secs() < CIRCUIT_BREAKER_COOLDOWN_SECS
        {
            return true;
        }
    }
    false
}

async fn circuit_record_failure(window_key: &str) {
    let mut guard = COMPUTATION_FAILURES.write().await;
    let entry = guard
        .entry(window_key.to_string())
        .or_insert((0, Instant::now()));
    entry.0 = entry.0.saturating_add(1);
    entry.1 = Instant::now();
}

async fn circuit_reset(window_key: &str) {
    let mut guard = COMPUTATION_FAILURES.write().await;
    guard.remove(window_key);
}

async fn compute_and_cache_metrics_internal(window_key: &'static str, window_hours: i64) {
    if get_transaction_database().await.is_none() {
        logger::debug(
            LogTag::Wallet,
            &format!(
                "Skipping {} recompute → transactions database not ready",
                window_key
            ),
        );
        return;
    }

    if circuit_should_skip(window_key).await {
        logger::info(
            LogTag::Wallet,
            &format!(
                "Circuit breaker active for {} → skipping cache recomputation",
                window_key
            ),
        );
        return;
    }

    match compute_and_cache_metrics(window_key, window_hours).await {
        Ok(_) => {
            circuit_reset(window_key).await;
        }
        Err(err) => {
            circuit_record_failure(window_key).await;
            logger::error(
                LogTag::Wallet,
                &format!(
                    "Failed to compute dashboard metrics for {}: {}",
                    window_key, err
                ),
            );
        }
    }
}

async fn compute_and_cache_metrics(
    window_key: &'static str,
    window_hours: i64,
) -> Result<(), String> {
    let start_time = Instant::now();

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Computing dashboard metrics for {} ({}h)",
            window_key, window_hours
        ),
    );

    let snapshot_limit = DEFAULT_PRECOMPUTED_SNAPSHOT_LIMIT;
    let token_limit = DEFAULT_PRECOMPUTED_TOKEN_LIMIT;

    let mut payload =
        compute_dashboard_payload_realtime(window_hours, snapshot_limit, token_limit).await?;

    let computed_at = Utc::now();
    let ttl_secs = ttl_for_window(window_key).max(5);
    let valid_until = computed_at + ChronoDuration::seconds(ttl_secs as i64);
    let duration_ms = start_time.elapsed().as_millis() as i64;

    let payload_blob = serialize_dashboard_payload(&payload)?;

    let cached_entry = CachedDashboardMetrics {
        window_key: window_key.to_string(),
        window_hours,
        snapshot_limit,
        token_limit,
        payload: payload_blob,
        payload_format: "json-gzip".to_string(),
        computed_at,
        valid_until,
        computation_duration_ms: Some(duration_ms),
        snapshot_count: payload.balance_trend.len(),
        flow_cache_rows: payload.flows.transactions_analyzed,
        last_processed_timestamp: None,
        last_processed_signature: None,
        window_start: if window_hours > 0 {
            Some(computed_at - ChronoDuration::hours(window_hours))
        } else {
            None
        },
    };

    {
        let db_guard = GLOBAL_WALLET_DB.lock().await;
        let db = db_guard
            .as_ref()
            .ok_or_else(|| "Wallet database not initialized".to_string())?;
        db.upsert_dashboard_metrics(&cached_entry)?;
    }

    let metadata = DashboardCacheMetadata {
        window_key: Some(window_key.to_string()),
        cached_at: Some(computed_at.to_rfc3339()),
        valid_until: Some(valid_until.to_rfc3339()),
        age_seconds: Some(0),
        next_update_in_seconds: Some(ttl_secs),
        freshness: DashboardCacheFreshness::Fresh,
        source: DashboardDataSource::Database,
        computation_duration_ms: Some(duration_ms as u64),
        snapshot_count: Some(cached_entry.snapshot_count),
    };

    payload.cache_metadata = Some(metadata.clone());

    let request_key = DashboardRequestKey {
        window_hours,
        snapshot_limit,
        max_tokens: token_limit,
    };

    {
        let mut cache_guard = API_RESPONSE_CACHE.write().await;
        cache_guard.insert(
            request_key,
            CachedDashboardResponse {
                data: payload.clone(),
                cached_at: Instant::now(),
            },
        );
        if cache_guard.len() > MAX_API_CACHE_ENTRIES {
            let cache_ttl_secs = with_config(|cfg| cfg.wallet.api_response_cache_ttl_secs.max(5));
            let cutoff = Instant::now() - Duration::from_secs(cache_ttl_secs.saturating_mul(2));
            cache_guard.retain(|_, entry| entry.cached_at > cutoff);
        }
    }

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Cached {} metrics: net={:.6} SOL, txs={}, computed_in={}ms, ttl={}s",
            window_key,
            payload.flows.net_sol,
            payload.flows.transactions_analyzed,
            duration_ms,
            ttl_secs
        ),
    );

    Ok(())
}

async fn warmup_dashboard_metrics() {
    logger::debug(
        LogTag::Wallet,
        "Precomputing wallet dashboard metrics during startup",
    );

    if get_transaction_database().await.is_none() {
        logger::debug(
            LogTag::Wallet,
            "Skipping dashboard warm-up → transactions database not ready",
        );
        return;
    }

    let windows = [("24h", 24_i64), ("7d", 168), ("30d", 720), ("all_time", 0)];
    for (key, hours) in windows {
        compute_and_cache_metrics_internal(key, hours).await;
    }

    logger::info(LogTag::Wallet, "Wallet dashboard metrics warm-up complete");
}

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

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct DashboardRequestKey {
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
}

#[derive(Debug, Clone)]
struct CachedDashboardResponse {
    data: WalletDashboardData,
    cached_at: Instant,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct CachePerformanceMetrics {
    total_requests: u64,
    memory_hits: u64,
    database_hits: u64,
    realtime_computations: u64,
    stale_responses: u64,
    total_latency_ms: u128,
    last_source: Option<DashboardDataSource>,
}

fn clamp_window_hours(window_hours: i64) -> i64 {
    // 0 = All Time (no filter)
    // Otherwise clamp to reasonable range (1 hour to 2 years)
    if window_hours == 0 {
        0
    } else {
        window_hours.clamp(1, 24 * 365 * 2)
    }
}

fn clamp_snapshot_limit(limit: usize) -> usize {
    limit.clamp(16, 2880)
}

fn clamp_token_limit(limit: usize) -> usize {
    limit.clamp(10, 1000)
}

fn calc_change_percent(current: f64, previous: f64) -> Option<f64> {
    if previous.abs() < f64::EPSILON {
        None
    } else {
        Some((current - previous) / previous * 100.0)
    }
}

fn short_mint_label(mint: &str) -> String {
    if mint.len() <= 4 {
        mint.to_string()
    } else {
        format!("{}…", &mint[..4])
    }
}

async fn compute_flow_metrics(window_hours: i64) -> Result<WalletFlowMetrics, String> {
    logger::debug(
        LogTag::Wallet,
        &format!("Computing flow metrics for window: {} hours", window_hours),
    );

    // All-time mode when window_hours <= 0
    if window_hours <= 0 {
        if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
            if let Ok(Some(min_ts)) = db.get_flow_cache_min_ts_sync() {
                if let Ok((inflow, outflow, tx_count)) =
                    db.aggregate_cached_flows_sync(min_ts, None)
                {
                    if tx_count > 0 {
                        logger::debug(
                            LogTag::Wallet,
                            &format!(
                                "All-time cached: inflow={:.6}, outflow={:.6}, txs={}",
                                inflow, outflow, tx_count
                            ),
                        );
                        return Ok(WalletFlowMetrics {
                            window_hours: 0,
                            inflow_sol: inflow,
                            outflow_sol: outflow,
                            net_sol: inflow - outflow,
                            transactions_analyzed: tx_count,
                        });
                    }
                }
            }
        }
        // Fallback to full aggregation from transactions DB (from epoch)
        let tx_db = get_transaction_database()
            .await
            .ok_or_else(|| "Transaction database not initialized".to_string())?;
        let epoch = DateTime::<Utc>::from(std::time::UNIX_EPOCH);
        let (inflow, outflow, tx_count) = tx_db
            .aggregate_sol_flows_since(epoch, None)
            .await
            .map_err(|e| format!("Failed to aggregate all-time SOL flows: {}", e))?;
        logger::debug(
            LogTag::Wallet,
            &format!(
                "All-time DB: inflow={:.6}, outflow={:.6}, txs={}",
                inflow, outflow, tx_count
            ),
        );
        return Ok(WalletFlowMetrics {
            window_hours: 0,
            inflow_sol: inflow,
            outflow_sol: outflow,
            net_sol: inflow - outflow,
            transactions_analyzed: tx_count,
        });
    }

    let window_hours = clamp_window_hours(window_hours);
    let window_start = Utc::now() - ChronoDuration::hours(window_hours);

    logger::debug(
        LogTag::Wallet,
        &format!("Window start: {}", window_start.to_rfc3339()),
    );

    // Try cached aggregation first
    if let Some(db) = GLOBAL_WALLET_DB.lock().await.as_ref() {
        match db.aggregate_cached_flows_sync(window_start, None) {
            Ok((inflow, outflow, tx_count)) => {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "Cached: inflow={:.6}, outflow={:.6}, txs={}",
                        inflow, outflow, tx_count
                    ),
                );
                if tx_count > 0 {
                    return Ok(WalletFlowMetrics {
                        window_hours,
                        inflow_sol: inflow,
                        outflow_sol: outflow,
                        net_sol: inflow - outflow,
                        transactions_analyzed: tx_count,
                    });
                }
            }
            Err(e) => {
                logger::debug(LogTag::Wallet, &format!("Cache aggregation failed: {}", e));
            }
        }
    }

    // Fallback to live aggregation from transactions DB
    logger::debug(
        LogTag::Wallet,
        "Using live aggregation from transactions DB",
    );

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_string())?;
    let (inflow, outflow, tx_count) = tx_db
        .aggregate_sol_flows_since(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate SOL flows: {}", e))?;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "DB aggregation: inflow={:.6}, outflow={:.6}, txs={}",
            inflow, outflow, tx_count
        ),
    );

    Ok(WalletFlowMetrics {
        window_hours,
        inflow_sol: inflow,
        outflow_sol: outflow,
        net_sol: inflow - outflow,
        transactions_analyzed: tx_count,
    })
}

async fn compute_daily_flows(window_hours: i64) -> Result<Vec<DailyFlowPoint>, String> {
    use chrono::NaiveDate;

    let window_hours = clamp_window_hours(window_hours);
    let (window_start, is_all_time) = if window_hours == 0 {
        (DateTime::<Utc>::from(std::time::UNIX_EPOCH), true)
    } else {
        (Utc::now() - ChronoDuration::hours(window_hours), false)
    };

    let tx_db = get_transaction_database()
        .await
        .ok_or_else(|| "Transaction database not initialized".to_string())?;

    let daily_data = tx_db
        .aggregate_daily_flows(window_start, None)
        .await
        .map_err(|e| format!("Failed to aggregate daily flows: {}", e))?;

    // Convert to DailyFlowPoint with timestamps
    let mut result: Vec<DailyFlowPoint> = daily_data
        .into_iter()
        .filter_map(|(date_str, inflow, outflow, tx_count)| {
            // Parse date string and convert to timestamp
            NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
                .map(|naive_dt| DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc))
                .map(|dt| DailyFlowPoint {
                    date: date_str,
                    timestamp: dt.timestamp(),
                    inflow,
                    outflow,
                    net: inflow - outflow,
                    tx_count,
                })
        })
        .collect();

    // Apply payload cap/decimation for very long ranges to avoid huge responses
    let (max_days, decimate_threshold_days) = with_config(|cfg| {
        (
            cfg.wallet.max_daily_flow_days,
            cfg.wallet.daily_flow_decimate_threshold_days,
        )
    });

    if result.len() > max_days {
        // Keep most recent max_days points
        result.sort_by_key(|p| p.timestamp);
        result = result.split_off(result.len() - max_days);
    }

    if result.len() > decimate_threshold_days {
        // Decimate older half to every Nth point while keeping recent quarter dense
        let len = result.len();
        let recent_keep = len / 4; // keep last quarter in full resolution
        let (older, recent) = result.split_at(len - recent_keep);
        // Choose stride to reduce older to about half of decimate_threshold_days
        let target_older = decimate_threshold_days - recent_keep.min(decimate_threshold_days / 2);
        let stride = ((older.len() as f64) / (target_older as f64))
            .ceil()
            .max(1.0) as usize;
        let decimated_older: Vec<DailyFlowPoint> = older
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                if i % stride == 0 {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect();
        let mut merged = decimated_older;
        merged.extend_from_slice(recent);
        result = merged;
    }

    logger::debug(
        LogTag::Wallet,
        &format!("Computed {} daily flow points", result.len()),
    );

    Ok(result)
}

async fn fetch_token_metadata_batch(
    mints: &[String],
) -> HashMap<String, crate::tokens::types::Token> {
    if mints.is_empty() {
        return HashMap::new();
    }

    stream::iter(mints.iter().cloned())
        .map(|mint| async move {
            match crate::tokens::get_full_token_async(&mint).await {
                Ok(Some(token)) => Some((mint, token)),
                Ok(None) => None,
                Err(err) => {
                    logger::debug(
                        LogTag::Wallet,
                        &format!("Failed to load token metadata for {}: {}", mint, err),
                    );
                    None
                }
            }
        })
        .buffer_unordered(TOKEN_METADATA_CONCURRENCY)
        .filter_map(|entry| async move { entry })
        .collect()
        .await
}

async fn enrich_token_overview(
    balances: Vec<TokenBalance>,
    max_tokens: usize,
) -> Vec<WalletTokenOverview> {
    let mut rows = Vec::with_capacity(balances.len());

    let mut unique_mints: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for balance in &balances {
        if seen.insert(balance.mint.clone()) {
            unique_mints.push(balance.mint.clone());
        }
    }

    let metadata_map: HashMap<String, crate::tokens::types::Token> =
        fetch_token_metadata_batch(&unique_mints).await;

    for balance in balances {
        let token_meta = metadata_map.get(&balance.mint);

        let (
            symbol,
            name,
            image_url,
            price_sol,
            price_usd,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        ) = if let Some(meta) = token_meta {
            let price_sol = if meta.price_sol > 0.0 {
                Some(meta.price_sol)
            } else {
                None
            };
            let price_usd = if meta.price_usd > 0.0 {
                Some(meta.price_usd)
            } else {
                None
            };
            let liquidity_usd = meta.liquidity_usd;
            let volume_24h = meta.volume_h24;
            let last_updated = Some(meta.market_data_last_fetched_at.to_rfc3339());
            let dex_id = Some(meta.data_source.as_str().to_string());

            let symbol = if meta.symbol.trim().is_empty() {
                short_mint_label(&balance.mint)
            } else {
                meta.symbol.clone()
            };

            (
                symbol,
                Some(meta.name.clone()),
                meta.image_url.clone(),
                price_sol,
                price_usd,
                liquidity_usd,
                volume_24h,
                last_updated,
                dex_id,
            )
        } else {
            (
                short_mint_label(&balance.mint),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
        };

        let value_sol = price_sol.map(|price| price * balance.balance_ui);

        rows.push(WalletTokenOverview {
            mint: balance.mint.clone(),
            symbol,
            name,
            image_url,
            balance_ui: balance.balance_ui,
            balance_raw: balance.balance,
            decimals: balance.decimals,
            is_token_2022: balance.is_token_2022,
            price_sol,
            price_usd,
            value_sol,
            liquidity_usd,
            volume_24h,
            last_updated,
            dex_id,
        });
    }

    rows.sort_by(|a, b| {
        let a_key = a.value_sol.unwrap_or(a.balance_ui);
        let b_key = b.value_sol.unwrap_or(b.balance_ui);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_tokens = clamp_token_limit(max_tokens);
    if rows.len() > max_tokens {
        rows.truncate(max_tokens);
    }

    rows
}

async fn compute_dashboard_payload_realtime(
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
) -> Result<WalletDashboardData, String> {
    let window_hours = clamp_window_hours(window_hours);
    let snapshot_limit = clamp_snapshot_limit(snapshot_limit);

    let mut snapshots = match get_recent_wallet_snapshots(snapshot_limit).await {
        Ok(snaps) => snaps,
        Err(err) => {
            if err.contains("not initialized") {
                if let Err(init_err) = initialize_wallet_database().await {
                    return Err(format!("Wallet database unavailable: {}", init_err));
                }

                match get_recent_wallet_snapshots(snapshot_limit).await {
                    Ok(snaps) => snaps,
                    Err(retry_err) => {
                        if retry_err.contains("not initialized") {
                            Vec::new()
                        } else {
                            return Err(retry_err);
                        }
                    }
                }
            } else {
                return Err(err);
            }
        }
    };
    if snapshots.is_empty() {
        let flows = compute_flow_metrics(window_hours).await?;
        let daily_flows = compute_daily_flows(window_hours)
            .await
            .unwrap_or_else(|_| Vec::new());
        return Ok(WalletDashboardData {
            summary: WalletSummarySnapshot {
                window_hours,
                current_sol_balance: 0.0,
                previous_sol_balance: None,
                sol_change: 0.0,
                sol_change_percent: None,
                token_count: 0,
                last_snapshot_time: None,
            },
            flows,
            balance_trend: Vec::new(),
            daily_flows,
            tokens: Vec::new(),
            nfts: Vec::new(),
            last_updated: None,
            cache_metadata: None,
        });
    }

    snapshots.sort_by(|a, b| a.snapshot_time.cmp(&b.snapshot_time));

    let latest_snapshot = snapshots
        .last()
        .cloned()
        .ok_or_else(|| "Latest snapshot unavailable".to_string())?;
    // Determine window_start for trend; for all-time (0), include all loaded snapshots
    let window_start = if window_hours == 0 {
        snapshots
            .first()
            .map(|s| s.snapshot_time)
            .unwrap_or(latest_snapshot.snapshot_time)
    } else {
        Utc::now() - ChronoDuration::hours(window_hours)
    };

    let baseline_snapshot = snapshots
        .iter()
        .find(|snap| snap.snapshot_time >= window_start)
        .or_else(|| snapshots.first())
        .cloned();

    let previous_sol_balance = baseline_snapshot.as_ref().map(|snap| snap.sol_balance);
    let sol_change =
        latest_snapshot.sol_balance - previous_sol_balance.unwrap_or(latest_snapshot.sol_balance);
    let sol_change_percent = previous_sol_balance
        .and_then(|prev| calc_change_percent(latest_snapshot.sol_balance, prev));

    let mut trend: Vec<WalletBalancePoint> = snapshots
        .iter()
        .filter(|snap| snap.snapshot_time >= window_start)
        .map(|snap| WalletBalancePoint {
            timestamp: snap.snapshot_time.timestamp(),
            sol_balance: snap.sol_balance,
        })
        .collect();

    if trend.is_empty() {
        trend.push(WalletBalancePoint {
            timestamp: latest_snapshot.snapshot_time.timestamp(),
            sol_balance: latest_snapshot.sol_balance,
        });
    }

    let mut tokens = Vec::new();
    let mut nfts = Vec::new();
    if let Some(snapshot_id) = latest_snapshot.id {
        let balances = get_snapshot_token_balances(snapshot_id).await?;
        tokens = enrich_token_overview(balances, max_tokens).await;

        // Get NFT balances
        let nft_balances = get_snapshot_nft_balances(snapshot_id)
            .await
            .unwrap_or_default();
        nfts = nft_balances
            .into_iter()
            .map(|nft| WalletNftOverview {
                mint: nft.mint,
                account_address: nft.account_address,
                name: nft.name,
                symbol: nft.symbol,
                image_url: nft.image_url,
                is_token_2022: nft.is_token_2022,
            })
            .collect();
    }

    let flows = compute_flow_metrics(window_hours).await?;

    // Compute daily flows for chart
    let daily_flows = compute_daily_flows(window_hours).await.unwrap_or_else(|e| {
        logger::warning(
            LogTag::Wallet,
            &format!("Failed to compute daily flows: {}", e),
        );
        Vec::new()
    });

    let summary = WalletSummarySnapshot {
        window_hours,
        current_sol_balance: latest_snapshot.sol_balance,
        previous_sol_balance,
        sol_change,
        sol_change_percent,
        token_count: latest_snapshot.total_tokens_count,
        last_snapshot_time: Some(latest_snapshot.snapshot_time.to_rfc3339()),
    };

    Ok(WalletDashboardData {
        summary,
        flows,
        balance_trend: trend,
        daily_flows,
        tokens,
        nfts,
        last_updated: Some(latest_snapshot.snapshot_time.to_rfc3339()),
        cache_metadata: None,
    })
}

pub async fn get_wallet_dashboard_data(
    window_hours: i64,
    snapshot_limit: usize,
    max_tokens: usize,
) -> Result<WalletDashboardData, String> {
    let clamped_window = clamp_window_hours(window_hours);
    let clamped_snapshot_limit = clamp_snapshot_limit(snapshot_limit);
    let clamped_token_limit = clamp_token_limit(max_tokens);

    let cache_ttl_secs = with_config(|cfg| cfg.wallet.api_response_cache_ttl_secs.max(5));
    let request_key = DashboardRequestKey {
        window_hours: clamped_window,
        snapshot_limit: clamped_snapshot_limit,
        max_tokens: clamped_token_limit,
    };

    let start = Instant::now();

    // Memory cache layer
    {
        let cache_guard = API_RESPONSE_CACHE.read().await;
        if let Some(entry) = cache_guard.get(&request_key) {
            if entry.cached_at.elapsed().as_secs() < cache_ttl_secs {
                let payload = entry.data.clone();
                let stale = payload
                    .cache_metadata
                    .as_ref()
                    .map(|meta| matches!(meta.freshness, DashboardCacheFreshness::Stale))
                    .unwrap_or(false);
                record_cache_metrics(
                    DashboardDataSource::Memory,
                    start.elapsed().as_millis(),
                    stale,
                )
                .await;
                return Ok(payload);
            }
        }
    }

    // Database cache layer
    if let Some((window_key, _canonical_hours)) = canonical_window(clamped_window) {
        let metrics = {
            let db_guard = GLOBAL_WALLET_DB.lock().await;
            match db_guard.as_ref() {
                Some(db) => db.get_dashboard_metrics(window_key)?,
                None => None,
            }
        };

        if let Some(metrics) = metrics {
            let covers_snapshots = metrics.snapshot_limit >= clamped_snapshot_limit;
            let covers_tokens = metrics.token_limit >= clamped_token_limit;
            let ttl_secs = ttl_for_window(window_key).max(5);
            let now = Utc::now();
            let valid = metrics.valid_until >= now;

            if covers_snapshots && covers_tokens {
                match deserialize_dashboard_payload(&metrics.payload) {
                    Ok(mut payload) => {
                        if payload.balance_trend.len() > clamped_snapshot_limit {
                            let start_index = payload.balance_trend.len() - clamped_snapshot_limit;
                            payload.balance_trend = payload
                                .balance_trend
                                .into_iter()
                                .skip(start_index)
                                .collect();
                        }
                        if payload.tokens.len() > clamped_token_limit {
                            payload.tokens.truncate(clamped_token_limit);
                        }

                        let age_secs = now
                            .signed_duration_since(metrics.computed_at)
                            .num_seconds()
                            .max(0) as u64;
                        let next_update = if metrics.valid_until > now {
                            Some((metrics.valid_until - now).num_seconds() as u64)
                        } else {
                            Some(0)
                        };

                        let freshness = if !valid {
                            DashboardCacheFreshness::Stale
                        } else if age_secs <= ttl_secs / 2 {
                            DashboardCacheFreshness::Fresh
                        } else {
                            DashboardCacheFreshness::Aging
                        };

                        let metadata = DashboardCacheMetadata {
                            window_key: Some(metrics.window_key.clone()),
                            cached_at: Some(metrics.computed_at.to_rfc3339()),
                            valid_until: Some(metrics.valid_until.to_rfc3339()),
                            age_seconds: Some(age_secs),
                            next_update_in_seconds: next_update,
                            freshness: freshness.clone(),
                            source: DashboardDataSource::Database,
                            computation_duration_ms: metrics
                                .computation_duration_ms
                                .map(|value| value as u64),
                            snapshot_count: Some(metrics.snapshot_count),
                        };
                        payload.cache_metadata = Some(metadata.clone());

                        if valid {
                            {
                                let mut cache_guard = API_RESPONSE_CACHE.write().await;
                                cache_guard.insert(
                                    request_key.clone(),
                                    CachedDashboardResponse {
                                        data: payload.clone(),
                                        cached_at: Instant::now(),
                                    },
                                );
                                if cache_guard.len() > MAX_API_CACHE_ENTRIES {
                                    let cutoff = Instant::now()
                                        - Duration::from_secs(cache_ttl_secs.saturating_mul(2));
                                    cache_guard.retain(|_, entry| entry.cached_at > cutoff);
                                }
                            }

                            record_cache_metrics(
                                DashboardDataSource::Database,
                                start.elapsed().as_millis(),
                                matches!(freshness, DashboardCacheFreshness::Stale),
                            )
                            .await;
                            return Ok(payload);
                        } else {
                            logger::debug(
                                LogTag::Wallet,
                                &format!(
                                    "Discarding stale dashboard cache for {} (age={}s, ttl={}s)",
                                    window_key, age_secs, ttl_secs
                                ),
                            );
                        }
                    }
                    Err(err) => {
                        logger::warning(
                            LogTag::Wallet,
                            &format!(
                                "Failed to deserialize dashboard cache for {}: {}",
                                window_key, err
                            ),
                        );
                    }
                }
            } else {
                logger::debug(
                    LogTag::Wallet,
                    &format!(
                        "Cache entry {} does not cover requested limits (snapshots={} tokens={})",
                        window_key, metrics.snapshot_limit, metrics.token_limit
                    ),
                );
            }

            if !valid {
                let cached_window_key = metrics.window_key.clone();
                let cached_window_hours = metrics.window_hours;
                tokio::spawn(async move {
                    if let Some((canonical_key, canonical_hours)) =
                        canonical_window(cached_window_hours)
                    {
                        if canonical_key == cached_window_key {
                            compute_and_cache_metrics_internal(canonical_key, canonical_hours)
                                .await;
                        }
                    }
                });
            }
        }
    }

    // Real-time computation fallback
    let mut payload = compute_dashboard_payload_realtime(
        clamped_window,
        clamped_snapshot_limit,
        clamped_token_limit,
    )
    .await?;

    let latency = start.elapsed().as_millis();
    let now = Utc::now();
    payload.cache_metadata = Some(DashboardCacheMetadata {
        window_key: canonical_window(clamped_window).map(|(key, _)| key.to_string()),
        cached_at: Some(now.to_rfc3339()),
        valid_until: None,
        age_seconds: Some(0),
        next_update_in_seconds: None,
        freshness: DashboardCacheFreshness::Realtime,
        source: DashboardDataSource::Realtime,
        computation_duration_ms: Some(latency as u64),
        snapshot_count: Some(payload.balance_trend.len()),
    });

    {
        let mut cache_guard = API_RESPONSE_CACHE.write().await;
        cache_guard.insert(
            request_key,
            CachedDashboardResponse {
                data: payload.clone(),
                cached_at: Instant::now(),
            },
        );
        if cache_guard.len() > MAX_API_CACHE_ENTRIES {
            let cutoff = Instant::now() - Duration::from_secs(cache_ttl_secs.saturating_mul(2));
            cache_guard.retain(|_, entry| entry.cached_at > cutoff);
        }
    }

    record_cache_metrics(DashboardDataSource::Realtime, latency, false).await;

    Ok(payload)
}

// =============================================================================
// WALLET DATABASE MANAGER
// =============================================================================

/// Database manager for wallet balance monitoring
pub struct WalletDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}

impl WalletDatabase {
    /// Create new WalletDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        let database_path = crate::paths::get_wallet_db_path();
        let database_path_str = database_path.to_string_lossy().to_string();

        logger::debug(
            LogTag::Wallet,
            &format!("Initializing wallet database at: {}", database_path_str),
        );

        // Configure connection manager
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(3) // Small pool for wallet monitoring
            .min_idle(Some(1))
            .build(manager)
            .map_err(|e| format!("Failed to create wallet connection pool: {}", e))?;

        let mut db = WalletDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: WALLET_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema().await?;

        logger::debug(LogTag::Wallet, "Wallet database initialized successfully");
        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Configure database settings
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;
        conn.busy_timeout(Duration::from_millis(30000))
            .map_err(|e| format!("Failed to set busy_timeout: {}", e))?;
        conn.pragma_update(None, "cache_size", &10000i64)
            .map_err(|e| format!("Failed to set cache_size: {}", e))?;
        conn.pragma_update(None, "temp_store", &"MEMORY")
            .map_err(|e| format!("Failed to set temp_store: {}", e))?;
        conn.pragma_update(None, "mmap_size", &30000000000i64)
            .map_err(|e| format!("Failed to set mmap_size: {}", e))?;

        // Create all tables
        conn.execute(SCHEMA_WALLET_SNAPSHOTS, [])
            .map_err(|e| format!("Failed to create wallet_snapshots table: {}", e))?;

        conn.execute(SCHEMA_TOKEN_BALANCES, [])
            .map_err(|e| format!("Failed to create token_balances table: {}", e))?;

        conn.execute(SCHEMA_NFT_BALANCES, [])
            .map_err(|e| format!("Failed to create nft_balances table: {}", e))?;

        conn.execute(SCHEMA_WALLET_METADATA, [])
            .map_err(|e| format!("Failed to create wallet_metadata table: {}", e))?;

        // Flow cache tables
        conn.execute(SCHEMA_SOL_FLOW_CACHE, [])
            .map_err(|e| format!("Failed to create sol_flow_cache table: {}", e))?;

        conn.execute(SCHEMA_WALLET_DASHBOARD_METRICS, [])
            .map_err(|e| format!("Failed to create wallet_dashboard_metrics table: {}", e))?;

        // Migrate existing schema if needed (add missing columns)
        conn.execute(
            "ALTER TABLE wallet_snapshots ADD COLUMN total_nfts_count INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .ok(); // Ignore error if column already exists

        // Create all indexes
        for index_sql in WALLET_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create wallet index: {}", e))?;
        }
        for index_sql in FLOW_CACHE_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create flow cache index: {}", e))?;
        }

        for index_sql in DASHBOARD_METRICS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create dashboard metrics index: {}", e))?;
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set wallet schema version: {}", e))?;

        // Store current wallet address in metadata
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {}", e))?;
        conn.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('current_wallet', ?1)",
            params![wallet_address],
        )
        .map_err(|e| format!("Failed to set current_wallet in metadata: {}", e))?;

        logger::debug(
            LogTag::Wallet,
            "Wallet database schema initialized with all tables and indexes",
        );

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get wallet database connection: {}", e))
    }

    /// Aggregate pre-cached SOL flows for a given time window
    pub fn aggregate_cached_flows_sync(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN sol_delta > 0 THEN sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN sol_delta < 0 THEN -sol_delta ELSE 0 END), 0), \
                COUNT(signature) \
             FROM sol_flow_cache \
             WHERE timestamp >= ?1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(from.to_rfc3339())];
        if let Some(to_ts) = to {
            query.push_str(&format!(" AND timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare cached flow aggregation query: {}", e))?;
        let (inflow, outflow, count) = stmt
            .query_row(params_refs.as_slice(), |row| {
                let inflow = row.get::<_, Option<f64>>(0)?.unwrap_or(0.0);
                let outflow = row.get::<_, Option<f64>>(1)?.unwrap_or(0.0);
                let count = row.get::<_, i64>(2)?.max(0) as usize;
                Ok((inflow, outflow, count))
            })
            .map_err(|e| format!("Failed to aggregate cached SOL flows: {}", e))?;
        Ok((inflow, outflow, count))
    }

    /// Upsert a batch of flow rows into cache
    pub fn upsert_flow_rows_sync(
        &self,
        rows: &[(String, DateTime<Utc>, f64)],
    ) -> Result<usize, String> {
        if rows.is_empty() {
            return Ok(0);
        }
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start flow cache transaction: {}", e))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO sol_flow_cache(signature, timestamp, sol_delta) VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| format!("Failed to prepare flow cache upsert: {}", e))?;
            for (sig, ts, delta) in rows.iter() {
                stmt.execute(params![sig, ts.to_rfc3339(), *delta])
                    .map_err(|e| format!("Failed to upsert flow row: {}", e))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit flow cache upserts: {}", e))?;
        Ok(rows.len())
    }

    /// Get the max timestamp present in the flow cache
    pub fn get_flow_cache_max_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MAX(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare max timestamp query: {}", e))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query max timestamp: {}", e))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached max timestamp: {}", e))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get the minimum timestamp present in the flow cache (earliest record)
    pub fn get_flow_cache_min_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MIN(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare min timestamp query: {}", e))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query min timestamp: {}", e))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached min timestamp: {}", e))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get flow cache stats (row count and latest timestamp)
    pub fn get_flow_cache_stats_sync(&self) -> Result<WalletFlowCacheStats, String> {
        let conn = self.get_connection()?;
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM sol_flow_cache", [], |row| row.get(0))
            .unwrap_or(0);
        let max_ts = self.get_flow_cache_max_ts_sync()?.map(|dt| dt.to_rfc3339());
        Ok(WalletFlowCacheStats {
            rows: rows.max(0) as u64,
            max_timestamp: max_ts,
        })
    }

    pub fn get_dashboard_metrics(
        &self,
        window_key: &str,
    ) -> Result<Option<CachedDashboardMetrics>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, \
                    computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, \
                    last_processed_timestamp, last_processed_signature, window_start \
                 FROM wallet_dashboard_metrics WHERE window_key = ?1",
            )
            .map_err(|e| format!("Failed to prepare dashboard metrics query: {}", e))?;

        let result = stmt
            .query_row(params![window_key], |row| {
                let computed_at_str: String = row.get(6)?;
                let valid_until_str: String = row.get(7)?;
                let computed_at = DateTime::parse_from_rfc3339(&computed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            6,
                            "computed_at".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?;
                let valid_until = DateTime::parse_from_rfc3339(&valid_until_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "valid_until".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?;

                let last_processed_ts: Option<String> = row.get(11).ok();
                let last_processed_timestamp = last_processed_ts
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let window_start_ts: Option<String> = row.get(13).ok();
                let window_start = window_start_ts
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                Ok(CachedDashboardMetrics {
                    window_key: row.get(0)?,
                    window_hours: row.get::<_, i64>(1)?,
                    snapshot_limit: row.get::<_, i64>(2)? as usize,
                    token_limit: row.get::<_, i64>(3)? as usize,
                    payload: row.get(4)?,
                    payload_format: row.get(5)?,
                    computed_at,
                    valid_until,
                    computation_duration_ms: row.get(8).ok(),
                    snapshot_count: row.get::<_, i64>(9)? as usize,
                    flow_cache_rows: row.get::<_, i64>(10)? as usize,
                    last_processed_timestamp,
                    last_processed_signature: row.get(12).ok(),
                    window_start,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to fetch dashboard metrics: {}", e))?;

        Ok(result)
    }

    pub fn upsert_dashboard_metrics(&self, metrics: &CachedDashboardMetrics) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO wallet_dashboard_metrics (
                window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format,
                computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows,
                last_processed_timestamp, last_processed_signature, window_start, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))",
            params![
                metrics.window_key,
                metrics.window_hours,
                metrics.snapshot_limit as i64,
                metrics.token_limit as i64,
                metrics.payload,
                metrics.payload_format,
                metrics.computed_at.to_rfc3339(),
                metrics.valid_until.to_rfc3339(),
                metrics.computation_duration_ms,
                metrics.snapshot_count as i64,
                metrics.flow_cache_rows as i64,
                metrics
                    .last_processed_timestamp
                    .as_ref()
                    .map(|ts| ts.to_rfc3339()),
                metrics.last_processed_signature,
                metrics.window_start.as_ref().map(|ts| ts.to_rfc3339()),
            ],
        )
        .map_err(|e| format!("Failed to upsert dashboard metrics: {}", e))?;
        Ok(())
    }

    pub fn invalidate_dashboard_metrics(&self, window_key: &str) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM wallet_dashboard_metrics WHERE window_key = ?1",
            params![window_key],
        )
        .map_err(|e| format!("Failed to invalidate dashboard metrics: {}", e))?;
        Ok(())
    }

    pub fn cleanup_expired_metrics(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM wallet_dashboard_metrics WHERE valid_until < datetime('now')",
                [],
            )
            .map_err(|e| format!("Failed to cleanup dashboard metrics: {}", e))?;
        Ok(deleted.max(0) as u64)
    }

    /// Save wallet snapshot with token balances (synchronous version)
    pub fn save_wallet_snapshot_sync(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64,
                    snapshot.total_nfts_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        // Insert NFT balances
        for nft_balance in &snapshot.nft_balances {
            conn.execute(
                r#"
                INSERT INTO nft_balances (
                    snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    snapshot_id,
                    nft_balance.mint,
                    nft_balance.account_address,
                    nft_balance.name,
                    nft_balance.symbol,
                    nft_balance.image_url,
                    nft_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert NFT balance: {}", e))?;
        }

        logger::debug(
            LogTag::Wallet,
            &format!(
                "Saved wallet snapshot ID {} with {} tokens, {} NFTs for {}",
                snapshot_id,
                snapshot.token_balances.len(),
                snapshot.nft_balances.len(),
                &snapshot.wallet_address[..8]
            ),
        );

        update_wallet_snapshot_status(snapshot.snapshot_time);

        Ok(snapshot_id)
    }

    /// Save wallet snapshot with token balances (async version)
    pub async fn save_wallet_snapshot(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64,
                    snapshot.total_nfts_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        // Insert NFT balances
        for nft_balance in &snapshot.nft_balances {
            conn.execute(
                r#"
                INSERT INTO nft_balances (
                    snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    snapshot_id,
                    nft_balance.mint,
                    nft_balance.account_address,
                    nft_balance.name,
                    nft_balance.symbol,
                    nft_balance.image_url,
                    nft_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert NFT balance: {}", e))?;
        }

        logger::debug(
            LogTag::Wallet,
            &format!(
                "Saved wallet snapshot ID {} with {} tokens, {} NFTs for {}",
                snapshot_id,
                snapshot.token_balances.len(),
                snapshot.nft_balances.len(),
                &snapshot.wallet_address[..8]
            ),
        );

        update_wallet_snapshot_status(snapshot.snapshot_time);

        Ok(snapshot_id)
    }

    /// Get SOL balance at or before a specific time (optimized for single value)
    /// Uses idx_wallet_snapshots_time index for fast descending time lookup
    pub fn get_balance_at_time_sync(
        &self,
        target_time: DateTime<Utc>,
    ) -> Result<Option<f64>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT sol_balance 
            FROM wallet_snapshots 
            WHERE datetime(snapshot_time) <= datetime(?1)
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                params![target_time.to_rfc3339()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query balance at time: {}", e))?;

        Ok(result)
    }

    /// Get the most recent snapshot timestamp (if any) without loading token data
    pub fn get_latest_snapshot_time(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;

        let snapshot_time_str: Option<String> = conn
            .query_row(
                r#"
            SELECT snapshot_time
            FROM wallet_snapshots
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#,
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to fetch latest wallet snapshot time: {}", e))?;

        if let Some(ts_str) = snapshot_time_str {
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|_| format!("Invalid snapshot_time stored: {}", ts_str))?
                .with_timezone(&Utc);
            Ok(Some(timestamp))
        } else {
            Ok(None)
        }
    }

    /// Get recent wallet snapshots (synchronous version)
    pub fn get_recent_snapshots_sync(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    total_nfts_count: row.get::<_, i64>(6)? as u32,
                    token_balances: Vec::new(), // Loaded separately if needed
                    nft_balances: Vec::new(),   // Loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?);
        }

        Ok(snapshots)
    }

    /// Get recent wallet snapshots (async version)
    pub async fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    total_nfts_count: row.get::<_, i64>(6)? as u32,
                    token_balances: Vec::new(), // Loaded separately if needed
                    nft_balances: Vec::new(),   // Loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?);
        }

        Ok(snapshots)
    }

    /// Get wallet monitoring statistics (synchronous version)
    pub fn get_monitor_stats_sync(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_string(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get wallet monitoring statistics (async version)
    pub async fn get_monitor_stats(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_string(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get token balances for a specific snapshot (synchronous version)
    pub fn get_token_balances_sync(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, COALESCE(decimals, 0), is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get::<_, i64>(5)? as u8,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?,
            );
        }

        Ok(balances)
    }

    /// Get NFT balances for a specific snapshot (synchronous version)
    pub fn get_nft_balances_sync(&self, snapshot_id: i64) -> Result<Vec<NftBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
            FROM nft_balances 
            WHERE snapshot_id = ?1
            ORDER BY name ASC
            "#,
            )
            .map_err(|e| format!("Failed to prepare nft balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(NftBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    account_address: row.get(3)?,
                    name: row.get(4)?,
                    symbol: row.get(5)?,
                    image_url: row.get(6)?,
                    is_token_2022: row.get(7)?,
                })
            })
            .map_err(|e| format!("Failed to execute nft balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse nft balance row: {}", e))?,
            );
        }

        Ok(balances)
    }

    /// Get token balances for a specific snapshot (async version)
    pub async fn get_token_balances(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, COALESCE(decimals, 0), is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get::<_, i64>(5)? as u8,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?,
            );
        }

        Ok(balances)
    }

    /// Cleanup old snapshots (keep last 1000) - synchronous version
    pub fn cleanup_old_snapshots_sync(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            logger::info(
                LogTag::Wallet,
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }

    /// Cleanup old snapshots (keep last 1000) - async version
    pub async fn cleanup_old_snapshots(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            logger::info(
                LogTag::Wallet,
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }
}

// =============================================================================
// GLOBAL WALLET DATABASE INSTANCE
// =============================================================================

/// Global wallet database instance
pub(crate) static GLOBAL_WALLET_DB: Lazy<Arc<Mutex<Option<WalletDatabase>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// Global wallet service metrics
static WALLET_METRICS_OPERATIONS: Lazy<Arc<std::sync::atomic::AtomicU64>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));
static WALLET_METRICS_ERRORS: Lazy<Arc<std::sync::atomic::AtomicU64>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));
static WALLET_METRICS_SNAPSHOTS_TAKEN: Lazy<Arc<std::sync::atomic::AtomicU64>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));
static WALLET_METRICS_FLOW_SYNCS: Lazy<Arc<std::sync::atomic::AtomicU64>> =
    Lazy::new(|| Arc::new(std::sync::atomic::AtomicU64::new(0)));

/// Get wallet service metrics
pub fn get_wallet_service_metrics() -> (u64, u64, u64, u64) {
    (
        WALLET_METRICS_OPERATIONS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_ERRORS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_SNAPSHOTS_TAKEN.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_FLOW_SYNCS.load(std::sync::atomic::Ordering::Relaxed),
    )
}

/// Initialize the global wallet database
pub async fn initialize_wallet_database() -> Result<(), String> {
    let mut db_lock = GLOBAL_WALLET_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = WalletDatabase::new().await?;
    let latest_snapshot_time = db.get_latest_snapshot_time()?;
    *db_lock = Some(db);

    hydrate_wallet_snapshot_status(latest_snapshot_time);

    logger::info(
        LogTag::Wallet,
        "Global wallet database initialized successfully",
    );
    Ok(())
}

// =============================================================================
// WALLET MONITORING SERVICE
// =============================================================================

/// Collect current wallet balance and token balances
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    // Get wallet address
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;

    let rpc_client = get_rpc_client();
    let snapshot_time = Utc::now();

    logger::debug(
        LogTag::Wallet,
        &format!("Collecting wallet snapshot for {}", &wallet_address[..8]),
    );

    // Add small delay to avoid overwhelming RPC client
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get SOL balance
    let sol_balance = rpc_client
        .get_sol_balance(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get SOL balance: {}", e))?;

    let sol_balance_lamports = crate::utils::sol_to_lamports(sol_balance);

    // Add another small delay before token accounts fetch
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get all token accounts (includes both tokens and NFTs)
    let token_accounts = rpc_client
        .get_all_token_accounts_str(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    // Separate fungible tokens and NFTs
    let mut token_balances = Vec::new();
    let mut nft_mints_with_accounts: Vec<(String, String, bool)> = Vec::new(); // (mint, account, is_token_2022)

    for account_info in &token_accounts {
        // Skip accounts with zero balance
        if account_info.balance == 0 {
            continue;
        }

        // Check if this is an NFT (decimals=0 and balance=1)
        if account_info.is_nft {
            nft_mints_with_accounts.push((
                account_info.mint.clone(),
                account_info.account.clone(),
                account_info.is_token_2022,
            ));
        } else {
            // Fungible token - use decimals from RPC response
            let decimals = account_info.decimals;
            let balance_ui = (account_info.balance as f64) / (10_f64).powi(decimals as i32);

            token_balances.push(TokenBalance {
                id: None,
                snapshot_id: None,
                mint: account_info.mint.clone(),
                balance: account_info.balance,
                balance_ui,
                decimals,
                is_token_2022: account_info.is_token_2022,
            });
        }
    }

    // Fetch NFT metadata from Metaplex
    let mut nft_balances = Vec::new();
    if !nft_mints_with_accounts.is_empty() {
        let nft_mints: Vec<String> = nft_mints_with_accounts
            .iter()
            .map(|(mint, _, _)| mint.clone())
            .collect();

        logger::debug(
            LogTag::Wallet,
            &format!("Fetching metadata for {} NFTs", nft_mints.len()),
        );

        let metadata_results = fetch_nft_metadata_batch(&nft_mints).await;

        for (mint, account, is_token_2022) in nft_mints_with_accounts {
            let (name, symbol, image_url) = metadata_results
                .get(&mint)
                .and_then(|result| result.as_ref().ok())
                .map(|meta| {
                    (
                        meta.name.clone(),
                        meta.symbol.clone(),
                        meta.image_url.clone(),
                    )
                })
                .unwrap_or((None, None, None));

            nft_balances.push(NftBalance {
                id: None,
                snapshot_id: None,
                mint,
                account_address: account,
                name,
                symbol,
                image_url,
                is_token_2022,
            });
        }
    }

    let total_tokens_count = token_balances.len() as u32;
    let total_nfts_count = nft_balances.len() as u32;

    logger::debug(
        LogTag::Wallet,
        &format!(
            "Collected snapshot: SOL {:.6}, {} tokens, {} NFTs",
            sol_balance, total_tokens_count, total_nfts_count
        ),
    );

    Ok(WalletSnapshot {
        id: None,
        wallet_address,
        snapshot_time,
        sol_balance,
        sol_balance_lamports,
        total_tokens_count,
        total_nfts_count,
        token_balances,
        nft_balances,
    })
}

/// Background service for wallet monitoring
pub async fn start_wallet_monitoring_service(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        monitor.instrument(async move {
            logger::info(LogTag::Wallet, "Wallet monitoring service started (instrumented)");

            // Initialize database
            if let Err(e) = initialize_wallet_database().await {
                logger::error(
                    LogTag::Wallet,
                    &format!("Failed to initialize wallet database: {}", e)
                );
                return;
            }

            warmup_dashboard_metrics().await;

            let snapshot_interval = with_config(|cfg| cfg.wallet.snapshot_interval_secs);
            let cache_interval_secs = with_config(|cfg| cfg.wallet.flow_cache_update_secs);
            let mut interval = tokio::time::interval(Duration::from_secs(snapshot_interval.max(10)));
            let mut flow_sync_interval = tokio::time::interval(Duration::from_secs(cache_interval_secs.max(1)));
            let (metrics_24h_secs, metrics_7d_secs, metrics_30d_secs, metrics_all_secs) =
                with_config(|cfg| {
                    (
                        cfg.wallet.dashboard_metrics_24h_interval_secs.max(30),
                        cfg.wallet.dashboard_metrics_7d_interval_secs.max(60),
                        cfg.wallet.dashboard_metrics_30d_interval_secs.max(300),
                        cfg.wallet.dashboard_metrics_alltime_interval_secs.max(300),
                    )
                });
            let mut metrics_24h_interval =
                tokio::time::interval(Duration::from_secs(metrics_24h_secs));
            let mut metrics_7d_interval =
                tokio::time::interval(Duration::from_secs(metrics_7d_secs));
            let mut metrics_30d_interval =
                tokio::time::interval(Duration::from_secs(metrics_30d_secs));
            let mut metrics_all_interval =
                tokio::time::interval(Duration::from_secs(metrics_all_secs));
            let mut cleanup_counter = 0;

            loop {
                tokio::select! {
                _ = shutdown.notified() => {
                    logger::info(LogTag::Wallet, "Wallet monitoring service shutting down");
                    break;
                }
                _ = interval.tick() => {
                    // Collect wallet snapshot
                    match collect_wallet_snapshot().await {
                        Ok(snapshot) => {
                            // Track metrics
                            WALLET_METRICS_OPERATIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            WALLET_METRICS_SNAPSHOTS_TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                            // Save to database
                            let db_guard = GLOBAL_WALLET_DB.lock().await;
                            match db_guard.as_ref() {
                                Some(db) => {
                                    match db.save_wallet_snapshot_sync(&snapshot) {
                                        Ok(snapshot_id) => {
                                            logger::debug(
                                                LogTag::Wallet,
                                                &format!(
                                                    "Saved snapshot ID {} - SOL: {:.6}, Tokens: {}",
                                                    snapshot_id,
                                                    snapshot.sol_balance,
                                                    snapshot.total_tokens_count
                                                )
                                            );
                                        }
                                        Err(e) => {
                                            logger::error(LogTag::Wallet, &format!("Failed to save wallet snapshot: {}", e));
                                        }
                                    }
                                }
                                None => {
                                    logger::error(LogTag::Wallet, "Wallet database not initialized");
                                }
                            }
                        }
                        Err(e) => {
                            WALLET_METRICS_ERRORS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            logger::error(LogTag::Wallet, &format!("Failed to collect wallet snapshot: {}", e));
                        }
                    }

                    // Cleanup old snapshots every 60 intervals (1 hour)
                    cleanup_counter += 1;
                    if cleanup_counter >= 60 {
                        cleanup_counter = 0;

                        let db_guard = GLOBAL_WALLET_DB.lock().await;
                        match db_guard.as_ref() {
                            Some(db) => {
                                if let Err(e) = db.cleanup_old_snapshots_sync() {
                                    logger::warning(LogTag::Wallet, &format!("Failed to cleanup old snapshots: {}", e));
                                }
                                if let Err(e) = db.cleanup_expired_metrics() {
                                    logger::warning(LogTag::Wallet, &format!(
                                        "Failed to cleanup expired dashboard metrics: {}",
                                        e
                                    ));
                                }
                            }
                            None => {
                                logger::warning(LogTag::Wallet, "Wallet database not initialized for cleanup");
                            }
                        }
                    }
                }
                _ = flow_sync_interval.tick() => {
                    // Periodically sync SOL flow cache from transactions DB
                    let (batch_size, lookback_secs) = with_config(|cfg| (cfg.wallet.flow_cache_backfill_batch, cfg.wallet.flow_cache_lookback_secs));
                    // Step 1: read current max cached ts under short lock
                    let start_ts = {
                        let db_guard = GLOBAL_WALLET_DB.lock().await;
                        if let Some(wallet_db) = db_guard.as_ref() {
                            match wallet_db.get_flow_cache_max_ts_sync() {
                                Ok(Some(ts)) => ts - ChronoDuration::seconds(lookback_secs as i64),
                                Ok(None) => Utc::now() - ChronoDuration::hours(24),
                                Err(_) => Utc::now() - ChronoDuration::hours(24),
                            }
                        } else {
                            // Wallet DB not ready yet
                            continue;
                        }
                    };

                    // Step 2: export rows from transactions DB without holding wallet lock
                    let rows = if let Some(tx_db) = get_transaction_database().await {
                        match tx_db.export_processed_for_wallet_flow(start_ts, batch_size).await {
                            Ok(rows) => rows,
                            Err(e) => {
                                logger::error(LogTag::Wallet, &format!("Failed to export processed rows: {}", e));
                                Vec::new()
                            }
                        }
                    } else { Vec::new() };

                    if rows.is_empty() { continue; }

                    // Step 3: upsert into wallet cache under short lock
                    let mapped: Vec<(String, DateTime<Utc>, f64)> = rows
                        .into_iter()
                        .map(|r| (r.signature, r.timestamp, r.sol_delta))
                        .collect();
                    let db_guard = GLOBAL_WALLET_DB.lock().await;
                    if let Some(wallet_db) = db_guard.as_ref() {
                        if let Err(e) = wallet_db.upsert_flow_rows_sync(&mapped) {
                            WALLET_METRICS_ERRORS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            logger::error(LogTag::Wallet, &format!("Failed to upsert flow cache rows: {}", e));
                        } else {
                            WALLET_METRICS_FLOW_SYNCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            logger::debug(LogTag::Wallet, &format!("Upserted {} flow cache rows", mapped.len()));
                        }
                    }
                }
                _ = metrics_24h_interval.tick() => {
                    compute_and_cache_metrics_internal("24h", 24).await;
                }
                _ = metrics_7d_interval.tick() => {
                    compute_and_cache_metrics_internal("7d", 168).await;
                }
                _ = metrics_30d_interval.tick() => {
                    compute_and_cache_metrics_internal("30d", 720).await;
                }
                _ = metrics_all_interval.tick() => {
                    compute_and_cache_metrics_internal("all_time", 0).await;
                }
            }
            }

            logger::info(LogTag::Wallet, "Wallet monitoring service stopped");
        })
    )
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Get recent wallet snapshots
pub async fn get_recent_wallet_snapshots(limit: usize) -> Result<Vec<WalletSnapshot>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            // Use the synchronous version to avoid lifetime issues
            db.get_recent_snapshots_sync(limit)
        }
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get wallet monitoring statistics
pub async fn get_wallet_monitor_stats() -> Result<WalletMonitorStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_monitor_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get token balances for a snapshot
pub async fn get_snapshot_token_balances(snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_balances_sync(snapshot_id),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get NFT balances for a snapshot
pub async fn get_snapshot_nft_balances(snapshot_id: i64) -> Result<Vec<NftBalance>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_nft_balances_sync(snapshot_id),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get current wallet status (latest snapshot data)
pub async fn get_current_wallet_status() -> Result<Option<WalletSnapshot>, String> {
    let snapshots = get_recent_wallet_snapshots(1).await?;
    Ok(snapshots.into_iter().next())
}

/// Get SOL balance at or before a specific time (optimized single-value query)
pub async fn get_balance_at_time(target_time: DateTime<Utc>) -> Result<Option<f64>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_balance_at_time_sync(target_time),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Public accessor for flow cache stats
pub async fn get_flow_cache_stats() -> Result<WalletFlowCacheStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_flow_cache_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}

pub async fn refresh_dashboard_cache(window_hours: i64) -> Result<(), String> {
    let window_hours = clamp_window_hours(window_hours);
    let (window_key, canonical_hours) =
        canonical_window(window_hours).ok_or_else(|| "Unsupported window".to_string())?;

    {
        let db_guard = GLOBAL_WALLET_DB.lock().await;
        if let Some(db) = db_guard.as_ref() {
            db.invalidate_dashboard_metrics(window_key)?;
        }
    }

    if let Err(err) = compute_and_cache_metrics(window_key, canonical_hours).await {
        circuit_record_failure(window_key).await;
        Err(err)
    } else {
        circuit_reset(window_key).await;
        Ok(())
    }
}

pub async fn get_dashboard_cache_metrics() -> CachePerformanceMetrics {
    CACHE_METRICS.read().await.clone()
}

pub async fn clear_dashboard_api_cache() {
    let mut guard = API_RESPONSE_CACHE.write().await;
    guard.clear();
}
