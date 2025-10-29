use crate::apis::dexscreener::types::DexScreenerPool;
use crate::apis::geckoterminal::types::GeckoTerminalPool;
use crate::apis::manager::get_api_manager;
use crate::logger::{self, LogTag};
use crate::pools::utils::is_sol_mint;
use crate::sol_price::get_sol_price;
use crate::tokens::database;
use crate::tokens::service::get_rate_coordinator;
use crate::tokens::types::{
    DexScreenerData, GeckoTerminalData, RugcheckData, Token, TokenError, TokenPoolInfo,
    TokenPoolSources, TokenPoolsSnapshot, TokenResult,
};
use crate::tokens::updates::RateLimitCoordinator;
use chrono::Utc;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{hash_map::Entry, HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex as AsyncMutex, Notify};

const TOKEN_SNAPSHOT_TTL_SECS: u64 = 30;
const DEXSCREENER_TTL_SECS: u64 = 30;
const GECKOTERMINAL_TTL_SECS: u64 = 60;
const RUGCHECK_TTL_SECS: u64 = 1800;
const MARKET_CACHE_CAPACITY: usize = 2000;
const SECURITY_CACHE_CAPACITY: usize = 3000;
const TOKEN_POOLS_TTL_SECS: u64 = 60;
const POOL_PREFETCH_DEBOUNCE_SECS: u64 = 20;

#[derive(Clone, Debug, Default)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub inserts: u64,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[derive(Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
}

impl<V> CacheEntry<V> {
    fn new(value: V) -> Self {
        let now = Instant::now();
        Self {
            value,
            inserted_at: now,
            last_accessed: now,
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() > ttl
    }

    fn touch(&mut self) {
        self.last_accessed = Instant::now();
    }
}

struct TimedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    ttl: Duration,
    capacity: usize,
    inner: Mutex<TimedCacheInner<K, V>>,
}

struct TimedCacheInner<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    map: HashMap<K, CacheEntry<V>>,
    order: VecDeque<K>,
    metrics: CacheMetrics,
}

impl<K, V> TimedCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            ttl,
            capacity,
            inner: Mutex::new(TimedCacheInner {
                map: HashMap::new(),
                order: VecDeque::new(),
                metrics: CacheMetrics::default(),
            }),
        }
    }

    fn get(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.lock().expect("cache poisoned");

        if let Some(entry) = inner.map.get_mut(key) {
            if entry.is_expired(self.ttl) {
                inner.map.remove(key);
                inner.order.retain(|k| k != key);
                inner.metrics.misses += 1;
                inner.metrics.expirations += 1;
                return None;
            }

            entry.touch();
            let value = entry.value.clone();
            let key_clone = key.clone();
            drop(entry);
            inner.order.retain(|k| k != key);
            inner.order.push_back(key_clone);
            inner.metrics.hits += 1;
            return Some(value);
        }

        inner.metrics.misses += 1;
        None
    }

    fn insert(&self, key: K, value: V) {
        let mut inner = self.inner.lock().expect("cache poisoned");

        if !inner.map.contains_key(&key) && inner.map.len() >= self.capacity {
            if let Some(lru) = inner.order.pop_front() {
                inner.map.remove(&lru);
                inner.metrics.evictions += 1;
            }
        }

        inner.map.insert(key.clone(), CacheEntry::new(value));
        inner.order.retain(|k| k != &key);
        inner.order.push_back(key);
        inner.metrics.inserts += 1;
    }

    fn len(&self) -> usize {
        let inner = self.inner.lock().expect("cache poisoned");
        inner.map.len()
    }

    fn metrics(&self) -> CacheMetrics {
        let inner = self.inner.lock().expect("cache poisoned");
        inner.metrics.clone()
    }

    fn clear(&self) {
        let mut inner = self.inner.lock().expect("cache poisoned");
        inner.map.clear();
        inner.order.clear();
    }
}

#[derive(Clone)]
struct TokenEntry {
    token: Token,
    refreshed_at: Instant,
}

#[derive(Clone)]
struct TokenPoolCacheEntry {
    snapshot: TokenPoolsSnapshot,
    refreshed_at: Instant,
}

struct TokenStore {
    ttl: Duration,
    entries: RwLock<HashMap<String, TokenEntry>>,
}

