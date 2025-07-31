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
                    &format!("🎯 CACHE HIT for {}: ${:.12} SOL", mint, price)
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
                &format!("❓ CACHE MISS for {}, triggered background update", mint)
            );
        }

        None
    }

    /// Get token price - waits for update on cache miss (blocking version for critical operations)
    pub async fn get_token_price_blocking(&self, mint: &str) -> Option<f64> {
        // Track this token request
        self.add_to_watch_list(mint, false).await;

        // First check cache
        if let Some(price) = self.get_cached_price(mint).await {
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
                let price = self.get_cached_price(mint).await;
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

    async fn get_cached_price(&self, mint: &str) -> Option<f64> {
        let cache = self.price_cache.read().await;
        if let Some(entry) = cache.get(mint) {
            let age_seconds = (Utc::now() - entry.last_updated).num_seconds();
            let is_expired = entry.is_expired();

            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_CHECK",
                    &format!(
                        "🔍 CACHE CHECK for {}: found_entry=YES, age={}s, max_age={}s, expired={}, price={:.12} SOL, source={}",
                        mint,
                        age_seconds,
                        PRICE_CACHE_MAX_AGE_SECONDS,
                        is_expired,
                        entry.price_sol.unwrap_or(0.0),
                        entry.source
                    )
                );
            }

            if !is_expired {
                if let Some(price) = entry.price_sol {
                    if price > 0.0 && price.is_finite() {
                        if is_debug_price_service_enabled() {
                            log(
                                LogTag::PriceService,
                                "CACHE_VALID",
                                &format!(
                                    "✅ VALID CACHE for {}: price={:.12} SOL, age={}s",
                                    mint,
                                    price,
                                    age_seconds
                                )
                            );
                        }
                        return Some(price);
                    } else {
                        if is_debug_price_service_enabled() {
                            log(
                                LogTag::PriceService,
                                "CACHE_INVALID_PRICE",
                                &format!(
                                    "❌ INVALID PRICE for {}: price={:.12} (not finite or zero)",
                                    mint,
                                    price
                                )
                            );
                        }
                    }
                } else {
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "CACHE_NO_PRICE",
                            &format!("❌ NO PRICE for {}: entry exists but price_sol is None", mint)
                        );
                    }
                }
            } else {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "CACHE_EXPIRED",
                        &format!(
                            "⏰ EXPIRED CACHE for {}: age={}s > max={}s",
                            mint,
                            age_seconds,
                            PRICE_CACHE_MAX_AGE_SECONDS
                        )
                    );
                }
            }
        } else {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_NOT_FOUND",
                    &format!("❓ NO CACHE ENTRY for {}", mint)
                );
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

        // For non-open positions, ALWAYS use API price, never pool
        let (pool_result, api_token) = if is_open_position {
            // Try pool price first for open positions if within limit
            let pool_result = {
                let mut pool_usage = self.pool_usage_count.write().await;
                if *pool_usage < POOL_PRICE_LIMIT_PER_CYCLE {
                    *pool_usage += 1;
                    drop(pool_usage);

                    let pool_service = get_pool_service();
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "POOL_REQUEST",
                            &format!("🏊 REQUESTING POOL PRICE for open position {} (will check real-time blockchain pools)", mint)
                        );
                    }

                    if pool_service.check_token_availability(mint).await {
                        // Get current API price for comparison if available
                        let api_price_sol = {
                            let cache = self.price_cache.read().await;
                            if let Some(cached_entry) = cache.get(&mint.to_string()) {
                                cached_entry.price_sol
                            } else {
                                None
                            }
                        };

                        if is_debug_price_service_enabled() {
                            log(
                                LogTag::PriceService,
                                "POOL_COMPARISON_PREP",
                                &format!(
                                    "🔍 Preparing price comparison for {}: API_price={:.12} SOL",
                                    mint,
                                    api_price_sol.unwrap_or(0.0)
                                )
                            );
                        }

                        let pool_result = pool_service.get_pool_price(mint, api_price_sol).await;
                        if is_debug_price_service_enabled() {
                            if let Some(ref result) = pool_result {
                                log(
                                    LogTag::PriceService,
                                    "POOL_SUCCESS",
                                    &format!(
                                        "✅ POOL PRICE SUCCESS for {}: ${:.12} SOL from pool {} (dex: {}) at {}",
                                        mint,
                                        result.price_sol.unwrap_or(0.0),
                                        result.pool_address,
                                        result.dex_id,
                                        result.calculated_at.format("%H:%M:%S%.3f")
                                    )
                                );

                                // Show detailed pool calculation information
                                log(
                                    LogTag::PriceService,
                                    "POOL_DETAILS",
                                    &format!(
                                        "🔍 POOL CALCULATION DETAILS for {}: liquidity=${:.2}, reserves_calculation_time={}, calculated_on_blockchain={}",
                                        mint,
                                        result.liquidity_usd,
                                        result.calculated_at.format("%H:%M:%S%.3f"),
                                        "real-time"
                                    )
                                );

                                // Show the price change if we have an API price for comparison
                                if let Some(api_sol) = api_price_sol {
                                    if let Some(pool_sol) = result.price_sol {
                                        let price_change = pool_sol - api_sol;
                                        let price_change_percent = if api_sol != 0.0 {
                                            ((pool_sol - api_sol) / api_sol) * 100.0
                                        } else {
                                            0.0
                                        };

                                        log(
                                            LogTag::PriceService,
                                            "POOL_VS_API",
                                            &format!(
                                                "💰 PRICE COMPARISON for {}: API={:.12} → POOL={:.12} SOL (change: {:+.12} SOL, {:+.4}%)",
                                                mint,
                                                api_sol,
                                                pool_sol,
                                                price_change,
                                                price_change_percent
                                            )
                                        );

                                        if price_change_percent.abs() > 5.0 {
                                            log(
                                                LogTag::PriceService,
                                                "SIGNIFICANT_CHANGE",
                                                &format!(
                                                    "🚨 SIGNIFICANT PRICE CHANGE for {}: {:+.4}% change from API to pool price!",
                                                    mint,
                                                    price_change_percent
                                                )
                                            );
                                        } else if price_change_percent.abs() < 0.001 {
                                            log(
                                                LogTag::PriceService,
                                                "STATIC_PRICE_WARNING",
                                                &format!(
                                                    "⚠️  POSSIBLE STATIC PRICE for {}: Only {:+.6}% change - pool reserves may be frozen or cached",
                                                    mint,
                                                    price_change_percent
                                                )
                                            );
                                        }
                                    }
                                }
                            } else {
                                log(
                                    LogTag::PriceService,
                                    "POOL_FAILED",
                                    &format!("❌ Pool service returned None for {} (no pools or calculation failed)", mint)
                                );
                            }
                        }
                        pool_result
                    } else {
                        if is_debug_price_service_enabled() {
                            log(
                                LogTag::PriceService,
                                "POOL_UNAVAILABLE",
                                &format!("❌ Pool not available for {} (no pools with sufficient liquidity)", mint)
                            );
                        }
                        None
                    }
                } else {
                    None
                }
            };

            // Try API if no pool result for open positions
            let api_token = if pool_result.is_none() {
                self.database
                    .get_token_by_mint(mint)
                    .map_err(|e| format!("Failed to get token from database: {}", e))?
            } else {
                None
            };

            (pool_result, api_token)
        } else {
            // For non-open positions, ONLY use API price
            let api_token = self.database
                .get_token_by_mint(mint)
                .map_err(|e| format!("Failed to get token from database: {}", e))?;
            (None, api_token)
        };

        // Create cache entry
        let cache_entry = match (pool_result, api_token) {
            (Some(pool_result), _) => {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "POOL_PRICE",
                        &format!(
                            "Using pool price for open position {}: ${:.8}",
                            mint,
                            pool_result.price_sol.unwrap_or(0.0)
                        )
                    );
                }
                PriceCacheEntry::from_pool_result(&pool_result)
            }
            (None, Some(api_token)) => {
                if is_debug_price_service_enabled() {
                    let position_type = if is_open_position {
                        "open position"
                    } else {
                        "non-open position"
                    };
                    log(
                        LogTag::PriceService,
                        "API_PRICE",
                        &format!(
                            "Using API price for {} {}: ${:.8}",
                            position_type,
                            mint,
                            api_token.price_sol.unwrap_or(0.0)
                        )
                    );
                }
                PriceCacheEntry::from_api_token(&api_token)
            }
            (None, None) => {
                return Err("No price data available".to_string());
            }
        };

        // Update cache
        let mut cache = self.price_cache.write().await;
        let old_entry = cache.get(mint);
        let old_price = old_entry.and_then(|e| e.price_sol);

        cache.insert(mint.to_string(), cache_entry.clone());

        if is_debug_price_service_enabled() {
            let price_change = match (old_price, cache_entry.price_sol) {
                (Some(old), Some(new)) => {
                    let change = new - old;
                    let change_percent = if old != 0.0 { (change / old) * 100.0 } else { 0.0 };

                    // Print to console for immediate visibility
                    println!(
                        "🔄 PRICE UPDATE for {}: {:.12} → {:.12} SOL ({:+.12} SOL, {:+.6}%)",
                        mint,
                        old,
                        new,
                        change,
                        change_percent
                    );

                    // Check if the price is completely static (exactly the same)
                    if (old - new).abs() < f64::EPSILON {
                        println!(
                            "⚠️  STATIC PRICE DETECTED for {}: Price has not changed at all (exactly {:.12} SOL)",
                            mint,
                            new
                        );
                    }

                    format!(
                        " (changed from ${:.12} to ${:.12} SOL, {:+.6}%)",
                        old,
                        new,
                        change_percent
                    )
                }
                (None, Some(new)) => {
                    println!("🆕 NEW PRICE for {}: {:.12} SOL (first time)", mint, new);
                    format!(" (new price: ${:.12} SOL)", new)
                }
                (Some(old), None) => {
                    println!("❌ PRICE REMOVED for {}: was {:.12} SOL", mint, old);
                    format!(" (removed price: was ${:.12} SOL)", old)
                }
                (None, None) => {
                    println!("❓ NO PRICE DATA for {}", mint);
                    " (no price data)".to_string()
                }
            };

            log(
                LogTag::PriceService,
                "CACHE_UPDATED",
                &format!(
                    "💾 CACHE UPDATED for {}: source={}, timestamp={}{}",
                    mint,
                    cache_entry.source,
                    cache_entry.last_updated.format("%H:%M:%S%.3f"),
                    price_change
                )
            );
        }

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

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "API_UPDATE_START",
                &format!("🔄 STARTING API UPDATE for {} tokens: {:?}", mints.len(), mints)
            );
        }

        let mut success_count = 0;
        let mut error_count = 0;

        // Update tokens
        for mint in mints {
            match self.update_single_token_price(mint).await {
                Ok(()) => {
                    success_count += 1;
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "API_UPDATE_SUCCESS",
                            &format!("✅ Successfully updated price for {}", mint)
                        );
                    }
                }
                Err(e) => {
                    error_count += 1;
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "API_UPDATE_ERROR",
                            &format!("❌ Failed to update price for {}: {}", mint, e)
                        );
                    }
                }
            }
        }

        if is_debug_price_service_enabled() {
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
    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "GLOBAL_REQUEST",
            &format!("🌐 GLOBAL price request for {}", mint)
        );
    }

    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        let result = service.get_token_price(mint).await;

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "GLOBAL_RESULT",
                &format!("🌐 GLOBAL price result for {}: ${:.12} SOL", mint, result.unwrap_or(0.0))
            );
        }

        return result;
    } else {
        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "GLOBAL_ERROR",
                &format!("❌ GLOBAL service not available for {}", mint)
            );
        }
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

    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
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
    } else {
        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "GLOBAL_BLOCKING_ERROR",
                &format!("❌ GLOBAL BLOCKING service not available for {}", mint)
            );
        }
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

    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "BATCH_START",
            &format!("🔄 BATCH PRICE REQUEST for {} tokens: {:?}", mints.len(), mints)
        );
    }

    // Use blocking version for more accurate prices, especially for open positions
    for mint in mints {
        let price = get_token_price_blocking_safe(mint).await;
        results.insert(mint.clone(), price);

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "BATCH_ITEM",
                &format!("📊 BATCH RESULT for {}: ${:.12} SOL", mint, price.unwrap_or(0.0))
            );
        }
    }

    if is_debug_price_service_enabled() {
        let found_prices = results
            .values()
            .filter(|p| p.is_some())
            .count();
        log(
            LogTag::PriceService,
            "BATCH_COMPLETE",
            &format!("✅ BATCH COMPLETE: {}/{} tokens have prices", found_prices, mints.len())
        );
    }

    results
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

    let service_guard = PRICE_SERVICE.lock().await;
    if let Some(ref service) = *service_guard {
        service.update_tokens_from_api(mints).await;
    } else {
        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "MONITOR_UPDATE_ERROR",
                "❌ MONITOR UPDATE FAILED: Price service not available"
            );
        }
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
