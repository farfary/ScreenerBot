/// Pool Price Background Manager
///
/// This module implements a background task system for pool price discovery and validation.
/// It operates independently from the trader to avoid blocking trading operations.
///
/// Key Features:
/// - Background task that runs pool price checks every 30 seconds
/// - Validates tokens can be decoded before marking them as valid
/// - Maintains a cache of successfully decoded tokens
/// - Updates global token list with pool prices asynchronously
/// - Non-blocking operations that don't interfere with trader or summary
///
/// The system prioritizes tokens based on:
/// 1. Open positions (highest priority)
/// 2. High liquidity tokens from the global token list
/// 3. Recently discovered tokens

use crate::logger::{ log, LogTag };
use crate::global::{ LIST_TOKENS, read_configs };
use crate::pool_price::PoolDiscoveryAndPricing;
use crate::positions::SAVED_POSITIONS;
use crate::utils::check_shutdown_or_delay;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };
use tokio::sync::Notify;
use once_cell::sync::Lazy;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Pool price update interval (30 seconds)
const POOL_PRICE_UPDATE_INTERVAL_SECS: u64 = 30;

/// Maximum number of tokens to check per cycle
const MAX_TOKENS_PER_CYCLE: usize = 20;

/// Maximum concurrent pool price checks
const MAX_CONCURRENT_POOL_CHECKS: usize = 3;

/// Pool price cache validity duration (5 minutes)
const POOL_PRICE_CACHE_DURATION_SECS: u64 = 300;

/// Maximum time to wait for pool price check (10 seconds)
const POOL_PRICE_TIMEOUT_SECS: u64 = 10;

// =============================================================================
// GLOBAL STATE FOR POOL PRICE MANAGEMENT
// =============================================================================

/// Cache for successfully decoded tokens (mint -> timestamp)
static VALIDATED_TOKENS: Lazy<Arc<Mutex<HashMap<String, Instant>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

/// Cache for pool prices (mint -> (price, timestamp))
static POOL_PRICE_CACHE: Lazy<Arc<Mutex<HashMap<String, (f64, Instant)>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

/// Tokens that failed to decode (to avoid retrying too often)
static FAILED_DECODE_TOKENS: Lazy<Arc<Mutex<HashMap<String, Instant>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

// =============================================================================
// POOL PRICE VALIDATION AND TRACKING
// =============================================================================

/// Marks a token as successfully validated for pool price decoding
pub fn mark_token_as_validated(mint: &str) {
    if let Ok(mut validated) = VALIDATED_TOKENS.lock() {
        validated.insert(mint.to_string(), Instant::now());
        log(
            LogTag::Pool,
            "VALIDATED",
            &format!("Token {} marked as validated for pool decoding", mint)
        );
    }
}

/// Marks a token as failed to decode (to avoid frequent retries)
pub fn mark_token_decode_failed(mint: &str) {
    if let Ok(mut failed) = FAILED_DECODE_TOKENS.lock() {
        failed.insert(mint.to_string(), Instant::now());
        log(LogTag::Pool, "DECODE_FAIL", &format!("Token {} marked as failed to decode", mint));
    }
}

/// Checks if a token has been successfully validated for pool decoding
pub fn is_token_validated(mint: &str) -> bool {
    if let Ok(validated) = VALIDATED_TOKENS.lock() {
        if let Some(&timestamp) = validated.get(mint) {
            // Consider validation valid for 24 hours
            return timestamp.elapsed() < Duration::from_secs(86400);
        }
    }
    false
}

/// Checks if a token recently failed to decode (avoid retrying too soon)
fn is_token_recently_failed(mint: &str) -> bool {
    if let Ok(failed) = FAILED_DECODE_TOKENS.lock() {
        if let Some(&timestamp) = failed.get(mint) {
            // Retry failed tokens after 1 hour
            return timestamp.elapsed() < Duration::from_secs(3600);
        }
    }
    false
}

/// Gets cached pool price if available and not expired
pub fn get_cached_pool_price(mint: &str) -> Option<f64> {
    if let Ok(cache) = POOL_PRICE_CACHE.lock() {
        if let Some(&(price, timestamp)) = cache.get(mint) {
            if timestamp.elapsed() < Duration::from_secs(POOL_PRICE_CACHE_DURATION_SECS) {
                return Some(price);
            }
        }
    }
    None
}

/// Updates pool price cache
fn update_pool_price_cache(mint: &str, price: f64) {
    if let Ok(mut cache) = POOL_PRICE_CACHE.lock() {
        cache.insert(mint.to_string(), (price, Instant::now()));
    }
}

// =============================================================================
// TOKEN PRIORITIZATION FOR POOL PRICE CHECKS
// =============================================================================

