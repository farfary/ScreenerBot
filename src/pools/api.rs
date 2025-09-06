/// Public API for the pools module
///
/// This module provides the clean public interface for the pools system.
/// Only these functions should be used by other modules - all internal
/// implementation details are hidden.

use super::cache;
use super::service;
use super::types::{ PriceResult, PoolError };

/// Get current price for a token
///
/// Returns the most recent price calculation for the specified token.
/// The price includes both USD and SOL values along with confidence metrics.
///
/// # Arguments
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `Some(PriceResult)` - Current price data if available and fresh
/// * `None` - No price available or price is stale
pub fn get_pool_price(mint: &str) -> Option<PriceResult> {
    if !service::is_pool_service_running() {
        return None;
    }

    cache::get_price(mint)
}

/// Get list of tokens with available prices
///
/// Returns all tokens that currently have fresh price data available.
/// Only tokens with prices newer than the configured TTL are included.
///
/// # Returns
/// * `Vec<String>` - List of token mint addresses with available prices
pub fn get_available_tokens() -> Vec<String> {
    if !service::is_pool_service_running() {
        return Vec::new();
    }

    cache::get_available_tokens()
}

/// Get price history for a token
///
/// Returns the complete price history for a token, up to the configured
/// maximum number of entries (typically 1000 most recent prices).
///
/// # Arguments
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `Vec<PriceResult>` - Price history ordered from oldest to newest
pub fn get_price_history(mint: &str) -> Vec<PriceResult> {
    if !service::is_pool_service_running() {
        return Vec::new();
    }

    cache::get_price_history(mint)
}

/// Check if a token has a current price available
///
/// This is a convenience function to quickly check price availability
/// without retrieving the actual price data.
///
/// # Arguments
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `true` - Price is available and fresh
/// * `false` - No price available or price is stale
pub fn has_current_price(mint: &str) -> bool {
    get_pool_price(mint).is_some()
}

/// Get cache statistics for monitoring
///
/// Returns statistics about the current state of the price cache system.
/// Useful for monitoring and debugging the pool service.
///
/// # Returns
/// * `CacheStats` - Current cache statistics
pub fn get_cache_stats() -> cache::CacheStats {
    cache::get_cache_stats()
}
