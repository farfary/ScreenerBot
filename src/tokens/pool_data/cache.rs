//! Caching layer for pool snapshots with TTL and stale fallback

use crate::chains::ChainId;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::tokens::database;
use crate::tokens::service::get_rate_coordinator;
use crate::tokens::types::{TokenPoolInfo, TokenPoolsSnapshot, TokenResult};
use crate::tokens::Error;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::array;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::{LazyLock, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use super::api;
use super::operations::{choose_canonical_pool, sort_pools_for_snapshot};
use super::utils::calculate_pool_metric;
use serde_json::json;

const TOKEN_POOLS_TTL_SECS: u64 = 60;
const POOL_PREFETCH_DEBOUNCE_SECS: u64 = 20;
const POOL_PREFETCH_WORKER_COUNT: usize = 8;
/// Max time `fetch_immediate` waits on a refresh slot already held by another
/// (possibly background/rate-limited) refresher before falling back to a direct,
/// instant data-server pool fetch. Keeps user-opened charts responsive.
const FOREIGN_REFRESH_WAIT: Duration = Duration::from_millis(2_500);

#[derive(Clone)]
struct TokenPoolCacheEntry {
    snapshot: TokenPoolsSnapshot,
    refreshed_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PoolCacheMetrics {
    pub entries: usize,
    pub fresh_entries: usize,
    pub stale_entries: usize,
}

type PoolCacheKey = (ChainId, String);

fn pool_cache_key(chain: ChainId, mint: &str) -> PoolCacheKey {
    (chain, mint.trim().to_owned())
}

static TOKEN_POOLS_CACHE: LazyLock<moka::sync::Cache<PoolCacheKey, TokenPoolCacheEntry>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(5_000)
            .time_to_live(Duration::from_secs(TOKEN_POOLS_TTL_SECS * 2))
            .build()
    });

static POOL_REFRESH_INFLIGHT: LazyLock<AsyncMutex<HashMap<PoolCacheKey, std::sync::Arc<Notify>>>> =
    LazyLock::new(|| AsyncMutex::new(HashMap::new()));

static POOL_PREFETCH_STATE: LazyLock<moka::sync::Cache<PoolCacheKey, Instant>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(5_000)
            .time_to_live(Duration::from_secs(POOL_PREFETCH_DEBOUNCE_SECS * 3))
            .build()
    });

static POOL_PREFETCH_SCHEDULER: LazyLock<Arc<PrefetchScheduler>> =
    LazyLock::new(|| Arc::new(PrefetchScheduler::new()));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrefetchPriority {
    High = 0,
    Normal = 1,
    Low = 2,
}

impl PrefetchPriority {
    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug)]
struct PrefetchTask {
    chain: ChainId,
    mint: String,
    priority: PrefetchPriority,
    allow_stale: bool,
}

struct PrefetchSchedulerInner {
    queues: [VecDeque<PrefetchTask>; 3],
}

impl PrefetchSchedulerInner {
    fn new() -> Self {
        Self {
            queues: array::from_fn(|_| VecDeque::new()),
        }
    }

    fn push(&mut self, task: PrefetchTask) {
        self.queues[task.priority.index()].push_back(task);
    }

    fn pop(&mut self) -> Option<PrefetchTask> {
        for queue in self.queues.iter_mut() {
            if let Some(task) = queue.pop_front() {
                return Some(task);
            }
        }
        None
    }
}

struct PrefetchScheduler {
    inner: AsyncMutex<PrefetchSchedulerInner>,
    notify: Notify,
    started: OnceLock<()>,
    worker_count: usize,
}

impl PrefetchScheduler {
    fn new() -> Self {
        Self {
            inner: AsyncMutex::new(PrefetchSchedulerInner::new()),
            notify: Notify::new(),
            started: OnceLock::new(),
            worker_count: POOL_PREFETCH_WORKER_COUNT,
        }
    }

    fn ensure_workers(self: &Arc<Self>) {
        if self.started.get().is_some() {
            return;
        }

        if self.started.set(()).is_ok() {
            for worker_id in 0..self.worker_count {
                let scheduler = Arc::clone(self);
                tokio::spawn(async move {
                    scheduler.worker_loop(worker_id).await;
                });
            }
        }
    }

