/// High-Performance Token Price Service
///
/// This module provides instant, thread-safe access to token prices with smart caching
/// and background monitoring. No timeouts on cache hits, automatic watch list management,
/// and prioritized pool price integration for open positions.

use crate::logger::{ log, LogTag };
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
/// Price cache TTL - optimized for fastest 5-second priority checking
const PRICE_CACHE_TTL_SECONDS: i64 = 5; // 5 seconds to match monitoring cycle
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 5; // 5 seconds maximum age for all prices

/// Time to keep watching a token after last request (in seconds)
const WATCH_TIMEOUT_SECONDS: i64 = 300; // 5 minutes

/// If fresh cache is missing, allow serving a slightly stale price up to this age (seconds)
/// This avoids N/A in UI while a background refresh runs
const STALE_RETURN_MAX_AGE_SECONDS: i64 = 180; // 3 minutes
/// Maximum allowed age for an open position price before forcing refresh - FASTEST 5s priority checking
const OPEN_POSITION_MAX_AGE_SECONDS: i64 = 3; // 3 seconds - force refresh every 5-second monitoring cycle
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
        // REMOVED: add_to_watch_list call - only open positions should be priority
        // Per user requirement: "only open positions is priority"

        // First check cache (instant, no await on lock contention)
        if let Some(price) = self.get_cached_price_maybe_stale(mint, true).await {
            // SELF-HEAL: if this is actually an open position but not in open_positions set
            // CONCURRENCY FIX: Avoid holding locks during external calls
            let mut needs_refresh = false;
            if let Some(service) = PRICE_SERVICE.get().cloned() {
                // Get cache entry without holding lock during external call
                let entry_opt = {
                    let cache = self.price_cache.read().await;
                    cache.get(mint).cloned()
                };

                if let Some(entry) = entry_opt {
                    let age_seconds = (Utc::now() - entry.last_updated).num_seconds();
                    // Check real open position state (no locks held during this call)
                    let real_open = crate::positions::is_open_position(mint).await;
                    if real_open {
                        // Quick lock scope for auto-heal registration
                        let needs_registration = {
                            let open_pos = self.open_positions.read().await;
                            !open_pos.contains(mint)
                        };

                        if needs_registration {
                            let mut open_pos = self.open_positions.write().await;
                            // Double-check pattern to avoid race conditions
                            if !open_pos.contains(mint) {
                                open_pos.insert(mint.to_string());
                                drop(open_pos);
                                // Open position tracking is sufficient - no watch list needed
                                if is_debug_price_service_enabled() {
                                    log(
                                        LogTag::PriceService,
                                        "OPEN_POS_HEAL",
                                        &format!(
                                            "⚕️ Auto-registered open position {} into price service (age={}s)",
                                            mint,
                                            age_seconds
                                        )
                                    );
                                }
                            }
                        }
                        // Force refresh if stale beyond strict open-position freshness
                        if age_seconds > OPEN_POSITION_MAX_AGE_SECONDS {
                            needs_refresh = true;
                        }
                    }
                }
                if needs_refresh {
                    let mint_clone = mint.to_string();
                    tokio::spawn(async move {
                        let _ = service.update_single_token_price(&mint_clone).await;
                    });
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "FORCED_REFRESH",
                            &format!("🔄 Forced background refresh for open position {}", mint)
                        );
                    }
                }
            }
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
        // REMOVED: add_to_watch_list call - only open positions should be priority
        // Per user requirement: "only open positions is priority"

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
        // CONCURRENCY FIX: Get both values without nested locks to prevent deadlock
        let (entry_opt, is_open) = {
            let cache = self.price_cache.read().await;
            let entry_opt = cache.get(mint).cloned();
            drop(cache); // Release cache lock before acquiring positions lock
            let positions = self.open_positions.read().await;
            let is_open = positions.contains(mint);
            (entry_opt, is_open)
        };

        if let Some(entry) = entry_opt {
            let age = Utc::now() - entry.last_updated;
            let age_seconds = age.num_seconds();
            let is_expired = entry.is_expired();

            // For open positions enforce tight freshness; treat as expired beyond OPEN_POSITION_MAX_AGE_SECONDS
            if is_open && age_seconds > OPEN_POSITION_MAX_AGE_SECONDS {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "OPEN_STALE",
                        &format!(
                            "⚠️ OPEN POSITION STALE {} age={}s > {}s (forcing refresh)",
                            mint,
                            age_seconds,
                            OPEN_POSITION_MAX_AGE_SECONDS
                        )
                    );
                }
                return None; // force refresh path
            }

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

    /// Remove a token from watch list and open positions tracking
    /// Should be called when a position is closed to prevent resource waste
    async fn remove_from_watch_list(&self, mint: &str) {
        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "REMOVE_WATCH",
                &format!("🗑️ Removing {} from watch list (position closed)", mint)
            );
        }

        // Remove from watch list
        {
            let mut watch_list = self.watch_list.write().await;
            watch_list.remove(mint);
        }

        // Remove from open positions set
        {
            let mut open_positions = self.open_positions.write().await;
            open_positions.remove(mint);
        }
    }

    async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        let pool_service = get_pool_service();
        let is_open = is_open_position(mint).await;
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
                    "🏊 REQUESTING POOL PRICE for {} (open={}, timeout={}ms)",
                    mint,
                    is_open,
                    POOL_PRICE_TIMEOUT_MS
                )
            );
        }

        let mut pool_result = None;
        if pool_service.check_token_availability(mint).await {
            if is_open {
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
            } else {
                pool_result = pool_service.get_pool_price(mint, cached_price_sol).await;
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

        // Always update tracking for open positions (fresh P&L) and log price changes;
        // previous logic only ran when `is_open` flag was true inside the price service's
        // own open_positions set. If that set lags real position state (initialization race)
        // we miss logs. So we query positions handle directly for safety.
        let is_really_open = if is_open {
            true
        } else {
            crate::positions::is_open_position(mint).await
        };
        if is_really_open {
            if let Some(cp) = cache_entry.price_sol {
                // Always call position tracking for open positions - let position logic handle change detection

                if let Some(h) = crate::positions::get_positions_handle().await {
                    // Don't block price update pipeline on positions actor; fire-and-forget with a short timeout
                    let mint_clone = mint.to_string();
                    tokio::spawn(async move {
                        let tracking = h.update_tracking(mint_clone, cp);
                        // Give the actor a short window; if busy with background work, skip silently
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_millis(250),
                            tracking
                        ).await;
                    });
                } else {
                    log(
                        LogTag::PriceService,
                        "ERROR",
                        &format!("❌ No positions handle available for tracking update of {}", mint)
                    );
                }
            } else {
                log(
                    LogTag::PriceService,
                    "WARN",
                    &format!("⚠️ No price available for open position {}", mint)
                );
            }
        }
        // AUTO-HEAL: if token is really open but NOT in internal open_positions set, add it
        // CONCURRENCY FIX: Use double-check pattern to avoid race conditions
        if is_really_open && !is_open {
            let needs_registration = {
                let open = self.open_positions.read().await;
                !open.contains(mint)
            };

            if needs_registration {
                let mut open = self.open_positions.write().await;
                // Double-check to avoid race condition
                if !open.contains(mint) {
                    open.insert(mint.to_string());
                    drop(open);
                    // Open position tracking is sufficient - no watch list needed
                    if is_debug_price_service_enabled() {
                        log(
                            LogTag::PriceService,
                            "OPEN_POS_HEAL",
                            &format!("⚕️ Auto-healed missing open position registration for {}", mint)
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn update_open_positions(&self, mints: Vec<String>) {
        log(
            LogTag::PriceService,
            "UPDATE_OPEN_POSITIONS",
            &format!("🔄 Updating open positions with {} mints: {:?}", mints.len(), mints)
        );

        let mut positions = self.open_positions.write().await;
        let old_count = positions.len();
        positions.clear();
        for mint in mints {
            positions.insert(mint.clone());
            // Open positions are tracked directly in the set, no watch list needed
        }
        let new_count = positions.len();
        drop(positions);

        log(
            LogTag::PriceService,
            "UPDATE_OPEN_POSITIONS_COMPLETE",
            &format!("✅ Open positions updated: {} -> {} positions", old_count, new_count)
        );
    }

    pub async fn get_priority_tokens(&self) -> Vec<String> {
        // FIXED: Only return open positions as priority tokens
        // Per user requirement: "only open positions is priority"
        // This eliminates resource waste from monitoring discovery tokens
        let positions = self.open_positions.read().await;
        let priority_tokens: Vec<String> = positions.iter().cloned().collect();
        drop(positions);

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "PRIORITY_TOKENS_ONLY_POSITIONS",
                &format!(
                    "🎯 Priority tokens limited to {} open positions only",
                    priority_tokens.len()
                )
            );
        }

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

    /// Aggressive cleanup: Remove tokens that are no longer open positions
    /// This should be called periodically to prevent resource waste
    pub async fn cleanup_closed_positions(&self) -> usize {
        let mut removed_count = 0;

        // Get current open position mints using positions handle
        let current_open_mints = match crate::positions::get_positions_handle().await {
            Some(handle) => {
                handle.get_open_mints().await.into_iter().collect::<std::collections::HashSet<_>>()
            }
            None => {
                if is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "CLEANUP_ERROR",
                        "Failed to get positions handle for cleanup"
                    );
                }
                return 0;
            }
        };

        if is_debug_price_service_enabled() {
            log(
                LogTag::PriceService,
                "CLEANUP_AGGRESSIVE",
                &format!(
                    "🧹 Starting aggressive cleanup - {} actual open positions",
                    current_open_mints.len()
                )
            );
        }

        // Clean watch list: remove tokens not in current open positions
        {
            let mut watch_list = self.watch_list.write().await;
            let initial_size = watch_list.len();

            watch_list.retain(|mint, entry| {
                let is_open = current_open_mints.contains(mint);
                let keep = is_open || !entry.is_expired();

                if !keep && is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "CLEANUP_REMOVE",
                        &format!("🗑️ Removing {} from watch list (not open position)", mint)
                    );
                }

                keep
            });

            removed_count += initial_size - watch_list.len();
        }

        // Clean open positions set: remove tokens not actually open
        {
            let mut open_positions = self.open_positions.write().await;
            let initial_size = open_positions.len();

            open_positions.retain(|mint| {
                let keep = current_open_mints.contains(mint);

                if !keep && is_debug_price_service_enabled() {
                    log(
                        LogTag::PriceService,
                        "CLEANUP_REMOVE",
                        &format!("🗑️ Removing {} from open positions set (not actually open)", mint)
                    );
                }

                keep
            });

            removed_count += initial_size - open_positions.len();
        }

        if removed_count > 0 {
            log(
                LogTag::PriceService,
                "CLEANUP_AGGRESSIVE_COMPLETE",
                &format!(
                    "🧹 Aggressive cleanup removed {} stale entries. Watch list now matches {} open positions",
                    removed_count,
                    current_open_mints.len()
                )
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

/// Update open positions in global service
pub async fn update_open_positions_safe(mints: Vec<String>) {
    if let Some(service) = PRICE_SERVICE.get() {
        service.update_open_positions(mints).await;
    }
}

/// Remove a closed position from watch list to prevent resource waste
pub async fn remove_closed_position_safe(mint: &str) {
    if let Some(service) = PRICE_SERVICE.get() {
        service.remove_from_watch_list(mint).await;
    }
}

/// Perform aggressive cleanup to remove closed positions from watch lists
pub async fn cleanup_closed_positions_safe() -> usize {
    if let Some(service) = PRICE_SERVICE.get() {
        return service.cleanup_closed_positions().await;
    }
    0
}

/// Force an immediate price refresh for a token (bypass cache freshness)
pub async fn force_refresh_token_price_safe(mint: &str) {
    if let Some(service) = PRICE_SERVICE.get() {
        // Directly call the underlying update without preliminary cache check
        let _ = service.update_single_token_price(mint).await;
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
