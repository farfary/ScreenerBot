/// Main pool service - centralized cache-based architecture
/// Uses PoolCache as single source of truth for all data storage

use crate::pools::{ PoolDiscovery, PoolAnalyzer, PoolFetcher };
use crate::pools::calculator::{ PoolCalculatorTask, CalculatorStats };
use crate::pools::types::{ PriceResult, PoolStats };
use crate::pools::cache::{ PoolCache, CacheStats };
use crate::pools::tokens::{ PoolTokenManager, PoolToken };
use crate::pools::analyzer::{ TokenAvailability, AnalysisStats };
use crate::pools::fetcher::FetcherStats;
use crate::pools::constants::TOKEN_REFRESH_INTERVAL_SECS;
use tokio::sync::{ OnceCell, RwLock };
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use crate::logger::{ log, LogTag };

/// Main pool service
pub struct PoolService {
    /// Central cache for all data storage
    cache: Arc<PoolCache>,

    /// Pool discovery service
    discovery: PoolDiscovery,

    /// Pool calculator background task
    calculator_task: PoolCalculatorTask,

    /// Pool analyzer service
    analyzer: PoolAnalyzer,

    /// Pool account data fetcher
    fetcher: PoolFetcher,

    /// Token manager for external data
    token_manager: PoolTokenManager,

    /// Token loading task status
    token_task_running: Arc<RwLock<bool>>,
}

