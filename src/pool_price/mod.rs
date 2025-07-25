/// Pool Price System - Main Module
///
/// This is the main entry point for the new pool price system. It provides:
/// - Simple interface: get_token_price(mint) -> Option<f64>
/// - Background task for monitoring open positions
/// - Rate limiting and caching
/// - Full pipeline: Discovery -> Decoding -> Calculation

pub mod types;
pub mod discovery;
pub mod decoder;
pub mod calculator;

// Re-export main types and functions
pub use types::*;
pub use discovery::{
    get_pool_addresses_for_token,
    preload_pools_for_tokens,
    cleanup_pool_cache,
    get_pool_cache_stats,
};
pub use decoder::fetch_and_decode_pools;
pub use calculator::{ calculate_token_price_from_pools, calculate_and_validate_price };

use crate::logger::{ log, LogTag };
use crate::positions::SAVED_POSITIONS;
use crate::utils::check_shutdown_or_delay;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };
use tokio::sync::Notify;
use once_cell::sync::Lazy;

// =============================================================================
// GLOBAL PRICE CACHE
// =============================================================================

/// Cache for calculated token prices (short-lived, ~30 seconds)
static PRICE_CACHE: Lazy<Mutex<HashMap<String, (f64, Instant)>>> = Lazy::new(||
    Mutex::new(HashMap::new())
);

/// Cache TTL for calculated prices (30 seconds)
const PRICE_CACHE_TTL_SECS: u64 = 30;

// =============================================================================
// MAIN INTERFACE
// =============================================================================

/// Get token price - main entry point for the pool price system
///
/// This function provides a simple interface that handles the full pipeline:
/// 1. Check cache for recent price
/// 2. Discover pool addresses via DexScreener API
/// 3. Fetch pool account data via Solana RPC
/// 4. Decode pool reserves based on program ID
/// 5. Calculate weighted average price
/// 6. Cache and return result
pub async fn get_token_price(mint: &str) -> Option<f64> {
    // Check cache first
    if let Some(cached_price) = get_cached_price(mint) {
        log(
            LogTag::Pool,
            "CACHE",
            &format!("Using cached price for {}: {:.12} SOL", mint, cached_price)
        );
        return Some(cached_price);
    }

    log(LogTag::Pool, "REQUEST", &format!("Calculating fresh price for token {}", mint));

    // Run the full pipeline
    match run_price_calculation_pipeline(mint).await {
        Ok(Some(calculated_price)) => {
            let price = calculated_price.price_sol;

            // Cache the result
            cache_price(mint, price);

            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Calculated price for {}: {:.12} SOL (confidence: {:.2})",
                    mint,
                    price,
                    calculated_price.confidence
                )
            );

            Some(price)
        }
        Ok(None) => {
            log(LogTag::Pool, "WARN", &format!("No price calculated for token {}", mint));
            None
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Price calculation failed for {}: {}", mint, e));
            None
        }
    }
}

/// Run the full price calculation pipeline
async fn run_price_calculation_pipeline(mint: &str) -> PoolPriceResult<Option<CalculatedPrice>> {
    // Step 1: Discover pool addresses
    log(LogTag::Pool, "STEP1", &format!("Discovering pools for {}", mint));
    let pool_addresses = discovery::get_pool_addresses_for_token(mint).await?;

    if pool_addresses.is_empty() {
        log(LogTag::Pool, "INFO", &format!("No pools found for token {}", mint));
        return Ok(None);
    }

    log(LogTag::Pool, "STEP1", &format!("Found {} pools for {}", pool_addresses.len(), mint));

    // Step 2: Fetch and decode pool data
    log(LogTag::Pool, "STEP2", &format!("Fetching and decoding {} pools", pool_addresses.len()));
    let decoded_pools = decoder::fetch_and_decode_pools(&pool_addresses).await?;

    // Step 3: Calculate price
    log(LogTag::Pool, "STEP3", &format!("Calculating price from decoded pools"));
    let calculated_price = calculator::PriceCalculator::calculate_token_price(
        mint,
        &decoded_pools
    )?;

    Ok(calculated_price)
}

// =============================================================================
// PRICE CACHING
// =============================================================================

/// Get cached price if still valid
fn get_cached_price(mint: &str) -> Option<f64> {
    if let Ok(cache) = PRICE_CACHE.lock() {
        if let Some((price, timestamp)) = cache.get(mint) {
            let age = timestamp.elapsed().as_secs();
            if age < PRICE_CACHE_TTL_SECS {
                return Some(*price);
            }
        }
    }
    None
}

/// Cache a calculated price
fn cache_price(mint: &str, price: f64) {
    if let Ok(mut cache) = PRICE_CACHE.lock() {
        cache.insert(mint.to_string(), (price, Instant::now()));

        // Cleanup old entries while we have the lock
        let now = Instant::now();
        cache.retain(
            |_, (_, timestamp)| now.duration_since(*timestamp).as_secs() < PRICE_CACHE_TTL_SECS
        );
    }
}

