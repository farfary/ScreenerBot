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
use crate::global::{ LIST_TOKENS, read_configs, is_debug_pool_prices_enabled };
use crate::pool_price::{ PoolDiscoveryAndPricing, cleanup_expired_pools, get_pool_cache_stats };
use crate::positions::SAVED_POSITIONS;
use crate::utils::check_shutdown_or_delay;

use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };
use tokio::sync::Notify;
use once_cell::sync::Lazy;
use serde_json;

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Pool price update interval (30 seconds as originally designed)
const POOL_PRICE_UPDATE_INTERVAL_SECS: u64 = 30;

/// Maximum number of tokens to check per cycle
const MAX_TOKENS_PER_CYCLE: usize = 50;

/// Maximum concurrent pool price checks
const MAX_CONCURRENT_POOL_CHECKS: usize = 3;

/// Maximum time to wait for pool price check (8 seconds)
const POOL_PRICE_TIMEOUT_SECS: u64 = 8;

/// Helper function for conditional debug logging - only shows when --debug-pool-prices is used
fn debug_log(log_type: &str, message: &str) {
    if is_debug_pool_prices_enabled() {
        log(LogTag::Pool, log_type, message);
    }
}

/// Helper function for regular pool logging - always visible for important operations
fn pool_log(log_type: &str, message: &str) {
    log(LogTag::Pool, log_type, message);
}

// =============================================================================
// GLOBAL STATE FOR POOL PRICE MANAGEMENT (Legacy - using new cache system)
// =============================================================================

/// Legacy cache for compatibility - now using the global pool cache system
static VALIDATED_TOKENS: Lazy<Arc<Mutex<HashMap<String, Instant>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

/// Legacy cache for compatibility - now using the global pool cache system
static FAILED_DECODE_TOKENS: Lazy<Arc<Mutex<HashMap<String, Instant>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

// =============================================================================
// POOL PRICE VALIDATION AND TRACKING (Updated for new cache system)
// =============================================================================

/// Marks a token as successfully validated for pool price decoding
pub fn mark_token_as_validated(mint: &str) {
    if let Ok(mut validated) = VALIDATED_TOKENS.lock() {
        validated.insert(mint.to_string(), Instant::now());
        debug_log("VALIDATED", &format!("Token {} marked as validated for pool decoding", mint));
    }
}

