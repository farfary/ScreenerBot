use crate::logger::{ log, LogTag };
use crate::global::is_debug_pool_cleanup_enabled;
use crate::pool_interface::TokenPriceInfo;
use crate::pool_discovery::{ PoolData, AccountInfo };
use crate::pool_constants::*;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Cleanup statistics
#[derive(Debug, Clone)]
pub struct PoolCleanupStats {
    pub total_cleanups: u64,
    pub successful_cleanups: u64,
    pub failed_cleanups: u64,
    pub price_cache_cleaned: u64,
    pub account_data_cleaned: u64,
    pub pool_data_cleaned: u64,
    pub tracked_tokens_cleaned: u64,
    pub total_items_cleaned: u64,
    pub average_cleanup_time_ms: f64,
    pub last_cleanup: Option<DateTime<Utc>>,
}

impl Default for PoolCleanupStats {
    fn default() -> Self {
        Self {
            total_cleanups: 0,
            successful_cleanups: 0,
            failed_cleanups: 0,
            price_cache_cleaned: 0,
            account_data_cleaned: 0,
            pool_data_cleaned: 0,
            tracked_tokens_cleaned: 0,
            total_items_cleaned: 0,
            average_cleanup_time_ms: 0.0,
            last_cleanup: None,
        }
    }
}

impl PoolCleanupStats {
    pub fn get_success_rate(&self) -> f64 {
        if self.total_cleanups == 0 {
            0.0
        } else {
            ((self.successful_cleanups as f64) / (self.total_cleanups as f64)) * 100.0
        }
    }

    pub fn record_cleanup(&mut self, success: bool, time_ms: f64, items_cleaned: usize) {
        self.total_cleanups += 1;
        if success {
            self.successful_cleanups += 1;
            self.total_items_cleaned += items_cleaned as u64;
        } else {
            self.failed_cleanups += 1;
        }

        // Update average time
        let total_time =
            self.average_cleanup_time_ms * ((self.total_cleanups - 1) as f64) + time_ms;
        self.average_cleanup_time_ms = total_time / (self.total_cleanups as f64);

        self.last_cleanup = Some(Utc::now());
    }
}

/// Service shared state for cleanup operations
#[derive(Debug, Clone)]
pub struct CleanupServiceState {
    /// All tokens being tracked
    pub tracked_tokens: HashMap<String, DateTime<Utc>>, // token_mint -> last_seen
    /// Best pool for each token (highest liquidity)
    pub best_pools: HashMap<String, PoolData>, // token_mint -> pool_data
    /// Account addresses to fetch data for
    pub account_queue: Vec<AccountInfo>,
    /// Raw account data cache
    pub account_data_cache: HashMap<String, (Vec<u8>, DateTime<Utc>)>, // address -> (data, timestamp)
}

/// Trait for types that can be cleaned up
pub trait Cleanupable {
    fn get_tracked_tokens(&self) -> &HashMap<String, DateTime<Utc>>;
    fn get_best_pools(&self) -> &HashMap<String, PoolData>;
    fn get_account_queue(&self) -> &Vec<AccountInfo>;
    fn get_account_data_cache(&self) -> &HashMap<String, (Vec<u8>, DateTime<Utc>)>;

    fn remove_tracked_token(&mut self, token_mint: &str);
    fn remove_best_pool(&mut self, token_mint: &str);
    fn remove_account_queue_item(&mut self, index: usize);
    fn remove_account_data(&mut self, address: &str);
}

/// Pool cleanup service
pub struct PoolCleanupService {
    stats: Arc<RwLock<PoolCleanupStats>>,
    debug_enabled: bool,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolCleanupService {
    /// Create new pool cleanup service
    pub fn new() -> Self {
        let debug_enabled = is_debug_pool_cleanup_enabled();

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool cleanup service debug mode enabled");
        }

        Self {
            stats: Arc::new(RwLock::new(PoolCleanupStats::default())),
            debug_enabled,
        }
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool cleanup service debug mode enabled (overridden)");
    }

    /// Clean up stale data from all caches and state
    pub async fn cleanup_all_data(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<usize, String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(LogTag::Pool, "CLEANUP_START", "🧹 Starting comprehensive data cleanup");
        }

        let mut total_cleaned = 0;

        // 1. Clean up stale price cache entries
        let price_cleaned = self.cleanup_price_cache(price_cache).await?;
        total_cleaned += price_cleaned;

        // 2. Clean up stale account data cache
        let account_cleaned = self.cleanup_account_data_cache(shared_state).await?;
        total_cleaned += account_cleaned;

