/// Thread-Safe Token Price Service
///
/// This module provides a thread-safe interface for accessing token prices
/// and data between the trader and tokens system. It eliminates direct
/// database access and provides a clean API for price lookups.

use crate::logger::{ log, LogTag };
use crate::global::is_debug_monitor_enabled;
use crate::tokens::types::ApiToken;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::blacklist::is_token_blacklisted;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };

// =============================================================================
// PRICE SERVICE CONFIGURATION
// =============================================================================

/// Maximum age for cached prices (in seconds for real-time updates)
const PRICE_CACHE_MAX_AGE_SECONDS: i64 = 5;

/// Priority boost for tokens with open positions (higher priority)
const OPEN_POSITION_PRIORITY: i32 = 100;

/// Priority boost for high liquidity tokens
const HIGH_LIQUIDITY_PRIORITY: i32 = 100;

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
}

impl PriceCacheEntry {
    pub fn is_expired(&self) -> bool {
        let age = Utc::now() - self.last_updated;
        age > Duration::seconds(PRICE_CACHE_MAX_AGE_SECONDS)
    }

    pub fn from_api_token(token: &ApiToken, priority: i32) -> Self {
        // Safety check: only store valid positive prices
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
        }
    }
}

// =============================================================================
// THREAD-SAFE PRICE SERVICE
// =============================================================================

pub struct TokenPriceService {
    price_cache: Arc<RwLock<HashMap<String, PriceCacheEntry>>>,
    database: TokenDatabase,
    open_positions: Arc<RwLock<Vec<String>>>, // Track mints with open positions
}

impl TokenPriceService {
    /// Create new price service instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;

        Ok(Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            database,
            open_positions: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Get current price for a specific token (thread-safe)
    pub async fn get_token_price(&self, mint: &str) -> Option<f64> {
        // Check cache first
        if let Some(price) = self.get_cached_price(mint).await {
            return Some(price);
        }

        // If not in cache or expired, request update for this specific token
        if let Err(e) = self.update_single_token_price(mint).await {
            log(LogTag::Trader, "WARN", &format!("Failed to update price for {}: {}", mint, e));
        }

        // Try cache again after update
        self.get_cached_price(mint).await
    }

    /// Get cached price if valid
    async fn get_cached_price(&self, mint: &str) -> Option<f64> {
        let cache = self.price_cache.read().await;

        if let Some(entry) = cache.get(mint) {
            if !entry.is_expired() {
                // Safety check: only return positive, finite prices
                if let Some(price) = entry.price_sol {
                    if price > 0.0 && price.is_finite() {
                        return Some(price);
                    }
                }
            }
        }

        None
    }

    /// Update price for a single token
    async fn update_single_token_price(&self, mint: &str) -> Result<(), String> {
        // Get token from database
        let token = self.database
            .get_token_by_mint(mint)
            .map_err(|e| format!("Failed to get token from database: {}", e))?;

        if let Some(token) = token {
            let priority = self.calculate_priority(&token).await;
            let entry = PriceCacheEntry::from_api_token(&token, priority);

            let mut cache = self.price_cache.write().await;
            cache.insert(mint.to_string(), entry);

            // Only log individual price updates in debug mode to reduce noise
            // Normal operation should be quiet for individual token updates
        }

        Ok(())
    }

    /// Update open positions list (called from trader)
    pub async fn update_open_positions(&self, mints: Vec<String>) {
        let mut positions = self.open_positions.write().await;
        *positions = mints;

        log(
            LogTag::Trader,
            "POSITIONS",
            &format!("Updated open positions tracking: {} tokens", positions.len())
        );
    }

    /// Get priority tokens for monitoring (open positions + ALL tradeable tokens)
    pub async fn get_priority_tokens_for_monitoring(&self) -> Result<Vec<String>, String> {
        let mut priority_mints = Vec::new();

        // Always include open positions with highest priority
        {
            let positions = self.open_positions.read().await;
            priority_mints.extend(positions.clone());
        }

        // Get ALL tokens that are not blacklisted for comprehensive monitoring
        // This ensures the trader has fresh prices for all potential trading candidates
        let all_tokens = self.database
            .get_all_tokens().await
            .map_err(|e| format!("Failed to get all tokens from database: {}", e))?;

        // Add all non-blacklisted tokens with some liquidity
        let additional_tokens: Vec<String> = all_tokens
            .into_iter()
            .filter(|token| !is_token_blacklisted(&token.mint))
            .filter(|token| !priority_mints.contains(&token.mint))
            .filter(|token| {
                // Include tokens with any liquidity > $100 (very low threshold)
                token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0) > 100.0
            })
            .map(|token| token.mint)
            .collect();

        priority_mints.extend(additional_tokens);

        log(
            LogTag::Trader,
            "MONITOR",
            &format!(
                "Priority tokens for monitoring: {} total (includes all tradeable tokens)",
                priority_mints.len()
            )
        );

        Ok(priority_mints)
    }

