use crate::tokens::database;
use crate::tokens::types::{
    DexScreenerData, GeckoTerminalData, RugcheckData, Token, TokenResult,
};
use once_cell::sync::Lazy;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

const TOKEN_SNAPSHOT_TTL_SECS: u64 = 30;
const DEXSCREENER_TTL_SECS: u64 = 30;
const GECKOTERMINAL_TTL_SECS: u64 = 60;
const RUGCHECK_TTL_SECS: u64 = 1800;
const MARKET_CACHE_CAPACITY: usize = 2000;
const SECURITY_CACHE_CAPACITY: usize = 3000;

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
        let mut stale_marker: Option<Instant> = None;

        {
            let guard = self.entries.read().expect("token store poisoned");
            let entry = match guard.get(mint) {
                Some(entry) => entry,
                None => return None,
            };

            if entry.refreshed_at.elapsed() <= self.ttl {
                return Some(entry.token.clone());
            }

            stale_marker = Some(entry.refreshed_at);
        }

        if let Some(expected_refreshed_at) = stale_marker {
            let mut guard = self.entries.write().expect("token store poisoned");
            if let Some(entry) = guard.get(mint) {
                let is_same_entry = entry.refreshed_at == expected_refreshed_at;
                let still_expired = entry.refreshed_at.elapsed() > self.ttl;
                if is_same_entry && still_expired {
                    guard.remove(mint);
                }
            }
        }

        None
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

