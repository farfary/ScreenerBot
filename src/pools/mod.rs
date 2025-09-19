/// New modular pool system for real-time price calculations
///
/// This module provides a centralized pool service that watches up to 100+ tokens
/// and provides real-time prices derived from various DEX pools (Raydium, Orca, etc.).
///
/// PUBLIC API (only these functions are exposed):
/// - start_pool_service() -> Initialize the pool service
/// - get_pool_price(mint) -> Get current price for a token
/// - get_available_tokens() -> Get list of tokens with available prices
/// - get_price_history(mint) -> Get price history for a token

use std::sync::Arc;
use tokio::sync::Notify;

mod service;
pub mod types; // Make types public
mod cache;
mod api;
mod db; // Database module for price history persistence
pub mod utils; // Utility functions for SOL detection and vault pairing

// Re-export only the public API
pub use api::{
    get_pool_price,
    get_available_tokens,
    get_price_history,
    get_cache_stats,
    get_extended_price_history,
    load_token_history_into_cache,
};
pub use service::{
    start_pool_service,
    stop_pool_service,
    is_pool_service_running,
    set_debug_token_override,
    get_debug_token_override,
};
pub use types::{ PriceResult, PoolError };
pub use discovery::{
    PoolDiscovery,
    ENABLE_DEXSCREENER_DISCOVERY,
    ENABLE_GECKOTERMINAL_DISCOVERY,
    ENABLE_RAYDIUM_DISCOVERY,
}; // Expose for configuration access
pub use chain_discovery::{ ChainPoolDiscovery, ChainPoolInfo }; // Expose chain discovery for debug tools

// Internal modules (not exposed)
pub mod discovery;
pub mod chain_discovery; // Chain-based pool discovery (bypassing APIs)
mod analyzer;
pub mod fetcher; // Public for debug tooling
pub mod calculator; // Public for debug tooling
pub mod decoders; // Temporarily public for debug tooling (consider gating with feature flag)

// Direct swap module - modular direct swap system
pub mod swap;

// Temporary re-exports for debug tooling (consider gating with a feature flag)
pub use fetcher::AccountData;

/// Initialize the pool service - this is the main entry point
///
/// This function starts all background tasks for pool monitoring and price calculation.
/// It's idempotent and can be called multiple times safely.
///
/// Returns a handle that can be used to monitor the service lifecycle.
pub async fn init_pool_service(
    shutdown: Arc<Notify>
) -> Result<tokio::task::JoinHandle<()>, PoolError> {
    start_pool_service().await?;

    // Return a dummy handle for now - this will be improved later
    let handle = tokio::spawn(async move {
        shutdown.notified().await;
    });

    Ok(handle)
}
