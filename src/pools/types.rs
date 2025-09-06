/// Core types for the pools module
/// Simple, SOL-focused structures without complexity

use chrono::{ DateTime, Utc };
use crate::pools::constants::{ DEFAULT_CONFIDENCE, MIN_CONFIDENCE_THRESHOLD, LIQUIDITY_MULTIPLIER };

/// Price result with SOL focus and pool information
#[derive(Debug, Clone)]
pub struct PriceResult {
    /// Token mint address
    pub token_mint: String,
    /// Price in SOL
    pub price_sol: f64,
    /// Price in USD (not used for trading, only for display/sorting)
    pub price_usd: Option<f64>,
    /// SOL reserves in the pool
    pub sol_reserves: f64,
    /// Token reserves in the pool
    pub token_reserves: f64,
    /// Pool address where price was calculated
    pub pool_address: String,
    /// Program ID of the pool
    pub program_id: String,
    /// Whether the price is available/reliable
    pub available: bool,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
    /// When this price was calculated
    pub updated_at: DateTime<Utc>,
}

impl PriceResult {
    /// Create a new price result
    pub fn new(
        token_mint: String,
        price_sol: f64,
        sol_reserves: f64,
        token_reserves: f64,
        pool_address: String,
        program_id: String
    ) -> Self {
        Self {
            token_mint,
            price_sol,
            price_usd: None, // USD price not used for trading
            sol_reserves,
            token_reserves,
            pool_address,
            program_id,
            available: true,
            confidence: DEFAULT_CONFIDENCE,
            updated_at: Utc::now(),
        }
    }

    /// Create an unavailable price result
    pub fn unavailable() -> Self {
        Self {
            token_mint: String::new(),
            price_sol: 0.0,
            price_usd: None,
            sol_reserves: 0.0,
            token_reserves: 0.0,
            pool_address: String::new(),
            program_id: String::new(),
            available: false,
            confidence: 0.0,
            updated_at: Utc::now(),
        }
    }

    /// Check if price is valid and available
    pub fn is_valid(&self) -> bool {
        self.available && self.price_sol > 0.0 && self.confidence > MIN_CONFIDENCE_THRESHOLD
    }

    /// Get liquidity in SOL (total SOL reserves * 2 for estimation)
    pub fn liquidity_sol(&self) -> f64 {
        self.sol_reserves * LIQUIDITY_MULTIPLIER
    }

    /// Set USD price (for display/sorting purposes only, not trading)
    pub fn with_usd_price(mut self, price_usd: Option<f64>) -> Self {
        self.price_usd = price_usd;
        self
    }
}

impl Default for PriceResult {
    fn default() -> Self {
        Self::unavailable()
    }
}

/// Pool statistics for dashboard/monitoring
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total number of cached pools
    pub total_pools: usize,
    /// Number of tokens with cached data
    pub cached_tokens: usize,
    /// Number of active discovery tasks
    pub active_discoveries: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Successful price fetches
    pub successful_price_fetches: usize,
    /// Failed price fetches
    pub failed_price_fetches: usize,
    /// Cache hits
    pub cache_hits: usize,
    /// Cache misses
    pub cache_misses: usize,
    /// Total tokens available
    pub total_tokens_available: usize,
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl PoolStats {
    /// Create new pool stats
    pub fn new(
        total_pools: usize,
        cached_tokens: usize,
        active_discoveries: usize,
        cache_hit_rate: f64
    ) -> Self {
        Self {
            total_pools,
            cached_tokens,
            active_discoveries,
            cache_hit_rate,
            successful_price_fetches: 0,
            failed_price_fetches: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_tokens_available: 0,
            updated_at: Utc::now(),
        }
    }
}

impl Default for PoolStats {
    fn default() -> Self {
        Self {
            total_pools: 0,
            cached_tokens: 0,
            active_discoveries: 0,
            cache_hit_rate: 0.0,
            successful_price_fetches: 0,
            failed_price_fetches: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_tokens_available: 0,
            updated_at: Utc::now(),
        }
    }
}

/// Pool information for internal use in pools module
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: String,
    pub pool_program_id: String,
    pub pool_type: String,
    pub token_0_mint: String,
    pub token_1_mint: String,
    pub token_0_reserve: u64,
    pub token_1_reserve: u64,
    pub token_0_decimals: u8,
    pub token_1_decimals: u8,
    pub liquidity_usd: Option<f64>,
    // Additional fields for calculator compatibility
    pub sol_reserve: f64,
    pub token_reserve: f64,
    pub program_id: String,
}

impl PoolInfo {
    pub fn new(
        pool_address: String,
        pool_program_id: String,
        pool_type: String,
        token_0_mint: String,
        token_1_mint: String,
        token_0_reserve: u64,
        token_1_reserve: u64,
        token_0_decimals: u8,
        token_1_decimals: u8,
        liquidity_usd: Option<f64>
    ) -> Self {
        // Convert reserves to f64 for calculator compatibility
        let sol_reserve = if token_0_mint == crate::pools::constants::SOL_MINT {
            (token_0_reserve as f64) / (10_f64).powi(9) // SOL has 9 decimals
        } else {
            (token_1_reserve as f64) / (10_f64).powi(9) // Assume quote token is SOL
        };

        let token_reserve = if token_0_mint == crate::pools::constants::SOL_MINT {
            (token_1_reserve as f64) / (10_f64).powi(token_1_decimals as i32)
        } else {
            (token_0_reserve as f64) / (10_f64).powi(token_0_decimals as i32)
        };

        Self {
            pool_address: pool_address.clone(),
            pool_program_id: pool_program_id.clone(),
            pool_type,
            token_0_mint,
            token_1_mint,
            token_0_reserve,
            token_1_reserve,
            token_0_decimals,
            token_1_decimals,
            liquidity_usd,
            sol_reserve,
            token_reserve,
            program_id: pool_program_id,
        }
    }
}