/// Get cache statistics
pub fn get_price_cache_stats() -> (usize, usize) {
    if let Ok(cache) = PRICE_CACHE.lock() {
        let total_entries = cache.len();
        let now = Instant::now();
        let valid_entries = cache
            .values()
            .filter(
                |(_, timestamp)| now.duration_since(*timestamp).as_secs() < PRICE_CACHE_TTL_SECS
            )
            .count();
        (total_entries, valid_entries)
    } else {
        (0, 0)
    }
}

/// Clear the price cache
pub fn clear_price_cache() {
    if let Ok(mut cache) = PRICE_CACHE.lock() {
        cache.clear();
        log(LogTag::Pool, "CACHE", "Cleared price cache");
    }
}

// =============================================================================
// BACKGROUND TASK FOR OPEN POSITIONS
// =============================================================================

/// Start pool price monitoring task for open positions only
///
/// This task runs in the background and continuously monitors prices for tokens
/// that have open positions. It preloads pool addresses and updates prices
/// according to the specified interval.
pub async fn start_pool_price_monitor(shutdown: Arc<Notify>) {
    log(LogTag::Pool, "STARTUP", "Starting pool price monitor for open positions");

    loop {
        // Check for shutdown signal
        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(POSITION_MONITORING_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Pool, "SHUTDOWN", "Pool price monitor shutting down");
            break;
        }

        // Get mints for open positions
        let open_position_mints = get_open_position_mints();

        if open_position_mints.is_empty() {
            log(LogTag::Pool, "INFO", "No open positions to monitor");
            continue;
        }

        log(
            LogTag::Pool,
            "MONITOR",
            &format!("Monitoring prices for {} open positions", open_position_mints.len())
        );

        // Update prices for open positions
        for mint in &open_position_mints {
            // This will trigger the full pipeline and cache the result
            match get_token_price(mint).await {
                Some(price) => {
                    log(
                        LogTag::Pool,
                        "UPDATE",
                        &format!("Updated price for {}: {:.12} SOL", mint, price)
                    );
                }
                None => {
                    log(LogTag::Pool, "WARN", &format!("Failed to update price for {}", mint));
                }
            }

            // Small delay between tokens to respect rate limits
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Cleanup expired caches
        cleanup_caches();
    }
}

/// Get list of mints for currently open positions
fn get_open_position_mints() -> Vec<String> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|pos| pos.exit_price.is_none())
            .map(|pos| pos.mint.clone())
            .collect()
    } else {
        Vec::new()
    }
}

/// Cleanup expired caches
fn cleanup_caches() {
    // Cleanup price cache (happens automatically in cache_price)
    let (total, valid) = get_price_cache_stats();
    if total > valid {
        log(LogTag::Pool, "CACHE", &format!("Price cache: {}/{} entries valid", valid, total));
    }

    // Cleanup pool address cache
    cleanup_pool_cache();
    let (total, valid) = get_pool_cache_stats();
    if total > 0 {
        log(
            LogTag::Pool,
            "CACHE",
            &format!("Pool address cache: {}/{} entries valid", valid, total)
        );
    }
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Preload prices for multiple tokens (batch operation)
pub async fn preload_token_prices(mints: &[String]) -> PoolPriceResult<usize> {
    log(LogTag::Pool, "BATCH", &format!("Preloading prices for {} tokens", mints.len()));

    let mut successful_loads = 0;

    for mint in mints {
        match get_token_price(mint).await {
            Some(_) => {
                successful_loads += 1;
            }
            None => {
                log(LogTag::Pool, "WARN", &format!("Failed to preload price for {}", mint));
            }
        }

        // Delay between requests to respect rate limits
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    log(
        LogTag::Pool,
        "BATCH",
        &format!("Successfully preloaded {}/{} token prices", successful_loads, mints.len())
    );
    Ok(successful_loads)
}

/// Get detailed price information (for debugging)
pub async fn get_detailed_price_info(mint: &str) -> PoolPriceResult<Option<CalculatedPrice>> {
    log(LogTag::Pool, "DEBUG", &format!("Getting detailed price info for {}", mint));
    run_price_calculation_pipeline(mint).await
}

/// Validate pool price system health
pub async fn validate_system_health() -> PoolPriceResult<bool> {
    log(LogTag::Pool, "HEALTH", "Validating pool price system health");

    // Test with SOL (should always have pools)
    let test_result = get_token_price(SOL_MINT).await;

    let is_healthy = test_result.is_some();

    if is_healthy {
        log(LogTag::Pool, "HEALTH", "Pool price system is healthy");
    } else {
        log(LogTag::Pool, "ERROR", "Pool price system health check failed");
    }

    Ok(is_healthy)
}
