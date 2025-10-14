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

/// Get extended price history from database
///
/// Returns price history for a token from the persistent database storage.
/// This can return more historical data than the in-memory cache.
///
/// # Arguments
/// * `mint` - Token mint address as string
/// * `limit` - Optional maximum number of entries to return
/// * `since_timestamp` - Optional Unix timestamp to filter entries newer than this time
///
/// # Returns
/// * `Ok(Vec<PriceResult>)` - Extended price history from database
/// * `Err(String)` - Database error message
pub async fn get_extended_price_history(
    mint: &str,
    limit: Option<usize>,
    since_timestamp: Option<i64>,
) -> Result<Vec<PriceResult>, String> {
    if !service::is_pool_service_running() {
        return Err("Pool service not running".to_string());
    }

    db::get_extended_price_history(mint, limit, since_timestamp).await
}

/// Load historical data for a token from database into cache
///
/// This function loads historical price data from the database into the
/// in-memory cache for faster subsequent access.
///
/// # Arguments
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `Ok(())` - Successfully loaded historical data
/// * `Err(String)` - Error loading data
pub async fn load_token_history_into_cache(mint: &str) -> Result<(), String> {
    if !service::is_pool_service_running() {
        return Err("Pool service not running".to_string());
    }

    cache::load_token_history_from_database(mint).await
}

/// Check if price history is available and sufficient for analysis
///
/// This function verifies that price history exists, is recent, and has
/// enough data points for meaningful technical analysis.
///
/// # Arguments
/// * `mint` - Token mint address as string
/// * `min_points` - Minimum number of price points required
/// * `max_age_seconds` - Maximum age of newest price in seconds
///
/// # Returns
/// * `Ok(true)` - Price history is available and sufficient
/// * `Ok(false)` - Price history exists but is insufficient
/// * `Err(String)` - Error accessing price history
pub fn check_price_history_quality(
    mint: &str,
    min_points: usize,
    max_age_seconds: u64,
) -> Result<bool, String> {
    if !service::is_pool_service_running() {
        return Err("Pool service not running".to_string());
    }

    let history = cache::get_price_history(mint);

    // Check if we have minimum required points
    if history.len() < min_points {
        return Ok(false);
    }

    // Check if the most recent price is fresh enough
    if let Some(latest_price) = history.last() {
        if !latest_price.is_fresh(max_age_seconds) {
            return Ok(false);
        }
    } else {
        return Ok(false);
    }

    // Check for data continuity (no major gaps)
    if history.len() > 1 {
        let mut has_major_gaps = false;
        for i in 1..history.len() {
            let prev_time = history[i - 1].get_utc_timestamp();
            let curr_time = history[i].get_utc_timestamp();
            let gap = (curr_time - prev_time).num_seconds().abs();

            // If gap is larger than 2 minutes, consider it significant
            if gap > 120 {
                has_major_gaps = true;
                break;
            }
        }

        if has_major_gaps {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Get price history statistics for monitoring and debugging
///
/// Returns detailed information about the quality and characteristics
/// of price history for a specific token.
///
/// # Arguments
/// * `mint` - Token mint address as string
///
/// # Returns
/// * `Ok(PriceHistoryStats)` - Detailed statistics about price history
/// * `Err(String)` - Error accessing price history
pub fn get_price_history_stats(mint: &str) -> Result<PriceHistoryStats, String> {
    if !service::is_pool_service_running() {
        return Err("Pool service not running".to_string());
    }

    let history = cache::get_price_history(mint);

    if history.is_empty() {
        return Ok(PriceHistoryStats {
            total_points: 0,
            age_newest_seconds: 0,
            age_oldest_seconds: 0,
            has_major_gaps: false,
            largest_gap_seconds: 0,
            average_interval_seconds: 0.0,
            price_range_sol: (0.0, 0.0),
        });
    }

    let newest_time = history.last().unwrap().get_utc_timestamp();
    let oldest_time = history.first().unwrap().get_utc_timestamp();
    let now = chrono::Utc::now();

    let age_newest_seconds = (now - newest_time).num_seconds().max(0) as u64;
    let age_oldest_seconds = (now - oldest_time).num_seconds().max(0) as u64;

    // Calculate gaps and intervals
    let mut largest_gap_seconds = 0i64;
    let mut total_intervals = 0i64;
    let mut has_major_gaps = false;

    if history.len() > 1 {
        for i in 1..history.len() {
            let prev_time = history[i - 1].get_utc_timestamp();
            let curr_time = history[i].get_utc_timestamp();
            let interval = (curr_time - prev_time).num_seconds().abs();

            total_intervals += interval;
            largest_gap_seconds = largest_gap_seconds.max(interval);

            if interval > 120 {
                // 2 minutes
                has_major_gaps = true;
            }
        }
    }

    let average_interval_seconds = if history.len() > 1 {
        (total_intervals as f64) / ((history.len() - 1) as f64)
    } else {
        0.0
    };

    // Calculate price range
    let prices: Vec<f64> = history.iter().map(|p| p.price_sol).collect();
    let min_price = prices.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_price = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    Ok(PriceHistoryStats {
        total_points: history.len(),
        age_newest_seconds,
        age_oldest_seconds,
        has_major_gaps,
        largest_gap_seconds: largest_gap_seconds as u64,
        average_interval_seconds,
        price_range_sol: (min_price, max_price),
    })
}

/// Statistics about price history quality and characteristics
#[derive(Debug, Clone)]
pub struct PriceHistoryStats {
    pub total_points: usize,
    pub age_newest_seconds: u64,
    pub age_oldest_seconds: u64,
    pub has_major_gaps: bool,
    pub largest_gap_seconds: u64,
    pub average_interval_seconds: f64,
    pub price_range_sol: (f64, f64), // (min, max)
}