/// Marks a token as failed to decode (to avoid frequent retries)
pub fn mark_token_decode_failed(mint: &str) {
    if let Ok(mut failed) = FAILED_DECODE_TOKENS.lock() {
        failed.insert(mint.to_string(), Instant::now());
        debug_log("DECODE_FAIL", &format!("Token {} marked as failed to decode", mint));
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
/// Reduced retry time for open positions to be more aggressive
fn is_token_recently_failed(mint: &str) -> bool {
    if let Ok(failed) = FAILED_DECODE_TOKENS.lock() {
        if let Some(&timestamp) = failed.get(mint) {
            // Retry failed tokens after 10 minutes (more aggressive for open positions)
            return timestamp.elapsed() < Duration::from_secs(600);
        }
    }
    false
}

/// Gets cached pool price if available and not expired (using new cache system)
/// Note: With the new address-only cache system, we don't cache prices anymore
/// This function returns None to indicate no cached price is available
pub fn get_cached_pool_price(mint: &str) -> Option<f64> {
    // Since we only cache addresses now (not prices), always return None
    // The caller should use the full discovery process to get fresh prices
    None
}

// =============================================================================
// TOKEN PRIORITIZATION FOR POOL PRICE CHECKS
// =============================================================================

/// Gets list of tokens prioritized for pool price checks
/// ONLY focuses on open position tokens - no need to check all tokens
fn get_prioritized_tokens_for_pool_checks() -> Vec<String> {
    let mut open_position_tokens = Vec::new();

    // ONLY Priority: Open position tokens (focus on what matters)
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        for position in positions.iter() {
            if position.exit_price.is_none() {
                // Position is open if no exit price
                open_position_tokens.push(position.mint.clone());
            }
        }
    }

    // Log the focus
    if !open_position_tokens.is_empty() {
        log(
            LogTag::Pool,
            "FOCUS",
            &format!(
                "Pool price manager focusing on {} open position tokens only",
                open_position_tokens.len()
            )
        );
    }

    open_position_tokens
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
                // Successfully got pool price - mark as validated (cache is handled in pool_price module)
                mark_token_as_validated(mint);
                debug_log("SUCCESS", &format!("Pool price for {}: {:.10} SOL", mint, price));
                Some(price)
            } else {
                // No valid pool price found
                mark_token_decode_failed(mint);
                debug_log("FAIL", &format!("No valid pool price found for {}", mint));
                None
            }
        }
        Ok(Ok(_)) => {
            // Empty results
            mark_token_decode_failed(mint);
            debug_log("FAIL", &format!("No pool results found for {}", mint));
            None
        }
        Ok(Err(e)) => {
            // Failed to get pool price - mark as failed
            mark_token_decode_failed(mint);
            debug_log("FAIL", &format!("Failed to get pool price for {}: {}", mint, e));
            None
        }
        Err(_) => {
            // Timeout occurred
            debug_log("TIMEOUT", &format!("Pool price check timeout for {}", mint));
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
    pool_log("START", "Pool price manager background task started");

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            pool_log("ERROR", &format!("Failed to load configs: {}", e));
            return;
        }
    };

    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Log cache statistics if in debug mode
    if is_debug_pool_prices_enabled() {
        let (total, valid, expired, failed) = get_pool_cache_stats();
        debug_log(
            "CACHE",
            &format!(
                "Starting with cache: {} total, {} valid, {} expired, {} failed tokens",
                total,
                valid,
                expired,
                failed
            )
        );
    }

    loop {
        let cycle_start = Instant::now();

        // Cleanup expired cache entries periodically
        if let Err(e) = cleanup_expired_pools() {
            debug_log("ERROR", &format!("Failed to cleanup expired pools: {}", e));
        }

        // Get prioritized tokens for this cycle (only open positions)
        let tokens_to_check = get_prioritized_tokens_for_pool_checks();

        if tokens_to_check.is_empty() {
            // No open positions, wait longer before next check
            debug_log("IDLE", "No open positions to check for pool prices");
            if
                check_shutdown_or_delay(
                    &shutdown,
                    Duration::from_secs(60) // Wait 1 minute when no positions
                ).await
            {
                break;
            }
            continue;
        } else {
            pool_log(
                "CYCLE",
                &format!("Checking pool prices for {} open position tokens", tokens_to_check.len())
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
                        debug_log("ERROR", &format!("Task error: {}", e));
                    }
                    Err(_) => {
                        debug_log("TIMEOUT", "Pool price check task timeout");
                    }
                }
            }

            // Update global token list with new pool prices
            if !pool_prices.is_empty() {
                update_global_tokens_with_pool_prices(&pool_prices);
            }

            pool_log(
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

    pool_log("STOP", "Pool price manager background task stopped");
}

// =============================================================================
// PUBLIC API FOR TRADER INTEGRATION
// =============================================================================

/// Gets the best available price for a token (pool price if validated, otherwise API price)
/// This is non-blocking and returns immediately
/// Enhanced for open positions with more aggressive fallback logic
pub fn get_best_available_price(mint: &str) -> Option<f64> {
    // First, try to get validated pool price from cache
    if is_token_validated(mint) {
        if let Some(pool_price) = get_cached_pool_price(mint) {
            log(
                LogTag::Pool,
                "CACHE_HIT",
                &format!("Pool price for {}: {:.10} SOL", mint, pool_price)
            );
            return Some(pool_price);
        }
    }

    // Fallback to API price from global token list (non-blocking)
    if let Ok(tokens) = LIST_TOKENS.try_read() {
        for token in tokens.iter() {
            if token.mint == mint {
                // Enhanced priority: DexScreener SOL > Pool price
                if let Some(price) = token.price_dexscreener_sol {
                    log(
                        LogTag::Pool,
                        "DEXSCR_SOL",
                        &format!("DexScreener SOL price for {}: {:.10}", mint, price)
                    );
                    return Some(price);
                }
                if let Some(price) = token.price_pool_sol {
                    log(
                        LogTag::Pool,
                        "POOL_SOL",
                        &format!("Pool SOL price for {}: {:.10}", mint, price)
                    );
                    return Some(price);
                }

                // Log when we can't find any price
                log(
                    LogTag::Pool,
                    "NO_PRICE",
                    &format!("No price found for {} in global token list", mint)
                );
                break;
            }
        }

        // Log when token is not found in list
        debug_log("NOT_FOUND", &format!("Token {} not found in global token list", mint));
    } else {
        debug_log("LOCK_FAIL", "Could not acquire read lock for token list");
    }

    None
}

