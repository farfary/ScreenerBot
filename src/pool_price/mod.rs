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
use crate::global::{ LIST_TOKENS, Token, read_configs };
use crate::pool_price::discovery::PoolDiscovery;
use crate::pool_price::decoder::PoolDecoder;
use crate::pool_price::calculator::PriceCalculator;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::Duration;
use tokio::sync::Notify;
use once_cell::sync::Lazy;
use chrono::Utc;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Debug logging function (conditional based on debug flag)
fn debug_log(log_type: &str, message: &str) {
    // For pool price debugging, we use regular log with Pool tag
    log(LogTag::Pool, log_type, message);
}

// =============================================================================
// GLOBAL PRICE CACHE
// =============================================================================

/// Cached price with timestamp
#[derive(Clone)]
struct CachedPrice {
    price: f64,
    timestamp: chrono::DateTime<Utc>,
}

/// Cache for calculated token prices (short-lived, ~30 seconds)
static PRICE_CACHE: Lazy<Mutex<HashMap<String, CachedPrice>>> = Lazy::new(||
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
/// 7. Update global token list with validated pool price
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

            // Update the global token list with validated pool price
            update_token_pool_price(mint, price).await;

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
        if let Some(cached) = cache.get(mint) {
            let age_secs = (Utc::now() - cached.timestamp).num_seconds() as u64;
            if age_secs < PRICE_CACHE_TTL_SECS {
                return Some(cached.price);
            }
        }
    }
    None
}

/// Cache a calculated price
fn cache_price(mint: &str, price: f64) {
    if let Ok(mut cache) = PRICE_CACHE.lock() {
        cache.insert(mint.to_string(), CachedPrice {
            price,
            timestamp: Utc::now(),
        });

        // Cleanup old entries while we have the lock
        let now = Utc::now();
        cache.retain(|_, cached| {
            let age_secs = (now - cached.timestamp).num_seconds() as u64;
            age_secs < PRICE_CACHE_TTL_SECS
        });
    }
}

/// Get cache statistics
pub fn get_price_cache_stats() -> (usize, usize) {
    if let Ok(cache) = PRICE_CACHE.lock() {
        let total_entries = cache.len();
        let now = Utc::now();
        let valid_entries = cache
            .values()
            .filter(|cached| {
                let age_secs = (now - cached.timestamp).num_seconds() as u64;
                age_secs < PRICE_CACHE_TTL_SECS
            })
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

/// Optimized batch processing using get_multiple_accounts for better performance
async fn run_batch_price_calculation(
    tokens: &[Token]
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if tokens.is_empty() {
        return Ok(());
    }

    log(
        LogTag::Pool,
        "BATCH",
        &format!("Running optimized batch calculation for {} tokens", tokens.len())
    );

    let configs = read_configs("configs.json").map_err(|e| format!("Config error: {}", e))?;
    let discovery = PoolDiscovery::new();
    let decoder = PoolDecoder::new()?;

    // Step 1: Discover pools for all tokens in batch
    let mut all_pools = Vec::new();

    for token in tokens {
        // Step 1: Discovery - find pools for this token
        match discovery.get_pool_addresses(&token.mint).await {
            Ok(pools) => {
                debug_log(
                    "DISCOVERY",
                    &format!("Found {} pools for token {}", pools.len(), token.symbol)
                );
                all_pools.extend(pools);
            }
            Err(e) => {
                debug_log("WARN", &format!("Pool discovery failed for {}: {}", token.symbol, e));
            }
        }
    }

    if all_pools.is_empty() {
        debug_log("WARN", "No pools found for any tokens in batch");
        return Ok(());
    }

    // Step 2: Batch fetch pool data using get_multiple_accounts (OPTIMIZATION)
    let pool_pubkeys: Result<Vec<Pubkey>, _> = all_pools
        .iter()
        .map(|pool| Pubkey::from_str(&pool.address))
        .collect();

    let pool_pubkeys: Vec<Pubkey> = match pool_pubkeys {
        Ok(keys) => keys,
        Err(e) => {
            debug_log("ERROR", &format!("Failed to parse pool addresses: {}", e));
            return Ok(());
        }
    };

    let pool_data_map = match decoder.fetch_multiple_pool_data(&pool_pubkeys).await {
        Ok(data) => data,
        Err(e) => {
            debug_log("ERROR", &format!("Batch pool data fetch failed: {}", e));
            return Ok(());
        }
    };

    debug_log(
        "SUCCESS",
        &format!("Batch fetched data for {}/{} pools", pool_data_map.len(), all_pools.len())
    );

    // Step 3: Process each token's pools and calculate prices
    for token in tokens {
        let token_pools: Vec<_> = all_pools
            .iter()
            .filter(|pool| {
                // Find pools for this specific token
                // Since PoolAddressInfo doesn't have token_a_mint/token_b_mint,
                // we assume the pool was found for this token via discovery
                if let Ok(pool_pubkey) = Pubkey::from_str(&pool.address) {
                    pool_data_map.contains_key(&pool_pubkey)
                } else {
                    false
                }
            })
            .collect();

        if token_pools.is_empty() {
            continue;
        }

        // Step 4: Find best price for this token
        let mut best_price = None;
        let mut best_liquidity = 0.0;

        for pool in token_pools {
            if let Ok(pool_pubkey) = Pubkey::from_str(&pool.address) {
                if let Some(account_data) = pool_data_map.get(&pool_pubkey) {
                    let decoded = decoder.decode_pool_data(
                        pool.address.clone(),
                        &account_data.program_id,
                        &pool.dex_name,
                        &account_data.account_data,
                        account_data.liquidity_usd
                    ).await;

                    match PriceCalculator::calculate_token_price(&token.mint, &[decoded.clone()]) {
                        Ok(Some(calculated_price)) => {
                            let price = calculated_price.price_sol;
                            let liquidity = decoded.liquidity_usd;
                            debug_log(
                                "CALC",
                                &format!(
                                    "Token {} price: {:.12} SOL (liquidity: ${:.2})",
                                    token.symbol,
                                    price,
                                    liquidity
                                )
                            );

                            // Choose pool with highest liquidity
                            if liquidity > best_liquidity {
                                best_price = Some(price);
                                best_liquidity = liquidity;
                            }
                        }
                        Ok(None) => {
                            debug_log("WARN", &format!("No price calculated for {}", token.symbol));
                        }
                        Err(e) => {
                            debug_log(
                                "WARN",
                                &format!("Price calculation failed for {}: {}", token.symbol, e)
                            );
                        }
                    }
                }
            }
        }

        // Step 5: Cache and update global token list with best price
        if let Some(price) = best_price {
            // Cache the price
            if let Ok(mut cache) = PRICE_CACHE.lock() {
                cache.insert(token.mint.clone(), CachedPrice {
                    price,
                    timestamp: Utc::now(),
                });
            }

            // Update global token list with validated pool price (NON-BLOCKING)
            update_token_pool_price(&token.mint, price).await;

            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Updated price for {} ({:.12} SOL) with liquidity ${:.2}",
                    token.symbol,
                    price,
                    best_liquidity
                )
            );
        }
    }

    Ok(())
}

