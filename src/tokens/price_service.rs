/// High-Performance Token Price Service
///
/// This module provides instant, thread-safe access to token prices with smart caching
/// and background monitoring. No timeouts on cache hits, automatic watch list management,
/// and prioritized pool price integration for open positions.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_price_service_enabled;
use crate::tokens::types::ApiToken;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::blacklist::is_token_blacklisted;
use crate::tokens::pool::{ get_pool_service, PoolPriceResult };
use tokio::sync::RwLock;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };
use once_cell::sync::Lazy;

// =============================================================================
// PRICE SERVICE CONFIGURATION
// =============================================================================

/// Maximum age for cached prices (in seconds)
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 5;

/// Time to keep watching a token after last request (in seconds)
const WATCH_TIMEOUT_SECONDS: i64 = 300; // 5 minutes

/// Pool price usage limit per cycle
const POOL_PRICE_LIMIT_PER_CYCLE: usize = 10;

// =============================================================================
// PRICE CACHE ENTRY
// =============================================================================

#[derive(Debug, Clone)]
pub struct PriceCacheEntry {
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub source: String, // "api", "pool"
}

impl PriceCacheEntry {
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age > Duration::seconds(PRICE_CACHE_MAX_AGE_SECONDS)
    }

    pub fn from_api_token(token: &ApiToken) -> Self {
        let price_sol = token.price_sol.and_then(|p| {
            if p > 0.0 && p.is_finite() { Some(p) } else { None }
        });

        Self {
            price_sol,
            price_usd: Some(token.price_usd),
            liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
            last_updated: Utc::now(),
            source: "api".to_string(),
        }
    }

    pub fn from_pool_result(pool_result: &PoolPriceResult) -> Self {
        Self {
            price_sol: pool_result.price_sol,
            price_usd: pool_result.price_usd,
            liquidity_usd: Some(pool_result.liquidity_usd),
            last_updated: pool_result.calculated_at,
            source: "pool".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct WatchEntry {
    last_requested: DateTime<Utc>,
    is_open_position: bool,
}

impl WatchEntry {
    fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_requested;
        age > Duration::seconds(WATCH_TIMEOUT_SECONDS)
    }
}

// =============================================================================
// THREAD-SAFE PRICE SERVICE
// =============================================================================

pub struct TokenPriceService {
    price_cache: Arc<RwLock<HashMap<String, PriceCacheEntry>>>,
    watch_list: Arc<RwLock<HashMap<String, WatchEntry>>>,
    open_positions: Arc<RwLock<HashSet<String>>>,
    database: TokenDatabase,
    pool_usage_count: Arc<RwLock<usize>>,
}

impl TokenPriceService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;
        Ok(Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            watch_list: Arc::new(RwLock::new(HashMap::new())),
            open_positions: Arc::new(RwLock::new(HashSet::new())),
            database,
            pool_usage_count: Arc::new(RwLock::new(0)),
        })
    }

    /// Get token price - instant cache lookup, no timeouts
    pub async fn get_token_price(&self, mint: &str) -> Option<f64> {
        // Track this token request
        self.add_to_watch_list(mint, false).await;

        // First check cache (instant, no await on lock contention)
        if let Some(price) = self.get_cached_price(mint).await {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_HIT",
                    &format!("Cached price for {}: ${:.8}", mint, price)
                );
            }
            return Some(price);
        }

        // Cache miss - trigger background update, but don't wait
        let mint_clone = mint.to_string();
        tokio::spawn(async move {
            let service_guard = PRICE_SERVICE.lock().await;
            if let Some(ref service) = *service_guard {
                let _ = service.update_single_token_price(&mint_clone).await;
            }
        });

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "CACHE_MISS",
                &format!("No cached price for {}, triggered background update", mint)
            );
        }

        None
    }

    async fn get_cached_price(&self, mint: &str) -> Option<f64> {
        let cache = self.price_cache.read().await;
        if let Some(entry) = cache.get(mint) {
            if !entry.is_expired() {
                if let Some(price) = entry.price_sol {
                    if price > 0.0 && price.is_finite() {
                        return Some(price);
                    }
                }
            }
        }
        None
    }

    async fn add_to_watch_list(&self, mint: &str, is_open_position: bool) {
        let mut watch_list = self.watch_list.write().await;
        watch_list.insert(mint.to_string(), WatchEntry {
            last_requested: Utc::now(),
            is_open_position,
        });
    }

    async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        // Check if this is an open position for pool price priority
        let is_open_position = {
            let positions = self.open_positions.read().await;
            positions.contains(mint)
        };

        // Try pool price first for open positions if within limit
        let pool_result = if is_open_position {
            let mut pool_usage = self.pool_usage_count.write().await;
            if *pool_usage < POOL_PRICE_LIMIT_PER_CYCLE {
                *pool_usage += 1;
                drop(pool_usage);

                let pool_service = get_pool_service();
                if pool_service.check_token_availability(mint).await {
                    pool_service.get_pool_price(mint, None).await
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Try API if no pool result
        let api_token = if pool_result.is_none() {
            self.database
                .get_token_by_mint(mint)
                .map_err(|e| format!("Failed to get token from database: {}", e))?
        } else {
            None
        };

        // Create cache entry
        let cache_entry = match (pool_result, api_token) {
            (Some(pool_result), _) => {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "POOL_PRICE",
                        &format!(
                            "Using pool price for {}: ${:.8}",
                            mint,
                            pool_result.price_sol.unwrap_or(0.0)
                        )
                    );
                }
                PriceCacheEntry::from_pool_result(&pool_result)
            }
            (None, Some(api_token)) => { PriceCacheEntry::from_api_token(&api_token) }
            (None, None) => {
                return Err("No price data available".to_string());
            }
        };

        // Update cache
        let mut cache = self.price_cache.write().await;
        cache.insert(mint.to_string(), cache_entry);

        Ok(())
    }

    pub async fn update_open_positions(&self, mints: Vec<String>) {
        let mut positions = self.open_positions.write().await;
        positions.clear();
        for mint in mints {
            positions.insert(mint.clone());
            // Mark these as open positions in watch list
            drop(positions);
            self.add_to_watch_list(&mint, true).await;
            positions = self.open_positions.write().await;
        }
    }

    pub async fn get_priority_tokens(&self) -> Vec<String> {
        let mut priority_tokens = Vec::new();

        // Add open positions first (highest priority)
        let positions = self.open_positions.read().await;
        priority_tokens.extend(positions.iter().cloned());
        drop(positions);

        // Add watched tokens that are not expired
        let watch_list = self.watch_list.read().await;
        for (mint, entry) in watch_list.iter() {
            if !entry.is_expired() && !priority_tokens.contains(mint) {
                priority_tokens.push(mint.clone());
            }
        }
        drop(watch_list);

        // Add some high liquidity tokens if we have space
        if priority_tokens.len() < 100 {
            if
                let Ok(high_liquidity_tokens) =
                    self.database.get_tokens_by_liquidity_threshold(10000.0).await
            {
                for token in high_liquidity_tokens.into_iter().take(50) {
                    if !is_token_blacklisted(&token.mint) && !priority_tokens.contains(&token.mint) {
                        priority_tokens.push(token.mint);
                    }
                }
            }
        }

        priority_tokens
    }

    pub async fn update_tokens_from_api(&self, mints: &[String]) {
        // Reset pool usage counter for new cycle
        {
            let mut pool_usage = self.pool_usage_count.write().await;
            *pool_usage = 0;
        }

        // Update tokens
        for mint in mints {
            let _ = self.update_single_token_price(mint).await;
        }
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut removed_count = 0;

        // Clean price cache
        {
            let mut cache = self.price_cache.write().await;
            let initial_size = cache.len();
            cache.retain(|_, entry| !entry.is_expired());
            removed_count += initial_size - cache.len();
        }

        // Clean watch list
        {
            let mut watch_list = self.watch_list.write().await;
            let initial_size = watch_list.len();
            watch_list.retain(|_, entry| !entry.is_expired());
            removed_count += initial_size - watch_list.len();
        }

        if removed_count > 0 && is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "CLEANUP",
                &format!("Removed {} expired entries", removed_count)
            );
        }

        removed_count
    }

    pub async fn get_cache_stats(&self) -> (usize, usize, usize) {
        let cache = self.price_cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache
            .values()
            .filter(|entry| entry.is_expired())
            .count();
        let valid_entries = total_entries - expired_entries;
        (total_entries, valid_entries, expired_entries)
    }
}