    async fn enqueue(self: &Arc<Self>, task: PrefetchTask) {
        self.ensure_workers();

        {
            let mut inner = self.inner.lock().await;
            inner.push(task);
        }

        self.notify.notify_one();
    }

    async fn worker_loop(self: Arc<Self>, worker_id: usize) {
        loop {
            let task = self.next_task().await;
            self.process_task(worker_id, task).await;
        }
    }

    async fn next_task(&self) -> PrefetchTask {
        loop {
            {
                let mut inner = self.inner.lock().await;
                if let Some(task) = inner.pop() {
                    return task;
                }
            }

            self.notify.notified().await;
        }
    }

    async fn process_task(&self, worker_id: usize, task: PrefetchTask) {
        let mint = task.mint.clone();
        let chain = task.chain;

        if let Some(snapshot) = get_cached_pool_snapshot(chain, &mint) {
            if is_snapshot_fresh(&snapshot) {
                return;
            }
        }

        let (should_refresh, _notifier) = begin_refresh_slot(chain, &mint).await;
        if !should_refresh {
            return;
        }

        let result = refresh_token_pools_and_cache(chain, &mint, task.allow_stale).await;

        complete_refresh_slot(chain, &mint).await;

        if let Err(err) = result {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] Background refresh failed for mint={} worker={} error={}",
                    mint, worker_id, err
                ),
            );

            record_token_event(
                &mint,
                "pool_prefetch_failed",
                Severity::Warn,
                serde_json::json!({
                    "error": err.to_string(),
                    "worker_id": worker_id,
                }),
            )
            .await;

            POOL_PREFETCH_STATE.invalidate(&pool_cache_key(chain, &mint));
        } else {
            POOL_PREFETCH_STATE.invalidate(&pool_cache_key(chain, &mint));
        }
    }
}

async fn begin_refresh_slot(chain: ChainId, mint: &str) -> (bool, Arc<Notify>) {
    let mut guard = POOL_REFRESH_INFLIGHT.lock().await;
    match guard.entry(pool_cache_key(chain, mint)) {
        std::collections::hash_map::Entry::Occupied(entry) => (false, entry.get().clone()),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let notify = Arc::new(Notify::new());
            entry.insert(notify.clone());
            (true, notify)
        }
    }
}

async fn complete_refresh_slot(chain: ChainId, mint: &str) {
    let notify = {
        let mut guard = POOL_REFRESH_INFLIGHT.lock().await;
        guard.remove(&pool_cache_key(chain, mint))
    };

    if let Some(notifier) = notify {
        notifier.notify_waiters();
    }
}

async fn enqueue_background_refresh(
    chain: ChainId,
    mint: String,
    priority: PrefetchPriority,
    allow_stale: bool,
) {
    let scheduler = Arc::clone(&POOL_PREFETCH_SCHEDULER);
    scheduler
        .enqueue(PrefetchTask {
            chain,
            mint,
            priority,
            allow_stale,
        })
        .await;
}

async fn schedule_background_refresh_if_due(
    chain: ChainId,
    mint: &str,
    priority: PrefetchPriority,
    allow_stale: bool,
) {
    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return;
    }

    let now = Instant::now();
    let key = pool_cache_key(chain, trimmed);
    if let Some(last) = POOL_PREFETCH_STATE.get(&key) {
        if now.duration_since(last) < Duration::from_secs(POOL_PREFETCH_DEBOUNCE_SECS) {
            return;
        }
    }
    POOL_PREFETCH_STATE.insert(key, now);

    enqueue_background_refresh(chain, trimmed.to_string(), priority, allow_stale).await;
}

fn pool_cache_ttl() -> Duration {
    Duration::from_secs(TOKEN_POOLS_TTL_SECS)
}

fn refreshed_at_from_snapshot(snapshot: &TokenPoolsSnapshot) -> Instant {
    let now = Instant::now();
    let age_secs = Utc::now()
        .signed_duration_since(snapshot.pool_data_last_fetched_at)
        .num_seconds()
        .max(0) as u64;
    // Cap age to prevent clock drift issues (max 5 minutes reasonable)
    let capped_age = age_secs.min(300);
    now.checked_sub(Duration::from_secs(capped_age))
        .unwrap_or(now)
}

