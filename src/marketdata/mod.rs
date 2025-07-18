pub mod database;
pub mod gecko_api;

pub use database::{
    MarketDatabase,
    TokenData,
    PoolData,
    MarketStats,
    LiquidityHistory,
    TokenBlacklist,
    RugDetectionEvent,
};
pub use gecko_api::GeckoTerminalClient;

use crate::discovery::DiscoveryDatabase;
use anyhow::{ Context, Result };
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

pub struct MarketData {
    database: Arc<MarketDatabase>,
    gecko_client: GeckoTerminalClient,
    discovery_db: Arc<DiscoveryDatabase>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<MarketStats>>,
}

impl MarketData {
    pub fn new(discovery_db: Arc<DiscoveryDatabase>) -> Result<Self> {
        let database = Arc::new(MarketDatabase::new()?);

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()
            .context("Failed to create HTTP client")?;

        let gecko_client = GeckoTerminalClient::new(client);

        let stats = MarketStats {
            total_tokens_tracked: 0,
            active_tokens: 0,
            total_pools: 0,
            last_update_run: chrono::Utc::now(),
            update_rate_per_hour: 0.0,
        };

        Ok(Self {
            database,
            gecko_client,
            discovery_db,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            println!("⚠️  Market data module is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        println!("💹 Market data module started");

        // Start background update loop
        let market_data = self.clone();
        tokio::spawn(async move {
            market_data.run_update_loop().await;
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        println!("🔻 Market data module stopped");
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    pub async fn get_stats(&self) -> MarketStats {
        self.stats.read().await.clone()
    }

    /// Get reference to the market database
    pub fn get_database(&self) -> Arc<MarketDatabase> {
        self.database.clone()
    }

    async fn run_update_loop(&self) {
        let mut interval = time::interval(Duration::from_secs(10)); // 10 second intervals
        let mut tokens_updated_this_session = 0u64;
        let start_time = chrono::Utc::now();

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                break;
            }
            drop(is_running);

            match self.update_token_data().await {
                Ok(updated_count) => {
                    tokens_updated_this_session += updated_count;

                    if updated_count > 0 {
                        println!("✅ Updated {} tokens with market data", updated_count);
                    }

                    // Update stats
                    let stats = match self.database.get_stats() {
                        Ok(stats) => stats,
                        Err(e) => {
                            eprintln!("❌ Failed to get market stats: {}", e);
                            continue;
                        }
                    };

                    *self.stats.write().await = stats;
                }
                Err(e) => {
                    eprintln!("❌ Market data update failed: {}", e);
                }
            }
        }
    }

    async fn update_token_data(&self) -> Result<u64> {
        // Get discovered tokens from discovery database
        let discovered_tokens = self.discovery_db
            .get_all_tokens()
            .context("Failed to get discovered tokens")?;

        if discovered_tokens.is_empty() {
            return Ok(0);
        }

        let mut updated_count = 0u64;

        // Process tokens in batches of 30 per API call
        let batch_size = 30;
        for batch in discovered_tokens.chunks(batch_size) {
            let mints: Vec<String> = batch
                .iter()
                .map(|t| t.mint.clone())
                .collect();

            // Filter out blacklisted tokens before processing
            let mut non_blacklisted_mints = Vec::new();
            for mint in &mints {
                match self.database.is_blacklisted(mint) {
                    Ok(false) => non_blacklisted_mints.push(mint.clone()),
                    Ok(true) => {
                        // Skip blacklisted tokens
                        continue;
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to check blacklist for {}: {}", mint, e);
                        // Include in batch if blacklist check fails
                        non_blacklisted_mints.push(mint.clone());
                    }
                }
            }

            if non_blacklisted_mints.is_empty() {
                continue; // Skip this batch if all tokens are blacklisted
            }

            match self.gecko_client.fetch_token_data_batch(&non_blacklisted_mints).await {
                Ok(results) => {
                    for (token_data, pools) in results {
                        // Check price change before saving
                        let prev_token = self.database.get_token(&token_data.mint).ok().flatten();
                        if let Some(prev) = &prev_token {
                            let old_price = prev.price_sol;
                            let new_price = token_data.price_sol;
                            if old_price > 0.0 {
                                let diff = ((new_price - old_price) / old_price).abs();
                                if diff > 0.01 {
                                    let pct = ((new_price - old_price) / old_price) * 100.0;
                                    // Find the earliest pool created_at as token age proxy
                                    let token_age = pools
                                        .iter()
                                        .map(|p| p.created_at)
                                        .min();
                                    let now = chrono::Utc::now();
                                    let age_str = if let Some(created_at) = token_age {
                                        let duration = now.signed_duration_since(created_at);
                                        let days = duration.num_days();
                                        let hours = duration.num_hours() % 24;
                                        let mins = duration.num_minutes() % 60;
                                        format!("age: {}d {}h {}m", days, hours, mins)
                                    } else {
                                        "age: unknown".to_string()
                                    };

                                    // Now we store prices directly in SOL terms, no conversion needed
                                    let market_cap_sol = token_data.market_cap;
                                    let liquidity_sol = token_data.liquidity_sol;

                                    // Price change detection - data saved to database for trader module to display
                                }
                            }
                        }

                        // Save token data
                        if let Err(e) = self.database.save_token(&token_data) {
                            eprintln!(
                                "❌ Failed to save token data for {}: {}",
                                token_data.mint,
                                e
                            );
                            continue;
                        }

                        // Save pool data
                        for pool in pools {
                            if let Err(e) = self.database.save_pool(&pool) {
                                eprintln!(
                                    "❌ Failed to save pool data for {}: {}",
                                    pool.pool_address,
                                    e
                                );
                            }
                        }

                        updated_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch market data batch: {}", e);
                }
            }

            // Small delay to respect API rate limits
            tokio::time::sleep(Duration::from_millis(10000)).await;
        }

        Ok(updated_count)
    }

    // Public API methods
    pub async fn get_token_data(&self, mint: &str) -> Result<Option<TokenData>> {
        self.database.get_token(mint)
    }

    pub async fn get_all_tokens(&self) -> Result<Vec<TokenData>> {
        self.database.get_all_tokens()
    }

    pub async fn get_token_pools(&self, mint: &str) -> Result<Vec<PoolData>> {
        self.database.get_token_pools(mint)
    }

    /// Get top tokens by volume for trader monitoring
    pub async fn get_top_tokens_by_volume(&self, limit: usize) -> Result<Vec<TokenData>> {
        self.database.get_top_tokens_by_volume(limit)
    }

    /// Check if a token is blacklisted
    pub async fn is_token_blacklisted(&self, mint: &str) -> Result<bool> {
        self.database.is_blacklisted(mint)
    }

    /// Add token to blacklist
    pub async fn blacklist_token(&self, blacklist_entry: &TokenBlacklist) -> Result<()> {
        self.database.add_to_blacklist(blacklist_entry)
    }

    /// Get blacklist entry for a token
    pub async fn get_blacklist_entry(&self, mint: &str) -> Result<Option<TokenBlacklist>> {
        self.database.get_blacklist_entry(mint)
    }

    /// Clean old liquidity history
    pub async fn cleanup_old_data(&self) -> Result<()> {
        self.database.cleanup_old_liquidity_history()
    }
}

// Implement Clone for MarketData (needed for tokio::spawn)
impl Clone for MarketData {
    fn clone(&self) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("ScreenerBot/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            database: Arc::clone(&self.database),
            gecko_client: GeckoTerminalClient::new(client),
            discovery_db: Arc::clone(&self.discovery_db),
            is_running: Arc::clone(&self.is_running),
            stats: Arc::clone(&self.stats),
        }
    }
}
