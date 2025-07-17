pub mod database;
pub mod decoders;
pub mod monitor;
pub mod types;
pub mod price_calculator;

pub use database::PoolDatabase;
pub use decoders::PoolDecoder;
pub use monitor::PoolMonitor;
pub use price_calculator::PriceCalculator;
pub use types::*;

use crate::marketdata::MarketData;
use crate::rpc::RpcManager;
use anyhow::{ Context, Result };
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main pool module for on-chain pool data monitoring and price calculation
pub struct PoolModule {
    database: Arc<PoolDatabase>,
    monitor: Arc<PoolMonitor>,
    price_calculator: Arc<PriceCalculator>,
    market_data: Arc<MarketData>,
    rpc_manager: Arc<RpcManager>,
    is_running: Arc<RwLock<bool>>,
}

impl PoolModule {
    pub fn new(market_data: Arc<MarketData>, rpc_manager: Arc<RpcManager>) -> Result<Self> {
        let database = Arc::new(PoolDatabase::new()?);
        let price_calculator = Arc::new(PriceCalculator::new());
        let monitor = Arc::new(
            PoolMonitor::new(
                Arc::clone(&database),
                Arc::clone(&rpc_manager),
                Arc::clone(&price_calculator)
            )?
        );

        Ok(Self {
            database,
            monitor,
            price_calculator,
            market_data,
            rpc_manager,
            is_running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the pool monitoring background task
    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            println!("⚠️  Pool module is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        println!("🏊 Pool module started");

        // Start background monitoring
        let monitor = Arc::clone(&self.monitor);
        let is_running = Arc::clone(&self.is_running);
        tokio::spawn(async move {
            monitor.run_monitoring_loop(is_running).await;
        });

        Ok(())
    }

    /// Stop the pool monitoring
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        println!("🔻 Pool module stopped");
    }

    /// Check if the module is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get pool information by address
    pub async fn get_pool_info(&self, pool_address: &str) -> Result<Option<PoolInfo>> {
        self.database.get_pool_info(pool_address)
    }

    /// Get current reserves for a pool
    pub async fn get_pool_reserves(&self, pool_address: &str) -> Result<Option<PoolReserve>> {
        self.database.get_latest_reserves(pool_address)
    }

    /// Get real-time price for a token from its pools
    pub async fn get_real_time_price(&self, token_mint: &str) -> Result<Option<f64>> {
        let pools = self.database.get_token_pools(token_mint)?;
        if pools.is_empty() {
            return Ok(None);
        }

        // Get the most liquid pool
        let best_pool = pools
            .iter()
            .max_by(|a, b|
                a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal)
            );

        if let Some(pool) = best_pool {
            if let Some(reserves) = self.database.get_latest_reserves(&pool.pool_address)? {
                let price = self.price_calculator.calculate_price(
                    &pool.pool_type,
                    &reserves,
                    token_mint,
                    &pool.base_token_mint,
                    &pool.quote_token_mint
                ).await?;
                return Ok(Some(price));
            }
        }

        Ok(None)
    }

    /// Get statistics about tracked pools
    pub async fn get_stats(&self) -> Result<PoolStats> {
        self.database.get_stats()
    }

    /// Force update a specific pool
    pub async fn force_update_pool(&self, pool_address: &str) -> Result<()> {
        self.monitor.update_single_pool(pool_address).await
    }

    /// Get pool history for a specific time range
    pub async fn get_pool_history(
        &self,
        pool_address: &str,
        hours_back: i64
    ) -> Result<Vec<PoolReserve>> {
        self.database.get_pool_history(pool_address, hours_back)
    }
}

impl Clone for PoolModule {
    fn clone(&self) -> Self {
        Self {
            database: Arc::clone(&self.database),
            monitor: Arc::clone(&self.monitor),
            price_calculator: Arc::clone(&self.price_calculator),
            market_data: Arc::clone(&self.market_data),
            rpc_manager: Arc::clone(&self.rpc_manager),
            is_running: Arc::clone(&self.is_running),
        }
    }
}