fn is_pool_entry_fresh(entry: &TokenPoolCacheEntry) -> bool {
    entry.refreshed_at.elapsed() <= pool_cache_ttl()
}

fn get_cached_pool_snapshot(chain: ChainId, mint: &str) -> Option<TokenPoolsSnapshot> {
    let entry = TOKEN_POOLS_CACHE.get(&pool_cache_key(chain, mint))?;
    if is_pool_entry_fresh(&entry) {
        Some(entry.snapshot.clone())
    } else {
        None
    }
}

fn get_cached_pool_snapshot_allow_stale(chain: ChainId, mint: &str) -> Option<TokenPoolsSnapshot> {
    TOKEN_POOLS_CACHE
        .get(&pool_cache_key(chain, mint))
        .map(|entry| entry.snapshot.clone())
}

fn store_pool_snapshot(chain: ChainId, snapshot: TokenPoolsSnapshot) {
    TOKEN_POOLS_CACHE.insert(
        pool_cache_key(chain, &snapshot.mint),
        TokenPoolCacheEntry {
            refreshed_at: refreshed_at_from_snapshot(&snapshot),
            snapshot,
        },
    );
}

fn is_snapshot_fresh(snapshot: &TokenPoolsSnapshot) -> bool {
    let refreshed_at = refreshed_at_from_snapshot(snapshot);
    refreshed_at.elapsed() <= pool_cache_ttl()
}

