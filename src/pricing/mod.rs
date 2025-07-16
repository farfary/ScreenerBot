use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{ Deserialize, Serialize };
use reqwest::Client;
use crate::database::Database;
use crate::logger::Logger;

pub mod gecko_terminal;
pub mod decoders;
pub mod pool_decoders;
pub mod price_calculator;
pub mod cache;

use gecko_terminal::GeckoTerminalClient;
use pool_decoders::PoolDecoderManager;
use price_calculator::PriceCalculator;
use cache::PriceCache;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub address: String,
    pub price_usd: f64,
    pub price_sol: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: f64,
    pub liquidity_usd: f64,
    pub timestamp: u64, // Unix timestamp in seconds
    pub source: PriceSource,
    pub is_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSource {
    GeckoTerminal,
    PoolCalculation,
    Cache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Option<u64>,
    pub pools: Vec<PoolInfo>,
    pub price: Option<TokenPrice>,
    pub last_updated: u64, // Unix timestamp in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: String,
    pub pool_type: PoolType,
    pub reserve_0: u64,
    pub reserve_1: u64,
    pub token_0: String,
    pub token_1: String,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub fee_tier: Option<f64>,
    pub last_updated: u64, // Unix timestamp
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PoolType {
    Raydium,
    PumpFun,
    Meteora,
    Orca,
    Serum,
    Unknown(String),
}

pub struct PricingManager {
    gecko_client: GeckoTerminalClient,
    #[allow(dead_code)]
    pool_decoder: PoolDecoderManager,
    price_calculator: PriceCalculator,
    cache: Arc<RwLock<PriceCache>>,
    database: Arc<Database>,
    logger: Arc<Logger>,
    update_interval: Duration,
    top_tokens_count: usize,
    priority_tokens: Arc<RwLock<Vec<String>>>,
    tiered_manager: Option<TieredPricingManager>,
}

impl PricingManager {
    pub fn new(
        database: Arc<Database>,
        logger: Arc<Logger>,
        update_interval_secs: u64,
        top_tokens_count: usize
    ) -> Self {
        let client = Client::new();

        Self {
            gecko_client: GeckoTerminalClient::new(client.clone()),
            pool_decoder: PoolDecoderManager::new(),
            price_calculator: PriceCalculator::new(),
            cache: Arc::new(RwLock::new(PriceCache::new())),
            database,
            logger,
            update_interval: Duration::from_secs(update_interval_secs),
            top_tokens_count,
            priority_tokens: Arc::new(RwLock::new(Vec::new())),
            tiered_manager: None,
        }
    }

    pub async fn start(&self) {
        Logger::pricing("Starting pricing manager...");

        let gecko_client = self.gecko_client.clone();
        let cache = self.cache.clone();
        let database = self.database.clone();
        let logger = self.logger.clone();
        let update_interval = self.update_interval;
        let top_tokens_count = self.top_tokens_count;
        let priority_tokens = self.priority_tokens.clone();

        // Start background price update task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);

            loop {
                interval.tick().await;

                if
                    let Err(e) = Self::update_token_prices(
                        &gecko_client,
                        &cache,
                        &database,
                        &logger,
                        top_tokens_count,
                        &priority_tokens
                    ).await
                {
                    Logger::error(&format!("FAILED to update prices: {}", e));
                }
            }
        });
    }

    async fn update_token_prices(
        gecko_client: &GeckoTerminalClient,
        cache: &Arc<RwLock<PriceCache>>,
        database: &Arc<Database>,
        logger: &Arc<Logger>,
        top_tokens_count: usize,
        priority_tokens: &Arc<RwLock<Vec<String>>>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get top tokens by liquidity from database
        let top_tokens = database.get_top_tokens_by_liquidity(top_tokens_count).await?;

        // Get priority tokens (positions, watch list, etc.)
        let priority_list = priority_tokens.read().await.clone();

        // Combine and deduplicate tokens
        let mut tokens_to_update: Vec<String> = top_tokens;
        tokens_to_update.extend(priority_list);
        tokens_to_update.sort();
        tokens_to_update.dedup();

        if tokens_to_update.is_empty() {
            return Ok(());
        }

        Logger::pricing(&format!("Updating {} token prices...", tokens_to_update.len()));

        // Update tokens in batches of 30 (API limit)
        const BATCH_SIZE: usize = 30;
        for chunk in tokens_to_update.chunks(BATCH_SIZE) {
            if
                let Err(e) = Self::update_token_batch(
                    gecko_client,
                    cache,
                    database,
                    logger,
                    chunk
                ).await
            {
                Logger::error(&format!("FAILED to update batch: {}", e));
            }

            // Rate limiting - wait between batches
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }

    async fn update_token_batch(
        gecko_client: &GeckoTerminalClient,
        cache: &Arc<RwLock<PriceCache>>,
        database: &Arc<Database>,
        _logger: &Arc<Logger>,
        token_addresses: &[String]
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Fetch token info from GeckoTerminal API
        let token_infos = gecko_client.get_multiple_tokens(token_addresses).await?;

        for token_info in token_infos {
            // Update cache
            {
                let mut cache_lock = cache.write().await;
                cache_lock.update_token_info(token_info.clone()).await;
            }

            // Update database
            database.update_token_info(&token_info).await?;

            Logger::debug(
                &format!(
                    "🏷️  PRICING: Updated {} - ${:.6} | Liq: ${:.0}",
                    token_info.symbol,
                    token_info.price
                        .as_ref()
                        .map(|p| p.price_usd)
                        .unwrap_or(0.0),
                    token_info.pools
                        .iter()
                        .map(|p| p.liquidity_usd)
                        .sum::<f64>()
                )
            );
        }

        Ok(())
    }

    pub async fn get_token_price(&self, token_address: &str) -> Option<TokenPrice> {
        // Use tiered pricing if available
        if let Some(ref tiered_manager) = self.tiered_manager {
            if let Some(token_price) = tiered_manager.get_token_price(token_address).await {
                return Some(token_price);
            }
        }

        // Fallback to basic pricing
        // Check cache first
        {
            let cache_lock = self.cache.read().await;
            if let Some(mut price) = cache_lock.get_token_price(token_address).await {
                price.is_cache = true;
                return Some(price);
            }
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

                    let mut result = price;
                    result.is_cache = false;
                    Some(result)
                } else {
                    None
                }
            }
            Err(e) => {
                Logger::error(
                    &format!("🏷️  PRICING: Failed to fetch price for {}: {}", token_address, e)
                );
                None
            }
        }
    }

    pub async fn calculate_real_price(&self, token_address: &str) -> Option<TokenPrice> {
        // Get token info from cache or API
        let token_info = {
            let cache_lock = self.cache.read().await;
            cache_lock.get_token_info(token_address).await
        };

        let token_info = match token_info {
            Some(info) => info,
            None => {
                // Try to fetch from API
                match self.gecko_client.get_token_info(token_address).await {
                    Ok(info) => info,
                    Err(_) => {
                        return None;
                    }
                }
            }
        };

        // Calculate price from top pools
        if !token_info.pools.is_empty() {
            match self.price_calculator.calculate_from_pools(&token_info.pools).await {
                Ok(calculated_price) => {
                    Logger::debug(
                        &format!(
                            "🏷️  PRICING: Calculated real price for {}: ${:.6}",
                            token_info.symbol,
                            calculated_price
                        )
                    );

                    Some(TokenPrice {
                        address: token_address.to_string(),
                        price_usd: calculated_price,
                        price_sol: None, // TODO: Calculate SOL price
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
                        source: PriceSource::PoolCalculation,
                        is_cache: false,
                    })
                }
                Err(e) => {
                    Logger::error(
                        &format!("🏷️  PRICING: Failed to calculate price from pools: {}", e)
                    );
                    None
                }
            }
        } else {
            None
        }
    }

    pub async fn get_cache_stats(&self) -> cache::CacheStats {
        self.cache.read().await.get_cache_stats().await
    }

    pub async fn add_priority_token(&self, token_address: String) {
        self.priority_tokens.write().await.push(token_address);
        Logger::pricing("Added priority token for frequent updates");
    }

    pub async fn remove_priority_token(&self, token_address: &str) {
        let mut priority_tokens = self.priority_tokens.write().await;
        priority_tokens.retain(|addr| addr != token_address);
    }

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
                Logger::error(
                    &format!("🏷️  PRICING: Failed to fetch token info for {}: {}", token_address, e)
                );
                None
            }
        }
    }

    pub async fn get_portfolio_value(&self, positions: &[(String, f64)]) -> f64 {
        let mut total_value = 0.0;

        for (token_address, amount) in positions {
            if let Some(price) = self.calculate_real_price(token_address).await {
                total_value += amount * price.price_usd;
            }
        }

        total_value
    }

    /// Initialize and start the tiered pricing system
    pub async fn enable_tiered_pricing(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let tiered_manager = TieredPricingManager::new(Arc::clone(&self.database));

        // Start the background update tasks
        tiered_manager.start_tiered_updates().await;

        self.tiered_manager = Some(tiered_manager);
        Logger::pricing("✅ Tiered pricing system enabled");

        Ok(())
    }

    /// Update token priorities for open positions
    pub async fn update_position_priorities(&self, positions: &[crate::types::WalletPosition]) {
        Logger::pricing(
            &format!(
                "🎯 PRIORITY UPDATE: Received {} positions for priority update",
                positions.len()
            )
        );

        if let Some(ref tiered_manager) = self.tiered_manager {
            let position_tokens: Vec<String> = positions
                .iter()
                .map(|p| p.mint.clone())
                .collect();

            Logger::pricing(
                &format!(
                    "🎯 PRIORITY UPDATE: Converting to {} token addresses for tiered manager",
                    position_tokens.len()
                )
            );
            for (i, token) in position_tokens.iter().enumerate() {
                Logger::pricing(&format!("  {}. {}...", i + 1, &token[..8]));
            }

            tiered_manager.set_open_positions(position_tokens).await;

            Logger::pricing(
                &format!(
                    "✅ PRIORITY UPDATE: Updated tiered pricing priorities for {} position tokens",
                    positions.len()
                )
            );
        } else {
            Logger::pricing(
                "⚠️ PRIORITY UPDATE: No tiered manager available, using basic priority system"
            );
            // Fallback: add to priority tokens for the basic system
            let mut priority_tokens = self.priority_tokens.write().await;
            for position in positions {
                if !priority_tokens.contains(&position.mint) {
                    priority_tokens.push(position.mint.clone());
                    Logger::pricing(
                        &format!(
                            "📌 PRIORITY UPDATE: Added {}... to basic priority list",
                            &position.mint[..8]
                        )
                    );
                }
            }

            if !positions.is_empty() {
                Logger::pricing(
                    &format!(
                        "� PRIORITY UPDATE: Added {} position tokens to basic priority list",
                        positions.len()
                    )
                );
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PricingTier {
    Critical, // Open positions - 5 seconds
    High, // High volume tokens - 30 seconds
    Medium, // Medium volume tokens - 1-2 minutes
    Low, // Low volume - 3+ minutes
}

#[derive(Debug, Clone)]
pub struct TokenPriority {
    pub address: String,
    pub tier: PricingTier,
    pub volume_24h: f64,
    pub last_updated: u64,
    pub is_open_position: bool,
    pub update_interval: Duration,
}

pub struct TieredPricingManager {
    gecko_client: GeckoTerminalClient,
    pool_decoder: PoolDecoderManager,
    price_calculator: PriceCalculator,
    cache: Arc<RwLock<PriceCache>>,
    database: Arc<Database>,

    // Tiered update system
    critical_tokens: Arc<RwLock<Vec<TokenPriority>>>, // Open positions
    high_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>, // Top volume
    medium_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>, // Medium volume
    low_priority_tokens: Arc<RwLock<Vec<TokenPriority>>>, // Low volume

    // Update intervals
    critical_interval: Duration, // 5 seconds
    high_interval: Duration, // 30 seconds
    medium_interval: Duration, // 90 seconds
    low_interval: Duration, // 180 seconds

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
            price_calculator: PriceCalculator::new(),
            cache: Arc::new(RwLock::new(PriceCache::new())),
            database: Arc::clone(&database),

            critical_tokens: Arc::new(RwLock::new(Vec::new())),
            high_priority_tokens: Arc::new(RwLock::new(Vec::new())),
            medium_priority_tokens: Arc::new(RwLock::new(Vec::new())),
            low_priority_tokens: Arc::new(RwLock::new(Vec::new())),

            critical_interval: Duration::from_secs(5),
            high_interval: Duration::from_secs(30),
            medium_interval: Duration::from_secs(90),
            low_interval: Duration::from_secs(180),

            max_requests_per_minute: 200, // Safe rate limit
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

    /// Add or update open position tokens (highest priority)
    pub async fn set_open_positions(&self, token_addresses: Vec<String>) {
        Logger::pricing(
            &format!("🎯 POSITIONS: Received {} open position tokens", token_addresses.len())
        );

        let mut critical = self.critical_tokens.write().await;
        critical.clear();

        for (i, address) in token_addresses.iter().enumerate() {
            Logger::pricing(
                &format!("🎯 POSITIONS: Adding critical token {}: {}...", i + 1, &address[..8])
            );
            critical.push(TokenPriority {
                address: address.clone(),
                tier: PricingTier::Critical,
                volume_24h: 0.0, // Will be updated when price is fetched
                last_updated: 0,
                is_open_position: true,
                update_interval: self.critical_interval,
            });
        }

        Logger::pricing(
            &format!(
                "🎯 POSITIONS: Set {} critical tokens for real-time pricing (5-second updates)",
                critical.len()
            )
        );
    }

    /// Categorize tokens by volume and assign pricing tiers
    pub async fn categorize_tokens_by_volume(&self, all_tokens: Vec<String>) {
        let mut tokens_with_volume = Vec::new();

        // Get volume data for all tokens
        for token in all_tokens.clone() {
            if let Some(cached_price) = self.get_cached_price(&token).await {
                tokens_with_volume.push((token, cached_price.volume_24h));
            } else {
                // For new tokens, start with low priority
                tokens_with_volume.push((token, 0.0));
            }
        }

        // Sort by volume descending
        tokens_with_volume.sort_by(|a, b|
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        );

        let total_tokens = tokens_with_volume.len();
        let high_threshold = total_tokens / 20; // Top 5% - high priority
        let medium_threshold = total_tokens / 5; // Top 20% - medium priority

        let mut high_priority = self.high_priority_tokens.write().await;
        let mut medium_priority = self.medium_priority_tokens.write().await;
        let mut low_priority = self.low_priority_tokens.write().await;

        high_priority.clear();
        medium_priority.clear();
        low_priority.clear();

        for (i, (address, volume)) in tokens_with_volume.into_iter().enumerate() {
            // Skip if it's already a critical token (open position)
            let critical = self.critical_tokens.read().await;
            if critical.iter().any(|t| t.address == address) {
                continue;
            }
            drop(critical);

            let (tier, interval, priority_list) = if i < high_threshold {
                (PricingTier::High, self.high_interval, &mut *high_priority)
            } else if i < medium_threshold {
                (PricingTier::Medium, self.medium_interval, &mut *medium_priority)
            } else {
                (PricingTier::Low, self.low_interval, &mut *low_priority)
            };

            priority_list.push(TokenPriority {
                address,
                tier,
                volume_24h: volume,
                last_updated: 0,
                is_open_position: false,
                update_interval: interval,
            });
        }

        Logger::pricing(
            &format!(
                "📊 Categorized tokens: {} high, {} medium, {} low priority",
                high_priority.len(),
                medium_priority.len(),
                low_priority.len()
            )
        );
    }

    /// Start the tiered pricing update system
    pub async fn start_tiered_updates(&self) {
        let critical_tokens = Arc::clone(&self.critical_tokens);
        let high_tokens = Arc::clone(&self.high_priority_tokens);
        let medium_tokens = Arc::clone(&self.medium_priority_tokens);
        let low_tokens = Arc::clone(&self.low_priority_tokens);

        let gecko_client = self.gecko_client.clone();
        let cache = Arc::clone(&self.cache);
        let database = Arc::clone(&self.database);
        let current_requests = Arc::clone(&self.current_requests);
        let last_reset = Arc::clone(&self.last_reset);
        let max_requests = self.max_requests_per_minute;

        // Critical tokens updater (5 seconds)
        let critical_task = {
            let tokens = Arc::clone(&critical_tokens);
            let client = gecko_client.clone();
            let cache = Arc::clone(&cache);
            let db = Arc::clone(&database);
            let requests = Arc::clone(&current_requests);
            let reset = Arc::clone(&last_reset);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    Self::update_token_tier(
                        &tokens,
                        &client,
                        &cache,
                        &db,
                        &requests,
                        &reset,
                        max_requests,
                        "CRITICAL"
                    ).await;
                }
            })
        };

        // High priority tokens updater (30 seconds)
        let high_task = {
            let tokens = Arc::clone(&high_tokens);
            let client = gecko_client.clone();
            let cache = Arc::clone(&cache);
            let db = Arc::clone(&database);
            let requests = Arc::clone(&current_requests);
            let reset = Arc::clone(&last_reset);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                loop {
                    interval.tick().await;
                    Self::update_token_tier(
                        &tokens,
                        &client,
                        &cache,
                        &db,
                        &requests,
                        &reset,
                        max_requests,
                        "HIGH"
                    ).await;
                }
            })
        };

        // Medium priority tokens updater (90 seconds)
        let medium_task = {
            let tokens = Arc::clone(&medium_tokens);
            let client = gecko_client.clone();
            let cache = Arc::clone(&cache);
            let db = Arc::clone(&database);
            let requests = Arc::clone(&current_requests);
            let reset = Arc::clone(&last_reset);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(90));
                loop {
                    interval.tick().await;
                    Self::update_token_tier(
                        &tokens,
                        &client,
                        &cache,
                        &db,
                        &requests,
                        &reset,
                        max_requests,
                        "MEDIUM"
                    ).await;
                }
            })
        };

        // Low priority tokens updater (180 seconds)
        let low_task = {
            let tokens = Arc::clone(&low_tokens);
            let client = gecko_client.clone();
            let cache = Arc::clone(&cache);
            let db = Arc::clone(&database);
            let requests = Arc::clone(&current_requests);
            let reset = Arc::clone(&last_reset);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(180));
                loop {
                    interval.tick().await;
                    Self::update_token_tier(
                        &tokens,
                        &client,
                        &cache,
                        &db,
                        &requests,
                        &reset,
                        max_requests,
                        "LOW"
                    ).await;
                }
            })
        };

        Logger::pricing("🚀 Started tiered pricing update system");

        // All tasks are now running in the background
        // Don't wait for them since they run indefinitely
    }

    /// Update a specific tier of tokens with rate limiting
    async fn update_token_tier(
        tokens: &Arc<RwLock<Vec<TokenPriority>>>,
        gecko_client: &GeckoTerminalClient,
        cache: &Arc<RwLock<PriceCache>>,
        database: &Arc<Database>,
        current_requests: &Arc<RwLock<u32>>,
        last_reset: &Arc<RwLock<u64>>,
        max_requests: u32,
        tier_name: &str
    ) {
        let mut tokens_lock = tokens.write().await;
        if tokens_lock.is_empty() {
            Logger::pricing(&format!("⚪ {} tier: No tokens to update", tier_name));
            return;
        }

        Logger::pricing(
            &format!("🔄 {} tier: Starting update for {} tokens", tier_name, tokens_lock.len())
        );

        // Rate limiting check
        let now = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        {
            let mut reset_time = last_reset.write().await;
            if now - *reset_time >= 60 {
                // Reset counter every minute
                *reset_time = now;
                *current_requests.write().await = 0;
            }
        }

        let current_count = *current_requests.read().await;
        if current_count >= max_requests {
            Logger::pricing(
                &format!(
                    "⚠️ {} tier: Rate limit reached ({}/{}), skipping update",
                    tier_name,
                    current_count,
                    max_requests
                )
            );
            return;
        }

        Logger::pricing(
            &format!("📊 {} tier: Rate limit OK ({}/{})", tier_name, current_count, max_requests)
        );

        // Determine how many tokens to update based on tier and remaining quota
        let remaining_quota = max_requests - current_count;
        let batch_size = match tier_name {
            "CRITICAL" => std::cmp::min(tokens_lock.len(), remaining_quota as usize),
            "HIGH" =>
                std::cmp::min(10, std::cmp::min(tokens_lock.len(), (remaining_quota as usize) / 4)),
            "MEDIUM" =>
                std::cmp::min(5, std::cmp::min(tokens_lock.len(), (remaining_quota as usize) / 8)),
            "LOW" =>
                std::cmp::min(3, std::cmp::min(tokens_lock.len(), (remaining_quota as usize) / 16)),
            _ => 1,
        };

        if batch_size == 0 {
            Logger::pricing(
                &format!("⚠️ {} tier: No tokens to update after rate limiting", tier_name)
            );
            return;
        }

        Logger::pricing(
            &format!("🎯 {} tier: Selected {} tokens for update", tier_name, batch_size)
        );

        // Sort by last_updated to prioritize stale prices
        tokens_lock.sort_by_key(|t| t.last_updated);

        let tokens_to_update: Vec<String> = tokens_lock
            .iter()
            .take(batch_size)
            .map(|t| t.address.clone())
            .collect();

        drop(tokens_lock);

        Logger::pricing(&format!("🔄 {} tier: Updating {} tokens", tier_name, batch_size));

        // Update prices for selected tokens
        let mut updated_count = 0;
        let mut failed_count = 0;

        for token_address in tokens_to_update {
            Logger::pricing(
                &format!("🌐 {} tier: Fetching price for {}...", tier_name, &token_address[..8])
            );

            if let Ok(token_info) = gecko_client.get_token_info(&token_address).await {
                if let Some(price_info) = token_info.price {
                    Logger::pricing(
                        &format!(
                            "💰 {} tier: Got price for {}: ${:.6} | {:.9} SOL",
                            tier_name,
                            &token_address[..8],
                            price_info.price_usd,
                            price_info.price_sol.unwrap_or(0.0)
                        )
                    );

                    // Check for significant price changes (>5%)
                    let mut show_big_change = false;
                    {
                        let cache_lock = cache.read().await;
                        if let Some(old_price) = cache_lock.get_token_price(&token_address).await {
                            let price_change =
                                ((price_info.price_usd - old_price.price_usd) /
                                    old_price.price_usd) *
                                100.0;
                            if price_change.abs() > 5.0 {
                                show_big_change = true;
                                Logger::pricing(
                                    &format!(
                                        "🚨 BIG PRICE CHANGE: {} | {:.2}% | ${:.6} -> ${:.6}",
                                        &token_address[..8],
                                        price_change,
                                        old_price.price_usd,
                                        price_info.price_usd
                                    )
                                );
                            }
                        }
                    }

                    // Update cache
                    {
                        let mut cache_lock = cache.write().await;
                        cache_lock.update_token_price(price_info.clone()).await;
                    }

                    // Update database
                    // Note: Database doesn't have a cache_token_price method, skipping DB cache

                    // Update last_updated time for this token
                    let mut tokens_lock = tokens.write().await;
                    if
                        let Some(token) = tokens_lock
                            .iter_mut()
                            .find(|t| t.address == token_address)
                    {
                        token.last_updated = now;
                        token.volume_24h = price_info.volume_24h;
                        Logger::pricing(
                            &format!(
                                "✅ {} tier: Updated {} volume: ${:.0}",
                                tier_name,
                                &token_address[..8],
                                price_info.volume_24h
                            )
                        );
                    }

                    updated_count += 1;
                } else {
                    Logger::pricing(
                        &format!("⚠️ {} tier: No price data for {}", tier_name, &token_address[..8])
                    );
                    failed_count += 1;
                }
            } else {
                Logger::pricing(
                    &format!(
                        "❌ {} tier: Failed to fetch data for {}",
                        tier_name,
                        &token_address[..8]
                    )
                );
                failed_count += 1;
            }

            // Increment request counter
            *current_requests.write().await += 1;

            // Small delay to avoid overwhelming the API
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Logger::pricing(
            &format!(
                "✅ {} tier: Update complete - {} updated, {} failed",
                tier_name,
                updated_count,
                failed_count
            )
        );
    }

    /// Get current price with tier-aware caching
    pub async fn get_token_price(&self, token_address: &str) -> Option<TokenPrice> {
        // Check cache first
        if let Some(cached_price) = self.get_cached_price(token_address).await {
            // Determine if cache is still valid based on token tier
            let cache_age =
                std::time::SystemTime
                    ::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() - cached_price.timestamp;

            let tier = self.get_token_tier(token_address).await;
            let max_age = match tier {
                PricingTier::Critical => 10, // 10 seconds max age for critical
                PricingTier::High => 60, // 1 minute for high priority
                PricingTier::Medium => 180, // 3 minutes for medium
                PricingTier::Low => 300, // 5 minutes for low
            };

            if cache_age <= max_age {
                return Some(cached_price);
            }
        }

        // If not in cache or stale, try to fetch fresh data
        // But only if we're not rate limited
        let current_count = *self.current_requests.read().await;
        if current_count < self.max_requests_per_minute {
            if let Ok(token_info) = self.gecko_client.get_token_info(token_address).await {
                if let Some(fresh_price) = token_info.price {
                    // Update cache
                    {
                        let mut cache_lock = self.cache.write().await;
                        cache_lock.update_token_price(fresh_price.clone()).await;
                    }

                    *self.current_requests.write().await += 1;
                    return Some(fresh_price);
                }
            }
        }

        // Fall back to stale cache if available
        self.get_cached_price(token_address).await
    }

    async fn get_cached_price(&self, token_address: &str) -> Option<TokenPrice> {
        let cache_lock = self.cache.read().await;
        cache_lock.get_token_price(token_address).await
    }

    async fn get_token_tier(&self, token_address: &str) -> PricingTier {
        // Check critical tokens first
        let critical = self.critical_tokens.read().await;
        if critical.iter().any(|t| t.address == token_address) {
            return PricingTier::Critical;
        }
        drop(critical);

        // Check high priority
        let high = self.high_priority_tokens.read().await;
        if high.iter().any(|t| t.address == token_address) {
            return PricingTier::High;
        }
        drop(high);

        // Check medium priority
        let medium = self.medium_priority_tokens.read().await;
        if medium.iter().any(|t| t.address == token_address) {
            return PricingTier::Medium;
        }

        // Default to low priority
        PricingTier::Low
    }

    /// Get pricing statistics
    pub async fn get_pricing_stats(&self) -> PricingStats {
        let critical_count = self.critical_tokens.read().await.len();
        let high_count = self.high_priority_tokens.read().await.len();
        let medium_count = self.medium_priority_tokens.read().await.len();
        let low_count = self.low_priority_tokens.read().await.len();
        let current_requests = *self.current_requests.read().await;

        PricingStats {
            critical_tokens: critical_count,
            high_priority_tokens: high_count,
            medium_priority_tokens: medium_count,
            low_priority_tokens: low_count,
            total_tokens: critical_count + high_count + medium_count + low_count,
            requests_per_minute: current_requests,
            max_requests_per_minute: self.max_requests_per_minute,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PricingStats {
    pub critical_tokens: usize,
    pub high_priority_tokens: usize,
    pub medium_priority_tokens: usize,
    pub low_priority_tokens: usize,
    pub total_tokens: usize,
    pub requests_per_minute: u32,
    pub max_requests_per_minute: u32,
}