impl TokenStore {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, mint: &str) -> Option<Token> {
        let guard = self.entries.read().expect("token store poisoned");
        let entry = guard.get(mint)?;
        if entry.refreshed_at.elapsed() > self.ttl {
            drop(guard);
            self.invalidate(mint);
            None
        } else {
            Some(entry.token.clone())
        }
    }

    fn set(&self, token: Token) {
        let mut guard = self.entries.write().expect("token store poisoned");
        guard.insert(
            token.mint.clone(),
            TokenEntry {
                token,
                refreshed_at: Instant::now(),
            },
        );
    }

    fn invalidate(&self, mint: &str) {
        let mut guard = self.entries.write().expect("token store poisoned");
        guard.remove(mint);
    }
}

static TOKEN_STORE: Lazy<TokenStore> =
    Lazy::new(|| TokenStore::new(Duration::from_secs(TOKEN_SNAPSHOT_TTL_SECS)));

static TOKEN_POOLS_CACHE: Lazy<RwLock<HashMap<String, TokenPoolCacheEntry>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static POOL_REFRESH_INFLIGHT: Lazy<AsyncMutex<HashMap<String, Arc<Notify>>>> =
    Lazy::new(|| AsyncMutex::new(HashMap::new()));

static POOL_PREFETCH_STATE: Lazy<AsyncMutex<HashMap<String, Instant>>> =
    Lazy::new(|| AsyncMutex::new(HashMap::new()));

fn pool_cache_ttl() -> Duration {
    Duration::from_secs(TOKEN_POOLS_TTL_SECS)
}

fn refreshed_at_from_snapshot(snapshot: &TokenPoolsSnapshot) -> Instant {
    let now = Instant::now();
    let age_secs = Utc::now()
        .signed_duration_since(snapshot.fetched_at)
        .num_seconds()
        .max(0) as u64;
    now.checked_sub(Duration::from_secs(age_secs))
        .unwrap_or(now)
}

fn is_pool_entry_fresh(entry: &TokenPoolCacheEntry) -> bool {
    entry.refreshed_at.elapsed() <= pool_cache_ttl()
}

fn get_cached_pool_snapshot(mint: &str) -> Option<TokenPoolsSnapshot> {
    let guard = TOKEN_POOLS_CACHE.read().ok()?;
    let entry = guard.get(mint)?;
    if is_pool_entry_fresh(entry) {
        Some(entry.snapshot.clone())
    } else {
        None
    }
}

fn get_cached_pool_snapshot_allow_stale(mint: &str) -> Option<TokenPoolsSnapshot> {
    let guard = TOKEN_POOLS_CACHE.read().ok()?;
    guard.get(mint).map(|entry| entry.snapshot.clone())
}

fn store_pool_snapshot(snapshot: TokenPoolsSnapshot) {
    let mut guard = TOKEN_POOLS_CACHE
        .write()
        .expect("token pools cache poisoned");
    guard.insert(
        snapshot.mint.clone(),
        TokenPoolCacheEntry {
            refreshed_at: refreshed_at_from_snapshot(&snapshot),
            snapshot,
        },
    );
}

fn remove_pool_snapshot(mint: &str) {
    let mut guard = TOKEN_POOLS_CACHE
        .write()
        .expect("token pools cache poisoned");
    guard.remove(mint);
}

static DEXSCREENER_CACHE: Lazy<TimedCache<String, DexScreenerData>> = Lazy::new(|| {
    TimedCache::new(
        Duration::from_secs(DEXSCREENER_TTL_SECS),
        MARKET_CACHE_CAPACITY,
    )
});

static GECKOTERMINAL_CACHE: Lazy<TimedCache<String, GeckoTerminalData>> = Lazy::new(|| {
    TimedCache::new(
        Duration::from_secs(GECKOTERMINAL_TTL_SECS),
        MARKET_CACHE_CAPACITY,
    )
});

static RUGCHECK_CACHE: Lazy<TimedCache<String, RugcheckData>> = Lazy::new(|| {
    TimedCache::new(
        Duration::from_secs(RUGCHECK_TTL_SECS),
        SECURITY_CACHE_CAPACITY,
    )
});