async fn refresh_token_pools_and_cache(
    chain: ChainId,
    mint: &str,
    allow_stale: bool,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let mint_trimmed = mint.trim();
    if mint_trimmed.is_empty() {
        return Err(Error::InvalidMint {
            value: "Mint address cannot be empty".to_owned(),
        });
    }

    // Fast path: use cached snapshot if already loaded and fresh
    if let Some(snapshot) = get_cached_pool_snapshot(chain, mint_trimmed) {
        if is_snapshot_fresh(&snapshot) || allow_stale {
            return Ok(Some(snapshot));
        }
    }

    // Pull persisted snapshot for reuse/fallback
    let persisted_snapshot = database::get_token_pools_async(mint_trimmed).await?;
    if let Some(snapshot) = persisted_snapshot.as_ref() {
        if is_snapshot_fresh(snapshot) {
            store_pool_snapshot(chain, snapshot.clone());
            return Ok(Some(snapshot.clone()));
        }
    }

    // When the internet is confirmed offline, skip the network fetch entirely —
    // it would only time out on DNS and flood the log with per-token warnings.
    // Serve the persisted snapshot (even if stale) when the caller allows it;
    // otherwise report "no data" quietly. Resumes automatically on reconnect and
    // never triggers at startup (Unknown state).
    if crate::connectivity::is_network_offline() {
        if allow_stale {
            if let Some(snapshot) = persisted_snapshot {
                store_pool_snapshot(chain, snapshot.clone());
                return Ok(Some(snapshot));
            }
        }
        return Ok(None);
    }

    let coordinator = get_rate_coordinator().ok_or_else(|| Error::NotInitialized {
        resource: "Rate limit coordinator not initialized".to_owned(),
    })?;

    let (pools_map, success_sources) = match api::fetch_from_sources(mint_trimmed, coordinator)
        .await
    {
        Ok(result) => result,
        Err(err) => {
            let message = err.to_string();
            if allow_stale {
                if let Some(snapshot) = persisted_snapshot.as_ref() {
                    logger::warning(
                        LogTag::Tokens,
                        &format!(
                            "[TOKEN_POOLS] Falling back to stale snapshot for mint={} due to error: {}",
                            mint_trimmed, message
                        ),
                    );
                    let age_secs = Utc::now()
                        .signed_duration_since(snapshot.pool_data_last_fetched_at)
                        .num_seconds()
                        .max(0);
                    record_token_event(
                        mint_trimmed,
                        "pool_snapshot_fallback",
                        Severity::Warn,
                        json!({
                            "reason": "fetch_error",
                            "error": message.clone(),
                            "allow_stale": allow_stale,
                            "snapshot_fetched_at": snapshot.pool_data_last_fetched_at.to_rfc3339(),
                            "snapshot_age_secs": age_secs,
                            "pool_count": snapshot.pools.len(),
                            "canonical_pool": snapshot.canonical_pool_address.clone(),
                        }),
                    )
                    .await;
                    store_pool_snapshot(chain, snapshot.clone());
                    return Ok(Some(snapshot.clone()));
                }
            }

            record_token_event(
                mint_trimmed,
                "pool_snapshot_fetch_error",
                Severity::Error,
                json!({
                    "error": message.clone(),
                    "allow_stale": allow_stale,
                    "had_persisted_snapshot": persisted_snapshot.is_some(),
                }),
            )
            .await;
            return Err(err);
        }
    };

    if pools_map.is_empty() && success_sources == 0 {
        if let Some(snapshot) = persisted_snapshot.as_ref() {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] No pool sources available for mint={} – using persisted snapshot",
                    mint_trimmed
                ),
            );
            let age_secs = Utc::now()
                .signed_duration_since(snapshot.pool_data_last_fetched_at)
                .num_seconds()
                .max(0);
            record_token_event(
                mint_trimmed,
                "pool_snapshot_fallback",
                Severity::Warn,
                json!({
                    "reason": "sources_unavailable",
                    "allow_stale": allow_stale,
                    "snapshot_fetched_at": snapshot.pool_data_last_fetched_at.to_rfc3339(),
                    "snapshot_age_secs": age_secs,
                    "pool_count": snapshot.pools.len(),
                    "canonical_pool": snapshot.canonical_pool_address.clone(),
                }),
            )
            .await;
            store_pool_snapshot(chain, snapshot.clone());
            return Ok(Some(snapshot.clone()));
        }
    }

    let mut pools: Vec<TokenPoolInfo> = pools_map.into_values().collect();
    sort_pools_for_snapshot(&mut pools);
    let canonical_pool_address = choose_canonical_pool(&pools);

    let prev_pool_count = persisted_snapshot
        .as_ref()
        .map(|snapshot| snapshot.pools.len())
        .unwrap_or_default();
    let prev_canonical = persisted_snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.canonical_pool_address.clone());
    let prev_fetched_at = persisted_snapshot
        .as_ref()
        .map(|snapshot| snapshot.pool_data_last_fetched_at.to_rfc3339());

    let snapshot = TokenPoolsSnapshot {
        mint: mint_trimmed.to_string(),
        pools,
        canonical_pool_address,
        pool_data_last_fetched_at: Utc::now(),
    };

    database::replace_token_pools_async(snapshot.clone()).await?;
    store_pool_snapshot(chain, snapshot.clone());

    let (top_pool, top_metric) = snapshot
        .pools
        .first()
        .map(|pool| (pool.pool_address.clone(), calculate_pool_metric(pool)))
        .unwrap_or_else(|| ("none".to_owned(), 0.0));
    let top_pool_value = if top_pool == "none" {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(top_pool.clone())
    };

    logger::info(
        LogTag::Tokens,
        &format!(
            "[TOKEN_POOLS] Updated pools for mint={} sources={} pool_count={} canonical={} top_metric={:.6}",
            mint_trimmed,
            success_sources,
            snapshot.pools.len(),
            snapshot
                .canonical_pool_address
                .as_deref()
                .unwrap_or("none"),
            top_metric
        ),
    );

    record_token_event(
        mint_trimmed,
        "pool_snapshot_updated",
        Severity::Info,
        json!({
            "pool_count": snapshot.pools.len(),
            "success_sources": success_sources,
            "canonical_pool": snapshot.canonical_pool_address.clone(),
            "top_pool": top_pool_value,
            "top_metric": top_metric,
            "previous_pool_count": prev_pool_count,
            "previous_canonical_pool": prev_canonical.clone(),
            "previous_snapshot_fetched_at": prev_fetched_at,
            "current_snapshot_fetched_at": snapshot.pool_data_last_fetched_at.to_rfc3339(),
            "first_time": persisted_snapshot.is_none(),
        }),
    )
    .await;

    let canonical_changed = snapshot.canonical_pool_address.as_deref() != prev_canonical.as_deref();
    if canonical_changed {
        record_token_event(
            mint_trimmed,
            "pool_canonical_changed",
            Severity::Info,
            json!({
                "previous": prev_canonical,
                "current": snapshot.canonical_pool_address.clone(),
                "top_metric": top_metric,
                "pool_count": snapshot.pools.len(),
            }),
        )
        .await;
    }

    Ok(Some(snapshot))
}

