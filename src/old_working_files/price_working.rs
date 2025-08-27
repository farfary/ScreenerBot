/// High-Performance Token Price Service
///
/// This module provides instant, thread-safe access to token prices with smart caching
/// and background monitoring. No timeouts on cache hits, automatic watch list management.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_price_service_enabled;
use crate::tokens::types::ApiToken;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::pool::{ get_pool_service, PoolPriceResult };
use crate::tokens::is_system_or_stable_token;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };
use tokio::sync::OnceCell;

// =============================================================================
// PRICE SERVICE CONFIGURATION
// =============================================================================

/// Maximum age for cached prices (in seconds)
/// Price cache TTL - optimized for fastest 5-second priority checking
const PRICE_CACHE_TTL_SECONDS: i64 = 5; // 5 seconds to match monitoring cycle
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 5; // 5 seconds maximum age for all prices

/// If fresh cache is missing, allow serving a slightly stale price up to this age (seconds)
/// This avoids N/A in UI while a background refresh runs
const STALE_RETURN_MAX_AGE_SECONDS: i64 = 180; // 3 minutes
/// Timeout for pool price calculations (increased from 300ms to handle RPC delays)
const POOL_PRICE_TIMEOUT_MS: u64 = 2000; // 2 seconds for pool calculations

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

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

// =============================================================================
// THREAD-SAFE PRICE SERVICE
// =============================================================================

pub struct TokenPriceService {
    price_cache: Arc<RwLock<HashMap<String, PriceCacheEntry>>>,
    database: TokenDatabase,
}

impl TokenPriceService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;
        Ok(Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            database,
        })
    }

    /// Get token price - instant cache lookup, no timeouts; will serve slightly stale values
    pub async fn get_token_price(&self, mint: &str) -> Option<f64> {
        // First check cache (instant, no await on lock contention)
        if let Some(price) = self.get_cached_price_maybe_stale(mint, true).await {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_HIT",
                    &format!("🎯 CACHE HIT for {}: ${:.12} SOL", mint, price)
                );
            }
            return Some(price);
        }

        // Cache miss - trigger background update, but don't wait
        let mint_clone = mint.to_string();
        if let Some(service) = PRICE_SERVICE.get().cloned() {
            tokio::spawn(async move {
                let _ = service.update_single_token_price(&mint_clone).await;
            });
        }

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "CACHE_MISS",
                &format!("❓ CACHE MISS for {}, triggered background update", mint)
            );
        }

        None
    }

    /// Get token price - waits for update on cache miss (blocking version for critical operations)
    pub async fn get_token_price_blocking(&self, mint: &str) -> Option<f64> {
        // First check cache
        if let Some(price) = self.get_cached_price_maybe_stale(mint, false).await {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_HIT_BLOCKING",
                    &format!("🎯 CACHED price for {}: ${:.12} SOL (blocking call)", mint, price)
                );
            }
            return Some(price);
        }

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "CACHE_MISS_BLOCKING",
                &format!("❓ CACHE MISS for {}, will update and wait for result", mint)
            );
        }

        // Cache miss - update immediately and wait
        match self.update_single_token_price(mint).await {
            Ok(()) => {
                // Try to get the price again after update
                // Try to get the price again after update (fresh only)
                let price = self
                    .get_cached_price_maybe_stale(mint, false).await
                    // Final fallback: allow stale cache so callers rarely see None
                    .or(self.get_cached_price_maybe_stale(mint, true).await);
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "CACHE_MISS_UPDATED",
                        &format!(
                            "✅ UPDATED and got price for {}: ${:.12} SOL",
                            mint,
                            price.unwrap_or(0.0)
                        )
                    );
                }
                price
            }
            Err(e) => {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "UPDATE_FAILED",
                        &format!("❌ FAILED to update price for {}: {}", mint, e)
                    );
                }
                None
            }
        }
    }

    async fn get_cached_price_maybe_stale(&self, mint: &str, allow_stale: bool) -> Option<f64> {
        let cache = self.price_cache.read().await;
        let entry_opt = cache.get(mint).cloned();
        drop(cache);

        if let Some(entry) = entry_opt {
            let age = Utc::now() - entry.last_updated;
            let age_seconds = age.num_seconds();
            let is_expired = entry.is_expired();

            if !is_expired || (allow_stale && age_seconds <= STALE_RETURN_MAX_AGE_SECONDS) {
                if let Some(price) = entry.price_sol {
                    if price > 0.0 && price.is_finite() {
                        return Some(price);
                    }
                }
            }
        }
        None
    }

    async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        let pool_service = get_pool_service();
        let cached_entry_opt = {
            let c = self.price_cache.read().await;
            c.get(mint).cloned()
        };
        let cached_price_sol = cached_entry_opt.as_ref().and_then(|e| e.price_sol);

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "POOL_REQUEST",
                &format!(
                    "🏊 REQUESTING POOL PRICE for {} (timeout={}ms)",
                    mint,
                    POOL_PRICE_TIMEOUT_MS
                )
            );
        }

        let mut pool_result = None;
        if pool_service.check_token_availability(mint).await {
            match
                tokio::time::timeout(
                    std::time::Duration::from_millis(POOL_PRICE_TIMEOUT_MS),
                    pool_service.get_pool_price(mint, cached_price_sol)
                ).await
            {
                Ok(res) => {
                    pool_result = res;
                }
                Err(_) => {
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "POOL_TIMEOUT",
                            &format!(
                                "⏱️ Pool price timeout for {} after {}ms",
                                mint,
                                POOL_PRICE_TIMEOUT_MS
                            )
                        );
                    }
                    // if cached pool price exists and not too old, keep it (skip update)
                    if let Some(cached) = &cached_entry_opt {
                        if cached.source == "pool" {
                            let age = (Utc::now() - cached.last_updated).num_seconds();
                            if age <= STALE_RETURN_MAX_AGE_SECONDS {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        let api_token = if pool_result.is_none() {
            self.database
                .get_token_by_mint(mint)
                .map_err(|e| format!("Failed to get token from database: {}", e))?
        } else {
            None
        };
        let (cache_entry, pool_info) = match (pool_result.as_ref(), api_token) {
            (Some(r), _) =>
                (
                    PriceCacheEntry::from_pool_result(&r),
                    (r.pool_type.clone(), Some(r.pool_address.clone())),
                ),
            (None, Some(api_t)) => (PriceCacheEntry::from_api_token(&api_t), (None, None)),
            (None, None) => {
                return Err("No price data available".into());
            }
        };

        let mut cache = self.price_cache.write().await;
        let old_price = cache.get(mint).and_then(|e| e.price_sol);
        cache.insert(mint.to_string(), cache_entry.clone());
        drop(cache);

        Ok(())
    }

    pub async fn update_tokens_from_api(&self, mints: &[String]) {
        log(
            LogTag::PriceService,
            "API_UPDATE_START",
            &format!("🔄 STARTING API UPDATE for {} tokens: {:?}", mints.len(), mints)
        );

        let mut success_count = 0;
        let mut error_count = 0;

        // Update tokens
        for mint in mints {
            match self.update_single_token_price(mint).await {
                Ok(()) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_count += 1;
                    log(
                        LogTag::PriceService,
                        "API_UPDATE_ERROR",
                        &format!("❌ Failed to update price for {}: {}", mint, e)
                    );
                }
            }
        }

        log(
            LogTag::PriceService,
            "API_UPDATE_COMPLETE",
            &format!(
                "✅ API UPDATE COMPLETE: {}/{} successful, {} errors",
                success_count,
                mints.len(),
                error_count
            )
        );
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

/// Global price service instance (single Arc, lock-free access)
pub static PRICE_SERVICE: OnceCell<Arc<TokenPriceService>> = OnceCell::const_new();

/// Initialize the global price service
pub async fn initialize_price_service() -> Result<(), Box<dyn std::error::Error>> {
    if PRICE_SERVICE.get().is_some() {
        return Ok(());
    }

    let service = Arc::new(TokenPriceService::new()?);
    let _ = PRICE_SERVICE.set(service).map_err(|_| "Price service already initialized")?;
    log(LogTag::PriceService, "INIT", "Price service initialized successfully");
    Ok(())
}

/// Get token price using global service - instant response for cached prices
pub async fn get_token_price_safe(mint: &str) -> Option<f64> {
    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "GLOBAL_REQUEST",
            &format!("🌐 GLOBAL price request for {}", mint)
        );
    }

    if let Some(service) = PRICE_SERVICE.get() {
        // Fast path: non-blocking to avoid UI stalls
        let result = service.get_token_price(mint).await;

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "GLOBAL_RESULT",
                &format!("🌐 GLOBAL price result for {}: ${:.12} SOL", mint, result.unwrap_or(0.0))
            );
        }

        return result;
    } else if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "GLOBAL_ERROR",
            &format!("❌ GLOBAL service not available for {}", mint)
        );
    }
    None
}