pub fn get_cached_dexscreener(mint: &str) -> Option<DexScreenerData> {
    DEXSCREENER_CACHE.get(&mint.to_string())
}

pub fn store_dexscreener(mint: &str, data: &DexScreenerData) {
    DEXSCREENER_CACHE.insert(mint.to_string(), data.clone());
}

pub fn dexscreener_cache_metrics() -> CacheMetrics {
    DEXSCREENER_CACHE.metrics()
}

pub fn dexscreener_cache_size() -> usize {
    DEXSCREENER_CACHE.len()
}

pub fn get_cached_geckoterminal(mint: &str) -> Option<GeckoTerminalData> {
    GECKOTERMINAL_CACHE.get(&mint.to_string())
}

pub fn store_geckoterminal(mint: &str, data: &GeckoTerminalData) {
    GECKOTERMINAL_CACHE.insert(mint.to_string(), data.clone());
}

pub fn geckoterminal_cache_metrics() -> CacheMetrics {
    GECKOTERMINAL_CACHE.metrics()
}

pub fn geckoterminal_cache_size() -> usize {
    GECKOTERMINAL_CACHE.len()
}

pub fn get_cached_rugcheck(mint: &str) -> Option<RugcheckData> {
    RUGCHECK_CACHE.get(&mint.to_string())
}

pub fn store_rugcheck(mint: &str, data: &RugcheckData) {
    RUGCHECK_CACHE.insert(mint.to_string(), data.clone());
}

pub fn rugcheck_cache_metrics() -> CacheMetrics {
    RUGCHECK_CACHE.metrics()
}

pub fn rugcheck_cache_size() -> usize {
    RUGCHECK_CACHE.len()
}

pub fn get_cached_token(mint: &str) -> Option<Token> {
    TOKEN_STORE.get(mint)
}

pub fn store_token_snapshot(token: Token) {
    TOKEN_STORE.set(token);
}

pub fn invalidate_token_snapshot(mint: &str) {
    TOKEN_STORE.invalidate(mint);
}

pub async fn refresh_token_snapshot(mint: &str) -> TokenResult<Option<Token>> {
    let token = database::get_full_token_async(mint).await?;
    match token.clone() {
        Some(snapshot) => store_token_snapshot(snapshot),
        None => invalidate_token_snapshot(mint),
    }
    Ok(token)
}

pub async fn get_full_token_async(mint: &str) -> TokenResult<Option<Token>> {
    if let Some(token) = get_cached_token(mint) {
        return Ok(Some(token));
    }
    refresh_token_snapshot(mint).await
}

pub fn clear_all_market_caches() {
    DEXSCREENER_CACHE.clear();
    GECKOTERMINAL_CACHE.clear();
}

pub fn clear_security_cache() {
    RUGCHECK_CACHE.clear();
}

