use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Main interface for the pool service
/// This provides the core functions that the trading system needs
#[async_trait::async_trait]
pub trait PoolInterface {
    /// Get current price for a token with full pool and API information
    /// Returns comprehensive price data including pool details and API price
    async fn get_price(&self, token_address: &str) -> Option<TokenPriceInfo>;

    /// Get price history for a token
    /// Returns vector of (timestamp, price) tuples
    async fn get_price_history(&self, token_address: &str) -> Vec<(DateTime<Utc>, f64)>;

    /// Get list of tokens with available prices for trading
    /// Returns token addresses that have active pools and recent price data
    async fn get_available_tokens(&self) -> Vec<String>;

    /// Get batch prices for multiple tokens
    /// Returns HashMap of token_address -> TokenPriceInfo
    async fn get_batch_prices(&self, token_addresses: &[String])
        -> HashMap<String, TokenPriceInfo>;
}

/// Comprehensive price information including pool and API data
#[derive(Debug, Clone)]
pub struct TokenPriceInfo {
    /// Token mint address
    pub token_mint: String,
    /// SOL price from pool calculation
    pub pool_price_sol: Option<f64>,
    /// USD price from pool calculation (if available)
    pub pool_price_usd: Option<f64>,
    /// API price in SOL (from external sources like DexScreener, etc.)
    pub api_price_sol: Option<f64>,
    /// API price in USD
    pub api_price_usd: Option<f64>,
    /// Pool address used for calculation
    pub pool_address: Option<String>,
    /// Pool type (Raydium, Orca, etc.)
    pub pool_type: Option<String>,
    /// SOL reserve amount in pool
    pub reserve_sol: Option<f64>,
    /// Token reserve amount in pool
    pub reserve_token: Option<f64>,
    /// Pool liquidity in USD
    pub liquidity_usd: Option<f64>,
    /// 24h volume in USD
    pub volume_24h_usd: Option<f64>,
    /// When this price was calculated
    pub calculated_at: DateTime<Utc>,
    /// Any error that occurred during calculation
    pub error: Option<String>,
}

/// Price data point for history
#[derive(Debug, Clone)]
pub struct PricePoint {
    pub timestamp: DateTime<Utc>,
    pub price_sol: f64,
    pub volume_24h: Option<f64>,
    pub liquidity_usd: Option<f64>,
}

/// Pool service statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub total_tokens_available: usize,
    pub total_pools_tracked: usize,
    pub successful_price_fetches: u64,
    pub failed_price_fetches: u64,
    pub cache_hits: u64,
    pub last_update: Option<DateTime<Utc>>,
    pub background_tasks_running: usize,
    pub total_background_tasks: usize,
}

/// Price result structure for legacy compatibility
#[derive(Debug, Clone)]
pub struct PriceResult {
    pub token_mint: String,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub pool_address: Option<String>,
    pub reserve_sol: Option<f64>,
    pub calculated_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl Default for PriceResult {
    fn default() -> Self {
        Self {
            token_mint: String::new(),
            price_sol: None,
            price_usd: None,
            pool_address: None,
            reserve_sol: None,
            calculated_at: Utc::now(),
            error: None,
        }
    }
}

impl PriceResult {
    /// Get SOL price (legacy compatibility)
    pub fn sol_price(&self) -> Option<f64> {
        self.price_sol
    }
}

/// Convert TokenPriceInfo to legacy PriceResult
impl From<TokenPriceInfo> for PriceResult {
    fn from(mut info: TokenPriceInfo) -> Self {
        Self {
            token_mint: info.token_mint.clone(),
            price_sol: info.pool_price_sol.or(info.api_price_sol),
            price_usd: info.pool_price_usd.or(info.api_price_usd),
            pool_address: info.pool_address,
            reserve_sol: info.reserve_sol,
            calculated_at: info.calculated_at,
            error: info.error,
        }
    }
}

/// Price options for legacy compatibility
#[derive(Debug, Clone, Default)]
pub struct PriceOptions {
    pub warm_on_miss: bool,
}

/// Cached pool information for legacy compatibility
#[derive(Debug, Clone)]
pub struct CachedPoolInfo {
    pub pair_address: String,
    pub dex_id: String,
    pub base_token: String,
    pub quote_token: String,
    pub price_native: f64,
    pub price_usd: f64,
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub created_at: u64,
    pub cached_at: DateTime<Utc>,
}

impl CachedPoolInfo {
    /// Create from token pair (placeholder implementation)
    pub fn from_token_pair(_pair: &str) -> Result<Self, String> {
        Ok(Self {
            pair_address: "placeholder".to_string(),
            dex_id: "unknown".to_string(),
            base_token: "unknown".to_string(),
            quote_token: "unknown".to_string(),
            price_native: 0.0,
            price_usd: 0.0,
            liquidity_usd: 0.0,
            volume_24h: 0.0,
            created_at: 0,
            cached_at: Utc::now(),
        })
    }
}

impl PriceOptions {
    pub fn default() -> Self {
        Self {
            warm_on_miss: false,
        }
    }
}
