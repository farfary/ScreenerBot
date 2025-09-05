/// Main pool service
/// Orchestrates pool discovery, calculation, and caching

use crate::pools::{PoolDiscovery, PoolCalculator};
use crate::pools::calculator::PriceResult;
use std::sync::OnceLock;

/// Main pool service
pub struct PoolService {
    discovery: PoolDiscovery,
    calculator: PoolCalculator,
}

impl PoolService {
    pub fn new() -> Self {
        Self {
            discovery: PoolDiscovery::new(),
            calculator: PoolCalculator::new(),
        }
    }

    /// Get price for a token
    pub async fn get_price(&self, _token_address: &str) -> Option<PriceResult> {
        // TODO: Implement price fetching
        // 1. Check cache
        // 2. Discover pools
        // 3. Calculate price
        // 4. Cache result
        None
    }

    /// Get prices for multiple tokens
    pub async fn get_batch_prices(&self, _tokens: &[String]) -> Vec<Option<PriceResult>> {
        // TODO: Implement batch price fetching
        Vec::new()
    }
}

// Global singleton
static POOL_SERVICE: OnceLock<PoolService> = OnceLock::new();

/// Initialize the global pool service
pub fn init_pool_service() -> &'static PoolService {
    POOL_SERVICE.get_or_init(|| PoolService::new())
}

/// Get the global pool service
pub fn get_pool_service() -> &'static PoolService {
    POOL_SERVICE.get().expect("Pool service not initialized")
}
