/// Main pool service
/// Orchestrates pool discovery, calculation, caching, token management, analysis, and account fetching

use crate::pools::{ PoolDiscovery, PoolAnalyzer, PoolFetcher };
use crate::pools::calculator::{ PoolCalculatorTask, CalculatorStats };
use crate::pools::types::{ PriceResult, PoolStats };
use crate::pools::cache::PoolCache;
use crate::pools::tokens::{ PoolTokenManager, PoolToken };
use crate::pools::analyzer::{ TokenAvailability, AnalysisStats };
use crate::pools::fetcher::{ CachedAccountData, FetcherStats };
use crate::pools::constants::TOKEN_REFRESH_INTERVAL_SECS;
use tokio::sync::{ OnceCell, RwLock };
use tokio::time::{ sleep, Duration };
use std::sync::Arc;
use std::collections::HashMap;
use crate::logger::{ log, LogTag };

/// Shared account data for thread-safe access
/// This is the central store for all fetched account data
#[derive(Debug, Clone)]
pub struct SharedAccountData {
    /// Account address
    pub address: String,
    /// Raw account data
    pub data: Vec<u8>,
    /// Account lamports
    pub lamports: u64,
    /// Account owner program
    pub owner: String,
    /// When this data was fetched
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    /// Whether the account exists
    pub exists: bool,
}

impl SharedAccountData {
    /// Create from CachedAccountData
    pub fn from_cached(cached: CachedAccountData) -> Self {
        Self {
            address: cached.address,
            data: cached.data,
            lamports: cached.lamports,
            owner: cached.owner,
            fetched_at: cached.fetched_at,
            exists: cached.exists,
        }
    }

    /// Check if data is expired (10 minutes TTL)
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.fetched_at);
        age.num_seconds() > 600 // 10 minutes
    }
}

/// Pre-prepared pool data for decoders
/// Contains all necessary account data for pool decoding
#[derive(Debug, Clone)]
pub struct PreparedPoolData {
    /// Pool address
    pub pool_address: String,
    /// Pool program ID
    pub program_id: String,
    /// Pool account data
    pub pool_account_data: Vec<u8>,
    /// Reserve account data (vault accounts)
    pub reserve_accounts_data: HashMap<String, Vec<u8>>,
}

impl PreparedPoolData {
    pub fn new(pool_address: String, program_id: String, pool_account_data: Vec<u8>) -> Self {
        Self {
            pool_address,
            program_id,
            pool_account_data,
            reserve_accounts_data: HashMap::new(),
        }
    }

    /// Add reserve account data
    pub fn add_reserve_account(&mut self, address: String, data: Vec<u8>) {
        self.reserve_accounts_data.insert(address, data);
    }
}

/// Main pool service
pub struct PoolService {
    discovery: PoolDiscovery,
    calculator_task: PoolCalculatorTask,
    analyzer: PoolAnalyzer,
    fetcher: PoolFetcher,
    cache: Arc<PoolCache>,
    token_manager: PoolTokenManager,
    tokens: Arc<RwLock<Vec<PoolToken>>>,
    token_task_running: Arc<RwLock<bool>>,
    /// Shared account data store (thread-safe, read-write access)
    /// Only written by fetcher, read by calculator and others
    shared_accounts: Arc<RwLock<HashMap<String, SharedAccountData>>>,
    /// Calculable tokens store for calculator task
    calculable_tokens: Arc<RwLock<HashMap<String, TokenAvailability>>>,
}

