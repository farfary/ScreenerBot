/// Thread-Safe Token Price Service
///
/// This module provides a thread-safe interface for accessing token prices
/// and data between the trader and tokens system. It includes integration
/// with the pool price system for more accurate and faster price data.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_price_service_enabled;
use crate::tokens::types::ApiToken;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::blacklist::is_token_blacklisted;
use crate::tokens::pool::{ get_pool_service, PoolPriceResult };
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };
use once_cell::sync::Lazy;

// =============================================================================
// PRICE SERVICE CONFIGURATION
// =============================================================================

/// Maximum age for cached prices (in seconds for real-time updates)
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 3;

/// Priority boost for tokens with open positions (higher priority)
const OPEN_POSITION_PRIORITY: i32 = 200;

/// Priority boost for high liquidity tokens
const HIGH_LIQUIDITY_PRIORITY: i32 = 10;

/// Minimum liquidity threshold for high priority (in USD)
const HIGH_LIQUIDITY_THRESHOLD: f64 = 2000.0;

// =============================================================================
// PRICE CACHE ENTRY
// =============================================================================

#[derive(Debug, Clone)]
pub struct PriceCacheEntry {
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub price_change_24h: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub priority: i32,
    pub source: String, // "api", "pool", or "hybrid"
}

impl PriceCacheEntry {
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age > Duration::seconds(PRICE_CACHE_MAX_AGE_SECONDS)
    }

    pub fn from_api_token(token: &ApiToken, priority: i32) -> Self {
        let price_sol = token.price_sol.and_then(|p| {
            if p > 0.0 && p.is_finite() { Some(p) } else { None }
        });

        Self {
            price_sol,
            price_usd: Some(token.price_usd),
            liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
            volume_24h: token.volume.as_ref().and_then(|v| v.h24),
            price_change_24h: token.price_change.as_ref().and_then(|pc| pc.h24),
            last_updated: Utc::now(),
            priority,
            source: "api".to_string(),
        }
    }

    pub fn from_pool_result(pool_result: &PoolPriceResult, priority: i32) -> Self {
        Self {
            price_sol: pool_result.price_sol,
            price_usd: pool_result.price_usd,
            liquidity_usd: Some(pool_result.liquidity_usd),
            volume_24h: Some(pool_result.volume_24h),
            price_change_24h: None,
            last_updated: pool_result.calculated_at,
            priority,
            source: "pool".to_string(),
        }
    }

    pub fn from_hybrid(api_token: &ApiToken, pool_result: &PoolPriceResult, priority: i32) -> Self {
        let price_sol = pool_result.price_sol.or_else(|| {
            api_token.price_sol.and_then(|p| {
                if p > 0.0 && p.is_finite() { Some(p) } else { None }
            })
        });

        let price_usd = pool_result.price_usd.or(Some(api_token.price_usd));

        Self {
            price_sol,
            price_usd,
            liquidity_usd: Some(
                pool_result.liquidity_usd.max(
                    api_token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0)
                )
            ),
            volume_24h: Some(
                pool_result.volume_24h.max(
                    api_token.volume
                        .as_ref()
                        .and_then(|v| v.h24)
                        .unwrap_or(0.0)
                )
            ),
            price_change_24h: api_token.price_change.as_ref().and_then(|pc| pc.h24),
            last_updated: Utc::now(),
            priority,
            source: "hybrid".to_string(),
        }
    }
}

// =============================================================================
// THREAD-SAFE PRICE SERVICE
// =============================================================================

pub struct TokenPriceService {
    price_cache: Arc<RwLock<HashMap<String, PriceCacheEntry>>>,
    database: TokenDatabase,
    open_positions: Arc<RwLock<Vec<String>>>,
}

