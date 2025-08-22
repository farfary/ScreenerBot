/// High-Performance Token Price Service
///
/// This module provides instant, thread-safe access to token prices with smart caching
/// and background monitoring. No timeouts on cache hits, automatic watch list management,
/// and prioritized pool price integration for open positions.

use crate::logger::{ log, LogTag, log_price_change };
use crate::global::is_debug_price_service_enabled;
use crate::tokens::types::ApiToken;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::pool::{ get_pool_service, PoolPriceResult };
use crate::tokens::is_system_or_stable_token;
use crate::positions::is_open_position;
use tokio::sync::RwLock;
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };
use tokio::sync::OnceCell;

// =============================================================================
// PRICE SERVICE CONFIGURATION
// =============================================================================

/// Maximum age for cached prices (in seconds)
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 10;

/// Time to keep watching a token after last request (in seconds)
const WATCH_TIMEOUT_SECONDS: i64 = 300; // 5 minutes

/// If fresh cache is missing, allow serving a slightly stale price up to this age (seconds)
/// This avoids N/A in UI while a background refresh runs
const STALE_RETURN_MAX_AGE_SECONDS: i64 = 180; // 3 minutes

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
}

impl TokenPriceService {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;
        Ok(Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            watch_list: Arc::new(RwLock::new(HashMap::new())),
            open_positions: Arc::new(RwLock::new(HashSet::new())),
            database,
        })
    }

    /// Get token price - instant cache lookup, no timeouts; will serve slightly stale values
    pub async fn get_token_price(&self, mint: &str) -> Option<f64> {
        // Track this token request
        self.add_to_watch_list(mint, false).await;

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
        // DEADLOCK FIX: Don't hold mutex during spawn
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
        // Track this token request
        self.add_to_watch_list(mint, false).await;

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
        if let Some(entry) = cache.get(mint) {
            let age_seconds = (Utc::now() - entry.last_updated).num_seconds();
            let is_expired = entry.is_expired();

            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "CACHE_CHECK",
                    &format!(
                        "🔍 CACHE CHECK for {}: found_entry=YES, age={}s, max_age={}s, expired={}, allow_stale={}, price={:.12} SOL, source={}",
                        mint,
                        age_seconds,
                        PRICE_CACHE_MAX_AGE_SECONDS,
                        is_expired,
                        allow_stale,
                        entry.price_sol.unwrap_or(0.0),
                        entry.source
                    )
                );
            }

            if !is_expired || (allow_stale && age_seconds <= STALE_RETURN_MAX_AGE_SECONDS) {
                if let Some(price) = entry.price_sol {
                    if price > 0.0 && price.is_finite() {
                        if is_debug_price_service_enabled() {
                            log(
                                LogTag::PriceService,
                                "CACHE_VALID",
                                &format!(
                                    "✅ VALID {}CACHE for {}: price={:.12} SOL, age={}s",
                                    if is_expired {
                                        "STALE-"
                                    } else {
                                        ""
                                    },
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
        // Skip system/stable tokens from watch list
        if is_system_or_stable_token(mint) {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "SKIP_SYSTEM",
                    &format!("Skipping system/stable token from watch list: {}", mint)
                );
            }
            return;
        }

        let mut watch_list = self.watch_list.write().await;
        watch_list.insert(mint.to_string(), WatchEntry {
            last_requested: Utc::now(),
            is_open_position,
        });
    }

    async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        // Always prefer on-chain pool price when available; fall back to API
        let pool_service = get_pool_service();

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "POOL_REQUEST",
                &format!("🏊 REQUESTING POOL PRICE for {} (real-time on-chain pools)", mint)
            );
        }

        let pool_result = if pool_service.check_token_availability(mint).await {
            // Use last known cached price (any source) for comparison hints
            let cached_price_sol = {
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
                        "🔍 Preparing price comparison for {}: cached_price={:.12} SOL",
                        mint,
                        cached_price_sol.unwrap_or(0.0)
                    )
                );
            }

            let pool_result = pool_service.get_pool_price(mint, cached_price_sol).await;
            if is_debug_price_service_enabled() {
                if let Some(ref result) = pool_result {
                    log(
                        LogTag::PriceService,
                        "POOL_SUCCESS",
                        &format!(
                            "✅ POOL PRICE SUCCESS for {}: ${:.12} SOL from pool {} ({}) at {}",
                            mint,
                            result.price_sol.unwrap_or(0.0),
                            result.pool_address,
                            result.pool_type.as_ref().unwrap_or(&"Unknown Pool".to_string()),
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

                    // Compare with cached price if present
                    if let Some(cached_sol) = cached_price_sol {
                        if let Some(pool_sol) = result.price_sol {
                            let price_change = pool_sol - cached_sol;
                            let price_change_percent = if cached_sol != 0.0 {
                                ((pool_sol - cached_sol) / cached_sol) * 100.0
                            } else {
                                0.0
                            };

                            log(
                                LogTag::PriceService,
                                "POOL_VS_CACHED",
                                &format!(
                                    "💰 PRICE COMPARISON for {}: CACHED={:.12} → POOL={:.12} SOL (change: {:+.12} SOL, {:+.4}%)",
                                    mint,
                                    cached_sol,
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
                                        "🚨 SIGNIFICANT PRICE CHANGE for {}: {:+.4}% change from cached to pool price!",
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
        };

        // Try API only if no pool result
        let api_token = if pool_result.is_none() {
            self.database
                .get_token_by_mint(mint)
                .map_err(|e| format!("Failed to get token from database: {}", e))?
        } else {
            None
        };

        // Create cache entry
        let (cache_entry, pool_info_for_logging) = match (pool_result.as_ref(), api_token) {
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
                let pool_info = (
                    pool_result.pool_type.clone(), // Only use actual pool type from decoder, no fallback to dex_id
                    Some(pool_result.pool_address.clone()),
                );
                (PriceCacheEntry::from_pool_result(&pool_result), pool_info)
            }
            (None, Some(api_token)) => {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "API_PRICE",
                        &format!(
                            "Using API price for {}: ${:.8}",
                            mint,
                            api_token.price_sol.unwrap_or(0.0)
                        )
                    );
                }
                (PriceCacheEntry::from_api_token(&api_token), (None, None))
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

        // Check if this is an open position and log colored price changes (always visible, not debug-only)
        if is_open_position(mint).await {
            if let (Some(old), Some(new)) = (old_price, cache_entry.price_sol) {
                // Only log if there's an actual price change
                if (old - new).abs() > f64::EPSILON {
                    // Get token symbol from database
                    let symbol = match self.database.get_token_by_mint(mint) {
                        Ok(Some(token)) => {
                            if !token.symbol.is_empty() {
                                token.symbol
                            } else {
                                mint[..8].to_string()
                            }
                        }
                        _ => mint[..8].to_string(), // Fallback to mint prefix
                    };

                    // Extract pool information from the captured pool info
                    let (pool_type, pool_address) = match cache_entry.source.as_str() {
                        "pool" => {
                            let (pool_type_opt, pool_address_opt) = &pool_info_for_logging;
                            (pool_type_opt.as_deref(), pool_address_opt.as_deref())
                        }
                        _ => (None, None),
                    };

                    // Get API price for comparison if this is a pool price update
                    let api_price = if cache_entry.source == "pool" {
                        // For pool updates, try to get API price for comparison
                        match self.database.get_token_by_mint(mint) {
                            Ok(Some(token)) => token.price_sol,
                            _ => None,
                        }
                    } else {
                        None
                    };

                    // Get current position for P&L calculation
                    let current_pnl = {
                        use crate::positions::{ get_open_positions, calculate_position_pnl };
                        if
                            let Some(position) = get_open_positions().await
                                .into_iter()
                                .find(|pos| pos.mint == mint)
                        {
                            let (pnl_sol, pnl_percent) = calculate_position_pnl(
                                &position,
                                Some(new)
                            ).await;
                            Some((pnl_sol, pnl_percent))
                        } else {
                            None
                        }
                    };

                    // Log the colored price change
                    log_price_change(
                        mint,
                        &symbol,
                        old,
                        new,
                        &cache_entry.source,
                        pool_type,
                        pool_address,
                        api_price,
                        current_pnl
                    );
                }
            }
        }

        if is_debug_price_service_enabled() {
            let price_change = match (old_price, cache_entry.price_sol) {
                (Some(old), Some(new)) => {
                    let change = new - old;
                    let change_percent = if old != 0.0 { (change / old) * 100.0 } else { 0.0 };

                    // Log to console for immediate visibility
                    log(
                        LogTag::PriceService,
                        "UPDATE",
                        &format!(
                            "🔄 PRICE UPDATE for {}: {:.12} → {:.12} SOL ({:+.12} SOL, {:+.6}%)",
                            mint,
                            old,
                            new,
                            change,
                            change_percent
                        )
                    );

                    // Check if the price is completely static (exactly the same)
                    if (old - new).abs() < f64::EPSILON {
                        log(
                            LogTag::PriceService,
                            "STATIC",
                            &format!(
                                "⚠️  STATIC PRICE DETECTED for {}: Price has not changed at all (exactly {:.12} SOL)",
                                mint,
                                new
                            )
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
                    log(
                        LogTag::PriceService,
                        "NEW",
                        &format!("🆕 NEW PRICE for {}: {:.12} SOL (first time)", mint, new)
                    );
                    format!(" (new price: ${:.12} SOL)", new)
                }
                (Some(old), None) => {
                    log(
                        LogTag::PriceService,
                        "REMOVED",
                        &format!("❌ PRICE REMOVED for {}: was {:.12} SOL", mint, old)
                    );
                    format!(" (removed price: was ${:.12} SOL)", old)
                }
                (None, None) => {
                    log(LogTag::PriceService, "NO_DATA", &format!("❓ NO PRICE DATA for {}", mint));
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
            // (Removed) pool watch list pinning: pool service now derives tokens from price service priority list
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

        priority_tokens
    }

    /// Get watch list statistics (total, expired, never_checked=not applicable here so always 0)
    pub async fn get_watch_list_stats(&self) -> (usize, usize, usize) {
        let watch_list = self.watch_list.read().await;
        let total = watch_list.len();
        let mut expired = 0usize;
        for entry in watch_list.values() {
            if entry.is_expired() {
                expired += 1;
            }
        }
        (total, expired, 0)
    }

    pub async fn update_tokens_from_api(&self, mints: &[String]) {
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

/// Global price service instance (single Arc, lock-free access)
pub static PRICE_SERVICE: OnceCell<Arc<TokenPriceService>> = OnceCell::const_new();

/// Initialize the global price service
pub async fn initialize_price_service() -> Result<(), Box<dyn std::error::Error>> {
    if PRICE_SERVICE.get().is_some() {
        log(LogTag::PriceService, "INIT_SKIP", "Price service already initialized, skipping");
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

/// Update open positions in global service
pub async fn update_open_positions_safe(mints: Vec<String>) {
    if let Some(service) = PRICE_SERVICE.get() {
        service.update_open_positions(mints).await;
    }
}

/// Get priority tokens for monitoring
pub async fn get_priority_tokens_safe() -> Vec<String> {
    if let Some(service) = PRICE_SERVICE.get() {
        return service.get_priority_tokens().await;
    }
    Vec::new()
}

/// Get watch list statistics (total, expired, never_checked) from price service
pub async fn get_price_watch_list_stats_safe() -> (usize, usize, usize) {
    if let Some(service) = PRICE_SERVICE.get() {
        return service.get_watch_list_stats().await;
    }
    (0, 0, 0)
}

/// Get multiple token prices in batch (for compatibility)
pub async fn get_token_prices_batch_safe(mints: &[String]) -> HashMap<String, Option<f64>> {
    use futures::stream::{ FuturesOrdered, StreamExt };

    if is_debug_price_service_enabled() {
        log(
            LogTag::PriceService,
            "BATCH_START",
            &format!("🔄 BATCH PRICE REQUEST for {} tokens: {:?}", mints.len(), mints)
        );
    }

    let mut results = HashMap::new();

    if let Some(service) = PRICE_SERVICE.get() {
        let mut futs = FuturesOrdered::new();
        for mint in mints.iter().cloned() {
            let svc = service.clone();
            futs.push_back(async move {
                // Non-blocking, cache-first; returns Some(stale_ok) fast or None and triggers refresh
                let price = svc.get_token_price(&mint).await;
                (mint, price)
            });
        }

        while let Some((mint, price)) = futs.next().await {
            if is_debug_price_service_enabled() {
                log(
                    LogTag::PriceService,
                    "BATCH_ITEM",
                    &format!("📊 BATCH RESULT for {}: ${:.12} SOL", mint, price.unwrap_or(0.0))
                );
            }
            results.insert(mint, price);
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

        return results;
    }

    // Fallback if service not initialized
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

/// Get cache statistics
pub async fn get_price_cache_stats() -> String {
    if let Some(service) = PRICE_SERVICE.get() {
        let (total, valid, expired) = service.get_cache_stats().await;
        return format!("Price Cache: {} total, {} valid, {} expired", total, valid, expired);
    }
    "Price Cache: Not available".to_string()
}

/// Cleanup expired cache entries
pub async fn cleanup_price_cache() -> usize {
    if let Some(service) = PRICE_SERVICE.get() {
        return service.cleanup_expired().await;
    }
    0
}
