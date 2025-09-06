/// Core types for the pools module
/// Simple, SOL-focused structures without complexity

use chrono::{DateTime, Utc};

/// Price result with SOL focus and pool information
#[derive(Debug, Clone)]
pub struct PriceResult {
    /// Price in SOL
    pub price_sol: f64,
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
        price_sol: f64,
        sol_reserves: f64,
        token_reserves: f64,
        pool_address: String,
        program_id: String,
    ) -> Self {
        Self {
            price_sol,
            sol_reserves,
            token_reserves,
            pool_address,
            program_id,
            available: true,
            confidence: 1.0,
            updated_at: Utc::now(),
        }
    }

    /// Create an unavailable price result
    pub fn unavailable() -> Self {
        Self {
            price_sol: 0.0,
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
        self.available && self.price_sol > 0.0 && self.confidence > 0.5
    }

    /// Get liquidity in SOL (total SOL reserves * 2 for estimation)
    pub fn liquidity_sol(&self) -> f64 {
        self.sol_reserves * 2.0
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
    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl PoolStats {
    /// Create new pool stats
    pub fn new(
        total_pools: usize,
        cached_tokens: usize,
        active_discoveries: usize,
        cache_hit_rate: f64,
    ) -> Self {
        Self {
            total_pools,
            cached_tokens,
            active_discoveries,
            cache_hit_rate,
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
            updated_at: Utc::now(),
        }
    }
}