// =============================================================================
// GLOBAL PRICE SERVICE INSTANCE
// =============================================================================

/// Global thread-safe price service instance
pub static PRICE_SERVICE: Lazy<Arc<tokio::sync::Mutex<Option<TokenPriceService>>>> = Lazy::new(||
    Arc::new(tokio::sync::Mutex::new(None))
);

/// Initialize the global price service
pub async fn initialize_price_service() -> Result<(), Box<dyn std::error::Error>> {
    let service = TokenPriceService::new()?;
    let mut global_service = PRICE_SERVICE.lock().await;
    *global_service = Some(service);
    log(LogTag::PriceService, "INIT", "Price service initialized successfully");
    Ok(())
}

async fn get_price_service_instance() -> Result<
    Arc<tokio::sync::Mutex<Option<TokenPriceService>>>,
    String
> {
    Ok(PRICE_SERVICE.clone())
}

/// Get token price using global service - instant response for cached prices
pub async fn get_token_price_safe(mint: &str) -> Option<f64> {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.get_token_price(mint).await;
    }
    None
}

/// Update open positions in global service
pub async fn update_open_positions_safe(mints: Vec<String>) {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        service.update_open_positions(mints).await;
    }
}

/// Get priority tokens for monitoring
pub async fn get_priority_tokens_safe() -> Vec<String> {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.get_priority_tokens().await;
    }
    Vec::new()
}

/// Get multiple token prices in batch (for compatibility)
pub async fn get_token_prices_batch_safe(mints: &[String]) -> HashMap<String, Option<f64>> {
    let mut results = HashMap::new();

    // For now, just call individual price lookups
    // This keeps it simple while maintaining the interface
    for mint in mints {
        let price = get_token_price_safe(mint).await;
        results.insert(mint.clone(), price);
    }

    results
}

/// Update multiple token prices (called from monitor)
pub async fn update_tokens_prices_safe(mints: &[String]) {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        service.update_tokens_from_api(mints).await;
    }
}

/// Get cache statistics
pub async fn get_price_cache_stats() -> String {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        let (total, valid, expired) = service.get_cache_stats().await;
        return format!("Price Cache: {} total, {} valid, {} expired", total, valid, expired);
    }
    "Price Cache: Not available".to_string()
}

/// Cleanup expired cache entries
pub async fn cleanup_price_cache() -> usize {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.cleanup_expired().await;
    }
    0
}
