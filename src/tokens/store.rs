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

static TOKEN_STORE: LazyLock<TokenStore> =
    LazyLock::new(|| TokenStore::new(Duration::from_secs(TOKEN_SNAPSHOT_TTL_SECS)));

static DEXSCREENER_CACHE: LazyLock<moka::sync::Cache<String, DexScreenerData>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(MARKET_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(DEXSCREENER_TTL_SECS))
            .build()
    });

static GECKOTERMINAL_CACHE: LazyLock<moka::sync::Cache<String, GeckoTerminalData>> =
    LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(MARKET_CACHE_CAPACITY)
            .time_to_live(Duration::from_secs(GECKOTERMINAL_TTL_SECS))
            .build()
    });

static RUGCHECK_CACHE: LazyLock<moka::sync::Cache<String, RugcheckData>> = LazyLock::new(|| {
    moka::sync::Cache::builder()
        .max_capacity(SECURITY_CACHE_CAPACITY)
        .time_to_live(Duration::from_secs(RUGCHECK_TTL_SECS))
        .build()
});

pub fn get_cached_dexscreener(mint: &str) -> Option<DexScreenerData> {
    DEXSCREENER_CACHE.get(&mint.to_string())
}

pub fn store_dexscreener(mint: &str, data: &DexScreenerData) {
    DEXSCREENER_CACHE.insert(mint.to_string(), data.clone());
}

pub fn dexscreener_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: DEXSCREENER_CACHE.entry_count(),
    }
}

pub fn dexscreener_cache_size() -> usize {
    DEXSCREENER_CACHE.entry_count() as usize
}

pub fn get_cached_geckoterminal(mint: &str) -> Option<GeckoTerminalData> {
    GECKOTERMINAL_CACHE.get(&mint.to_string())
}

pub fn store_geckoterminal(mint: &str, data: &GeckoTerminalData) {
    GECKOTERMINAL_CACHE.insert(mint.to_string(), data.clone());
}

pub fn geckoterminal_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: GECKOTERMINAL_CACHE.entry_count(),
    }
}

pub fn geckoterminal_cache_size() -> usize {
    GECKOTERMINAL_CACHE.entry_count() as usize
}

pub fn get_cached_rugcheck(mint: &str) -> Option<RugcheckData> {
    RUGCHECK_CACHE.get(&mint.to_string())
}

pub fn store_rugcheck(mint: &str, data: &RugcheckData) {
    RUGCHECK_CACHE.insert(mint.to_string(), data.clone());
}

pub fn rugcheck_cache_metrics() -> CacheMetrics {
    CacheMetrics {
        hits: 0,
        misses: 0,
        evictions: 0,
        expirations: 0,
        inserts: RUGCHECK_CACHE.entry_count(),
    }
}

pub fn rugcheck_cache_size() -> usize {
    RUGCHECK_CACHE.entry_count() as usize
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
    DEXSCREENER_CACHE.invalidate_all();
    GECKOTERMINAL_CACHE.invalidate_all();
}

pub fn clear_security_cache() {
    RUGCHECK_CACHE.invalidate_all();
}