async fn get_snapshot_internal(
    chain: ChainId,
    mint: &str,
    allow_stale: bool,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidMint {
            value: "Mint address cannot be empty".to_owned(),
        });
    }

    if let Some(snapshot) = get_cached_pool_snapshot(chain, trimmed) {
        if is_snapshot_fresh(&snapshot) {
            return Ok(Some(snapshot));
        }

        if allow_stale {
            schedule_background_refresh_if_due(chain, trimmed, PrefetchPriority::Normal, true)
                .await;
            return Ok(Some(snapshot));
        }
    } else if allow_stale {
        if let Some(snapshot) = get_cached_pool_snapshot_allow_stale(chain, trimmed) {
            schedule_background_refresh_if_due(chain, trimmed, PrefetchPriority::Normal, true)
                .await;
            return Ok(Some(snapshot));
        }
    }

    let (should_refresh, notifier) = { begin_refresh_slot(chain, trimmed).await };

    if !should_refresh {
        notifier.notified().await;
        if allow_stale {
            if let Some(snapshot) = get_cached_pool_snapshot_allow_stale(chain, trimmed) {
                return Ok(Some(snapshot));
            }
            return database::get_token_pools_async(trimmed).await;
        }
        return Ok(get_cached_pool_snapshot(chain, trimmed));
    }

    let result = refresh_token_pools_and_cache(chain, trimmed, allow_stale).await;

    complete_refresh_slot(chain, trimmed).await;

    result
}

/// Get fresh pool snapshot for a token (60s cache, API fetch if stale)
pub async fn get_snapshot(chain: ChainId, mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
    get_snapshot_internal(chain, mint, false).await
}

/// Get pool snapshot with stale fallback allowed
pub async fn get_snapshot_allow_stale(
    chain: ChainId,
    mint: &str,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    get_snapshot_internal(chain, mint, true).await
}

/// Prefetch pool snapshots for multiple tokens (debounced, background)
pub async fn prefetch(chain: ChainId, mints: &[String]) {
    if mints.is_empty() {
        return;
    }

    let now = Instant::now();
    let open_position_mints: HashSet<String> = crate::positions::get_open_mints()
        .await
        .into_iter()
        .collect();
    let priced_tokens: HashSet<String> = crate::pools::get_available_tokens().into_iter().collect();

    let mut schedule: Vec<(String, PrefetchPriority)> = Vec::new();

    {
        for mint in mints {
            let trimmed = mint.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(snapshot) = get_cached_pool_snapshot(chain, trimmed) {
                if is_snapshot_fresh(&snapshot) {
                    continue;
                }
            }

            let key = pool_cache_key(chain, trimmed);
            if let Some(last) = POOL_PREFETCH_STATE.get(&key) {
                if now.duration_since(last) < Duration::from_secs(POOL_PREFETCH_DEBOUNCE_SECS) {
                    continue;
                }
            }

            let priority = if open_position_mints.contains(trimmed) {
                PrefetchPriority::High
            } else if !priced_tokens.contains(trimmed) {
                PrefetchPriority::Normal
            } else {
                PrefetchPriority::Low
            };

            POOL_PREFETCH_STATE.insert(key, now);
            schedule.push((trimmed.to_string(), priority));
        }
    }

    if schedule.is_empty() {
        return;
    }

    for (mint, priority) in schedule {
        enqueue_background_refresh(chain, mint, priority, true).await;
    }
}