pub fn clear_pool_cache() {
    if let Ok(mut guard) = TOKEN_POOLS_CACHE.write() {
        guard.clear();
    }

    if let Ok(mut guard) = POOL_PREFETCH_STATE.try_lock() {
        guard.clear();
    }

}
fn parse_gecko_token_id(value: &str) -> Option<String> {
    let candidate = value
        .trim()
        .rsplit(|c| c == ':' || c == '_')
        .next()
        .unwrap_or(value)
        .trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

fn merge_pool_sources(target: &mut TokenPoolSources, incoming: TokenPoolSources) {
    if incoming.dexscreener.is_some() {
        target.dexscreener = incoming.dexscreener;
    }
    if incoming.geckoterminal.is_some() {
        target.geckoterminal = incoming.geckoterminal;
    }
}

fn merge_pool_info(target: &mut TokenPoolInfo, incoming: TokenPoolInfo) {
    if target.dex.is_none() {
        target.dex = incoming.dex.clone();
    }

    if !target.is_sol_pair && incoming.is_sol_pair {
        target.is_sol_pair = true;
    }

    if let Some(liquidity_usd) = incoming.liquidity_usd {
        target.liquidity_usd = Some(match target.liquidity_usd {
            Some(existing) => existing.max(liquidity_usd),
            None => liquidity_usd,
        });
    }

    if let Some(liquidity_token) = incoming.liquidity_token {
        target.liquidity_token = Some(match target.liquidity_token {
            Some(existing) => existing.max(liquidity_token),
            None => liquidity_token,
        });
    }

    if let Some(liquidity_sol) = incoming.liquidity_sol {
        target.liquidity_sol = Some(match target.liquidity_sol {
            Some(existing) => existing.max(liquidity_sol),
            None => liquidity_sol,
        });
    }

    if let Some(volume) = incoming.volume_h24 {
        target.volume_h24 = Some(match target.volume_h24 {
            Some(existing) => existing.max(volume),
            None => volume,
        });
    }

    if let Some(price_usd) = incoming.price_usd {
        target.price_usd = Some(price_usd);
    }

    if let Some(price_sol) = incoming.price_sol {
        target.price_sol = Some(price_sol);
    }

    if incoming.price_native.is_some() {
        target.price_native = incoming.price_native.clone();
    }

    target.fetched_at = target.fetched_at.max(incoming.fetched_at);
    merge_pool_sources(&mut target.sources, incoming.sources);
}

fn pool_metric(pool: &TokenPoolInfo) -> f64 {
    pool.liquidity_sol
        .or(pool.liquidity_usd)
        .or(pool.volume_h24)
        .unwrap_or(0.0)
}

fn choose_canonical_pool(pools: &[TokenPoolInfo]) -> Option<String> {
    pools
        .iter()
        .filter(|pool| pool.is_sol_pair)
        .max_by(|a, b| {
            let metric_a = pool_metric(a);
            let metric_b = pool_metric(b);
            match metric_a
                .partial_cmp(&metric_b)
                .unwrap_or(Ordering::Equal)
            {
                Ordering::Equal => {
                    let vol_a = a.volume_h24.unwrap_or(0.0);
                    let vol_b = b.volume_h24.unwrap_or(0.0);
                    vol_a
                        .partial_cmp(&vol_b)
                        .unwrap_or(Ordering::Equal)
                }
                ordering => ordering,
            }
        })
        .map(|pool| pool.pool_address.clone())
}

fn sort_pools_for_snapshot(pools: &mut [TokenPoolInfo]) {
    pools.sort_by(|a, b| {
        match (b.is_sol_pair, a.is_sol_pair) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => match pool_metric(b)
                .partial_cmp(&pool_metric(a))
                .unwrap_or(Ordering::Equal)
            {
                Ordering::Equal => a
                    .pool_address
                    .cmp(&b.pool_address),
                ordering => ordering,
            },
        }
    });
}

fn snapshot_is_fresh(snapshot: &TokenPoolsSnapshot) -> bool {
    let age = Utc::now()
        .signed_duration_since(snapshot.fetched_at)
        .num_seconds();
    age >= 0 && age <= TOKEN_POOLS_TTL_SECS as i64
}

fn ingest_pool_entry(map: &mut HashMap<String, TokenPoolInfo>, info: TokenPoolInfo) {
    if info.pool_address.is_empty() {
        return;
    }

    match map.entry(info.pool_address.clone()) {
        Entry::Vacant(slot) => {
            slot.insert(info);
        }
        Entry::Occupied(mut slot) => {
            merge_pool_info(slot.get_mut(), info);
        }
    }
}

fn convert_dexscreener_pool(pool: &DexScreenerPool) -> Option<TokenPoolInfo> {
    if pool.pair_address.trim().is_empty() {
        return None;
    }

    let base_mint = pool.base_token_address.trim();
    let quote_mint = pool.quote_token_address.trim();

    if base_mint.is_empty() || quote_mint.is_empty() {
        return None;
    }

    let price_usd = parse_f64(&pool.price_usd);
    let price_sol = parse_f64(&pool.price_native);
    let price_native = if pool.price_native.trim().is_empty() {
        None
    } else {
        Some(pool.price_native.clone())
    };

    let liquidity_token = pool.liquidity_base;
    let liquidity_sol = if is_sol_mint(quote_mint) {
        pool.liquidity_quote
    } else if is_sol_mint(base_mint) {
        pool.liquidity_base
    } else {
        None
    };

    let sources = TokenPoolSources {
        dexscreener: serde_json::to_value(pool).ok(),
        ..TokenPoolSources::default()
    };

    Some(TokenPoolInfo {
        pool_address: pool.pair_address.clone(),
        dex: if pool.dex_id.trim().is_empty() {
            None
        } else {
            Some(pool.dex_id.clone())
        },
        base_mint: base_mint.to_string(),
        quote_mint: quote_mint.to_string(),
        is_sol_pair: is_sol_mint(base_mint) || is_sol_mint(quote_mint),
        liquidity_usd: pool.liquidity_usd,
        liquidity_token,
        liquidity_sol,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native,
        sources,
        fetched_at: pool.fetched_at,
    })
}