impl TokenPriceService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;
        Ok(Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            database,
            open_positions: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn get_token_price(&self, mint: &str) -> Option<f64> {
        // First check cache (very fast, no await on empty cache)
        if let Some(price) = self.get_cached_price(mint).await {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_HIT",
                    &format!("Using cached price for {}: ${:.8}", mint, price)
                );
            }
            return Some(price);
        }

        // Cache miss - try to update with timeout protection
        let update_future = self.update_single_token_price_hybrid(mint);
        match tokio::time::timeout(std::time::Duration::from_secs(3), update_future).await {
            Ok(Ok(())) => {
                // Successfully updated, try cache again
                self.get_cached_price(mint).await
            }
            Ok(Err(e)) => {
                log(
                    LogTag::PriceService,
                    "WARN",
                    &format!("Failed to update price for {}: {}", mint, e)
                );
                None
            }
            Err(_) => {
                log(LogTag::PriceService, "TIMEOUT", &format!("Price update timeout for {}", mint));
                None
            }
        }
    }

    async fn get_cached_price(&self, mint: &str) -> Option<f64> {
        // Use a shorter read lock scope for better performance
        let entry = {
            let cache = self.price_cache.read().await;
            cache.get(mint).cloned()
        };

        if let Some(entry) = entry {
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

    /// Batch get prices for multiple tokens - much faster than individual calls
    pub async fn get_token_prices_batch(&self, mints: &[String]) -> HashMap<String, Option<f64>> {
        let mut results = HashMap::new();
        let mut missing_mints = Vec::new();

        // First, check cache for all mints
        {
            let cache = self.price_cache.read().await;
            for mint in mints {
                if let Some(entry) = cache.get(mint) {
                    if !entry.is_expired() {
                        if let Some(price) = entry.price_sol {
                            if price > 0.0 && price.is_finite() {
                                results.insert(mint.clone(), Some(price));
                                continue;
                            }
                        }
                    }
                }
                missing_mints.push(mint.clone());
            }
        }

        // Update missing mints in parallel (with timeout)
        if !missing_mints.is_empty() {
            let update_futures: Vec<_> = missing_mints
                .iter()
                .map(|mint| {
                    let mint_clone = mint.clone();
                    async move {
                        let update_future = self.update_single_token_price_hybrid(&mint_clone);
                        match
                            tokio::time::timeout(
                                std::time::Duration::from_secs(3),
                                update_future
                            ).await
                        {
                            Ok(Ok(())) => (mint_clone.clone(), true),
                            Ok(Err(_)) | Err(_) => (mint_clone.clone(), false),
                        }
                    }
                })
                .collect();

            let update_results = futures::future::join_all(update_futures).await;

            // Get updated prices for successfully updated mints
            for (mint, success) in update_results {
                if success {
                    if let Some(price) = self.get_cached_price(&mint).await {
                        results.insert(mint, Some(price));
                    } else {
                        results.insert(mint, None);
                    }
                } else {
                    results.insert(mint, None);
                }
            }
        }

        results
    }

    async fn update_single_token_price_hybrid(&self, mint: &str) -> Result<(), String> {
        let pool_service = get_pool_service();

        let api_token = self.database
            .get_token_by_mint(mint)
            .map_err(|e| format!("Failed to get token from database: {}", e))?;

        let priority = if let Some(ref token) = api_token {
            self.calculate_priority(token).await
        } else {
            0
        };

        let pool_result = if pool_service.check_token_availability(mint).await {
            let api_price_sol = api_token.as_ref().and_then(|t| t.price_sol);
            pool_service.get_pool_price(mint, api_price_sol).await
        } else {
            None
        };

        let cache_entry = match (api_token, pool_result.clone()) {
            (Some(api_token), Some(pool_result)) => {
                PriceCacheEntry::from_hybrid(&api_token, &pool_result, priority)
            }
            (Some(api_token), None) => { PriceCacheEntry::from_api_token(&api_token, priority) }
            (None, Some(pool_result)) => {
                PriceCacheEntry::from_pool_result(&pool_result, priority)
            }
            (None, None) => {
                return Err("No price data available".to_string());
            }
        };

        let mut cache = self.price_cache.write().await;
        cache.insert(mint.to_string(), cache_entry);

        if pool_result.is_some() {
            pool_service.add_to_watch_list(mint, priority).await;
        }

        Ok(())
    }

    pub async fn update_open_positions(&self, mints: Vec<String>) {
        let mut positions = self.open_positions.write().await;
        *positions = mints;
    }

    pub async fn get_priority_tokens_for_monitoring(&self) -> Result<Vec<String>, String> {
        let mut priority_mints = Vec::new();
        let positions = self.open_positions.read().await;
        priority_mints.extend(positions.clone());

        let all_tokens = self.database
            .get_all_tokens().await
            .map_err(|e| format!("Failed to get all tokens: {}", e))?;

        for token in all_tokens {
            if !is_token_blacklisted(&token.mint) && !priority_mints.contains(&token.mint) {
                priority_mints.push(token.mint);
            }
        }

        Ok(priority_mints)
    }
}

static mut GLOBAL_PRICE_SERVICE: Option<TokenPriceService> = None;
static PRICE_SERVICE_INIT: std::sync::Once = std::sync::Once::new();

pub fn init_price_service() -> Result<&'static TokenPriceService, Box<dyn std::error::Error>> {
    unsafe {
        PRICE_SERVICE_INIT.call_once(|| {
            match TokenPriceService::new() {
                Ok(service) => {
                    GLOBAL_PRICE_SERVICE = Some(service);
                }
                Err(e) => panic!("Failed to initialize price service: {}", e),
            }
        });
        Ok(GLOBAL_PRICE_SERVICE.as_ref().unwrap())
    }
}

