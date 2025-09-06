/// Pool Price System
///
/// This module provides pool-based price calculation system for Solana DeFi pools.
/// Supports multiple DEX protocols with unified interface.

use std::sync::Arc;

pub mod constants;
pub mod discovery;
pub mod calculator;
pub mod service;
pub mod decoders;
pub mod tokens;
pub mod cache;
pub mod types;

// Re-export main components
pub use constants::*;
pub use types::{PriceResult, PoolStats};
pub use service::{
    PoolService,
    init_pool_service,
    get_pool_service,
    start_pool_service,
    stop_pool_service,
};
pub use calculator::PoolCalculator;
pub use discovery::PoolDiscovery;
pub use tokens::{ PoolTokenManager, PoolToken };
pub use cache::{ PoolCache, CacheStats };

// Convenience functions for easy access
pub async fn init_pools() -> &'static Arc<PoolService> {
    init_pool_service().await
}

pub async fn get_pools() -> &'static Arc<PoolService> {
    get_pool_service().await
}
