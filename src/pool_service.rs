use crate::logger::{log, LogTag};
use crate::pool_interface::{
    PoolInterface, PoolStats, PriceOptions, PricePoint, PriceResult, TokenPriceInfo,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Price cache TTL in seconds
const PRICE_CACHE_TTL_SECS: i64 = 30;

/// Maximum number of tokens to track
const MAX_TRACKED_TOKENS: usize = 10000;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Simple pool service that provides cached price data
pub struct PoolService {
    /// Price cache: token_mint -> TokenPriceInfo
    price_cache: Arc<RwLock<HashMap<String, TokenPriceInfo>>>,
    /// Available tokens list
    available_tokens: Arc<RwLock<Vec<String>>>,
    /// Service statistics
    stats: Arc<RwLock<PoolStats>>,
    /// Service state
    is_running: Arc<RwLock<bool>>,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolService {
    /// Create new pool service
    pub fn new() -> Self {
        Self {
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            available_tokens: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(PoolStats::default())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the pool service
    pub async fn start(&self) {
        let mut running = self.is_running.write().await;
        if *running {
            log(
                LogTag::Pool,
                "SERVICE_ALREADY_RUNNING",
                "Pool service is already running",
            );
            return;
        }
        *running = true;
        drop(running);

        log(LogTag::Pool, "SERVICE_START", "🚀 Starting Pool Service");

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.last_update = Some(Utc::now());
        }

        log(
            LogTag::Pool,
            "SERVICE_READY",
            "✅ Pool Service started successfully",
        );
    }

    /// Stop the pool service
    pub async fn stop(&self) {
        let mut running = self.is_running.write().await;
        *running = false;

        log(LogTag::Pool, "SERVICE_STOP", "🛑 Pool Service stopped");
    }

    /// Update available tokens list
    pub async fn update_available_tokens(&self, tokens: Vec<String>) {
        let mut available = self.available_tokens.write().await;
        *available = tokens;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_tokens_available = available.len();
        stats.last_update = Some(Utc::now());
    }

    /// Update price cache with new price data
    pub async fn update_price_cache(&self, token_mint: String, price_info: TokenPriceInfo) {
        let mut cache = self.price_cache.write().await;
        cache.insert(token_mint, price_info);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.successful_price_fetches += 1;
        stats.last_update = Some(Utc::now());
    }

    /// Get service statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Check if service is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

// =============================================================================
// POOL INTERFACE IMPLEMENTATION
// =============================================================================

#[async_trait]
impl PoolInterface for PoolService {
    /// Get current price for a token
    async fn get_price(&self, token_address: &str) -> Option<TokenPriceInfo> {
        let cache = self.price_cache.read().await;
        let price_info = cache.get(token_address)?;

        // Check if price is still fresh
        let now = Utc::now();
        let age = now.signed_duration_since(price_info.calculated_at);
        if age.num_seconds() > PRICE_CACHE_TTL_SECS {
            // Price is stale, return None
            return None;
        }

        // Update cache hit stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_hits += 1;
        }

        Some(price_info.clone())
    }

    /// Get price history for a token (placeholder implementation)
    async fn get_price_history(&self, _token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
        // TODO: Implement price history retrieval from database
        vec![]
    }

    /// Get list of tokens with available prices
    async fn get_available_tokens(&self) -> Vec<String> {
        let available = self.available_tokens.read().await;
        available.clone()
    }

    /// Get batch prices for multiple tokens
    async fn get_batch_prices(
        &self,
        token_addresses: &[String],
    ) -> HashMap<String, TokenPriceInfo> {
        let cache = self.price_cache.read().await;
        let mut result = HashMap::new();

        for token_address in token_addresses {
            if let Some(price_info) = cache.get(token_address) {
                // Check if price is still fresh
                let now = Utc::now();
                let age = now.signed_duration_since(price_info.calculated_at);
                if age.num_seconds() <= PRICE_CACHE_TTL_SECS {
                    result.insert(token_address.clone(), price_info.clone());
                }
            }
        }

        // Update cache hit stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_hits += result.len() as u64;
        }

        result
    }
}

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

use std::sync::OnceLock;

static POOL_SERVICE: OnceLock<PoolService> = OnceLock::new();

/// Initialize the global pool service instance
pub fn init_pool_service() -> &'static PoolService {
    POOL_SERVICE.get_or_init(|| {
        log(LogTag::Pool, "INIT", "🏗️ Initializing Pool Service");
        PoolService::new()
    })
}

/// Get the global pool service instance
pub fn get_pool_service() -> &'static PoolService {
    POOL_SERVICE.get().expect("Pool service not initialized")
}

// =============================================================================
// LEGACY COMPATIBILITY FUNCTIONS
// =============================================================================

/// Legacy compatibility: Get price for a token (returns SOL price only)
pub async fn get_price(token_address: &str) -> Option<f64> {
    if let Some(price_info) = get_pool_service().get_price(token_address).await {
        price_info.get_best_sol_price()
    } else {
        None
    }
}

/// Legacy compatibility: Get full price result
pub async fn get_price_full(
    token_address: &str,
    _options: Option<PriceOptions>,
    _warm: bool,
) -> Option<PriceResult> {
    if let Some(price_info) = get_pool_service().get_price(token_address).await {
        Some(PriceResult::from(price_info))
    } else {
        Some(PriceResult {
            token_mint: token_address.to_string(),
            price_sol: None,
            price_usd: None,
            pool_address: None,
            reserve_sol: None,
            calculated_at: Utc::now(),
            error: Some("Price not available".to_string()),
        })
    }

}

/// Legacy compatibility: Get price history for a token
pub async fn get_price_history(token_address: &str) -> Vec<(DateTime<Utc>, f64)> {
    get_pool_service().get_price_history(token_address).await
}

/// Legacy compatibility: Get tokens with recent pools info
pub async fn get_tokens_with_recent_pools_infos(_window_seconds: i64) -> Vec<String> {
    get_pool_service().get_available_tokens().await
}

/// Check if a token has available price data
pub async fn check_token_availability(token_address: &str) -> bool {
    get_pool_service().get_price(token_address).await.is_some()
}

/// Start monitoring service (placeholder)
pub async fn start_monitoring() {
    log(LogTag::Pool, "INFO", "Pool service monitoring started");
}

/// Stop monitoring service (placeholder)
pub async fn stop_monitoring() {
    log(LogTag::Pool, "INFO", "Pool service monitoring stopped");
}

/// Clear token from all caches (placeholder)
pub async fn clear_token_from_all_caches(_token_mint: &str) {
    // Placeholder implementation - no actual cache clearing needed
}
