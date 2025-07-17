use crate::pool::database::PoolDatabase;
use crate::pool::decoders::{ DecoderFactory, PoolDecoder };
use crate::pool::price_calculator::PriceCalculator;
use crate::pool::types::*;
use crate::rpc::RpcManager;
use anyhow::{ Context, Result };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

/// Pool monitor for tracking on-chain pool data
pub struct PoolMonitor {
    database: Arc<PoolDatabase>,
    rpc_manager: Arc<RpcManager>,
    price_calculator: Arc<PriceCalculator>,
    decoders: HashMap<PoolType, Box<dyn PoolDecoder>>,
    config: PoolMonitorConfig,
    stats: Arc<RwLock<PoolStats>>,
}

impl PoolMonitor {
    pub fn new(
        database: Arc<PoolDatabase>,
        rpc_manager: Arc<RpcManager>,
        price_calculator: Arc<PriceCalculator>
    ) -> Result<Self> {
        let mut decoders = HashMap::new();

        // Initialize decoders for each pool type
        decoders.insert(
            PoolType::Raydium,
            DecoderFactory::create_for_type(PoolType::Raydium).unwrap()
        );
        decoders.insert(PoolType::Orca, DecoderFactory::create_for_type(PoolType::Orca).unwrap());
        decoders.insert(
            PoolType::Meteora,
            DecoderFactory::create_for_type(PoolType::Meteora).unwrap()
        );
        decoders.insert(
            PoolType::PumpFun,
            DecoderFactory::create_for_type(PoolType::PumpFun).unwrap()
        );
        decoders.insert(PoolType::Serum, DecoderFactory::create_for_type(PoolType::Serum).unwrap());

        let config = PoolMonitorConfig::default();
        let stats = Arc::new(
            RwLock::new(PoolStats {
                total_pools: 0,
                active_pools: 0,
                pools_by_type: HashMap::new(),
                total_reserves_history: 0,
                last_update: chrono::Utc::now(),
                update_rate_per_hour: 0.0,
            })
        );

        Ok(Self {
            database,
            rpc_manager,
            price_calculator,
            decoders,
            config,
            stats,
        })
    }

    /// Run the main monitoring loop
    pub async fn run_monitoring_loop(&self, is_running: Arc<RwLock<bool>>) {
        let mut interval = time::interval(Duration::from_secs(self.config.update_interval_seconds));
        let mut last_cleanup = chrono::Utc::now();

        loop {
            interval.tick().await;

            let running = is_running.read().await;
            if !*running {
                break;
            }
            drop(running);

            // Update pool data
            if let Err(e) = self.update_all_pools().await {
                eprintln!("❌ Failed to update pools: {}", e);
            }

            // Update statistics
            if let Err(e) = self.update_stats().await {
                eprintln!("❌ Failed to update pool stats: {}", e);
            }

            // Cleanup old data (once per hour)
            let now = chrono::Utc::now();
            if now.signed_duration_since(last_cleanup).num_hours() >= 1 {
                if let Err(e) = self.cleanup_old_data().await {
                    eprintln!("❌ Failed to cleanup old data: {}", e);
                }
                last_cleanup = now;
            }
        }
    }

