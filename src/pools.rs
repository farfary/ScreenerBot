use std::time::Duration;
use std::collections::HashMap;
use serde::{ Deserialize, Serialize };
use crate::global::{ is_shutdown };
use crate::logger::{ log, LogLevel };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub address: String,
    pub token_a: String,
    pub token_b: String,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub liquidity: u64,
    pub fee_rate: f64,
    pub last_updated: u64,
    pub volume_24h: f64,
    pub price: f64,
}

#[derive(Debug)]
pub struct PoolManager {
    pools: HashMap<String, Pool>,
    monitored_tokens: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_pools: usize,
    pub total_liquidity: f64,
    pub total_volume_24h: f64,
    pub monitored_tokens: usize,
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_pools: 0,
            total_liquidity: 0.0,
            total_volume_24h: 0.0,
            monitored_tokens: 0,
        }
    }
}

impl PoolManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            monitored_tokens: Vec::new(),
        }
    }

    pub async fn discover_pools(&mut self) -> anyhow::Result<()> {
        log("POOLS", LogLevel::Info, "Discovering new pools...");

        let mock_pools = vec![Pool {
            address: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
            token_a: "So11111111111111111111111111111111111111112".to_string(),
            token_b: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            token_a_amount: 1000000000,
            token_b_amount: 50000000000,
            liquidity: 75000000000,
            fee_rate: 0.0025,
            last_updated: chrono::Utc::now().timestamp() as u64,
            volume_24h: 1250000.0,
            price: 50.0,
        }];

        for pool in mock_pools {
            self.pools.insert(pool.address.clone(), pool);
        }

        log("POOLS", LogLevel::Info, &format!("Discovered {} pools", self.pools.len()));
        Ok(())
    }

    pub async fn update_pool_data(&mut self, pool_address: &str) -> anyhow::Result<()> {
        if let Some(pool) = self.pools.get_mut(pool_address) {
            pool.last_updated = chrono::Utc::now().timestamp() as u64;
            log("POOLS", LogLevel::Info, &format!("Updated pool data for {}", pool_address));
        }
        Ok(())
    }

    pub fn get_pool(&self, address: &str) -> Option<&Pool> {
        self.pools.get(address)
    }

    pub fn get_all_pools(&self) -> &HashMap<String, Pool> {
        &self.pools
    }

    pub fn add_monitored_token(&mut self, token: String) {
        if !self.monitored_tokens.contains(&token) {
            self.monitored_tokens.push(token);
        }
    }

    pub fn get_pools_for_token(&self, token: &str) -> Vec<&Pool> {
        self.pools
            .values()
            .filter(|pool| (pool.token_a == token || pool.token_b == token))
            .collect()
    }

    pub fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            total_pools: self.pools.len(),
            total_liquidity: self.pools
                .values()
                .map(|p| p.liquidity as f64)
                .sum(),
            total_volume_24h: self.pools
                .values()
                .map(|p| p.volume_24h)
                .sum(),
            monitored_tokens: self.monitored_tokens.len(),
        }
    }
}

// Global pool manager instance
use std::sync::{ Arc, Mutex };
use once_cell::sync::Lazy;

pub static POOL_MANAGER: Lazy<Arc<Mutex<Option<PoolManager>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

pub async fn initialize_pool_manager() -> anyhow::Result<()> {
    let manager = PoolManager::new();
    let mut global_manager = POOL_MANAGER.lock().unwrap();
    *global_manager = Some(manager);
    Ok(())
}

pub fn start_pools_manager() {
    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;

        if let Err(e) = initialize_pool_manager().await {
            log("POOLS", LogLevel::Error, &format!("Failed to initialize pool manager: {}", e));
            return;
        }

        log("POOLS", LogLevel::Info, "Pool Manager initialized successfully");

        let delays = crate::global::get_task_delays();

        loop {
            if is_shutdown() {
                log("POOLS", LogLevel::Info, "Pool Manager shutting down...");
                break;
            }

            // Simple pool discovery without holding mutex across await
            let discovery_result = {
                // Just try to discover pools without complex async operations for now
                let manager_guard = POOL_MANAGER.lock().unwrap();
                manager_guard.is_some()
            };

            if discovery_result {
                log("POOLS", LogLevel::Info, "Pool manager is active");
            }

            tokio::time::sleep(Duration::from_secs(delays.pools_delay)).await;
        }
    });
}