pub fn get_price_service() -> &'static TokenPriceService {
    unsafe {
        if GLOBAL_PRICE_SERVICE.is_none() {
            init_price_service().expect("Failed to initialize price service");
        }
        GLOBAL_PRICE_SERVICE.as_ref().unwrap()
    }
}

impl TokenPriceService {
    /// Calculate priority for a token based on open positions and liquidity
    async fn calculate_priority(&self, token: &ApiToken) -> i32 {
        let mut priority = 0;

        // Check if this token has an open position
        {
            let positions = self.open_positions.read().await;
            if positions.contains(&token.mint) {
                priority += OPEN_POSITION_PRIORITY;

                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "DEBUG",
                        &format!(
                            "Token {} has open position, priority +{}",
                            token.mint,
                            OPEN_POSITION_PRIORITY
                        )
                    );
                }
            }
        }

        // High liquidity boost
        if let Some(liquidity) = token.liquidity.as_ref().and_then(|l| l.usd) {
            if liquidity > HIGH_LIQUIDITY_THRESHOLD {
                priority += HIGH_LIQUIDITY_PRIORITY;

                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "DEBUG",
                        &format!(
                            "Token {} has high liquidity (${:.2}), priority +{}",
                            token.mint,
                            liquidity,
                            HIGH_LIQUIDITY_PRIORITY
                        )
                    );
                }
            }
        }

        priority
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> (usize, usize, usize) {
        let cache = self.price_cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache
            .values()
            .filter(|entry| entry.is_expired())
            .count();
        let valid_entries = total_entries - expired_entries;

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "DEBUG",
                &format!(
                    "Cache stats: {} total, {} valid, {} expired",
                    total_entries,
                    valid_entries,
                    expired_entries
                )
            );
        }

        (total_entries, valid_entries, expired_entries)
    }

    /// Clear expired entries from cache
    pub async fn cleanup_expired_entries(&self) -> usize {
        let mut cache = self.price_cache.write().await;
        let initial_size = cache.len();

        cache.retain(|_, entry| !entry.is_expired());

        let removed_count = initial_size - cache.len();

        if removed_count > 0 {
            log(
                LogTag::PriceService,
                "CLEANUP",
                &format!("Removed {} expired price entries from cache", removed_count)
            );
        }

        removed_count
    }

    /// Get token info including price and metadata
    pub async fn get_token_info(&self, mint: &str) -> Option<PriceCacheEntry> {
        let cache = self.price_cache.read().await;
        cache.get(mint).cloned()
    }

    /// Force refresh a token's data from database
    pub async fn force_refresh_token(&self, mint: &str) -> Result<(), String> {
        self.update_single_token_price(mint).await
    }

    /// Update a single token's price (backward compatibility)
    pub async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        self.update_single_token_price_hybrid(mint).await
    }

    /// Update multiple token prices (backward compatibility)
    pub async fn update_tokens_prices(&self, mints: &[String]) -> Result<usize, String> {
        let mut updated_count = 0;
        for mint in mints {
            if let Ok(()) = self.update_single_token_price_hybrid(mint).await {
                updated_count += 1;
            }
        }
        Ok(updated_count)
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

/// Get token price using global service (thread-safe API for trader)
pub async fn get_token_price_safe(mint: &str) -> Option<f64> {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.get_token_price(mint).await;
    }

    log(LogTag::PriceService, "WARN", &format!("Price service not available for token: {}", mint));
    None
}

/// Get multiple token prices in batch (much faster than individual calls)
pub async fn get_token_prices_batch_safe(mints: &[String]) -> HashMap<String, Option<f64>> {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.get_token_prices_batch(mints).await;
    }

    log(LogTag::PriceService, "WARN", "Price service not available for batch request");
    mints
        .iter()
        .map(|mint| (mint.clone(), None))
        .collect()
}

/// Update open positions in global service (called from trader)
pub async fn update_open_positions_safe(mints: Vec<String>) {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        service.update_open_positions(mints).await;
    }
}

/// Get priority tokens for monitoring (called from monitor)
pub async fn get_priority_tokens_safe() -> Vec<String> {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.get_priority_tokens_for_monitoring().await.unwrap_or_default();
    }

    Vec::new()
}

/// Update multiple token prices (called from monitor)
pub async fn update_tokens_prices_safe(mints: &[String]) -> usize {
    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        return service.update_tokens_prices(mints).await.unwrap_or(0);
    }

    0
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
        return service.cleanup_expired_entries().await;
    }

    0
}

/// Cleanup price service on shutdown
pub async fn cleanup_price_service() {
    cleanup_price_cache().await;
}