/// Update global token list with validated pool price
/// This allows the price to be used by the trader without blocking
async fn update_token_pool_price(mint: &str, price: f64) {
    use crate::global::LIST_TOKENS;

    // Non-blocking update to global token list
    if let Ok(mut tokens) = LIST_TOKENS.try_write() {
        for token in tokens.iter_mut() {
            if token.mint == mint {
                token.price_pool_sol = Some(price);
                log(
                    LogTag::Pool,
                    "UPDATE",
                    &format!(
                        "Updated global token list with pool price for {}: {:.12} SOL",
                        mint,
                        price
                    )
                );
                break;
            }
        }
    } else {
        // If we can't get the lock, don't block - the cached price will still be available
        log(
            LogTag::Pool,
            "WARN",
            &format!("Could not update token list with pool price for {}, using cache only", mint)
        );
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

        // Process positions in batches using optimized get_multiple_accounts
        const BATCH_SIZE: usize = 10;
        for batch in open_position_mints.chunks(BATCH_SIZE) {
            log(
                LogTag::Pool,
                "BATCH",
                &format!(
                    "Processing optimized batch of {} positions using get_multiple_accounts",
                    batch.len()
                )
            );

            // Get tokens for this batch from global list
            let batch_tokens: Vec<Token> = {
                if let Ok(tokens) = LIST_TOKENS.try_read() {
                    tokens
                        .iter()
                        .filter(|token| batch.iter().any(|mint| *mint == token.mint))
                        .cloned()
                        .collect()
                } else {
                    log(LogTag::Pool, "WARN", "Could not read token list for batch processing");
                    continue;
                }
            };

            if !batch_tokens.is_empty() {
                // Use optimized batch processing pipeline
                if let Err(e) = run_batch_price_calculation(&batch_tokens).await {
                    log(LogTag::Pool, "ERROR", &format!("Batch processing failed: {}", e));
                }
            }

            // Longer delay between batches
            if batch.len() == BATCH_SIZE {
                log(LogTag::Pool, "BATCH", "Waiting between batches to respect rate limits");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
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
