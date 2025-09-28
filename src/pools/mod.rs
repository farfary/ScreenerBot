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

mod api;
mod cache;
mod db; // Database module for price history persistence
mod service;
pub mod types; // Make types public
pub mod utils; // Utility functions for SOL detection and vault pairing

// Re-export only the public API
pub use api::{
    check_price_history_quality, get_available_tokens, get_cache_stats, get_extended_price_history,
    get_pool_price, get_price_history, get_price_history_stats, load_token_history_into_cache,
    PriceHistoryStats,
};
pub use discovery::{
    get_canonical_pool_address, PoolDiscovery, ENABLE_DEXSCREENER_DISCOVERY,
    ENABLE_GECKOTERMINAL_DISCOVERY, ENABLE_RAYDIUM_DISCOVERY,
};
pub use service::{
    get_debug_token_override, is_pool_service_running, set_debug_token_override,
    start_pool_service, stop_pool_service,
};
pub use types::{PoolError, PriceResult}; // Expose for configuration access

// Internal modules (not exposed)
mod analyzer;
pub mod calculator; // Public for debug tooling
pub mod decoders;
pub mod discovery;
pub mod fetcher; // Public for debug tooling // Temporarily public for debug tooling (consider gating with feature flag)

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
    shutdown: Arc<Notify>,
) -> Result<tokio::task::JoinHandle<()>, PoolError> {
    start_pool_service().await?;

    // Return a dummy handle for now - this will be improved later
    let handle = tokio::spawn(async move {
        shutdown.notified().await;
    });

    Ok(handle)
}