/// Forces a pool price check for a specific token (for immediate use)
/// This is non-blocking and will update the cache for future use
/// Enhanced for open positions with immediate cache update
pub async fn request_immediate_pool_price_check(mint: &str) {
    if is_token_recently_failed(mint) {
        debug_log("SKIP_RECENT_FAIL", &format!("Skipping recently failed token: {}", mint));
        return; // Don't retry recently failed tokens
    }

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            debug_log("CONFIG_ERROR", &format!("Failed to load configs: {}", e));
            return;
        }
    };

    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Spawn a quick background check with priority for open positions
    let mint_clone = mint.to_string();
    tokio::spawn(async move {
        log(
            LogTag::Pool,
            "IMMEDIATE_START",
            &format!("Starting immediate pool price check for {}", mint_clone)
        );

        if let Some(price) = check_token_pool_price(&pool_service, &mint_clone).await {
            log(
                LogTag::Pool,
                "IMMEDIATE_SUCCESS",
                &format!("Immediate pool price check for {}: {:.10} SOL", mint_clone, price)
            );

            // Update global token list immediately
            let mut prices = std::collections::HashMap::new();
            prices.insert(mint_clone.clone(), price);
            update_global_tokens_with_pool_prices(&prices);
        } else {
            log(
                LogTag::Pool,
                "IMMEDIATE_FAIL",
                &format!("Immediate pool price check failed for {}", mint_clone)
            );
        }
    });
}

