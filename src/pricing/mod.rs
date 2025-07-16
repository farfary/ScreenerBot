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
    pool_decoder: PoolDecoderManager,
    price_calculator: PriceCalculator,
    cache: Arc<RwLock<PriceCache>>,
    database: Arc<Database>,
    logger: Arc<Logger>,
    update_interval: Duration,
    top_tokens_count: usize,
    priority_tokens: Arc<RwLock<Vec<String>>>,
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
}