fn convert_geckoterminal_pool(pool: &GeckoTerminalPool, sol_price_usd: f64) -> Option<TokenPoolInfo> {
    if pool.pool_address.trim().is_empty() {
        return None;
    }

    let base_mint = parse_gecko_token_id(&pool.base_token_id).unwrap_or_else(|| pool.base_token_id.clone());
    let quote_mint = parse_gecko_token_id(&pool.quote_token_id).unwrap_or_else(|| pool.quote_token_id.clone());

    if base_mint.is_empty() || quote_mint.is_empty() {
        return None;
    }

    let is_sol_pair = is_sol_mint(&base_mint) || is_sol_mint(&quote_mint);

    let (price_native_str, price_usd) = if pool.mint == base_mint {
        (
            if pool.base_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.base_token_price_native.clone())
            },
            parse_f64(&pool.base_token_price_usd)
                .or_else(|| parse_f64(&pool.token_price_usd)),
        )
    } else if pool.mint == quote_mint {
        (
            if pool.quote_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.quote_token_price_native.clone())
            },
            parse_f64(&pool.quote_token_price_usd)
                .or_else(|| parse_f64(&pool.token_price_usd)),
        )
    } else {
        (
            if pool.base_token_price_native.trim().is_empty() {
                None
            } else {
                Some(pool.base_token_price_native.clone())
            },
            parse_f64(&pool.token_price_usd),
        )
    };

    let price_sol = price_native_str
        .as_ref()
        .and_then(|value| parse_f64(value))
        .or_else(|| {
            price_usd.and_then(|usd| {
                if sol_price_usd > 0.0 {
                    Some(usd / sol_price_usd)
                } else {
                    None
                }
            })
        });

    let liquidity_usd = pool.reserve_usd;
    let liquidity_sol = if is_sol_pair && sol_price_usd > 0.0 {
        liquidity_usd.map(|usd| (usd / 2.0) / sol_price_usd)
    } else {
        None
    };

    let sources = TokenPoolSources {
        geckoterminal: serde_json::to_value(pool).ok(),
        ..TokenPoolSources::default()
    };

    Some(TokenPoolInfo {
        pool_address: pool.pool_address.clone(),
        dex: if pool.dex_id.trim().is_empty() {
            None
        } else {
            Some(pool.dex_id.clone())
        },
        base_mint,
        quote_mint,
        is_sol_pair,
        liquidity_usd,
        liquidity_token: None,
        liquidity_sol,
        volume_h24: pool.volume_h24,
        price_usd,
        price_sol,
        price_native: price_native_str,
        sources,
        fetched_at: pool.fetched_at,
    })
}

