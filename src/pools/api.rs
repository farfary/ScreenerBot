/// Public API for the pools module
///
/// This module provides the clean public interface for the pools system.
/// Only these functions should be used by other modules - all internal
/// implementation details are hidden.
use super::cache;
use super::db;
use super::service;
use super::types::{PoolDescriptor, PoolError, PriceResult};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

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

/// Get pools associated with a token from the analyzer's in-memory directory.
/// Returns a single canonical pool first (if present) followed by other pools.
pub fn get_token_pools(mint: &str) -> Vec<PoolDescriptor> {
    if !service::is_pool_service_running() {
        return Vec::new();
    }

    let analyzer = match service::get_pool_analyzer() {
        Some(analyzer) => analyzer,
        None => return Vec::new(),
    };

    let mint_pubkey = match Pubkey::from_str(mint) {
        Ok(key) => key,
        Err(_) => return Vec::new(),
    };

    let mut pools = analyzer.get_pools_for_token(&mint_pubkey);

    if let Some(canonical) = analyzer.get_canonical_pool(&mint_pubkey) {
        if let Some(position) = pools
            .iter()
            .position(|pool| pool.pool_id == canonical.pool_id)
        {
            if position != 0 {
                let canonical_pool = pools.remove(position);
                pools.insert(0, canonical_pool);
            }
        }
    }

    pools
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
