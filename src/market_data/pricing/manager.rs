use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use reqwest::Client;

use crate::database::Database;
use crate::logger::Logger;
use crate::market_data::models::{ PriceSource as ModelsPriceSource, * };
use crate::market_data::sources::*;
use crate::market_data::pricing::*;
use crate::market_data::PoolDecoderManager;

/// Main market data manager that coordinates all pricing operations
pub struct MarketDataManager {
    gecko_client: GeckoTerminalClient,
    pool_decoder: PoolDecoderManager,
    price_calculator: PoolPriceCalculator,
    cache: Arc<RwLock<PriceCache>>,
    database: Arc<Database>,
    logger: Arc<Logger>,
    config: PricingConfig,
    priority_tokens: Arc<RwLock<Vec<String>>>,
    tiered_manager: Option<TieredPricingManager>,
    dynamic_manager: Option<Arc<DynamicPricingManager>>,
    metrics: Arc<RwLock<PricingMetrics>>,
}

impl MarketDataManager {
    /// Create a new pricing manager with basic configuration
    pub fn new(database: Arc<Database>, logger: Arc<Logger>, config: PricingConfig) -> Self {
        let client = Client::new();

        Self {
            gecko_client: GeckoTerminalClient::new(client.clone()),
            pool_decoder: PoolDecoderManager::new(),
            price_calculator: PoolPriceCalculator::new(),
            cache: Arc::new(
                RwLock::new(PriceCache::with_config(config.cache_ttl(), config.max_cache_size))
            ),
            database,
            logger,
            config,
            priority_tokens: Arc::new(RwLock::new(Vec::new())),
            tiered_manager: None,
            dynamic_manager: None,
            metrics: Arc::new(RwLock::new(PricingMetrics::default())),
        }
    }

    /// Create a new MarketDataManager with dynamic pricing enabled
    pub fn with_dynamic_pricing(
        database: Arc<Database>,
        logger: Arc<Logger>,
        config: PricingConfig,
        dynamic_config: crate::config::DynamicPricingConfig
    ) -> Self {
        let client = Client::new();
        let gecko_client = Arc::new(GeckoTerminalClient::new(client.clone()));

        let dynamic_manager = if dynamic_config.enabled {
            Some(
                Arc::new(
                    DynamicPricingManager::new(
                        dynamic_config,
                        gecko_client.clone(),
                        database.clone(),
                        logger.clone()
                    )
                )
            )
        } else {
            None
        };

        Self {
            gecko_client: (*gecko_client).clone(),
            pool_decoder: PoolDecoderManager::new(),
            price_calculator: PoolPriceCalculator::new(),
            cache: Arc::new(
                RwLock::new(PriceCache::with_config(config.cache_ttl(), config.max_cache_size))
            ),
            database,
            logger,
            config,
            priority_tokens: Arc::new(RwLock::new(Vec::new())),
            tiered_manager: None,
            dynamic_manager,
            metrics: Arc::new(RwLock::new(PricingMetrics::default())),
        }
    }

    /// Start the pricing manager
    pub async fn start(&self) {
        Logger::info("🚀 Starting pricing manager...");

        // Start dynamic pricing manager if enabled
        if let Some(dynamic_manager) = &self.dynamic_manager {
            Logger::info("🎯 Dynamic pricing is enabled - starting advanced pricing system");
            if let Err(e) = dynamic_manager.start().await {
                Logger::error(&format!("❌ Failed to start dynamic pricing manager: {}", e));
                Logger::info("⚠️  Falling back to traditional pricing system...");
            } else {
                Logger::success("✅ Dynamic pricing manager started successfully");
                Logger::info("🔥 Advanced pricing system is now active with liquidity-based intervals");
                return; // Use dynamic pricing instead of traditional pricing
            }
        } else {
            Logger::info("📊 Dynamic pricing is disabled - using traditional pricing system");
        }

        // Fallback to traditional pricing if dynamic pricing is not available
        self.start_traditional_pricing().await;
    }