        // 3. Clean up stale pool data
        let pool_cleaned = self.cleanup_pool_data(shared_state).await?;
        total_cleaned += pool_cleaned;

        // 4. Clean up inactive tracked tokens
        let tokens_cleaned = self.cleanup_tracked_tokens(shared_state).await?;
        total_cleaned += tokens_cleaned;

        // 5. Clean up stale account queue entries
        let queue_cleaned = self.cleanup_account_queue(shared_state).await?;
        total_cleaned += queue_cleaned;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.price_cache_cleaned += price_cleaned as u64;
            stats.account_data_cleaned += account_cleaned as u64;
            stats.pool_data_cleaned += pool_cleaned as u64;
            stats.tracked_tokens_cleaned += tokens_cleaned as u64;
            stats.record_cleanup(true, start_time.elapsed().as_millis() as f64, total_cleaned);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLEANUP_COMPLETE",
                &format!(
                    "🧹 Cleanup completed: {} items cleaned (Price: {}, Account: {}, Pool: {}, Tokens: {}, Queue: {})",
                    total_cleaned,
                    price_cleaned,
                    account_cleaned,
                    pool_cleaned,
                    tokens_cleaned,
                    queue_cleaned
                )
            );
        }

        Ok(total_cleaned)
    }

    /// Clean up stale price cache entries
    async fn cleanup_price_cache(
        &self,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<usize, String> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        {
            let mut cache = price_cache.write().await;
            let mut to_remove = Vec::new();

            for (token_mint, price_info) in cache.iter() {
                let age = now.signed_duration_since(price_info.calculated_at);
                if age.num_seconds() > PRICE_CACHE_TTL_SECS {
                    to_remove.push(token_mint.clone());
                }
            }

            for token_mint in to_remove {
                cache.remove(&token_mint);
                cleaned_count += 1;
            }
        }

        if self.debug_enabled && cleaned_count > 0 {
            log(
                LogTag::Pool,
                "PRICE_CACHE_CLEANUP",
                &format!("Cleaned {} stale price cache entries", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Clean up stale account data cache
    async fn cleanup_account_data_cache(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>
    ) -> Result<usize, String> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        {
            let mut state = shared_state.write().await;
            let mut to_remove = Vec::new();

            for (address, (_, fetch_time)) in state.account_data_cache.iter() {
                let age = now.signed_duration_since(*fetch_time);
                if age.num_seconds() > ACCOUNT_DATA_CACHE_TTL_SECS {
                    to_remove.push(address.clone());
                }
            }

            for address in to_remove {
                state.account_data_cache.remove(&address);
                cleaned_count += 1;
            }
        }

        if self.debug_enabled && cleaned_count > 0 {
            log(
                LogTag::Pool,
                "ACCOUNT_DATA_CLEANUP",
                &format!("Cleaned {} stale account data cache entries", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Clean up stale pool data
    async fn cleanup_pool_data(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>
    ) -> Result<usize, String> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        {
            let mut state = shared_state.write().await;
            let mut to_remove = Vec::new();

            for (token_mint, pool_data) in state.best_pools.iter() {
                let age = now.signed_duration_since(pool_data.last_updated);
                if age.num_seconds() > POOL_DATA_CACHE_TTL_SECS {
                    to_remove.push(token_mint.clone());
                }
            }

            for token_mint in to_remove {
                state.best_pools.remove(&token_mint);
                cleaned_count += 1;
            }
        }

        if self.debug_enabled && cleaned_count > 0 {
            log(
                LogTag::Pool,
                "POOL_DATA_CLEANUP",
                &format!("Cleaned {} stale pool data entries", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Clean up inactive tracked tokens
    async fn cleanup_tracked_tokens(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>
    ) -> Result<usize, String> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        {
            let mut state = shared_state.write().await;
            let mut to_remove = Vec::new();

            for (token_mint, last_seen) in state.tracked_tokens.iter() {
                let age = now.signed_duration_since(*last_seen);
                if age.num_seconds() > TRACKED_TOKENS_TTL_SECS {
                    to_remove.push(token_mint.clone());
                }
            }

            for token_mint in to_remove {
                state.tracked_tokens.remove(&token_mint);
                cleaned_count += 1;
            }
        }

        if self.debug_enabled && cleaned_count > 0 {
            log(
                LogTag::Pool,
                "TRACKED_TOKENS_CLEANUP",
                &format!("Cleaned {} inactive tracked tokens", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Clean up stale account queue entries
    async fn cleanup_account_queue(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>
    ) -> Result<usize, String> {
        let mut cleaned_count = 0;
        let now = Utc::now();

        {
            let mut state = shared_state.write().await;
            let mut to_remove = Vec::new();

            for (i, account_info) in state.account_queue.iter().enumerate() {
                if let Some(last_fetched) = account_info.last_fetched {
                    let age = now.signed_duration_since(last_fetched);
                    if age.num_seconds() > ACCOUNT_DATA_CACHE_TTL_SECS {
                        to_remove.push(i);
                    }
                }
            }

            // Remove from the end to maintain indices
            for &index in to_remove.iter().rev() {
                state.account_queue.remove(index);
                cleaned_count += 1;
            }
        }

        if self.debug_enabled && cleaned_count > 0 {
            log(
                LogTag::Pool,
                "ACCOUNT_QUEUE_CLEANUP",
                &format!("Cleaned {} stale account queue entries", cleaned_count)
            );
        }

        Ok(cleaned_count)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolCleanupStats {
        self.stats.read().await.clone()
    }

    /// Get cache sizes for monitoring
    pub async fn get_cache_sizes(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> (usize, usize, usize, usize, usize) {
        let state = shared_state.read().await;
        let price_cache_size = price_cache.read().await.len();

        (
            price_cache_size,
            state.tracked_tokens.len(),
            state.best_pools.len(),
            state.account_data_cache.len(),
            state.account_queue.len(),
        )
    }

    /// Clean up stale data using generic cleanup functions
    pub async fn cleanup_with_functions<F1, F2, F3, F4, F5>(
        &self,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>,
        cleanup_price_cache: F1,
        cleanup_account_data: F2,
        cleanup_pool_data: F3,
        cleanup_tracked_tokens: F4,
        cleanup_account_queue: F5
    )
        -> Result<usize, String>
        where
            F1: FnOnce() -> Result<usize, String>,
            F2: FnOnce() -> Result<usize, String>,
            F3: FnOnce() -> Result<usize, String>,
            F4: FnOnce() -> Result<usize, String>,
            F5: FnOnce() -> Result<usize, String>
    {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(LogTag::Pool, "CLEANUP_START", "🧹 Starting comprehensive data cleanup");
        }

        let mut total_cleaned = 0;

        // 1. Clean up stale price cache entries
        let price_cleaned = cleanup_price_cache()?;
        total_cleaned += price_cleaned;

        // 2. Clean up stale account data cache
        let account_cleaned = cleanup_account_data()?;
        total_cleaned += account_cleaned;

        // 3. Clean up stale pool data
        let pool_cleaned = cleanup_pool_data()?;
        total_cleaned += pool_cleaned;

        // 4. Clean up inactive tracked tokens
        let tokens_cleaned = cleanup_tracked_tokens()?;
        total_cleaned += tokens_cleaned;

        // 5. Clean up stale account queue entries
        let queue_cleaned = cleanup_account_queue()?;
        total_cleaned += queue_cleaned;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.price_cache_cleaned += price_cleaned as u64;
            stats.account_data_cleaned += account_cleaned as u64;
            stats.pool_data_cleaned += pool_cleaned as u64;
            stats.tracked_tokens_cleaned += tokens_cleaned as u64;
            stats.record_cleanup(true, start_time.elapsed().as_millis() as f64, total_cleaned);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLEANUP_COMPLETE",
                &format!(
                    "🧹 Cleanup completed: {} items cleaned (Price: {}, Account: {}, Pool: {}, Tokens: {}, Queue: {})",
                    total_cleaned,
                    price_cleaned,
                    account_cleaned,
                    pool_cleaned,
                    tokens_cleaned,
                    queue_cleaned
                )
            );
        }

        Ok(total_cleaned)
    }

    /// Clean up stale data directly from ServiceState (for pool_service.rs integration)
    pub async fn cleanup_service_state(
        &self,
        shared_state: &Arc<RwLock<CleanupServiceState>>,
        price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
    ) -> Result<(usize, CleanupServiceState), String> {
        let start_time = Instant::now();

        if self.debug_enabled {
            log(LogTag::Pool, "CLEANUP_START", "🧹 Starting comprehensive data cleanup");
        }

        let mut total_cleaned = 0;

        // 1. Clean up stale price cache entries
        let price_cleaned = self.cleanup_price_cache(price_cache).await?;
        total_cleaned += price_cleaned;

        // 2. Clean up stale account data cache
        let account_cleaned = self.cleanup_account_data_cache(shared_state).await?;
        total_cleaned += account_cleaned;

        // 3. Clean up stale pool data
        let pool_cleaned = self.cleanup_pool_data(shared_state).await?;
        total_cleaned += pool_cleaned;

        // 4. Clean up inactive tracked tokens
        let tokens_cleaned = self.cleanup_tracked_tokens(shared_state).await?;
        total_cleaned += tokens_cleaned;

        // 5. Clean up stale account queue entries
        let queue_cleaned = self.cleanup_account_queue(shared_state).await?;
        total_cleaned += queue_cleaned;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.price_cache_cleaned += price_cleaned as u64;
            stats.account_data_cleaned += account_cleaned as u64;
            stats.pool_data_cleaned += pool_cleaned as u64;
            stats.tracked_tokens_cleaned += tokens_cleaned as u64;
            stats.record_cleanup(true, start_time.elapsed().as_millis() as f64, total_cleaned);
        }

        if self.debug_enabled {
            log(
                LogTag::Pool,
                "CLEANUP_COMPLETE",
                &format!(
                    "🧹 Cleanup completed: {} items cleaned (Price: {}, Account: {}, Pool: {}, Tokens: {}, Queue: {})",
                    total_cleaned,
                    price_cleaned,
                    account_cleaned,
                    pool_cleaned,
                    tokens_cleaned,
                    queue_cleaned
                )
            );
        }

        // Get the cleaned state
        let cleaned_state = shared_state.read().await.clone();

        Ok((total_cleaned, cleaned_state))
    }
}

// =============================================================================
// GLOBAL INSTANCE MANAGEMENT
// =============================================================================

static GLOBAL_POOL_CLEANUP: std::sync::OnceLock<PoolCleanupService> = std::sync::OnceLock::new();

/// Initialize the global pool cleanup service
pub fn init_pool_cleanup() -> &'static PoolCleanupService {
    GLOBAL_POOL_CLEANUP.get_or_init(|| {
        log(LogTag::Pool, "INIT", "Initializing global pool cleanup service");
        PoolCleanupService::new()
    })
}

/// Get the global pool cleanup service
pub fn get_pool_cleanup() -> &'static PoolCleanupService {
    GLOBAL_POOL_CLEANUP.get().expect("Pool cleanup service not initialized")
}

// =============================================================================
// CONVENIENCE FUNCTIONS
// =============================================================================

/// Clean up all data (convenience function)
pub async fn cleanup_all_data(
    shared_state: &Arc<RwLock<CleanupServiceState>>,
    price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
) -> Result<usize, String> {
    get_pool_cleanup().cleanup_all_data(shared_state, price_cache).await
}

/// Clean up service state (convenience function)
pub async fn cleanup_service_state(
    shared_state: &Arc<RwLock<CleanupServiceState>>,
    price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
) -> Result<(usize, CleanupServiceState), String> {
    get_pool_cleanup().cleanup_service_state(shared_state, price_cache).await
}

/// Get cleanup statistics (convenience function)
pub async fn get_pool_cleanup_stats() -> PoolCleanupStats {
    get_pool_cleanup().get_stats().await
}

/// Get cache sizes (convenience function)
pub async fn get_pool_cleanup_cache_sizes(
    shared_state: &Arc<RwLock<CleanupServiceState>>,
    price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>
) -> (usize, usize, usize, usize, usize) {
    get_pool_cleanup().get_cache_sizes(shared_state, price_cache).await
}

/// Clean up data with functions (convenience function)
pub async fn cleanup_with_functions<F1, F2, F3, F4, F5>(
    price_cache: &Arc<RwLock<HashMap<String, TokenPriceInfo>>>,
    cleanup_price_cache: F1,
    cleanup_account_data: F2,
    cleanup_pool_data: F3,
    cleanup_tracked_tokens: F4,
    cleanup_account_queue: F5
)
    -> Result<usize, String>
    where
        F1: FnOnce() -> Result<usize, String>,
        F2: FnOnce() -> Result<usize, String>,
        F3: FnOnce() -> Result<usize, String>,
        F4: FnOnce() -> Result<usize, String>,
        F5: FnOnce() -> Result<usize, String>
{
    get_pool_cleanup().cleanup_with_functions(
        price_cache,
        cleanup_price_cache,
        cleanup_account_data,
        cleanup_pool_data,
        cleanup_tracked_tokens,
        cleanup_account_queue
    ).await
}