/// Gets list of tokens prioritized for pool price checks
fn get_prioritized_tokens_for_pool_checks() -> Vec<String> {
    let mut prioritized_tokens = Vec::new();

    // Priority 1: Open position tokens (highest priority)
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        for position in positions.iter() {
            if position.exit_price.is_none() {
                // Position is open if no exit price
                prioritized_tokens.push(position.mint.clone());
            }
        }
    }

    // Priority 2: High liquidity tokens from global list
    if let Ok(tokens) = LIST_TOKENS.try_read() {
        let mut high_liquidity_tokens: Vec<_> = tokens
            .iter()
            .filter(|token| {
                // Only include tokens with good liquidity that we haven't already added
                if prioritized_tokens.contains(&token.mint) {
                    return false;
                }

                let liquidity_usd = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);

                liquidity_usd > 50000.0 // Only tokens with >$50k liquidity
            })
            .map(|token| token.mint.clone())
            .collect();

        // Sort by liquidity (highest first)
        high_liquidity_tokens.sort_by(|a, b| {
            let liquidity_a = tokens
                .iter()
                .find(|t| &t.mint == a)
                .and_then(|t| t.liquidity.as_ref().and_then(|l| l.usd))
                .unwrap_or(0.0);
            let liquidity_b = tokens
                .iter()
                .find(|t| &t.mint == b)
                .and_then(|t| t.liquidity.as_ref().and_then(|l| l.usd))
                .unwrap_or(0.0);
            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        prioritized_tokens.extend(high_liquidity_tokens);
    }

    // Limit to maximum tokens per cycle
    prioritized_tokens.truncate(MAX_TOKENS_PER_CYCLE);
    prioritized_tokens
}

// =============================================================================
// BACKGROUND POOL PRICE CHECKING
// =============================================================================

/// Checks pool price for a single token with validation
async fn check_token_pool_price(pool_service: &PoolDiscoveryAndPricing, mint: &str) -> Option<f64> {
    // Skip if token recently failed to decode
    if is_token_recently_failed(mint) {
        return None;
    }

    // Check cache first
    if let Some(cached_price) = get_cached_pool_price(mint) {
        return Some(cached_price);
    }

    // Attempt to get pool price with timeout
    let pool_price_result = tokio::time::timeout(
        Duration::from_secs(POOL_PRICE_TIMEOUT_SECS),
        pool_service.get_token_pool_prices(mint)
    ).await;

    match pool_price_result {
        Ok(Ok(pool_results)) if !pool_results.is_empty() => {
            // Find the best pool price (highest liquidity)
            let best_result = pool_results
                .iter()
                .filter(|result| result.calculation_successful && result.calculated_price > 0.0)
                .max_by(|a, b|
                    a.liquidity_usd
                        .partial_cmp(&b.liquidity_usd)
                        .unwrap_or(std::cmp::Ordering::Equal)
                );

            if let Some(best) = best_result {
                let price = best.calculated_price;
                // Successfully got pool price - mark as validated and cache it
                mark_token_as_validated(mint);
                update_pool_price_cache(mint, price);
                log(
                    LogTag::Pool,
                    "SUCCESS",
                    &format!("Pool price for {}: {:.10} SOL", mint, price)
                );
                Some(price)
            } else {
                // No valid pool price found
                mark_token_decode_failed(mint);
                log(LogTag::Pool, "FAIL", &format!("No valid pool price found for {}", mint));
                None
            }
        }
        Ok(Ok(_)) => {
            // Empty results
            mark_token_decode_failed(mint);
            log(LogTag::Pool, "FAIL", &format!("No pool results found for {}", mint));
            None
        }
        Ok(Err(e)) => {
            // Failed to get pool price - mark as failed
            mark_token_decode_failed(mint);
            log(LogTag::Pool, "FAIL", &format!("Failed to get pool price for {}: {}", mint, e));
            None
        }
        Err(_) => {
            // Timeout occurred
            log(LogTag::Pool, "TIMEOUT", &format!("Pool price check timeout for {}", mint));
            None
        }
    }
}

/// Updates global token list with pool prices (non-blocking)
fn update_global_tokens_with_pool_prices(pool_prices: &HashMap<String, f64>) {
    if pool_prices.is_empty() {
        return;
    }

    match LIST_TOKENS.try_write() {
        Ok(mut tokens) => {
            let mut updated_count = 0;
            for token in tokens.iter_mut() {
                if let Some(&pool_price) = pool_prices.get(&token.mint) {
                    token.price_pool_sol = Some(pool_price);
                    updated_count += 1;
                }
            }
            log(
                LogTag::Pool,
                "UPDATE",
                &format!("Updated {} tokens with pool prices", updated_count)
            );
        }
        Err(_) => {
            // Non-blocking - if we can't get write lock, skip this update
            log(
                LogTag::Pool,
                "SKIP",
                "Could not acquire write lock for token list, skipping pool price update"
            );
        }
    }
}