/// Fetch pool snapshot immediately (bypasses background queue)
/// Use this for user-viewed tokens that need immediate data
pub async fn fetch_immediate(
    chain: ChainId,
    mint: &str,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidMint {
            value: "Mint address cannot be empty".to_owned(),
        });
    }

    // Check if fresh cache exists - return immediately
    if let Some(snapshot) = get_cached_pool_snapshot(chain, trimmed) {
        if is_snapshot_fresh(&snapshot) {
            return Ok(Some(snapshot));
        }
    }

    // INSTANT PATH: this is the user-viewed path (chart open, token details), so
    // latency matters. The data server is the central pool registry and answers
    // in ~200ms. The full multi-source refresh below `tokio::join!`s DexScreener +
    // GeckoTerminal and BLOCKS on the slowest one — and under Gecko 429 penalty a
    // provider's internal retry/backoff can take 60-90s, which is exactly why
    // charts for tokens without a local pool appeared "stuck" (the fast server
    // result was wasted behind the slow providers). So try the server FIRST and
    // return its pools immediately; then kick a debounced background refresh to
    // enrich the snapshot with per-pool price/volume from the direct providers.
    if let Some(snapshot) = server_only_snapshot(chain, trimmed).await {
        schedule_background_refresh_if_due(chain, trimmed, PrefetchPriority::High, true).await;
        return Ok(Some(snapshot));
    }

    // Server missed: fall back to the direct multi-source refresh, bypassing the
    // background queue.
    let (should_refresh, notifier) = begin_refresh_slot(chain, trimmed).await;

    if !should_refresh {
        // Another refresh already holds the slot (often a background prefetch
        // worker draining the rate-limited discovery queue, 60-90s away). Only
        // wait a short bounded window, then return whatever is cached rather than
        // blocking the user's request on the slow pipeline.
        let _ = tokio::time::timeout(FOREIGN_REFRESH_WAIT, notifier.notified()).await;
        return Ok(get_cached_pool_snapshot(chain, trimmed)
            .or_else(|| get_cached_pool_snapshot_allow_stale(chain, trimmed)));
    }

    // Do the actual fetch
    let result = refresh_token_pools_and_cache(chain, trimmed, true).await;

    complete_refresh_slot(chain, trimmed).await;

    result
}

/// Build and cache an instant snapshot from the data server's pool registry
/// alone (no DexScreener/GeckoTerminal). Used as the fast fallback when a chart
/// is opened while a slow background refresh already holds this mint's slot, so
/// the OHLCV monitor gets a usable SOL pool in ~200ms instead of waiting on the
/// rate-limited pipeline. Returns `None` when the server is disabled/misses.
async fn server_only_snapshot(chain: ChainId, mint: &str) -> Option<TokenPoolsSnapshot> {
    let server_pools = super::server::fetch_pools_from_server(mint).await?;
    if server_pools.is_empty() {
        return None;
    }

    let mut pools_map: HashMap<String, TokenPoolInfo> = HashMap::new();
    for info in server_pools {
        super::operations::ingest_pool_entry(&mut pools_map, info);
    }
    let mut pools: Vec<TokenPoolInfo> = pools_map.into_values().collect();
    sort_pools_for_snapshot(&mut pools);
    let canonical_pool_address = choose_canonical_pool(&pools);

    let snapshot = TokenPoolsSnapshot {
        mint: mint.to_string(),
        pools,
        canonical_pool_address,
        pool_data_last_fetched_at: Utc::now(),
    };

    // Persist so the OHLCV monitor and other consumers see it immediately; the
    // in-flight full refresh will overwrite it with the enriched snapshot.
    let _ = database::replace_token_pools_async(snapshot.clone()).await;
    store_pool_snapshot(chain, snapshot.clone());
    Some(snapshot)
}

/// Clear pool cache (for testing/reset)
pub fn clear_cache(chain: ChainId) {
    TOKEN_POOLS_CACHE.invalidate_entries_if(move |(entry_chain, _), _| *entry_chain == chain);
    POOL_PREFETCH_STATE.invalidate_entries_if(move |(entry_chain, _), _| *entry_chain == chain);
}

/// Get pool cache metrics
pub fn metrics() -> PoolCacheMetrics {
    TOKEN_POOLS_CACHE.run_pending_tasks();
    let entries = TOKEN_POOLS_CACHE.entry_count() as usize;
    // moka auto-evicts expired entries, so all remaining are within TTL window
    PoolCacheMetrics {
        entries,
        fresh_entries: entries,
        stale_entries: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_cache_keys_always_include_the_explicit_chain() {
        let key = pool_cache_key(ChainId::Solana, " mint ");

        assert_eq!(key, (ChainId::Solana, "mint".to_owned()));
    }
}
