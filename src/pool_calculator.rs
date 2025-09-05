use crate::logger::{ log, LogTag };
use crate::pool_interface::TokenPriceInfo;
use crate::pool_constants::*;
use crate::global::is_debug_pool_calculator_enabled;
use async_trait::async_trait;
use chrono::{ DateTime, Utc };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Pool price information structure
#[derive(Debug, Clone)]
pub struct PoolPriceInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_mint: String,
    pub price_sol: f64,
    pub token_reserve: u64,
    pub sol_reserve: u64,
    pub token_decimals: u8,
    pub sol_decimals: u8,
}

/// Pool information structure
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_vault: Option<String>,
    pub token_1_vault: Option<String>,
    pub token_0_reserve: u64,
    pub token_1_reserve: u64,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    pub lp_mint: Option<String>,
    pub lp_supply: Option<u64>,
    pub creator: Option<String>,
    pub status: Option<u32>,
    pub liquidity_usd: Option<f64>,
    /// sqrt_price for concentrated liquidity pools (Orca Whirlpool)
    pub sqrt_price: Option<u128>,
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub calculations_attempted: u64,
    pub calculations_successful: u64,
    pub calculations_failed: u64,
    pub cache_hits: u64,
    pub average_calculation_time_ms: f64,
    pub pools_by_program: HashMap<String, u64>,
}

impl PoolStats {
    pub fn new() -> Self {
        Self {
            calculations_attempted: 0,
            calculations_successful: 0,
            calculations_failed: 0,
            cache_hits: 0,
            average_calculation_time_ms: 0.0,
            pools_by_program: HashMap::new(),
        }
    }

    pub fn record_calculation(&mut self, success: bool, time_ms: f64, program_id: &str) {
        self.calculations_attempted += 1;
        if success {
            self.calculations_successful += 1;
        } else {
            self.calculations_failed += 1;
        }

        // Update average calculation time
        let total_time =
            self.average_calculation_time_ms * ((self.calculations_attempted - 1) as f64);
        self.average_calculation_time_ms =
            (total_time + time_ms) / (self.calculations_attempted as f64);

        // Track pools by program
        *self.pools_by_program.entry(program_id.to_string()).or_insert(0) += 1;
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.calculations_attempted == 0 {
            0.0
        } else {
            ((self.calculations_successful as f64) / (self.calculations_attempted as f64)) * 100.0
        }
    }
}

/// Pool price calculator service
pub struct PoolCalculator {
    stats: Arc<RwLock<PoolStats>>,
    debug_enabled: bool,
}

// =============================================================================
// IMPLEMENTATIONS
// =============================================================================

impl PoolCalculator {
    /// Create new pool calculator
    pub fn new() -> Self {
        let debug_enabled = is_debug_pool_calculator_enabled();

        if debug_enabled {
            log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled");
        }

        Self {
            stats: Arc::new(RwLock::new(PoolStats::new())),
            debug_enabled,
        }
    }

    /// Enable debug mode
    pub fn enable_debug(&mut self) {
        self.debug_enabled = true;
        log(LogTag::Pool, "DEBUG", "Pool calculator debug mode enabled (overridden)");
    }

    /// Calculate token price from pool information
    pub async fn calculate_token_price(
        &self,
        pool_info: &PoolInfo,
        token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        let start_time = Instant::now();

        // Calculate price based on pool type
        let price_info = match pool_info.pool_program_id.as_str() {
            RAYDIUM_CPMM_PROGRAM_ID => {
                self.calculate_raydium_cpmm_price(pool_info, token_mint).await?
            }
            RAYDIUM_LEGACY_AMM_PROGRAM_ID => {
                self.calculate_raydium_legacy_amm_price(pool_info, token_mint).await?
            }
            RAYDIUM_CLMM_PROGRAM_ID => {
                self.calculate_raydium_clmm_price(pool_info, token_mint).await?
            }
            METEORA_DAMM_V2_PROGRAM_ID => {
                self.calculate_meteora_damm_v2_price(pool_info, token_mint).await?
            }
            METEORA_DLMM_PROGRAM_ID => {
                self.calculate_meteora_dlmm_price(pool_info, token_mint).await?
            }
            ORCA_WHIRLPOOL_PROGRAM_ID => {
                self.calculate_orca_whirlpool_price(pool_info, token_mint).await?
            }
            PUMP_FUN_AMM_PROGRAM_ID => {
                self.calculate_pump_fun_amm_price(pool_info, token_mint).await?
            }
            _ => {
                return Err(
                    format!(
                        "Price calculation not supported for program: {}",
                        pool_info.pool_program_id
                    )
                );
            }
        };

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.record_calculation(
                price_info.is_some(),
                start_time.elapsed().as_millis() as f64,
                &pool_info.pool_program_id
            );
        }

        if self.debug_enabled && price_info.is_some() {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Price calculated: {:.12} SOL for {} in {:.2}ms",
                    price_info.as_ref().unwrap().price_sol,
                    token_mint,
                    start_time.elapsed().as_millis()
                )
            );
        }

        Ok(price_info)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }

    // =============================================================================
    // PRIVATE METHODS
    // =============================================================================

    // Placeholder implementations for price calculators
    // These would contain the actual price calculation logic from pool_old.rs

    async fn calculate_raydium_cpmm_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Raydium CPMM price calculation
        Ok(None)
    }

    async fn calculate_raydium_legacy_amm_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Raydium Legacy AMM price calculation
        Ok(None)
    }

    async fn calculate_raydium_clmm_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Raydium CLMM price calculation
        Ok(None)
    }

    async fn calculate_meteora_damm_v2_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Meteora DAMM v2 price calculation
        Ok(None)
    }

    async fn calculate_meteora_dlmm_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Meteora DLMM price calculation
        Ok(None)
    }

    async fn calculate_orca_whirlpool_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Orca Whirlpool price calculation
        Ok(None)
    }

    async fn calculate_pump_fun_amm_price(
        &self,
        _pool_info: &PoolInfo,
        _token_mint: &str
    ) -> Result<Option<PoolPriceInfo>, String> {
        // TODO: Implement Pump Fun AMM price calculation
        Ok(None)
    }
}

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

use std::sync::OnceLock;

static POOL_CALCULATOR: OnceLock<PoolCalculator> = OnceLock::new();

/// Initialize the global pool calculator instance
pub fn init_pool_calculator() -> &'static PoolCalculator {
    POOL_CALCULATOR.get_or_init(|| {
        log(LogTag::Pool, "INIT", "🏗️ Initializing Pool Calculator");
        PoolCalculator::new()
    })
}

/// Get the global pool calculator instance
pub fn get_pool_calculator() -> &'static PoolCalculator {
    POOL_CALCULATOR.get().expect("Pool calculator not initialized")
}