impl PoolService {
    pub fn new() -> Self {
        let cache = Arc::new(PoolCache::new());
        let discovery = PoolDiscovery::new(cache.clone());
        let analyzer = PoolAnalyzer::new(cache.clone());
        let fetcher = PoolFetcher::new(cache.clone());
        let calculator_task = PoolCalculatorTask::new(cache.clone());

        Self {
            discovery,
            calculator_task,
            analyzer,
            fetcher,
            cache,
            token_manager: PoolTokenManager::new(),
            tokens: Arc::new(RwLock::new(Vec::new())),
            token_task_running: Arc::new(RwLock::new(false)),
            shared_accounts: Arc::new(RwLock::new(HashMap::new())),
            calculable_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start the pool service (starts discovery and token loading tasks)
    pub async fn start(&self) {
        log(LogTag::Pool, "SERVICE_START", "🚀 Starting Pool Service");

        // Start token loading task first
        self.start_token_loading_task().await;

        // Start continuous discovery task
        self.discovery.start_discovery_task().await;

        // Start calculator task
        self.calculator_task.start_task(
            self.shared_accounts.clone(),
            self.calculable_tokens.clone()
        ).await;

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

    /// Get price for a token
    pub async fn get_price(&self, token_address: &str) -> Option<PriceResult> {
        // Check price cache (calculator task automatically updates prices)
        if let Some(cached_price) = self.cache.get_cached_price(token_address).await {
            log(
                LogTag::Pool,
                "PRICE_CACHE_HIT",
                &format!("💾 Cache hit for {}", &token_address[..8])
            );
            return Some(cached_price);
        }

        // If no cached price, the calculator task will handle calculation when pool data is ready
        log(
            LogTag::Pool,
            "PRICE_NOT_AVAILABLE",
            &format!("❌ No price available for {} (calculation in progress)", &token_address[..8])
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
                self.cache.cache_tokens(tokens.clone()).await;

                // Trigger analysis after loading tokens
                if let Err(e) = self.analyzer.analyze_all_tokens(&tokens).await {
                    log(
                        LogTag::Pool,
                        "TOKENS_ANALYSIS_ERROR",
                        &format!("❌ Failed to analyze tokens after loading: {}", e)
                    );
                } else {
                    let stats = self.analyzer.get_analysis_stats().await;
                    log(
                        LogTag::Pool,
                        "TOKENS_ANALYSIS_SUCCESS",
                        &format!(
                            "✅ Analysis complete: {}/{} calculable, {} trading ready",
                            stats.calculable_tokens,
                            stats.total_tokens,
                            stats.trading_ready_tokens
                        )
                    );

                    // Trigger account fetching after successful analysis
                    if let Err(e) = self.internal_fetch_accounts().await {
                        log(
                            LogTag::Pool,
                            "TOKENS_FETCH_ERROR",
                            &format!("❌ Failed to fetch accounts after analysis: {}", e)
                        );
                    } else {
                        let fetcher_stats = self.get_fetcher_stats().await;
                        log(
                            LogTag::Pool,
                            "TOKENS_FETCH_SUCCESS",
                            &format!(
                                "✅ Account fetch complete: {} cached accounts",
                                fetcher_stats.existing_accounts
                            )
                        );
                    }
                }
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

    // =============================================================================
    // ANALYZER METHODS
    // =============================================================================

    /// Analyze all tokens for calculability and pool availability
    pub async fn analyze_all_tokens(&self) -> Result<(), String> {
        let tokens = self.get_tokens().await;
        self.analyzer.analyze_all_tokens(&tokens).await?;

        // Update calculable tokens store for calculator task
        self.update_calculable_tokens_store().await?;

        Ok(())
    }

    /// Analyze a specific token
    pub async fn analyze_token(&self, token_mint: &str) -> Result<(), String> {
        self.analyzer.re_analyze_token(token_mint).await?;

        // Update calculable tokens store for calculator task
        self.update_calculable_tokens_store().await?;

        Ok(())
    }

    /// Update calculable tokens store from analyzer results
    async fn update_calculable_tokens_store(&self) -> Result<(), String> {
        let calculable_token_mints = self.analyzer.get_calculable_tokens().await;
        let mut calculable_map = self.calculable_tokens.write().await;

        // Clear existing data
        calculable_map.clear();

        // Add all calculable tokens
        for token_mint in calculable_token_mints {
            if let Some(availability) = self.analyzer.get_token_availability(&token_mint).await {
                calculable_map.insert(token_mint, availability);
            }
        }

        log(
            LogTag::Pool,
            "CALCULABLE_TOKENS_UPDATED",
            &format!("📊 Updated calculable tokens store: {} tokens", calculable_map.len())
        );

        Ok(())
    }

    /// Get token availability information
    pub async fn get_token_availability(&self, token_mint: &str) -> Option<TokenAvailability> {
        self.analyzer.get_token_availability(token_mint).await
    }

    /// Get all calculable tokens
    pub async fn get_calculable_tokens(&self) -> Vec<String> {
        self.analyzer.get_calculable_tokens().await
    }

    /// Get tokens ready for trading
    pub async fn get_trading_ready_tokens(&self) -> Vec<String> {
        self.analyzer.get_trading_ready_tokens().await
    }

    /// Get required account addresses for RPC fetching
    pub async fn get_required_accounts(&self) -> Vec<String> {
        self.analyzer.get_required_accounts().await
    }

    /// Get analysis statistics
    pub async fn get_analysis_stats(&self) -> AnalysisStats {
        self.analyzer.get_analysis_stats().await
    }

    /// Trigger analysis after tokens or pools are updated
    pub async fn trigger_analysis(&self) -> Result<(), String> {
        log(LogTag::Pool, "TRIGGER_ANALYSIS", "🔬 Triggering token analysis");

        let tokens = self.get_tokens().await;
        if tokens.is_empty() {
            return Err("No tokens available for analysis".to_string());
        }

        self.analyzer.analyze_all_tokens(&tokens).await?;

        // Update calculable tokens store for calculator task
        self.update_calculable_tokens_store().await?;

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

    /// Fetch all required account data using the fetcher (internal task only)
    /// This method should only be called internally as a background task
    async fn internal_fetch_accounts(&self) -> Result<(), String> {
        // Get required accounts from analyzer
        let required_accounts = self.analyzer.get_required_accounts().await;

        if required_accounts.is_empty() {
            log(LogTag::Pool, "SERVICE_FETCH_SKIP", "No required accounts to fetch");
            return Ok(());
        }

        log(
            LogTag::Pool,
            "SERVICE_FETCH_START",
            &format!("📦 Fetching {} required accounts", required_accounts.len())
        );

        // Use fetcher to fetch all required accounts
        self.fetcher.fetch_all_required_accounts(&required_accounts).await?;

        // Update shared account data from fetcher cache
        self.update_shared_accounts_from_fetcher().await?;

        log(LogTag::Pool, "SERVICE_FETCH_COMPLETE", "✅ Account fetching completed");

        Ok(())
    }

    /// Update shared accounts from fetcher cache (private method)
    async fn update_shared_accounts_from_fetcher(&self) -> Result<(), String> {
        let fetcher_accounts = self.fetcher.get_all_cached_accounts().await;
        let mut shared = self.shared_accounts.write().await;

        let mut updated_count = 0;
        for (address, cached_data) in fetcher_accounts {
            if !cached_data.is_expired() {
                shared.insert(address, SharedAccountData::from_cached(cached_data));
                updated_count += 1;
            }
        }

        log(
            LogTag::Pool,
            "SERVICE_SHARED_UPDATE",
            &format!("📋 Updated {} shared account entries", updated_count)
        );

        Ok(())
    }

    /// Get account data from shared store (thread-safe read access)
    pub async fn get_shared_account_data(&self, address: &str) -> Option<SharedAccountData> {
        let shared = self.shared_accounts.read().await;
        shared
            .get(address)
            .filter(|data| !data.is_expired())
            .cloned()
    }

    /// Get multiple account data from shared store
    pub async fn get_multiple_shared_account_data(
        &self,
        addresses: &[String]
    ) -> HashMap<String, SharedAccountData> {
        let shared = self.shared_accounts.read().await;
        let mut result = HashMap::new();

        for address in addresses {
            if let Some(data) = shared.get(address) {
                if !data.is_expired() {
                    result.insert(address.clone(), data.clone());
                }
            }
        }

        result
    }

    /// Prepare pool data for decoder (used by calculator)
    pub async fn prepare_pool_data(
        &self,
        pool_address: &str,
        program_id: &str,
        reserve_addresses: &[String]
    ) -> Result<PreparedPoolData, String> {
        // Get pool account data
        let pool_data = self
            .get_shared_account_data(pool_address).await
            .ok_or_else(|| format!("Pool account data not found for {}", pool_address))?;

        if !pool_data.exists {
            return Err(format!("Pool account {} does not exist", pool_address));
        }

        // Get reserve account data
        let reserve_data = self.get_multiple_shared_account_data(reserve_addresses).await;

        // Create prepared pool data
        let mut prepared = PreparedPoolData::new(
            pool_address.to_string(),
            program_id.to_string(),
            pool_data.data
        );

        // Add reserve account data
        for address in reserve_addresses {
            if let Some(reserve_account) = reserve_data.get(address) {
                if reserve_account.exists {
                    prepared.add_reserve_account(address.clone(), reserve_account.data.clone());
                }
            }
        }

        Ok(prepared)
    }

    /// Test decoding a specific pool using the fetcher-based decoder
    pub async fn test_decode_pool(
        &self,
        pool_address: &str,
        program_id: &str
    ) -> Result<(), String> {
        use crate::pools::decoders::{ DecoderFactory };

        log(
            LogTag::Pool,
            "SERVICE_DECODE_TEST",
            &format!(
                "🧪 Testing decode for pool {} with program {}",
                &pool_address[..8],
                &program_id[..8]
            )
        );

        // Create decoder factory
        let factory = DecoderFactory::new();

        // Get appropriate decoder
        let decoder = factory
            .get_decoder(program_id)
            .ok_or_else(|| format!("No decoder found for program ID: {}", program_id))?;

        // Prepare data for decoding (assumes vault addresses are already extracted and cached)
        let prepared_data = self.prepare_pool_data(pool_address, program_id, &[]).await?;

        // Test decoding
        match decoder.decode_pool_data(&prepared_data) {
            Ok(decoded_result) => {
                log(
                    LogTag::Pool,
                    "SERVICE_DECODE_SUCCESS",
                    &format!(
                        "✅ Successfully decoded pool: {} -> {} reserves: {}/{}",
                        &decoded_result.pool_address[..8],
                        decoded_result.pool_type,
                        decoded_result.token_a_reserve,
                        decoded_result.token_b_reserve
                    )
                );
                Ok(())
            }
            Err(e) => {
                log(
                    LogTag::Pool,
                    "SERVICE_DECODE_FAILED",
                    &format!("❌ Failed to decode pool {}: {}", &pool_address[..8], e)
                );
                Err(e)
            }
        }
    }

    /// Fetch account data for specific addresses
    pub async fn fetch_accounts(&self, addresses: &[String]) -> Result<(), String> {
        self.fetcher.fetch_all_required_accounts(addresses).await
    }

    /// Get cached account data for an address
    pub async fn get_cached_account_data(&self, address: &str) -> Option<CachedAccountData> {
        self.fetcher.get_cached_account_data(address).await
    }

    /// Get all cached account data
    pub async fn get_all_cached_accounts(
        &self
    ) -> std::collections::HashMap<String, CachedAccountData> {
        self.fetcher.get_all_cached_accounts().await
    }

    /// Clean expired account cache entries
    pub async fn clean_expired_account_cache(&self) -> usize {
        self.fetcher.clean_expired_cache().await
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
        log(LogTag::Pool, "PIPELINE_START", "🚀 Starting full pipeline: analysis + fetching");

        // Step 1: Analyze all tokens
        self.trigger_analysis().await?;

        // Step 2: Fetch all required accounts (internal method)
        self.internal_fetch_accounts().await?;

        // Step 3: Log final statistics
        let analysis_stats = self.get_analysis_stats().await;
        let fetcher_stats = self.get_fetcher_stats().await;

        log(
            LogTag::Pool,
            "PIPELINE_COMPLETE",
            &format!(
                "✅ Pipeline complete: {}/{} tokens calculable, {} accounts cached",
                analysis_stats.calculable_tokens,
                analysis_stats.total_tokens,
                fetcher_stats.existing_accounts
            )
        );

        Ok(())
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