async fn fetch_pools_from_sources(
    mint: &str,
    coordinator: Arc<RateLimitCoordinator>,
) -> TokenResult<(HashMap<String, TokenPoolInfo>, usize)> {
    let api = get_api_manager();
    let sol_price = get_sol_price();

    let should_fetch_dex = api.dexscreener.is_enabled();
    let should_fetch_gecko = api.geckoterminal.is_enabled();

    let mint_owned = mint.to_string();

    let dex_future = {
        let api = api.clone();
        let coordinator = coordinator.clone();
        let mint = mint_owned.clone();
        async move {
            if should_fetch_dex {
                coordinator.acquire_dexscreener_pools().await?;
                api.dexscreener
                    .fetch_token_pools(&mint, None)
                    .await
                    .map_err(|e| TokenError::Api {
                        source: "DexScreener".to_string(),
                        message: e,
                    })
            } else {
                Ok(Vec::new())
            }
        }
    };

    let gecko_future = {
        let api = api.clone();
        let coordinator = coordinator.clone();
        let mint = mint_owned.clone();
        async move {
            if should_fetch_gecko {
                coordinator.acquire_geckoterminal().await?;
                api.geckoterminal
                    .fetch_pools(&mint)
                    .await
                    .map_err(|e| TokenError::Api {
                        source: "GeckoTerminal".to_string(),
                        message: e,
                    })
            } else {
                Ok(Vec::new())
            }
        }
    };

    let (dex_result, gecko_result) = tokio::join!(dex_future, gecko_future);

    let mut pools_map: HashMap<String, TokenPoolInfo> = HashMap::new();
    let mut success_sources = 0usize;
    let mut failures: Vec<String> = Vec::new();

    match dex_result {
        Ok(pools) => {
            if should_fetch_dex {
                success_sources += 1;
            }
            for pool in pools.iter() {
                if let Some(info) = convert_dexscreener_pool(pool) {
                    ingest_pool_entry(&mut pools_map, info);
                }
            }
        }
        Err(err) => {
            let message = err.to_string();
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] DexScreener fetch failed for mint={}: {}",
                    mint, message
                ),
            );
            failures.push(format!("DexScreener→{}", message));
        }
    }

    match gecko_result {
        Ok(pools) => {
            if should_fetch_gecko {
                success_sources += 1;
            }
            for pool in pools.iter() {
                if let Some(info) = convert_geckoterminal_pool(pool, sol_price) {
                    ingest_pool_entry(&mut pools_map, info);
                }
            }
        }
        Err(err) => {
            let message = err.to_string();
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] GeckoTerminal fetch failed for mint={}: {}",
                    mint, message
                ),
            );
            failures.push(format!("GeckoTerminal→{}", message));
        }
    }

    let attempted_sources = (should_fetch_dex as usize) + (should_fetch_gecko as usize);
    if attempted_sources > 0 && success_sources == 0 {
        let combined = if failures.is_empty() {
            "all pool sources failed without details".to_string()
        } else {
            failures.join(" | ")
        };
        return Err(TokenError::Api {
            source: "TokenPools".to_string(),
            message: combined,
        });
    }

    if attempted_sources == 0 {
        logger::warning(
            LogTag::Tokens,
            &format!(
                "[TOKEN_POOLS] No pool sources enabled for mint={} – returning empty snapshot",
                mint
            ),
        );
    }

    Ok((pools_map, success_sources))
}

async fn refresh_token_pools_and_cache(
    mint: &str,
    allow_stale: bool,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let mint_trimmed = mint.trim();
    if mint_trimmed.is_empty() {
        return Err(TokenError::InvalidMint("Mint address cannot be empty".to_string()));
    }

    // Fast path: use cached snapshot if already loaded and fresh
    if let Some(snapshot) = get_cached_pool_snapshot(mint_trimmed) {
        if snapshot_is_fresh(&snapshot) || allow_stale {
            return Ok(Some(snapshot));
        }
    }

    // Pull persisted snapshot for reuse/fallback
    let persisted_snapshot = database::get_token_pools_async(mint_trimmed).await?;
    if let Some(snapshot) = persisted_snapshot.as_ref() {
        if snapshot_is_fresh(snapshot) {
            store_pool_snapshot(snapshot.clone());
            return Ok(Some(snapshot.clone()));
        }
    }

    let coordinator = get_rate_coordinator().ok_or_else(|| {
        TokenError::Database("Rate limit coordinator not initialized".to_string())
    })?;

    let (pools_map, success_sources) = match fetch_pools_from_sources(mint_trimmed, coordinator.clone()).await {
        Ok(result) => result,
        Err(err) => {
            if allow_stale {
                if let Some(snapshot) = persisted_snapshot {
                    logger::warning(
                        LogTag::Tokens,
                        &format!(
                            "[TOKEN_POOLS] Falling back to stale snapshot for mint={} due to error: {}",
                            mint_trimmed, err
                        ),
                    );
                    store_pool_snapshot(snapshot.clone());
                    return Ok(Some(snapshot));
                }
            }
            return Err(err);
        }
    };

    if pools_map.is_empty() && success_sources == 0 {
        if let Some(snapshot) = persisted_snapshot {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[TOKEN_POOLS] No pool sources available for mint={} – using persisted snapshot",
                    mint_trimmed
                ),
            );
            store_pool_snapshot(snapshot.clone());
            return Ok(Some(snapshot));
        }
    }

    let mut pools: Vec<TokenPoolInfo> = pools_map.into_values().collect();
    sort_pools_for_snapshot(&mut pools);
    let canonical_pool_address = choose_canonical_pool(&pools);

    let snapshot = TokenPoolsSnapshot {
        mint: mint_trimmed.to_string(),
        pools,
        canonical_pool_address,
        fetched_at: Utc::now(),
    };

    database::replace_token_pools_async(snapshot.clone()).await?;
    store_pool_snapshot(snapshot.clone());

    let (top_pool, top_metric) = snapshot
        .pools
        .first()
        .map(|pool| (pool.pool_address.clone(), pool_metric(pool)))
        .unwrap_or_else(|| ("none".to_string(), 0.0));

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

    Ok(Some(snapshot))
}

