//! Caching infrastructure for wallet dashboard metrics

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, LazyLock, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::transactions::get_transaction_database;

use super::database::GLOBAL_WALLET_DB;
use super::dashboard::compute_dashboard_payload_realtime;
use super::types::*;

// Constants
const DEFAULT_PRECOMPUTED_SNAPSHOT_LIMIT: usize = 600;
const DEFAULT_PRECOMPUTED_TOKEN_LIMIT: usize = 250;
const MAX_API_CACHE_ENTRIES: usize = 128;
const CIRCUIT_BREAKER_THRESHOLD: u32 = 3;
const CIRCUIT_BREAKER_COOLDOWN_SECS: u64 = 300;

// =============================================================================
// WALLET SNAPSHOT STATUS CACHE
// =============================================================================

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

static WALLET_SNAPSHOT_STATUS: LazyLock<WalletSnapshotStatusCache> =
    LazyLock::new(WalletSnapshotStatusCache::default);

pub(super) fn update_wallet_snapshot_status(timestamp: DateTime<Utc>) {
    WALLET_SNAPSHOT_STATUS.mark_ready(timestamp);
}

pub(super) fn hydrate_wallet_snapshot_status(timestamp: Option<DateTime<Utc>>) {
    WALLET_SNAPSHOT_STATUS.set(timestamp);
}

pub fn get_cached_wallet_snapshot_status() -> WalletSnapshotStatus {
    WALLET_SNAPSHOT_STATUS.snapshot()
}

// =============================================================================
// DASHBOARD METRICS CACHE
// =============================================================================

#[derive(Debug, Clone)]
pub(super) struct CachedDashboardMetrics {
    pub(super) window_key: String,
    pub(super) window_hours: i64,
    pub(super) snapshot_limit: usize,
    pub(super) token_limit: usize,
    pub(super) payload: Vec<u8>,
    pub(super) payload_format: String,
    pub(super) computed_at: DateTime<Utc>,
    pub(super) valid_until: DateTime<Utc>,
    pub(super) computation_duration_ms: Option<i64>,
    pub(super) snapshot_count: usize,
    pub(super) flow_cache_rows: usize,
    pub(super) last_processed_timestamp: Option<DateTime<Utc>>,
    pub(super) last_processed_signature: Option<String>,
    pub(super) window_start: Option<DateTime<Utc>>,
}

pub(super) static API_RESPONSE_CACHE: LazyLock<
    Arc<RwLock<HashMap<DashboardRequestKey, CachedDashboardResponse>>>,
> = LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

pub(super) static CACHE_METRICS: LazyLock<Arc<RwLock<CachePerformanceMetrics>>> =
    LazyLock::new(|| Arc::new(RwLock::new(CachePerformanceMetrics::default())));

static COMPUTATION_FAILURES: LazyLock<Arc<RwLock<HashMap<String, (u32, Instant)>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

// =============================================================================
// COMPRESSION UTILITIES
// =============================================================================

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

pub(super) fn serialize_dashboard_payload(payload: &WalletDashboardData) -> Result<Vec<u8>, String> {
    let mut sanitized = payload.clone();
    sanitized.cache_metadata = None;
    let json = serde_json::to_vec(&sanitized)
        .map_err(|e| format!("Failed to serialize dashboard payload: {}", e))?;
    compress_bytes(&json)
}

pub(super) fn deserialize_dashboard_payload(raw: &[u8]) -> Result<WalletDashboardData, String> {
    let json_bytes = decompress_bytes(raw)?;
    serde_json::from_slice::<WalletDashboardData>(&json_bytes)
        .map_err(|e| format!("Failed to deserialize dashboard payload: {}", e))
}

// =============================================================================
// CACHE UTILITIES
// =============================================================================

pub(super) fn canonical_window(window_hours: i64) -> Option<(&'static str, i64)> {
    match window_hours {
        24 => Some(("24h", 24)),
        168 => Some(("7d", 168)),
        720 => Some(("30d", 720)),
        0 => Some(("all_time", 0)),
        _ => None,
    }
}

pub(super) fn ttl_for_window(window_key: &str) -> u64 {
    with_config(|cfg| match window_key {
        "24h" => cfg.wallet.dashboard_metrics_24h_interval_secs,
        "7d" => cfg.wallet.dashboard_metrics_7d_interval_secs,
        "30d" => cfg.wallet.dashboard_metrics_30d_interval_secs,
        "all_time" => cfg.wallet.dashboard_metrics_alltime_interval_secs,
        _ => cfg.wallet.dashboard_metrics_24h_interval_secs,
    })
}

pub(super) async fn record_cache_metrics(source: DashboardDataSource, latency_ms: u128, stale: bool) {
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

// =============================================================================
// CIRCUIT BREAKER
// =============================================================================

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

pub(super) async fn circuit_record_failure(window_key: &str) {
    let mut guard = COMPUTATION_FAILURES.write().await;
    let entry = guard
        .entry(window_key.to_string())
        .or_insert((0, Instant::now()));
    entry.0 = entry.0.saturating_add(1);
    entry.1 = Instant::now();
}

pub(super) async fn circuit_reset(window_key: &str) {
    let mut guard = COMPUTATION_FAILURES.write().await;
    guard.remove(window_key);
}

// =============================================================================
// CACHE COMPUTATION
// =============================================================================

pub(super) async fn compute_and_cache_metrics_internal(window_key: &'static str, window_hours: i64) {
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

pub(super) async fn compute_and_cache_metrics(
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

pub(super) async fn warmup_dashboard_metrics() {
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