/// Get token price using global service - waits for update on cache miss (blocking version)
pub async fn get_token_price_blocking_safe(mint: &str) -> Option<f64> {
    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "GLOBAL_BLOCKING_REQUEST",
            &format!("🌐 GLOBAL BLOCKING price request for {}", mint)
        );
    }

    if let Some(service) = PRICE_SERVICE.get() {
        let result = service.get_token_price_blocking(mint).await;

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "GLOBAL_BLOCKING_RESULT",
                &format!(
                    "🌐 GLOBAL BLOCKING price result for {}: ${:.12} SOL",
                    mint,
                    result.unwrap_or(0.0)
                )
            );
        }

        return result;
    } else if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "GLOBAL_BLOCKING_ERROR",
            &format!("❌ GLOBAL BLOCKING service not available for {}", mint)
        );
    }
    None
}

/// Force an immediate price refresh for a token (bypass cache freshness)
pub async fn force_refresh_token_price_safe(mint: &str) {
    if let Some(service) = PRICE_SERVICE.get() {
        // Directly call the underlying update without preliminary cache check
        let _ = service.update_single_token_price(mint).await;
    }
}

/// Update multiple token prices (called from monitor)
pub async fn update_tokens_prices_safe(mints: &[String]) {
    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "MONITOR_UPDATE_REQUEST",
            &format!("🔧 MONITOR requesting price updates for {} tokens: {:?}", mints.len(), mints)
        );
    }

    if let Some(service) = PRICE_SERVICE.get() {
        service.update_tokens_from_api(mints).await;
    } else if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "MONITOR_UPDATE_ERROR",
            "❌ MONITOR UPDATE FAILED: Price service not available"
        );
    }
}