async fn get_token_pools_snapshot_internal(
    mint: &str,
    allow_stale: bool,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    let trimmed = mint.trim();
    if trimmed.is_empty() {
        return Err(TokenError::InvalidMint("Mint address cannot be empty".to_string()));
    }

    if let Some(snapshot) = get_cached_pool_snapshot(trimmed) {
        if snapshot_is_fresh(&snapshot) || allow_stale {
            return Ok(Some(snapshot));
        }
    } else if allow_stale {
        if let Some(snapshot) = get_cached_pool_snapshot_allow_stale(trimmed) {
            return Ok(Some(snapshot));
        }
    }

    let (should_refresh, notifier) = {
        let mut guard = POOL_REFRESH_INFLIGHT
            .lock()
            .await;
        match guard.entry(trimmed.to_string()) {
            Entry::Occupied(entry) => (false, entry.get().clone()),
            Entry::Vacant(entry) => {
                let notify = Arc::new(Notify::new());
                entry.insert(notify.clone());
                (true, notify)
            }
        }
    };

    if !should_refresh {
        notifier.notified().await;
        if allow_stale {
            if let Some(snapshot) = get_cached_pool_snapshot_allow_stale(trimmed) {
                return Ok(Some(snapshot));
            }
            return database::get_token_pools_async(trimmed).await;
        }
        return Ok(get_cached_pool_snapshot(trimmed));
    }

    let result = refresh_token_pools_and_cache(trimmed, allow_stale).await;

    {
        let mut guard = POOL_REFRESH_INFLIGHT
            .lock()
            .await;
        guard.remove(trimmed);
    }
    notifier.notify_waiters();

    result
}

pub async fn get_token_pools_snapshot(mint: &str) -> TokenResult<Option<TokenPoolsSnapshot>> {
    get_token_pools_snapshot_internal(mint, false).await
}

pub async fn get_token_pools_snapshot_allow_stale(
    mint: &str,
) -> TokenResult<Option<TokenPoolsSnapshot>> {
    get_token_pools_snapshot_internal(mint, true).await
}

pub async fn prefetch_token_pools(mints: &[String]) {
    if mints.is_empty() {
        return;
    }

    let now = Instant::now();
    let mut schedule: Vec<String> = Vec::new();

    {
        let mut prefetch_state = POOL_PREFETCH_STATE.lock().await;
        for mint in mints {
            let trimmed = mint.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(snapshot) = get_cached_pool_snapshot(trimmed) {
                if snapshot_is_fresh(&snapshot) {
                    continue;
                }
            }

            if let Some(last) = prefetch_state.get(trimmed) {
                if now.duration_since(*last) < Duration::from_secs(POOL_PREFETCH_DEBOUNCE_SECS) {
                    continue;
                }
            }

            prefetch_state.insert(trimmed.to_string(), now);
            schedule.push(trimmed.to_string());
        }
    }

    for mint in schedule {
        tokio::spawn(async move {
            if let Err(err) = get_token_pools_snapshot_internal(&mint, true).await {
                logger::warning(
                    LogTag::Tokens,
                    &format!(
                        "[TOKEN_POOLS] Prefetch failed for mint={} error={}",
                        mint, err
                    ),
                );
            }
        });
    }
}