    /// Start traditional pricing updates
    async fn start_traditional_pricing(&self) {
        let gecko_client = self.gecko_client.clone();
        let cache = self.cache.clone();
        let database = self.database.clone();
        let logger = self.logger.clone();
        let config = self.config.clone();
        let priority_tokens = self.priority_tokens.clone();
        let metrics = self.metrics.clone();

        Logger::info(
            &format!(
                "🔄 Starting traditional pricing updates - Interval: {}s, Top tokens: {}",
                config.update_interval_secs,
                config.top_tokens_count
            )
        );

        // Start background price update task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.update_interval());

            loop {
                interval.tick().await;

                Logger::debug("🔍 Starting price update cycle...");

                if
                    let Err(e) = Self::update_token_prices(
                        &gecko_client,
                        &cache,
                        &database,
                        &logger,
                        &config,
                        &priority_tokens,
                        &metrics
                    ).await
                {
                    Logger::error(&format!("❌ Failed to update prices: {}", e));
                } else {
                    Logger::debug("✅ Price update cycle completed successfully");
                }
            }
        });

        Logger::info("✅ Traditional pricing started successfully");
    }

    /// Update token prices from various sources
    async fn update_token_prices(
        gecko_client: &GeckoTerminalClient,
        cache: &Arc<RwLock<PriceCache>>,
        database: &Arc<Database>,
        logger: &Arc<Logger>,
        config: &PricingConfig,
        priority_tokens: &Arc<RwLock<Vec<String>>>,
        metrics: &Arc<RwLock<PricingMetrics>>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get tokens to update
        let mut tokens_to_update = Vec::new();

        // Add priority tokens
        {
            let priority = priority_tokens.read().await;
            if !priority.is_empty() {
                Logger::debug(&format!("📌 Adding {} priority tokens to update queue", priority.len()));
                tokens_to_update.extend(priority.clone());
            }
        }

        // Add top tokens from database
        match database.get_top_tokens_by_liquidity(config.top_tokens_count).await {
            Ok(top_tokens) => {
                Logger::debug(&format!("📊 Adding {} top tokens by liquidity to update queue", top_tokens.len()));
                tokens_to_update.extend(top_tokens);
            }
            Err(e) => {
                Logger::error(&format!("❌ Failed to get top tokens: {}", e));
            }
        }

        // Remove duplicates
        tokens_to_update.sort();
        tokens_to_update.dedup();

        if tokens_to_update.is_empty() {
            Logger::warn("⚠️  No tokens to update");
            return Ok(());
        }

        Logger::info(&format!("🔄 Updating prices for {} tokens", tokens_to_update.len()));

        // Update in batches to avoid rate limiting
        const BATCH_SIZE: usize = 30;
        let total_batches = (tokens_to_update.len() + BATCH_SIZE - 1) / BATCH_SIZE;
        
        for (batch_idx, batch) in tokens_to_update.chunks(BATCH_SIZE).enumerate() {
            Logger::debug(&format!("📦 Processing batch {}/{} ({} tokens)", batch_idx + 1, total_batches, batch.len()));
            
            if
                let Err(e) = Self::update_token_batch(
                    gecko_client,
                    cache,
                    database,
                    logger,
                    batch,
                    metrics
                ).await
            {
                Logger::error(&format!("❌ Failed to update batch {}: {}", batch_idx + 1, e));
            } else {
                Logger::debug(&format!("✅ Batch {} completed successfully", batch_idx + 1));
            }

            // Rate limiting pause
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Update metrics
        {
            let mut metrics_lock = metrics.write().await;
            metrics_lock.total_tokens_tracked = tokens_to_update.len();
            metrics_lock.last_update = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }

        Logger::info(&format!("✅ Completed price updates for {} tokens", tokens_to_update.len()));

        Ok(())
    }

    /// Update a batch of tokens
    async fn update_token_batch(
        gecko_client: &GeckoTerminalClient,
        cache: &Arc<RwLock<PriceCache>>,
        database: &Arc<Database>,
        _logger: &Arc<Logger>,
        token_addresses: &[String],
        metrics: &Arc<RwLock<PricingMetrics>>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Fetch token info from GeckoTerminal API
        let token_infos = gecko_client.get_multiple_tokens(token_addresses).await?;

        // Update metrics
        {
            let mut metrics_lock = metrics.write().await;
            metrics_lock.gecko_requests += 1;
        }

        for token_info in token_infos {
            // Get previous price for comparison
            let previous_price = {
                let cache_lock = cache.read().await;
                cache_lock.get_token_price(&token_info.address).await
            };

            // Update cache
            {
                let mut cache_lock = cache.write().await;
                cache_lock.update_token_info(token_info.clone()).await;
            }

            // Update database
            database.update_token_info(&token_info).await?;

            // Log price changes with detailed information
            if let Some(current_price) = &token_info.price {
                let total_liquidity = token_info.pools
                    .iter()
                    .map(|p| p.liquidity_usd)
                    .sum::<f64>();

                let total_volume = token_info.pools
                    .iter()
                    .map(|p| p.volume_24h)
                    .sum::<f64>();

                if let Some(prev_price) = previous_price {
                    let price_change = current_price.price_usd - prev_price.price_usd;
                    let price_change_percent = if prev_price.price_usd > 0.0 {
                        (price_change / prev_price.price_usd) * 100.0
                    } else {
                        0.0
                    };

                    // Only log if there's a significant price change (>0.1%)
                    if price_change_percent.abs() > 0.1 {
                        let change_symbol = if price_change > 0.0 { "📈" } else { "📉" };
                        let change_color = if price_change > 0.0 { "🟢" } else { "🔴" };
                        
                        Logger::info(
                            &format!(
                                "{} {} PRICE CHANGE: {} | ${:.8} → ${:.8} ({:+.2}%) | Liquidity: ${:.0} | Volume: ${:.0}",
                                change_symbol,
                                change_color,
                                token_info.symbol,
                                prev_price.price_usd,
                                current_price.price_usd,
                                price_change_percent,
                                total_liquidity,
                                total_volume
                            )
                        );
                    } else {
                        Logger::debug(
                            &format!(
                                "💰 Updated pricing for {} - ${:.8} | Liquidity: ${:.0} | Volume: ${:.0}",
                                token_info.symbol,
                                current_price.price_usd,
                                total_liquidity,
                                total_volume
                            )
                        );
                    }
                } else {
                    // First time seeing this token
                    Logger::info(
                        &format!(
                            "🆕 NEW TOKEN TRACKED: {} | ${:.8} | Liquidity: ${:.0} | Volume: ${:.0}",
                            token_info.symbol,
                            current_price.price_usd,
                            total_liquidity,
                            total_volume
                        )
                    );
                }
            } else {
                Logger::warn(
                    &format!(
                        "⚠️  No price data available for token: {} ({})",
                        token_info.symbol,
                        token_info.address
                    )
                );
            }
        }

        Ok(())
    }

    /// Get token price from cache or fetch from source
    pub async fn get_token_price(&self, token_address: &str) -> Option<TokenPrice> {
        // Use tiered pricing if available
        if let Some(ref tiered_manager) = self.tiered_manager {
            if let Some(token_price) = tiered_manager.get_token_price(token_address).await {
                return Some(token_price);
            }
        }

        // Check cache first
        {
            let cache_lock = self.cache.read().await;
            if let Some(mut price) = cache_lock.get_token_price(token_address).await {
                price.is_cached = true;

                // Update metrics
                {
                    let mut metrics_lock = self.metrics.write().await;
                    metrics_lock.cache_hits += 1;
                }

                return Some(price);
            }
        }

        // Update cache miss metric
        {
            let mut metrics_lock = self.metrics.write().await;
            metrics_lock.cache_misses += 1;
        }

        // If not in cache, try to fetch from API
        match self.gecko_client.get_token_info(token_address).await {
            Ok(token_info) => {
                if let Some(price) = token_info.price.clone() {
                    // Update cache
                    {
                        let mut cache_lock = self.cache.write().await;
                        cache_lock.update_token_info(token_info).await;
                    }

                    // Update metrics
                    {
                        let mut metrics_lock = self.metrics.write().await;
                        metrics_lock.gecko_requests += 1;
                    }

                    let mut result = price;
                    result.is_cached = false;
                    Some(result)
                } else {
                    None
                }
            }
            Err(e) => {
                Logger::error(&format!("Failed to fetch price for {}: {}", token_address, e));
                None
            }
        }
    }

    /// Get token information from cache or fetch from source
    pub async fn get_token_info(&self, token_address: &str) -> Option<TokenInfo> {
        // Check cache first
        {
            let cache_lock = self.cache.read().await;
            if let Some(info) = cache_lock.get_token_info(token_address).await {
                return Some(info);
            }
        }

        // If not in cache, fetch from API
        match self.gecko_client.get_token_info(token_address).await {
            Ok(token_info) => {
                // Update cache
                {
                    let mut cache_lock = self.cache.write().await;
                    cache_lock.update_token_info(token_info.clone()).await;
                }

                Some(token_info)
            }
            Err(e) => {
                Logger::error(&format!("Failed to fetch token info for {}: {}", token_address, e));
                None
            }
        }
    }

    /// Calculate price from pool data
    pub async fn calculate_price_from_pools(&self, token_address: &str) -> Option<TokenPrice> {
        let token_info = self.get_token_info(token_address).await?;

        if token_info.pools.is_empty() {
            return None;
        }

        match self.price_calculator.calculate_from_pools(&token_info.pools).await {
            Ok(price_usd) => {
                // Update metrics
                {
                    let mut metrics_lock = self.metrics.write().await;
                    metrics_lock.pool_calculations += 1;
                }

                Some(TokenPrice {
                    address: token_address.to_string(),
                    price_usd,
                    price_sol: None,
                    market_cap: None,
                    volume_24h: token_info.pools
                        .iter()
                        .map(|p| p.volume_24h)
                        .sum(),
                    liquidity_usd: token_info.pools
                        .iter()
                        .map(|p| p.liquidity_usd)
                        .sum(),
                    timestamp: std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    source: ModelsPriceSource::PoolCalculation,
                    is_cached: false,
                })
            }
            Err(e) => {
                Logger::error(&format!("Failed to calculate price from pools: {}", e));
                None
            }
        }
    }

    /// Add a token to priority updates
    pub async fn add_priority_token(&self, token_address: String) {
        self.priority_tokens.write().await.push(token_address);
        Logger::info("Added priority token for frequent updates");
    }

    /// Remove a token from priority updates
    pub async fn remove_priority_token(&self, token_address: &str) {
        let mut priority_tokens = self.priority_tokens.write().await;
        priority_tokens.retain(|addr| addr != token_address);
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> crate::market_data::pricing::CacheStats {
        self.cache.read().await.get_cache_stats().await
    }

    /// Get pricing metrics
    pub async fn get_metrics(&self) -> PricingMetrics {
        self.metrics.read().await.clone()
    }

    /// Calculate portfolio value
    pub async fn get_portfolio_value(&self, positions: &[(String, f64)]) -> f64 {
        let mut total_value = 0.0;

        for (token_address, amount) in positions {
            if let Some(price) = self.get_token_price(token_address).await {
                total_value += amount * price.price_usd;
            }
        }

        total_value
    }

    /// Enable tiered pricing
    pub async fn enable_tiered_pricing(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tiered_manager = TieredPricingManager::new(Arc::clone(&self.database));
        tiered_manager.start_tiered_updates().await;
        self.tiered_manager = Some(tiered_manager);
        Logger::info("Tiered pricing system enabled");
        Ok(())
    }

    /// Dynamic pricing management methods
    pub async fn add_token_to_dynamic_pricing(
        &self,
        token_address: String,
        initial_liquidity: f64
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dynamic_manager) = &self.dynamic_manager {
            dynamic_manager.add_token(token_address, initial_liquidity).await?;
        }
        Ok(())
    }

    pub async fn remove_token_from_dynamic_pricing(
        &self,
        token_address: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dynamic_manager) = &self.dynamic_manager {
            dynamic_manager.remove_token(token_address).await?;
        }
        Ok(())
    }

    pub async fn get_dynamic_pricing_stats(
        &self
    ) -> Option<crate::market_data::pricing::DynamicPricingStats> {
        if let Some(dynamic_manager) = &self.dynamic_manager {
            Some(dynamic_manager.get_stats().await)
        } else {
            None
        }
    }

    pub fn is_dynamic_pricing_enabled(&self) -> bool {
        self.dynamic_manager.is_some()
    }
}

