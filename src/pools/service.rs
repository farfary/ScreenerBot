/// Main pool service
/// Orchestrates pool discovery, calculation, caching, and token management

use crate::pools::{ PoolDiscovery, PoolCalculator };
use crate::pools::types::{ PriceResult, PoolStats };
use crate::pools::cache::PoolCache;
use crate::pools::tokens::{ PoolTokenManager, PoolToken };
use crate::pools::constants::TOKEN_REFRESH_INTERVAL_SECS;
use tokio::sync::{ OnceCell, RwLock };
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use crate::logger::{ log, LogTag };

/// Main pool service
pub struct PoolService {
    discovery: PoolDiscovery,
    calculator: PoolCalculator,
    cache: Arc<PoolCache>,
    token_manager: PoolTokenManager,
    tokens: Arc<RwLock<Vec<PoolToken>>>,
    token_task_running: Arc<RwLock<bool>>,
}

impl PoolService {
    pub fn new() -> Self {
        let cache = Arc::new(PoolCache::new());
        let discovery = PoolDiscovery::new(cache.clone());

        Self {
            discovery,
            calculator: PoolCalculator::new(),
            cache,
            token_manager: PoolTokenManager::new(),
            tokens: Arc::new(RwLock::new(Vec::new())),
            token_task_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the pool service (starts discovery and token loading tasks)
    pub async fn start(&self) {
        log(LogTag::Pool, "SERVICE_START", "🚀 Starting Pool Service");

        // Start token loading task first
        self.start_token_loading_task().await;

        // Start continuous discovery task
        self.discovery.start_discovery_task().await;

        log(LogTag::Pool, "SERVICE_READY", "✅ Pool Service ready");
    }

    /// Stop the pool service
    pub async fn stop(&self) {
        log(LogTag::Pool, "SERVICE_STOP", "🛑 Stopping Pool Service");

        // Stop token loading task
        self.stop_token_loading_task().await;

        // Stop discovery task
        self.discovery.stop_discovery_task().await;
    }

    /// Get price for a token
    pub async fn get_price(&self, token_address: &str) -> Option<PriceResult> {
        // 1. Check price cache first
        if let Some(cached_price) = self.cache.get_cached_price(token_address).await {
            log(
                LogTag::Pool,
                "PRICE_CACHE_HIT",
                &format!("💾 Cache hit for {}", &token_address[..8])
            );
            return Some(cached_price);
        }

        // 2. Check if we have pools cached
        if let Some(pools) = self.cache.get_cached_pools(token_address).await {
            if !pools.is_empty() {
                // 3. Calculate price from best pool
                let best_pool = pools
                    .into_iter()
                    .max_by(|a, b|
                        a.liquidity_usd
                            .partial_cmp(&b.liquidity_usd)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    );

                if let Some(pool) = best_pool {
                    match self.calculator.calculate_price(&pool, token_address).await {
                        Ok(Some(price_result)) => {
                            // 4. Cache the result
                            self.cache.cache_price(token_address, price_result.clone()).await;

                            // 5. Add to price history
                            self.cache.add_price_to_history(
                                token_address,
                                price_result.price_sol
                            ).await;

                            log(
                                LogTag::Pool,
                                "PRICE_CALCULATED",
                                &format!("💰 Calculated price for {}", &token_address[..8])
                            );
                            return Some(price_result);
                        }
                        Ok(None) => {
                            log(
                                LogTag::Pool,
                                "PRICE_CALC_NONE",
                                &format!("❌ No price calculated for {}", &token_address[..8])
                            );
                        }
                        Err(e) => {
                            log(
                                LogTag::Pool,
                                "PRICE_CALC_ERROR",
                                &format!(
                                    "❌ Price calculation error for {}: {}",
                                    &token_address[..8],
                                    e
                                )
                            );
                        }
                    }
                }
            }
        }

        // 5. If no pools cached, discovery task will handle it in background
        log(
            LogTag::Pool,
            "PRICE_NOT_AVAILABLE",
            &format!("❌ No price available for {} (discovery in progress)", &token_address[..8])
        );
        None
    }

    /// Get prices for multiple tokens
    pub async fn get_batch_prices(&self, tokens: &[String]) -> Vec<Option<PriceResult>> {
        let mut results = Vec::new();

        for token in tokens {
            results.push(self.get_price(token).await);
        }

        results
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> crate::pools::cache::CacheStats {
        self.cache.get_stats().await
    }

    /// Get pool statistics for dashboard
    pub async fn get_stats(&self) -> PoolStats {
        let cache_stats = self.cache.get_stats().await;

        // Calculate a simple hit rate based on ratio of cached prices to tokens
        let hit_rate = if cache_stats.tokens_count > 0 {
            (cache_stats.prices_count as f64) / (cache_stats.tokens_count as f64)
        } else {
            0.0
        };

        PoolStats::new(
            cache_stats.pools_count,
            cache_stats.tokens_count,
            cache_stats.in_progress_count,
            hit_rate.min(1.0) // Cap at 1.0
        )
    }

    /// Get available tokens (tokens with cached pools)
    pub async fn get_available_tokens(&self) -> Vec<String> {
        self.cache.get_tokens_with_pools().await
    }

    /// Get price history for a token
    pub async fn get_price_history(
        &self,
        token_address: &str
    ) -> Vec<(chrono::DateTime<chrono::Utc>, f64)> {
        self.cache.get_price_history(token_address).await
    }

    /// Get price history since a specific time
    pub async fn get_price_history_since(
        &self,
        token_address: &str,
        since: chrono::DateTime<chrono::Utc>
    ) -> Vec<(chrono::DateTime<chrono::Utc>, f64)> {
        self.cache.get_price_history_since(token_address, since).await
    }

    /// Force refresh pools for a token (bypass cache)
    pub async fn refresh_pools(
        &self,
        token_address: &str
    ) -> Result<Vec<crate::pools::types::PoolInfo>, String> {
        let pools = self.discovery.discover_pools(token_address).await?;
        self.cache.cache_pools(token_address, pools.clone()).await;
        Ok(pools)
    }

    /// Start periodic token loading task
    async fn start_token_loading_task(&self) {
        let mut is_running = self.token_task_running.write().await;
        if *is_running {
            log(LogTag::Pool, "TOKEN_TASK", "Token loading task already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        log(LogTag::Pool, "TOKEN_TASK_START", "🚀 Starting periodic token loading task");

        // Load tokens immediately
        self.load_tokens().await;

        // Clone necessary data for the background task
        let token_manager = self.token_manager.clone();
        let tokens = self.tokens.clone();
        let cache = self.cache.clone();
        let is_running = self.token_task_running.clone();

        tokio::spawn(async move {
            loop {
                // Wait for the refresh interval
                sleep(Duration::from_secs(TOKEN_REFRESH_INTERVAL_SECS)).await;

                // Check if we should stop
                {
                    let running = is_running.read().await;
                    if !*running {
                        log(LogTag::Pool, "TOKEN_TASK_STOP", "🛑 Token loading task stopped");
                        break;
                    }
                }

                // Load top tokens by liquidity
                match token_manager.get_top_liquidity_tokens().await {
                    Ok(new_tokens) => {
                        log(
                            LogTag::Pool,
                            "TOKENS_REFRESHED",
                            &format!("🔄 Refreshed {} top liquidity tokens", new_tokens.len())
                        );

                        // Update in-memory list
                        {
                            let mut tokens_guard = tokens.write().await;
                            *tokens_guard = new_tokens.clone();
                        }

                        // Update cache
                        cache.cache_tokens(new_tokens).await;
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "TOKENS_REFRESH_ERROR",
                            &format!("❌ Failed to refresh tokens: {}", e)
                        );
                    }
                }
            }
        });
    }

    /// Stop token loading task
    async fn stop_token_loading_task(&self) {
        let mut is_running = self.token_task_running.write().await;
        *is_running = false;
    }

    /// Load tokens immediately
    async fn load_tokens(&self) {
        match self.token_manager.get_top_liquidity_tokens().await {
            Ok(tokens) => {
                log(
                    LogTag::Pool,
                    "TOKENS_LOADED",
                    &format!("📊 Loaded {} top liquidity tokens", tokens.len())
                );

                // Update in-memory list
                {
                    let mut tokens_guard = self.tokens.write().await;
                    *tokens_guard = tokens.clone();
                }

                // Update cache
                self.cache.cache_tokens(tokens).await;
            }
            Err(e) => {
                log(LogTag::Pool, "TOKENS_LOAD_ERROR", &format!("❌ Failed to load tokens: {}", e));
            }
        }
    }

    /// Get current tokens in memory
    pub async fn get_tokens(&self) -> Vec<PoolToken> {
        let tokens_guard = self.tokens.read().await;
        tokens_guard.clone()
    }

    /// Get token mints for pool operations
    pub async fn get_token_mints(&self) -> Vec<String> {
        let tokens_guard = self.tokens.read().await;
        tokens_guard
            .iter()
            .map(|t| t.mint.clone())
            .collect()
    }
}

// Global singleton
static POOL_SERVICE: OnceCell<Arc<PoolService>> = OnceCell::const_new();

/// Initialize the global pool service
pub async fn init_pool_service() -> &'static Arc<PoolService> {
    POOL_SERVICE.get_or_init(|| async { Arc::new(PoolService::new()) }).await
}

/// Get the global pool service
pub async fn get_pool_service() -> &'static Arc<PoolService> {
    POOL_SERVICE.get().expect("Pool service not initialized")
}

/// Start the global pool service
pub async fn start_pool_service() {
    let service = init_pool_service().await;
    service.start().await;
}

/// Stop the global pool service
pub async fn stop_pool_service() {
    if let Some(service) = POOL_SERVICE.get() {
        service.stop().await;
    }
}