/// Triggers immediate pool price checks for all open positions
/// This is called when displaying positions to ensure fresh data
pub async fn refresh_open_position_prices() {
    let open_position_mints = get_prioritized_tokens_for_pool_checks();

    if open_position_mints.is_empty() {
        return;
    }

    log(
        LogTag::Pool,
        "REFRESH_ALL",
        &format!("Refreshing pool prices for {} open positions", open_position_mints.len())
    );

    // First, try to bootstrap token data for open positions if global list is empty
    bootstrap_open_position_tokens(&open_position_mints).await;

    // Then trigger immediate checks for all open positions
    for mint in open_position_mints {
        request_immediate_pool_price_check(&mint).await;
        // Small delay to avoid overwhelming the system
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Bootstrap function to fetch basic token information for open positions
/// when the global token list is empty or missing these tokens
async fn bootstrap_open_position_tokens(mints: &[String]) {
    // Check if we need to bootstrap (if tokens are missing from global list)
    let missing_mints = {
        if let Ok(tokens) = LIST_TOKENS.try_read() {
            if tokens.is_empty() {
                // Global list is empty, bootstrap all
                mints.to_vec()
            } else {
                // Check which mints are missing
                mints
                    .iter()
                    .filter(|mint| !tokens.iter().any(|t| &t.mint == *mint))
                    .cloned()
                    .collect()
            }
        } else {
            // Can't read global list, bootstrap all
            mints.to_vec()
        }
    };

    if missing_mints.is_empty() {
        return;
    }

    log(
        LogTag::Pool,
        "BOOTSTRAP",
        &format!("Bootstrapping {} missing tokens for open positions", missing_mints.len())
    );

    // Try to fetch basic token information from DexScreener API
    let configs = match read_configs("configs.json") {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::Pool, "BOOTSTRAP_ERROR", &format!("Failed to load configs: {}", e));
            return;
        }
    };

    // Fetch token info from DexScreener API
    for mint in missing_mints {
        if let Ok(token_info) = fetch_basic_token_info(&configs, &mint).await {
            // Add to global token list
            if let Ok(mut tokens) = LIST_TOKENS.try_write() {
                tokens.push(token_info.clone());
                log(
                    LogTag::Pool,
                    "BOOTSTRAP_ADD",
                    &format!("Added {} ({}) to global token list", token_info.symbol, mint)
                );
            }
        }

        // Small delay between API calls
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Fetch basic token information from DexScreener API for a single mint
async fn fetch_basic_token_info(
    configs: &crate::global::Configs,
    mint: &str
) -> Result<crate::global::Token, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", mint);

    log(LogTag::Pool, "FETCH_INFO", &format!("Fetching token info for {}", mint));

    let response = client.get(&url).timeout(Duration::from_secs(10)).send().await?;

    if !response.status().is_success() {
        return Err(format!("DexScreener API error: {}", response.status()).into());
    }

    let response_text = response.text().await?;
    let api_response: serde_json::Value = serde_json::from_str(&response_text)?;

    // Parse the response to create a Token struct
    if let Some(pairs) = api_response["pairs"].as_array() {
        if let Some(pair) = pairs.first() {
            let base_token = &pair["baseToken"];

            // Create a basic Token struct with available information
            let token = crate::global::Token {
                mint: mint.to_string(),
                symbol: base_token["symbol"].as_str().unwrap_or("UNKNOWN").to_string(),
                name: base_token["name"].as_str().unwrap_or("Unknown Token").to_string(),
                decimals: base_token["decimals"].as_u64().unwrap_or(9) as u8,
                chain: "solana".to_string(),

                // Set basic fields
                logo_url: None,
                coingecko_id: None,
                website: None,
                description: None,
                tags: Vec::new(),
                is_verified: false,
                created_at: None,

                // Price information from DexScreener
                price_dexscreener_sol: pair["priceNative"].as_str().and_then(|s| s.parse().ok()),
                price_dexscreener_usd: pair["priceUsd"].as_str().and_then(|s| s.parse().ok()),
                price_pool_sol: None,
                price_pool_usd: None,
                pools: Vec::new(),

                // DexScreener specific fields
                dex_id: pair["dexId"].as_str().map(|s| s.to_string()),
                pair_address: pair["pairAddress"].as_str().map(|s| s.to_string()),
                pair_url: pair["url"].as_str().map(|s| s.to_string()),
                labels: Vec::new(),
                fdv: pair["fdv"].as_f64(),
                market_cap: pair["marketCap"].as_f64(),
                txns: None, // Would need more complex parsing
                volume: None, // Would need more complex parsing
                price_change: None, // Would need more complex parsing
                liquidity: None, // Would need more complex parsing
                info: None,
                boosts: None,
            };

            log(
                LogTag::Pool,
                "FETCH_SUCCESS",
                &format!(
                    "Fetched info for {} ({}): {} SOL",
                    token.symbol,
                    mint,
                    token.price_dexscreener_sol.map_or("N/A".to_string(), |p| format!("{:.10}", p))
                )
            );

            return Ok(token);
        }
    }

    Err("No token data found in DexScreener response".into())
}

/// Debug function to diagnose why a token price is showing as N/A
pub fn debug_token_price_lookup(mint: &str) -> String {
    let mut debug_info = Vec::new();

    // Check validated tokens cache
    if is_token_validated(mint) {
        debug_info.push("✓ Token is validated".to_string());
        if let Some(price) = get_cached_pool_price(mint) {
            debug_info.push(format!("✓ Cached pool price: {:.10} SOL", price));
        } else {
            debug_info.push("✗ No cached pool price".to_string());
        }
    } else {
        debug_info.push("✗ Token not validated".to_string());
    }

    // Check if recently failed
    if is_token_recently_failed(mint) {
        debug_info.push("⚠ Token recently failed to decode".to_string());
    }

    // Check global token list
    if let Ok(tokens) = LIST_TOKENS.try_read() {
        if let Some(token) = tokens.iter().find(|t| t.mint == mint) {
            debug_info.push("✓ Found in global token list".to_string());
            debug_info.push(format!("  - DexScreener SOL: {:?}", token.price_dexscreener_sol));
            debug_info.push(format!("  - Pool SOL: {:?}", token.price_pool_sol));
            debug_info.push(format!("  - DexScreener USD: {:?}", token.price_dexscreener_usd));
            debug_info.push(format!("  - Pool USD: {:?}", token.price_pool_usd));
        } else {
            debug_info.push("✗ Not found in global token list".to_string());
        }
    } else {
        debug_info.push("✗ Could not read global token list".to_string());
    }

    debug_info.join("\n")
}