/// Pricing tiers for different update frequencies
#[derive(Debug, Clone, PartialEq)]
pub enum PricingTier {
    Critical, // Open positions - 5 seconds
    High, // High volume tokens - 30 seconds
    Medium, // Medium volume tokens - 1-2 minutes
    Low, // Low volume - 3+ minutes
}

/// Token priority information for tiered pricing
#[derive(Debug, Clone)]
pub struct TokenPriority {
    pub address: String,
    pub tier: PricingTier,
    pub volume_24h: f64,
    pub last_updated: u64,
    pub is_open_position: bool,
    pub update_interval: Duration,
}

/// Tiered pricing manager for different update frequencies
pub struct TieredPricingManager {
    gecko_client: GeckoTerminalClient,
    pool_decoder: PoolDecoderManager,
    price_calculator: PoolPriceCalculator,
    cache: Arc<RwLock<PriceCache>>,
    database: Arc<Database>,

    // Tiered update system
    critical_tokens: Arc<RwLock<Vec<TokenPriority>>>,
    high_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>,
    medium_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>,
    low_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>,

    // Update intervals
    critical_interval: Duration,
    high_interval: Duration,
    medium_interval: Duration,
    low_interval: Duration,

    // Rate limiting
    max_requests_per_minute: u32,
    current_requests: Arc<RwLock<u32>>,
    last_reset: Arc<RwLock<u64>>,
}