    /// Bulk update prices for multiple tokens
    pub async fn update_tokens_prices(&self, mints: &[String]) -> Result<usize, String> {
        let mut updated_count = 0;

        // Get tokens from database
        let tokens = self.database
            .get_tokens_by_mints(mints).await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        // Update cache with fresh data
        {
            let mut cache = self.price_cache.write().await;

            for token in &tokens {
                let priority = self.calculate_priority(token).await;
                let entry = PriceCacheEntry::from_api_token(token, priority);
                cache.insert(token.mint.clone(), entry);
                updated_count += 1;
            }
        }

        // Only log summary for significant updates or errors
        if updated_count > 0 && mints.len() > 20 && is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "UPDATE",
                &format!("Updated {} token prices in cache", updated_count)
            );
        }

        Ok(updated_count)
    }

    /// Calculate priority for a token based on open positions and liquidity
    async fn calculate_priority(&self, token: &ApiToken) -> i32 {
        let mut priority = 0;

        // Check if this token has an open position
        {
            let positions = self.open_positions.read().await;
            if positions.contains(&token.mint) {
                priority += OPEN_POSITION_PRIORITY;
            }
        }

        // High liquidity boost
        if let Some(liquidity) = token.liquidity.as_ref().and_then(|l| l.usd) {
            if liquidity > HIGH_LIQUIDITY_THRESHOLD {
                priority += HIGH_LIQUIDITY_PRIORITY;
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
                LogTag::Trader,
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
}

// =============================================================================
// GLOBAL PRICE SERVICE INSTANCE
// =============================================================================

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

/// Global thread-safe price service instance
pub static PRICE_SERVICE: Lazy<Arc<Mutex<Option<TokenPriceService>>>> = Lazy::new(||
    Arc::new(Mutex::new(None))
);

/// Initialize the global price service
pub async fn initialize_price_service() -> Result<(), Box<dyn std::error::Error>> {
    let service = TokenPriceService::new()?;

    let mut global_service = PRICE_SERVICE.lock().await;
    *global_service = Some(service);

    log(LogTag::System, "INIT", "Price service initialized successfully");

    Ok(())
}

/// Get token price using global service (thread-safe API for trader)
pub async fn get_token_price_safe(mint: &str) -> Option<f64> {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            return service.get_token_price(mint).await;
        }
    }

    log(LogTag::Trader, "WARN", &format!("Price service not available for token: {}", mint));
    None
}

/// Update open positions in global service (called from trader)
pub async fn update_open_positions_safe(mints: Vec<String>) {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            service.update_open_positions(mints).await;
        }
    }
}

/// Get priority tokens for monitoring (called from monitor)
pub async fn get_priority_tokens_safe() -> Vec<String> {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            return service.get_priority_tokens_for_monitoring().await.unwrap_or_default();
        }
    }

    Vec::new()
}

/// Update multiple token prices (called from monitor)
pub async fn update_tokens_prices_safe(mints: &[String]) -> usize {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            return service.update_tokens_prices(mints).await.unwrap_or(0);
        }
    }

    0
}

/// Get cache statistics
pub async fn get_price_cache_stats() -> String {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            let (total, valid, expired) = service.get_cache_stats().await;
            return format!("Price Cache: {} total, {} valid, {} expired", total, valid, expired);
        }
    }

    "Price Cache: Not available".to_string()
}

/// Cleanup expired cache entries
pub async fn cleanup_price_cache() -> usize {
    if let Ok(service_guard) = PRICE_SERVICE.try_lock() {
        if let Some(ref service) = *service_guard {
            return service.cleanup_expired_entries().await;
        }
    }

    0
}

/// Cleanup price service on shutdown
pub async fn cleanup_price_service() {
    cleanup_price_cache().await;
}
