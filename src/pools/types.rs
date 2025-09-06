/// Core types for the pools module
/// Simple, SOL-focused structures without complexity

use chrono::{ DateTime, Utc };

/// Price result with SOL focus and pool information
#[derive(Debug, Clone)]
pub struct PriceResult {
    /// Token mint address
    pub token_mint: String,
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
            token_mint: String::new(),
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

/// Price request options for configuring price requests
#[derive(Debug, Clone)]
pub struct PriceOptions {
    /// Include pool price calculation
    pub include_pool: bool,
    /// Include API price lookup
    pub include_api: bool,
    /// Allow cached results (respects cache TTL)
    pub allow_cache: bool,
    /// Force fresh calculation (bypass cache)
    pub force_refresh: bool,
    /// Timeout for the entire operation (seconds)
    pub timeout_secs: Option<u64>,
}

impl Default for PriceOptions {
    fn default() -> Self {
        Self {
            include_pool: true,
            include_api: true,
            allow_cache: true,
            force_refresh: false,
            timeout_secs: Some(10),
        }
    }
}

impl PriceOptions {
    /// Create options for pool only
    pub fn pool_only() -> Self {
        Self {
            include_pool: true,
            include_api: false,
            ..Default::default()
        }
    }

    /// Create options for API only
    pub fn api_only() -> Self {
        Self {
            include_pool: false,
            include_api: true,
            ..Default::default()
        }
    }

    /// Create options with forced refresh
    pub fn force_refresh() -> Self {
        Self {
            force_refresh: true,
            allow_cache: false,
            ..Default::default()
        }
    }
}

/// Token price information structure for entry analysis
#[derive(Debug, Clone)]
pub struct TokenPriceInfo {
    /// Token mint address
    pub token_mint: String,
    /// Price in SOL from pool calculation
    pub pool_price_sol: Option<f64>,
    /// Price in SOL from API sources
    pub api_price_sol: Option<f64>,
    /// SOL reserves in the pool
    pub reserve_sol: Option<f64>,
    /// Token reserves in the pool
    pub reserve_token: Option<f64>,
    /// Pool address
    pub pool_address: Option<String>,
    /// When this info was created
    pub updated_at: DateTime<Utc>,
}

impl TokenPriceInfo {
    /// Create new token price info
    pub fn new(token_mint: String) -> Self {
        Self {
            token_mint,
            pool_price_sol: None,
            api_price_sol: None,
            reserve_sol: None,
            reserve_token: None,
            pool_address: None,
            updated_at: Utc::now(),
        }
    }

    /// Create from pool service price result
    pub fn from_price_result(token_mint: String, price_result: PriceResult) -> Self {
        Self {
            token_mint,
            pool_price_sol: Some(price_result.price_sol),
            api_price_sol: None,
            reserve_sol: Some(price_result.sol_reserves),
            reserve_token: Some(price_result.token_reserves),
            pool_address: Some(price_result.pool_address),
            updated_at: price_result.updated_at,
        }
    }

    /// Get the best available price (pool first, then API)
    pub fn get_best_price(&self) -> Option<f64> {
        self.pool_price_sol.or(self.api_price_sol)
    }
}