impl PoolService {
    pub fn new() -> Self {
        let cache = Arc::new(PoolCache::new());
        let discovery = PoolDiscovery::new(cache.clone());
        let analyzer = PoolAnalyzer::new(cache.clone());
        let fetcher = PoolFetcher::new(cache.clone());
        let calculator_task = PoolCalculatorTask::new(cache.clone());

        Self {
            cache,
            discovery,
            calculator_task,
            analyzer,
            fetcher,
            token_manager: PoolTokenManager::new(),
            token_task_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the pool service (starts all background tasks)
    pub async fn start(&self) {
        log(LogTag::Pool, "SERVICE_START", "🚀 Starting Pool Service");

        // Start token loading task first
        self.start_token_loading_task().await;

        // Start discovery task
        self.discovery.start_discovery_task().await;

        // Start calculator task
        self.calculator_task.start_task(self.cache.clone()).await;

        log(LogTag::Pool, "SERVICE_READY", "✅ Pool Service ready");
    }

    /// Stop the pool service
    pub async fn stop(&self) {
        log(LogTag::Pool, "SERVICE_STOP", "🛑 Stopping Pool Service");

        // Stop token loading task
        self.stop_token_loading_task().await;

        // Stop discovery task
        self.discovery.stop_discovery_task().await;

        // Stop calculator task
        self.calculator_task.stop_task().await;
    }

    /// Get price for a token (from cache only)
    pub async fn get_price(&self, token_address: &str) -> Option<PriceResult> {
        self.cache.get_price(token_address).await
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
    pub async fn get_cache_stats(&self) -> CacheStats {
        self.cache.get_stats().await
    }

    /// Get pool statistics for dashboard
    pub async fn get_stats(&self) -> PoolStats {
        let cache_stats = self.cache.get_stats().await;

        // Calculate hit rate based on ratio of cached prices to tokens
        let hit_rate = if cache_stats.tokens_count > 0 {
            (cache_stats.prices_count as f64) / (cache_stats.tokens_count as f64)
        } else {
            0.0
        };

        PoolStats::new(
            cache_stats.tokens_count,
            cache_stats.pools_count,
            cache_stats.prices_count,
            hit_rate,
            cache_stats.updated_at
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
        self.cache.store_pools(token_address, pools.clone()).await;
        Ok(pools)
    }

    /// Get current tokens in memory
    pub async fn get_tokens(&self) -> Vec<PoolToken> {
        self.cache.get_tokens().await
    }

    /// Get token mints for pool operations
    pub async fn get_token_mints(&self) -> Vec<String> {
        let tokens = self.get_tokens().await;
        tokens
            .into_iter()
            .map(|t| t.mint)
            .collect()
    }

    // =============================================================================
    // TOKEN MANAGEMENT
    // =============================================================================

    /// Start periodic token loading task
    async fn start_token_loading_task(&self) {
        let mut is_running = self.token_task_running.write().await;
        if *is_running {
            log(LogTag::Pool, "TOKEN_TASK_RUNNING", "Token loading task already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        log(LogTag::Pool, "TOKEN_TASK_START", "🚀 Starting periodic token loading task");

        // Load tokens immediately
        self.load_tokens().await;

        // Clone necessary data for the background task
        let token_manager = self.token_manager.clone();
        let cache = self.cache.clone();
        let is_running = self.token_task_running.clone();

        tokio::spawn(async move {
            while *is_running.read().await {
                sleep(Duration::from_secs(TOKEN_REFRESH_INTERVAL_SECS)).await;

                if *is_running.read().await {
                    log(LogTag::Pool, "TOKEN_TASK_REFRESH", "🔄 Refreshing tokens");

                    match token_manager.get_tokens().await {
                        Ok(tokens) => {
                            log(
                                LogTag::Pool,
                                "TOKEN_TASK_LOADED",
                                &format!("📦 Loaded {} tokens", tokens.len())
                            );
                            cache.store_tokens(tokens).await;
                        }
                        Err(e) => {
                            log(
                                LogTag::Pool,
                                "TOKEN_TASK_ERROR",
                                &format!("❌ Token loading error: {}", e)
                            );
                        }
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
        log(LogTag::Pool, "LOAD_TOKENS", "🔄 Loading tokens");

        match self.token_manager.get_tokens().await {
            Ok(tokens) => {
                log(LogTag::Pool, "TOKENS_LOADED", &format!("📦 Loaded {} tokens", tokens.len()));
                self.cache.store_tokens(tokens).await;
            }
            Err(e) => {
                log(LogTag::Pool, "TOKENS_ERROR", &format!("❌ Token loading error: {}", e));
            }
        }
    }

    // =============================================================================
    // ANALYZER METHODS
    // =============================================================================

    /// Analyze all tokens for calculability and pool availability
    pub async fn analyze_all_tokens(&self) -> Result<(), String> {
        let tokens = self.get_tokens().await;
        self.analyzer.analyze_all_tokens(&tokens).await
    }

    /// Analyze a specific token
    pub async fn analyze_token(&self, token_mint: &str) -> Result<(), String> {
        self.analyzer.re_analyze_token(token_mint).await
    }

    /// Get token availability information
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        self.cache.get_token_availability(token_mint).await
    }

    /// Get all calculable tokens
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        self.cache.get_calculable_tokens().await
    }

    /// Get tokens ready for trading
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        self.cache.get_trading_ready_tokens().await
    }

    /// Get required account addresses for RPC fetching
    pub async fn get_required_accounts(&self) -> Vec<String> {
        self.cache.get_required_accounts().await
    }

    /// Get analysis statistics
    pub async fn get_analysis_stats(&self) -> AnalysisStats {
        self.cache.get_analysis_stats().await
    }

    /// Trigger analysis after tokens or pools are updated
    pub async fn trigger_analysis(&self) -> Result<(), String> {
        log(LogTag::Pool, "TRIGGER_ANALYSIS", "🔬 Triggering token analysis");

        let tokens = self.get_tokens().await;
        if tokens.is_empty() {
            return Err("No tokens available for analysis".to_string());
        }

        self.analyzer.analyze_all_tokens(&tokens).await?;

        let stats = self.get_analysis_stats().await;
        log(
            LogTag::Pool,
            "ANALYSIS_STATS",
            &format!(
                "📊 Analysis complete: {}/{} calculable, {} trading ready, {} accounts needed",
                stats.calculable_tokens,
                stats.total_tokens,
                stats.trading_ready_tokens,
                stats.required_accounts
            )
        );

        Ok(())
    }

    // =============================================================================
    // CALCULATOR METHODS
    // =============================================================================

    /// Get calculator statistics
    pub async fn get_calculator_stats(&self) -> CalculatorStats {
        self.calculator_task.get_calculator_stats().await
    }

    /// Check if calculator task is running
    pub async fn is_calculator_running(&self) -> bool {
        self.calculator_task.is_running().await
    }

    // =============================================================================
    // FETCHER METHODS
    // =============================================================================

    /// Fetch account data for specific addresses
    pub async fn fetch_accounts(&self, addresses: &[String]) -> Result<(), String> {
        self.fetcher.fetch_all_required_accounts(addresses).await
    }

    /// Get fetcher statistics
    pub async fn get_fetcher_stats(&self) -> FetcherStats {
        self.fetcher.get_fetcher_stats().await
    }

    /// Force refresh specific account addresses (bypass cache)
    pub async fn force_refresh_accounts(&self, addresses: &[String]) -> Result<(), String> {
        self.fetcher.force_refresh_addresses(addresses).await
    }

    /// Check if fetcher is currently running
    pub async fn is_fetching_accounts(&self) -> bool {
        self.fetcher.is_fetching().await
    }

    /// Trigger full analysis and account fetching pipeline
    pub async fn trigger_full_pipeline(&self) -> Result<(), String> {
        // 1. Analyze tokens first
        self.trigger_analysis().await?;

        // 2. Get required accounts from analysis
        let required_accounts = self.get_required_accounts().await;

        // 3. Fetch required accounts
        if !required_accounts.is_empty() {
            self.fetch_accounts(&required_accounts).await?;
        }

        log(LogTag::Pool, "PIPELINE_COMPLETE", "✅ Full pipeline completed");
        Ok(())
    }

    /// Clean expired cache entries
    pub async fn clean_expired_cache(&self) -> usize {
        self.cache.clean_expired_accounts().await
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
        log(LogTag::Pool, "GLOBAL_SERVICE_STOP", "🛑 Global pool service stopped");
    }
}