// =============================================================================
// MAIN BACKGROUND TASK
// =============================================================================

/// Main background task for pool price management
pub async fn pool_price_manager(shutdown: Arc<Notify>) {
    log(LogTag::Pool, "START", "Pool price manager background task started");

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Failed to load configs: {}", e));
            return;
        }
    };

    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    loop {
        let cycle_start = Instant::now();

        // Get prioritized tokens for this cycle
        let tokens_to_check = get_prioritized_tokens_for_pool_checks();

        if tokens_to_check.is_empty() {
            log(LogTag::Pool, "IDLE", "No tokens to check for pool prices");
        } else {
            log(
                LogTag::Pool,
                "CYCLE",
                &format!("Checking pool prices for {} tokens", tokens_to_check.len())
            );

            // Store the count before moving tokens_to_check
            let total_tokens_count = tokens_to_check.len();

            // Process tokens with concurrency control
            let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_POOL_CHECKS));
            let mut handles = Vec::new();

            for mint in tokens_to_check {
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    _ => {
                        log(
                            LogTag::Pool,
                            "WARN",
                            &format!("Could not acquire permit for {}", mint)
                        );
                        continue;
                    }
                };

                let pool_service_clone = PoolDiscoveryAndPricing::new(&configs.rpc_url);
                let mint_clone = mint.clone();

                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive
                    let result = check_token_pool_price(&pool_service_clone, &mint_clone).await;
                    (mint_clone, result)
                });

                handles.push(handle);
            }

            // Collect results with timeout
            let mut pool_prices = HashMap::new();
            let mut successful_checks = 0;

            for handle in handles {
                match tokio::time::timeout(Duration::from_secs(15), handle).await {
                    Ok(Ok((mint, Some(price)))) => {
                        pool_prices.insert(mint, price);
                        successful_checks += 1;
                    }
                    Ok(Ok((mint, None))) => {
                        // Failed to get price, already logged
                    }
                    Ok(Err(e)) => {
                        log(LogTag::Pool, "ERROR", &format!("Task error: {}", e));
                    }
                    Err(_) => {
                        log(LogTag::Pool, "TIMEOUT", "Pool price check task timeout");
                    }
                }
            }

            // Update global token list with new pool prices
            if !pool_prices.is_empty() {
                update_global_tokens_with_pool_prices(&pool_prices);
            }

            log(
                LogTag::Pool,
                "COMPLETE",
                &format!(
                    "Pool price cycle complete: {}/{} successful checks in {:.2}s",
                    successful_checks,
                    total_tokens_count,
                    cycle_start.elapsed().as_secs_f64()
                )
            );
        }

        // Wait for next cycle or shutdown
        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(POOL_PRICE_UPDATE_INTERVAL_SECS)
            ).await
        {
            break;
        }
    }

    log(LogTag::Pool, "STOP", "Pool price manager background task stopped");
}

// =============================================================================
// PUBLIC API FOR TRADER INTEGRATION
// =============================================================================

/// Gets the best available price for a token (pool price if validated, otherwise API price)
/// This is non-blocking and returns immediately
pub fn get_best_available_price(mint: &str) -> Option<f64> {
    // First, try to get validated pool price from cache
    if is_token_validated(mint) {
        if let Some(pool_price) = get_cached_pool_price(mint) {
            return Some(pool_price);
        }
    }

    // Fallback to API price from global token list (non-blocking)
    if let Ok(tokens) = LIST_TOKENS.try_read() {
        for token in tokens.iter() {
            if token.mint == mint {
                // Priority: DexScreener SOL > Pool price
                return token.price_dexscreener_sol.or(token.price_pool_sol);
            }
        }
    }

    None
}

/// Forces a pool price check for a specific token (for immediate use)
/// This is non-blocking and will update the cache for future use
pub async fn request_immediate_pool_price_check(mint: &str) {
    if is_token_recently_failed(mint) {
        return; // Don't retry recently failed tokens
    }

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(_) => {
            return;
        }
    };

    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Spawn a quick background check
    let mint_clone = mint.to_string();
    tokio::spawn(async move {
        if let Some(price) = check_token_pool_price(&pool_service, &mint_clone).await {
            log(
                LogTag::Pool,
                "IMMEDIATE",
                &format!("Immediate pool price check for {}: {:.10} SOL", mint_clone, price)
            );
        }
    });
}
