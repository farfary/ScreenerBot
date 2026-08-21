//! Token store — central in-memory store for all discovered tokens with thread-safe access.

use crate::chains::ChainId;
use crate::tokens::database;
use crate::tokens::types::{DexScreenerData, GeckoTerminalData, RugcheckData, Token, TokenResult};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const TOKEN_SNAPSHOT_TTL_SECS: u64 = 30;
const DEXSCREENER_TTL_SECS: u64 = 30;
const GECKOTERMINAL_TTL_SECS: u64 = 60;
const RUGCHECK_TTL_SECS: u64 = 300; // 5 minutes - security data shouldn't be stale
const MARKET_CACHE_CAPACITY: u64 = 2000;
const SECURITY_CACHE_CAPACITY: u64 = 3000;

#[derive(Clone, Debug, Default)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub expirations: u64,
    pub inserts: u64,
}

impl CacheMetrics {
    /// Calculate cache hit rate as a fraction (0.0 to 1.0)
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
struct TokenEntry {
    token: Token,
    refreshed_at: Instant,
}

struct TokenStore {
    ttl: Duration,
    entries: RwLock<HashMap<(ChainId, String), TokenEntry>>,
}

impl TokenStore {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            entries: RwLock::new(HashMap::new()),
        }
    }

    fn get(&self, chain: ChainId, mint: &str) -> Option<Token> {
        let mut stale_marker: Option<Instant> = None;

        {
            let guard = self.entries.read().expect("token store poisoned");
            let entry = match guard.get(&(chain, mint.to_owned())) {
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
            if let Some(entry) = guard.get(&(chain, mint.to_owned())) {
                let is_same_entry = entry.refreshed_at == expected_refreshed_at;
                let still_expired = entry.refreshed_at.elapsed() > self.ttl;
                if is_same_entry && still_expired {
                    guard.remove(&(chain, mint.to_owned()));
                }
            }
        }

        None
    }

    fn set(&self, chain: ChainId, token: Token) {
        let mut guard = self.entries.write().expect("token store poisoned");
        guard.insert(
            (chain, token.mint.clone()),
            TokenEntry {
                token,
                refreshed_at: Instant::now(),
            },
        );
    }

    fn invalidate(&self, chain: ChainId, mint: &str) {
        let mut guard = self.entries.write().expect("token store poisoned");
        guard.remove(&(chain, mint.to_owned()));
    }
}

static TOKEN_STORE: LazyLock<TokenStore> =
    LazyLock::new(|| TokenStore::new(Duration::from_secs(TOKEN_SNAPSHOT_TTL_SECS)));

type CacheKey = (ChainId, String);

fn cache_key(chain: ChainId, mint: &str) -> CacheKey {
    (chain, mint.to_owned())
}

static DEXSCREENER_CACHE: LazyLock<moka::sync::Cache<CacheKey, DexScreenerData>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(MARKET_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(DEXSCREENER_TTL_SECS))
            .build()
    });

static GECKOTERMINAL_CACHE: LazyLock<moka::sync::Cache<CacheKey, GeckoTerminalData>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(MARKET_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(GECKOTERMINAL_TTL_SECS))
            .build()
    });

static RUGCHECK_CACHE: LazyLock<moka::sync::Cache<CacheKey, RugcheckData>> = LazyLock::new(|| {
    moka::sync::Cache::builder()
        .max_capacity(SECURITY_CACHE_CAPACITY)
        .time_to_live(Duration::from_secs(RUGCHECK_TTL_SECS))
        .build()
});

/// Retrieve cached DexScreener data for a token mint
pub fn get_cached_dexscreener(chain: ChainId, mint: &str) -> Option<DexScreenerData> {
    DEXSCREENER_CACHE.get(&cache_key(chain, mint))
}

/// Cache DexScreener data for a token mint
pub fn store_dexscreener(chain: ChainId, mint: &str, data: &DexScreenerData) {
    DEXSCREENER_CACHE.insert(cache_key(chain, mint), data.clone());
}

/// Get DexScreener cache usage metrics
pub fn dexscreener_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: DEXSCREENER_CACHE.entry_count(),
    }
}

/// Number of entries in the DexScreener cache
pub fn dexscreener_cache_size() -> usize {
    DEXSCREENER_CACHE.entry_count() as usize
}

/// Retrieve cached GeckoTerminal data for a token mint
pub fn get_cached_geckoterminal(chain: ChainId, mint: &str) -> Option<GeckoTerminalData> {
    GECKOTERMINAL_CACHE.get(&cache_key(chain, mint))
}

/// Cache GeckoTerminal data for a token mint
pub fn store_geckoterminal(chain: ChainId, mint: &str, data: &GeckoTerminalData) {
    GECKOTERMINAL_CACHE.insert(cache_key(chain, mint), data.clone());
}

/// Get GeckoTerminal cache usage metrics
pub fn geckoterminal_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: GECKOTERMINAL_CACHE.entry_count(),
    }
}

/// Number of entries in the GeckoTerminal cache
pub fn geckoterminal_cache_size() -> usize {
    GECKOTERMINAL_CACHE.entry_count() as usize
}

/// Retrieve cached Rugcheck data for a token mint
pub fn get_cached_rugcheck(chain: ChainId, mint: &str) -> Option<RugcheckData> {
    RUGCHECK_CACHE.get(&cache_key(chain, mint))
}

/// Cache Rugcheck data for a token mint
pub fn store_rugcheck(chain: ChainId, mint: &str, data: &RugcheckData) {
    RUGCHECK_CACHE.insert(cache_key(chain, mint), data.clone());
}

/// Get Rugcheck cache usage metrics
pub fn rugcheck_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: RUGCHECK_CACHE.entry_count(),
    }
}

/// Number of entries in the Rugcheck cache
pub fn rugcheck_cache_size() -> usize {
    RUGCHECK_CACHE.entry_count() as usize
}

/// Retrieve a cached assembled token snapshot
pub fn get_cached_token(chain: ChainId, mint: &str) -> Option<Token> {
    TOKEN_STORE.get(chain, mint)
}

/// Cache an assembled token snapshot
pub fn store_token_snapshot(chain: ChainId, token: Token) {
    TOKEN_STORE.set(chain, token);
}

/// Remove a token snapshot from the cache
pub fn invalidate_token_snapshot(chain: ChainId, mint: &str) {
    TOKEN_STORE.invalidate(chain, mint);
}

/// Rebuild and cache a token snapshot from database
pub async fn refresh_token_snapshot(chain: ChainId, mint: &str) -> TokenResult<Option<Token>> {
    let token = database::get_full_token_async(mint).await?;
    match token.clone() {
        Some(snapshot) => store_token_snapshot(chain, snapshot),
        None => invalidate_token_snapshot(chain, mint),
    }
    Ok(token)
}

/// Get a full token, using cache if available or rebuilding from database
pub async fn get_full_token_async(chain: ChainId, mint: &str) -> TokenResult<Option<Token>> {
    if let Some(token) = get_cached_token(chain, mint) {
        return Ok(Some(token));
    }
    refresh_token_snapshot(chain, mint).await
}

/// Invalidate all DexScreener and GeckoTerminal cache entries
pub fn clear_all_market_caches() {
    DEXSCREENER_CACHE.invalidate_all();
    GECKOTERMINAL_CACHE.invalidate_all();
}

/// Invalidate all Rugcheck cache entries
pub fn clear_security_cache() {
    RUGCHECK_CACHE.invalidate_all();
}