impl TieredPricingManager {
    pub fn new(database: Arc<Database>) -> Self {
        let client = Client::new();

        Self {
            gecko_client: GeckoTerminalClient::new(client.clone()),
            pool_decoder: PoolDecoderManager::new(),
            price_calculator: PoolPriceCalculator::new(),
            cache: Arc::new(RwLock::new(PriceCache::new())),
            database,

            critical_tokens: Arc::new(RwLock::new(Vec::new())),
            high_priority_tokens: Arc::new(RwLock::new(Vec::new())),
            medium_priority_tokens: Arc::new(RwLock::new(Vec::new())),
            low_priority_tokens: Arc::new(RwLock::new(Vec::new())),

            critical_interval: Duration::from_secs(5),
            high_interval: Duration::from_secs(30),
            medium_interval: Duration::from_secs(90),
            low_interval: Duration::from_secs(180),

            max_requests_per_minute: 200,
            current_requests: Arc::new(RwLock::new(0)),
            last_reset: Arc::new(
                RwLock::new(
                    std::time::SystemTime
                        ::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                )
            ),
        }
    }

    pub async fn get_token_price(&self, _token_address: &str) -> Option<TokenPrice> {
        // Implementation for tiered pricing
        // This would check the appropriate tier and return cached price
        None
    }

    pub async fn start_tiered_updates(&self) {
        // Implementation for starting tiered update tasks
        // This would spawn tasks for each tier with different intervals
    }

    pub async fn set_open_positions(&self, _token_addresses: Vec<String>) {
        // Implementation for setting critical tokens
    }

    pub async fn categorize_tokens_by_volume(&self, _all_tokens: Vec<String>) {
        // Implementation for categorizing tokens by volume
    }

    async fn get_cached_price(&self, _token_address: &str) -> Option<TokenPrice> {
        // Implementation for getting cached price
        None
    }

    async fn update_token_tier(
        _tokens: &Arc<RwLock<Vec<TokenPriority>>>,
        _client: &GeckoTerminalClient,
        _cache: &Arc<RwLock<PriceCache>>,
        _database: &Arc<Database>,
        _requests: &Arc<RwLock<u32>>,
        _reset: &Arc<RwLock<u64>>,
        _max_requests: u32,
        _tier_name: &str
    ) {
        // Implementation for updating a specific tier
    }
}