    /// Update all active pools
    async fn update_all_pools(&self) -> Result<()> {
        let pools = self.database.get_all_active_pools().context("Failed to get active pools")?;

        if pools.is_empty() {
            return Ok(());
        }

        let mut tasks = Vec::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_updates));

        for pool in pools {
            let semaphore = Arc::clone(&semaphore);
            let pool_address = pool.pool_address.clone();
            let monitor = self.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                if let Err(e) = monitor.update_single_pool(&pool_address).await {
                    eprintln!("❌ Failed to update pool {}: {}", pool_address, e);
                }
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    /// Update a single pool
    pub async fn update_single_pool(&self, pool_address: &str) -> Result<()> {
        let pool_pubkey = Pubkey::from_str(pool_address).context("Invalid pool address")?;

        // Get pool info from database
        let pool_info = self.database
            .get_pool_info(pool_address)
            .context("Failed to get pool info")?;

        let pool_info = match pool_info {
            Some(info) => info,
            None => {
                // If pool not in database, try to discover it
                return self.discover_and_add_pool(pool_address).await;
            }
        };

        // Get decoder for this pool type
        let decoder = match self.decoders.get(&pool_info.pool_type) {
            Some(decoder) => decoder,
            None => {
                eprintln!("❌ No decoder available for pool type: {}", pool_info.pool_type);
                return Ok(());
            }
        };

        // Get account data from RPC
        let account_data = self.rpc_manager
            .get_account_data(&pool_pubkey).await
            .context("Failed to get account data")?;

        if account_data.is_empty() {
            eprintln!("❌ Empty account data for pool: {}", pool_address);
            return Ok(());
        }

        // Get current slot
        let slot = self.rpc_manager.get_slot().await.context("Failed to get current slot")?;

        // Decode reserves
        let reserves = decoder
            .decode_reserves(pool_address, &account_data, slot).await
            .context("Failed to decode reserves")?;

        // Save reserves to database
        self.database.save_pool_reserves(&reserves).context("Failed to save pool reserves")?;

        // Calculate price change if we have previous data
        if
            let Some(prev_reserves) = self.database
                .get_latest_reserves(pool_address)
                .unwrap_or(None)
        {
            let price_change = self.calculate_price_change(
                &pool_info,
                &prev_reserves,
                &reserves
            ).await?;

            if price_change.abs() > 0.01 {
                // 1% change threshold
                self.log_price_change(&pool_info, price_change, &reserves).await;
            }
        }

        Ok(())
    }

    /// Discover and add a new pool
    async fn discover_and_add_pool(&self, pool_address: &str) -> Result<()> {
        let pool_pubkey = Pubkey::from_str(pool_address).context("Invalid pool address")?;

        // Get account data
        let account_data = self.rpc_manager
            .get_account_data(&pool_pubkey).await
            .context("Failed to get account data")?;

        if account_data.is_empty() {
            return Err(anyhow::anyhow!("Empty account data for pool: {}", pool_address));
        }

        // Try to find a decoder that can handle this data
        let decoder = DecoderFactory::find_decoder(&account_data).ok_or_else(||
            anyhow::anyhow!("No decoder found for pool: {}", pool_address)
        )?;

        // Decode pool info
        let pool_info = decoder
            .decode_pool_info(pool_address, &account_data).await
            .context("Failed to decode pool info")?;

        // Check if pool meets minimum liquidity requirement
        if pool_info.liquidity_usd < self.config.min_liquidity_usd {
            return Ok(()); // Skip low liquidity pools
        }

        // Save pool info to database
        self.database.save_pool_info(&pool_info).context("Failed to save pool info")?;

        println!("✅ Added new pool: {} ({})", pool_address, pool_info.pool_type);

        // Get initial reserves
        let slot = self.rpc_manager.get_slot().await.context("Failed to get current slot")?;

        let reserves = decoder
            .decode_reserves(pool_address, &account_data, slot).await
            .context("Failed to decode reserves")?;

        // Save initial reserves
        self.database.save_pool_reserves(&reserves).context("Failed to save initial reserves")?;

        Ok(())
    }

    /// Calculate price change between two reserve states
    async fn calculate_price_change(
        &self,
        pool_info: &PoolInfo,
        prev_reserves: &PoolReserve,
        new_reserves: &PoolReserve
    ) -> Result<f64> {
        // Calculate previous price
        let prev_price = self.price_calculator.calculate_price(
            &pool_info.pool_type,
            prev_reserves,
            &pool_info.base_token_mint,
            &pool_info.base_token_mint,
            &pool_info.quote_token_mint
        ).await?;

        // Calculate new price
        let new_price = self.price_calculator.calculate_price(
            &pool_info.pool_type,
            new_reserves,
            &pool_info.base_token_mint,
            &pool_info.base_token_mint,
            &pool_info.quote_token_mint
        ).await?;

        if prev_price == 0.0 {
            return Ok(0.0);
        }

        let change = ((new_price - prev_price) / prev_price) * 100.0;
        Ok(change)
    }

    /// Log significant price changes
    async fn log_price_change(
        &self,
        pool_info: &PoolInfo,
        price_change: f64,
        reserves: &PoolReserve
    ) {
        use colored::*;

        let change_str = if price_change >= 0.0 {
            format!("+{:.2}%", price_change).green().bold()
        } else {
            format!("{:.2}%", price_change).red().bold()
        };

        let pool_type_str = format!("{}", pool_info.pool_type).bright_cyan();
        let pool_addr_short = &pool_info.pool_address[..8];

        println!(
            "{} {} {} | Price Change: {} | Base: {} | Quote: {}",
            "POOL UPDATE:".on_bright_black().bold().white(),
            pool_type_str,
            pool_addr_short,
            change_str,
            reserves.base_token_amount,
            reserves.quote_token_amount
        );
    }

    /// Update statistics
    async fn update_stats(&self) -> Result<()> {
        let stats = self.database.get_stats().context("Failed to get database stats")?;

        *self.stats.write().await = stats;
        Ok(())
    }

    /// Cleanup old data
    async fn cleanup_old_data(&self) -> Result<()> {
        let keep_days = 7; // Keep 7 days of history
        let deleted = self.database
            .cleanup_old_reserves(keep_days)
            .context("Failed to cleanup old reserves")?;

        if deleted > 0 {
            println!("🧹 Cleaned up {} old reserve records", deleted);
        }

        Ok(())
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    /// Add pools from market data
    pub async fn sync_pools_from_market_data(
        &self,
        market_data: &crate::marketdata::MarketData
    ) -> Result<()> {
        // Get all tokens from market data
        let tokens = market_data
            .get_all_tokens().await
            .context("Failed to get tokens from market data")?;

        let mut added_pools = 0;

        for token in tokens {
            // Get pools for this token from market data
            let pools = market_data
                .get_token_pools(&token.mint).await
                .context("Failed to get token pools")?;

            for pool in pools {
                let pool_addr = &pool.pool_address;
                // Check if pool already exists
                if self.database.get_pool_info(pool_addr).unwrap_or(None).is_none() {
                    // Try to add this pool
                    if let Err(e) = self.discover_and_add_pool(pool_addr).await {
                        eprintln!("❌ Failed to add pool {}: {}", pool_addr, e);
                    } else {
                        added_pools += 1;
                    }
                }
            }
        }

        if added_pools > 0 {
            println!("✅ Added {} new pools from market data", added_pools);
        }

        Ok(())
    }
}

impl Clone for PoolMonitor {
    fn clone(&self) -> Self {
        // Create new decoders for the clone
        let mut decoders = HashMap::new();
        decoders.insert(
            PoolType::Raydium,
            DecoderFactory::create_for_type(PoolType::Raydium).unwrap()
        );
        decoders.insert(PoolType::Orca, DecoderFactory::create_for_type(PoolType::Orca).unwrap());
        decoders.insert(
            PoolType::Meteora,
            DecoderFactory::create_for_type(PoolType::Meteora).unwrap()
        );
        decoders.insert(
            PoolType::PumpFun,
            DecoderFactory::create_for_type(PoolType::PumpFun).unwrap()
        );
        decoders.insert(PoolType::Serum, DecoderFactory::create_for_type(PoolType::Serum).unwrap());

        Self {
            database: Arc::clone(&self.database),
            rpc_manager: Arc::clone(&self.rpc_manager),
            price_calculator: Arc::clone(&self.price_calculator),
            decoders,
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
        }
    }
}
