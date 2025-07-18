pub mod database;

pub use database::{
    MarketDatabase,
    TokenData,
    PoolData,
    MarketStats,
    LiquidityHistory,
    TokenBlacklist,
    RugDetectionEvent,
};

use crate::discovery::DiscoveryDatabase;
use crate::pairs::{ PairsClient, TokenPair };
use anyhow::{ Context, Result };
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;
use chrono::Utc;

pub struct MarketData {
    database: Arc<MarketDatabase>,
    pairs_client: Arc<PairsClient>,
    discovery_db: Arc<DiscoveryDatabase>,
    is_running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<MarketStats>>,
}

impl MarketData {
    pub fn new(discovery_db: Arc<DiscoveryDatabase>) -> Result<Self> {
        let database = Arc::new(MarketDatabase::new()?);
        let pairs_client = Arc::new(PairsClient::new().context("Failed to create pairs client")?);

        let stats = MarketStats {
            total_tokens_tracked: 0,
            active_tokens: 0,
            total_pools: 0,
            last_update_run: chrono::Utc::now(),
            update_rate_per_hour: 0.0,
        };

        Ok(Self {
            database,
            pairs_client,
            discovery_db,
            is_running: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(stats)),
        })
    }

    /// Convert TokenPair from DexScreener API to our TokenData model
    fn token_pair_to_token_data(pair: &TokenPair) -> Result<TokenData> {
        // Determine which token is the base (not SOL)
        let (token_address, token_symbol, token_name) = if
            pair.base_token.address != "So11111111111111111111111111111111111111112"
        {
            (
                pair.base_token.address.clone(),
                pair.base_token.symbol.clone(),
                pair.base_token.name.clone(),
            )
        } else {
            (
                pair.quote_token.address.clone(),
                pair.quote_token.symbol.clone(),
                pair.quote_token.name.clone(),
            )
        };

        let price_native = pair.price_native_float().context("Failed to parse native price")?;
        let price_usd = pair.price_usd_float().context("Failed to parse USD price")?;

        // Get liquidity metrics if available
        let (liquidity_usd, liquidity_base, liquidity_quote) = if
            let Some(ref liq) = pair.liquidity
        {
            (liq.usd, liq.base, liq.quote)
        } else {
            (0.0, 0.0, 0.0)
        };

        Ok(TokenData {
            mint: token_address,
            symbol: token_symbol,
            name: token_name,
            decimals: 6, // Default for most Solana tokens, could be improved
            price_native,
            price_usd,
            price_change_24h: pair.price_change.h24,
            volume_24h: pair.volume.h24,
            market_cap: pair.market_cap,
            fdv: pair.fdv,
            liquidity_usd,
            liquidity_base,
            liquidity_quote,
            top_pool_address: Some(pair.pair_address.clone()),
            dex_id: pair.dex_id.clone(),
            pair_created_at: Some(pair.pair_created_at),
            source: "dexscreener".to_string(),
            last_updated: Utc::now(),
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

        // Process tokens in batches to respect API limits
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

            // Convert Vec<String> to Vec<&str> for the API
            let mint_refs: Vec<&str> = non_blacklisted_mints
                .iter()
                .map(|s| s.as_str())
                .collect();

            // Fetch token pairs from DexScreener
            match self.pairs_client.get_multiple_token_pairs(&mint_refs).await {
                Ok(all_pairs) => {
                    // Group pairs by token and convert to TokenData
                    for mint in &non_blacklisted_mints {
                        // Filter pairs for this specific token (as base token, not quote)
                        let token_pairs: Vec<&TokenPair> = all_pairs
                            .iter()
                            .filter(|pair| {
                                pair.base_token.address == *mint &&
                                    pair.quote_token.address ==
                                        "So11111111111111111111111111111111111111112" // Only SOL pairs
                            })
                            .collect();

                        if
                            let Some(best_pair) = self.pairs_client.get_best_pair(
                                token_pairs.into_iter().cloned().collect()
                            )
                        {
                            // Convert TokenPair to TokenData
                            match Self::token_pair_to_token_data(&best_pair) {
                                Ok(token_data) => {
                                    // Check for significant price changes
                                    if
                                        let Ok(Some(prev_token)) = self.database.get_token(
                                            &token_data.mint
                                        )
                                    {
                                        let old_price = prev_token.price_native;
                                        let new_price = token_data.price_native;
                                        if old_price > 0.0 {
                                            let diff = ((new_price - old_price) / old_price).abs();
                                            if diff > 0.01 {
                                                let pct =
                                                    ((new_price - old_price) / old_price) * 100.0;

                                                // Calculate token age
                                                let age_str = if
                                                    let Some(created_at) =
                                                        token_data.pair_created_at
                                                {
                                                    let created_time = chrono::DateTime
                                                        ::from_timestamp_millis(created_at as i64)
                                                        .unwrap_or(Utc::now());
                                                    let duration =
                                                        Utc::now().signed_duration_since(
                                                            created_time
                                                        );
                                                    let days = duration.num_days();
                                                    let hours = duration.num_hours() % 24;
                                                    let mins = duration.num_minutes() % 60;
                                                    format!("age: {}d {}h {}m", days, hours, mins)
                                                } else {
                                                    "age: unknown".to_string()
                                                };
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

                                    updated_count += 1;
                                }
                                Err(e) => {
                                    eprintln!("❌ Failed to convert pair data for {}: {}", mint, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch token pairs from DexScreener: {}", e);
                }
            }
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
        Self {
            database: Arc::clone(&self.database),
            pairs_client: Arc::clone(&self.pairs_client),
            discovery_db: Arc::clone(&self.discovery_db),
            is_running: Arc::clone(&self.is_running),
            stats: Arc::clone(&self.stats),
        }
    }
}
