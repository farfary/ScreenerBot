/// Pool caching system
/// Manages all caching for the pool service including tokens, pools, accounts, and prices

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{ DateTime, Utc };
use crate::pools::types::{ PriceResult, PoolInfo };
use crate::pools::tokens::PoolToken;
use crate::pools::constants::MAX_PRICE_HISTORY_POINTS;
use crate::pools::constants::*;

/// Cached pool data with metadata
#[derive(Debug, Clone)]
pub struct CachedPool {
    pub pool_info: PoolInfo,
    pub cached_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl CachedPool {
    pub fn new(pool_info: PoolInfo) -> Self {
        let now = Utc::now();
        Self {
            pool_info,
            cached_at: now,
            last_updated: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() > POOL_CACHE_TTL_SECONDS
    }
}

/// Cached account data from RPC
#[derive(Debug, Clone)]
pub struct CachedAccount {
    pub account_data: Vec<u8>,
    pub cached_at: DateTime<Utc>,
}

impl CachedAccount {
    pub fn new(account_data: Vec<u8>) -> Self {
        Self {
            account_data,
            cached_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() > 60 // Account data expires in 1 minute
    }
}

/// Cached price data
#[derive(Debug, Clone)]
pub struct CachedPrice {
    pub price_result: PriceResult,
    pub cached_at: DateTime<Utc>,
}

impl CachedPrice {
    pub fn new(price_result: PriceResult) -> Self {
        Self {
            price_result,
            cached_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.cached_at;
        age.num_seconds() > PRICE_CACHE_TTL_SECONDS
    }
}

/// Main cache manager for the pool service
pub struct PoolCache {
    /// Token cache: token_mint -> PoolToken
    tokens: Arc<RwLock<HashMap<String, PoolToken>>>,

    /// Pool cache: token_mint -> Vec<CachedPool>
    pools: Arc<RwLock<HashMap<String, Vec<CachedPool>>>>,

    /// Account data cache: pool_address -> CachedAccount
    accounts: Arc<RwLock<HashMap<String, CachedAccount>>>,

    /// Price cache: token_mint -> CachedPrice
    prices: Arc<RwLock<HashMap<String, CachedPrice>>>,

    /// Price history cache: token_mint -> Vec<(timestamp, price_sol)>
    price_history: Arc<RwLock<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>,

    /// Track which tokens are currently being processed (to avoid duplicates)
    in_progress: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl PoolCache {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            pools: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
            prices: Arc::new(RwLock::new(HashMap::new())),
            price_history: Arc::new(RwLock::new(HashMap::new())),
            in_progress: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // =============================================================================
    // TOKEN CACHE METHODS
    // =============================================================================

    /// Cache tokens
    pub async fn cache_tokens(&self, tokens: Vec<PoolToken>) {
        let mut cache = self.tokens.write().await;
        for token in tokens {
            cache.insert(token.mint.clone(), token);
        }
    }

    /// Get cached tokens
    pub async fn get_cached_tokens(&self) -> Vec<PoolToken> {
        let cache = self.tokens.read().await;
        cache.values().cloned().collect()
    }

    /// Check if token exists in cache
    pub async fn has_token(&self, token_mint: &str) -> bool {
        let cache = self.tokens.read().await;
        cache.contains_key(token_mint)
    }

    // =============================================================================
    // POOL CACHE METHODS
    // =============================================================================

    /// Get cached pools for a token
    pub async fn get_cached_pools(&self, token_mint: &str) -> Option<Vec<PoolInfo>> {
        let cache = self.pools.read().await;
        if let Some(cached_pools) = cache.get(token_mint) {
            // Check if any pools are still valid
            let valid_pools: Vec<PoolInfo> = cached_pools
                .iter()
                .filter(|cp| !cp.is_expired())
                .map(|cp| cp.pool_info.clone())
                .collect();

            if !valid_pools.is_empty() {
                return Some(valid_pools);
            }
        }
        None
    }

    /// Check if token has cached pools (not expired)
    pub async fn has_cached_pools(&self, token_mint: &str) -> bool {
        self.get_cached_pools(token_mint).await.is_some()
    }

    /// Get tokens that don't have cached pools
    pub async fn get_tokens_without_pools(&self) -> Vec<String> {
        let tokens_cache = self.tokens.read().await;
        let pools_cache = self.pools.read().await;

        let mut tokens_without_pools = Vec::new();

        for token_mint in tokens_cache.keys() {
            let has_valid_pools = if let Some(cached_pools) = pools_cache.get(token_mint) {
                cached_pools.iter().any(|cp| !cp.is_expired())
            } else {
                false
            };

            // If no valid pools, add to tokens without pools
            if !has_valid_pools {
                tokens_without_pools.push(token_mint.clone());
            }
        }

        tokens_without_pools
    }

    /// Get tokens that have cached pools available
    pub async fn get_tokens_with_pools(&self) -> Vec<String> {
        let tokens_cache = self.tokens.read().await;
        let pools_cache = self.pools.read().await;

        let mut tokens_with_pools = Vec::new();

        for token_mint in tokens_cache.keys() {
            if let Some(cached_pools) = pools_cache.get(token_mint) {
                // Check if there are valid pools
                let has_valid_pools = cached_pools.iter().any(|cp| !cp.is_expired());
                if has_valid_pools {
                    tokens_with_pools.push(token_mint.clone());
                }
            }
        }

        tokens_with_pools
    }

    /// Cache pools for a token
    pub async fn cache_pools(&self, token_mint: &str, pools: Vec<PoolInfo>) {
        let mut cache = self.pools.write().await;
        let cached_pools: Vec<CachedPool> = pools.into_iter().map(CachedPool::new).collect();
        cache.insert(token_mint.to_string(), cached_pools);
    }

    // =============================================================================
    // ACCOUNT DATA CACHE METHODS
    // =============================================================================

    /// Cache account data for a pool
    pub async fn cache_account_data(&self, pool_address: &str, account_data: Vec<u8>) {
        let mut cache = self.accounts.write().await;
        cache.insert(pool_address.to_string(), CachedAccount::new(account_data));
    }

    /// Get cached account data for a pool
    pub async fn get_cached_account_data(&self, pool_address: &str) -> Option<Vec<u8>> {
        let cache = self.accounts.read().await;
        if let Some(cached_account) = cache.get(pool_address) {
            if !cached_account.is_expired() {
                return Some(cached_account.account_data.clone());
            }
        }
        None
    }

    // =============================================================================
    // PRICE CACHE METHODS
    // =============================================================================

    /// Cache price result for a token
    pub async fn cache_price(&self, token_mint: &str, price_result: PriceResult) {
        let mut cache = self.prices.write().await;
        cache.insert(token_mint.to_string(), CachedPrice::new(price_result));
    }

    /// Get cached price for a token
    pub async fn get_cached_price(&self, token_mint: &str) -> Option<PriceResult> {
        let cache = self.prices.read().await;
        if let Some(cached_price) = cache.get(token_mint) {
            if !cached_price.is_expired() {
                return Some(cached_price.price_result.clone());
            }
        }
        None
    }

    // =============================================================================
    // PRICE HISTORY CACHE METHODS
    // =============================================================================

    /// Add price to history for a token
    pub async fn add_price_to_history(&self, token_mint: &str, price_sol: f64) {
        let mut cache = self.price_history.write().await;
        let history = cache.entry(token_mint.to_string()).or_insert_with(Vec::new);

        // Add new price point
        history.push((Utc::now(), price_sol));

        // Keep only last MAX_PRICE_HISTORY_POINTS for memory efficiency
        if history.len() > MAX_PRICE_HISTORY_POINTS {
            history.remove(0);
        }
    }

    /// Get price history for a token
    pub async fn get_price_history(&self, token_mint: &str) -> Vec<(DateTime<Utc>, f64)> {
        let cache = self.price_history.read().await;
        cache.get(token_mint).cloned().unwrap_or_default()
    }

    /// Get price history for a specific time range
    pub async fn get_price_history_since(
        &self,
        token_mint: &str,
        since: DateTime<Utc>
    ) -> Vec<(DateTime<Utc>, f64)> {
        let cache = self.price_history.read().await;
        if let Some(history) = cache.get(token_mint) {
            history
                .iter()
                .filter(|(timestamp, _)| *timestamp >= since)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Clear old price history entries (keep last 6 hours)
    pub async fn cleanup_price_history(&self) {
        let cutoff = Utc::now() - chrono::Duration::hours(6);
        let mut cache = self.price_history.write().await;

        for history in cache.values_mut() {
            history.retain(|(timestamp, _)| *timestamp > cutoff);
        }

        // Remove empty entries
        cache.retain(|_, history| !history.is_empty());
    }

    // =============================================================================
    // IN-PROGRESS TRACKING METHODS
    // =============================================================================

    /// Mark token as being processed
    pub async fn mark_in_progress(&self, token_mint: &str) -> bool {
        let mut cache = self.in_progress.write().await;
        if cache.contains_key(token_mint) {
            // Already being processed
            false
        } else {
            cache.insert(token_mint.to_string(), Utc::now());
            true
        }
    }

    /// Mark token as completed processing
    pub async fn mark_completed(&self, token_mint: &str) {
        let mut cache = self.in_progress.write().await;
        cache.remove(token_mint);
    }

    /// Clean up old in-progress entries (older than 5 minutes)
    pub async fn cleanup_in_progress(&self) {
        let mut cache = self.in_progress.write().await;
        let cutoff = Utc::now() - chrono::Duration::minutes(5);
        cache.retain(|_, start_time| *start_time > cutoff);
    }

    // =============================================================================
    // CLEANUP METHODS
    // =============================================================================

    /// Clean up expired cache entries
    pub async fn cleanup_expired(&self) -> (usize, usize, usize) {
        let mut pools_cleaned = 0;
        let mut accounts_cleaned = 0;
        let mut prices_cleaned = 0;

        // Clean expired pools (legacy)
        {
            let mut pools_cache = self.pools.write().await;
            pools_cache.retain(|_, cached_pools| {
                let before = cached_pools.len();
                cached_pools.retain(|cp| !cp.is_expired());
                pools_cleaned += before - cached_pools.len();
                !cached_pools.is_empty() // Remove token entry if no valid pools remain
            });
        }

        // Only one pool cache now, no separate discovery cache

        // Clean expired accounts
        {
            let mut accounts_cache = self.accounts.write().await;
            let before = accounts_cache.len();
            accounts_cache.retain(|_, cached_account| !cached_account.is_expired());
            accounts_cleaned = before - accounts_cache.len();
        }

        // Clean expired prices
        {
            let mut prices_cache = self.prices.write().await;
            let before = prices_cache.len();
            prices_cache.retain(|_, cached_price| !cached_price.is_expired());
            prices_cleaned = before - prices_cache.len();
        }

        // Clean up old price history
        self.cleanup_price_history().await;

        // Clean up old in-progress entries
        self.cleanup_in_progress().await;

        (pools_cleaned, accounts_cleaned, prices_cleaned)
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let tokens_count = self.tokens.read().await.len();
        let pools_count = self.pools.read().await.len();
        let accounts_count = self.accounts.read().await.len();
        let prices_count = self.prices.read().await.len();
        let in_progress_count = self.in_progress.read().await.len();

        CacheStats {
            tokens_count,
            pools_count, // Single unified pool cache
            accounts_count,
            prices_count,
            in_progress_count,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub tokens_count: usize,
    pub pools_count: usize,
    pub accounts_count: usize,
    pub prices_count: usize,
    pub in_progress_count: usize,
}
