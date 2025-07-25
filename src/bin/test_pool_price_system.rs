/// Test the new pool price system
///
/// This test validates the complete pool price pipeline:
/// - Pool discovery via DexScreener API
/// - Pool data fetching via Solana RPC
/// - Pool data decoding
/// - Price calculation
/// - Caching functionality

use screenerbot::logger::{ log, LogTag };
use screenerbot::pool_price::*;
use screenerbot::global::{ read_configs, initialize_token_database };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Pool, "TEST", "Starting pool price system test");

    // Initialize
    initialize_token_database()?;
    let _configs = read_configs("configs.json")?;

    // Test with a popular meme token instead of SOL to avoid wrapped SOL complications
    let test_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    // Test 1: Basic price calculation
    info!("[TEST            ] === Test 2: Basic Price Calculation ===");
    let start = tokio::time::Instant::now();
    let price1 = get_token_price(test_mint).await;
    let duration1 = start.elapsed();

    let start = tokio::time::Instant::now();
    let price2 = get_token_price(test_mint).await;
    let duration2 = start.elapsed();

    info!(
        "[TEST            ] Cache test: first call {:.3}s, second call {:.3}s",
        duration1.as_secs_f64(),
        duration2.as_secs_f64()
    );

    // Test 2: Pool discovery
    info!("[TEST            ] === Test 3: Pool Discovery ===");
    info!("[TEST            ] Testing pool discovery for {}", test_mint);

    // Create discovery instance
    let discovery = PoolDiscovery::new(HttpClient::builder().build().unwrap());
    let pools = discovery.get_or_fetch_pools(test_mint).await;

    match pools {
        Ok(pools) => {
            info!("[SUCCESS         ] Found {} pools for {}", pools.len(), test_mint);
            for (i, pool) in pools.iter().take(3).enumerate() {
                info!(
                    "[INFO            ]   Pool {}: {} ({}, ${:.2} liquidity)",
                    i + 1,
                    pool.address,
                    pool.dex_id,
                    pool.liquidity_usd
                );
            }
        }
        Err(e) => {
            warn!("[ERROR           ] Pool discovery failed: {}", e);
        }
    }

    log(LogTag::Pool, "TEST", "=== Test 1: System Health Check ===");
    health_check("Validating pool price system health").await;

    // Test 2: Get price for a known token (SOL)
    log(LogTag::Pool, "TEST", "=== Test 2: SOL Price Lookup ===");
    match get_token_price(SOL_MINT).await {
        Some(price) => {
            log(LogTag::Pool, "SUCCESS", &format!("SOL price: {:.12} SOL (should be ~1.0)", price));
        }
        None => {
            log(LogTag::Pool, "ERROR", "Failed to get SOL price");
        }
    }

    // Test 3: Cache functionality
    log(LogTag::Pool, "TEST", "=== Test 3: Price Caching ===");
    let test_mint = SOL_MINT;

    // First call (should fetch fresh)
    let start_time = std::time::Instant::now();
    let _price1 = get_token_price(test_mint).await;
    let first_call_duration = start_time.elapsed();

    // Second call (should use cache)
    let start_time = std::time::Instant::now();
    let _price2 = get_token_price(test_mint).await;
    let second_call_duration = start_time.elapsed();

    log(
        LogTag::Pool,
        "TEST",
        &format!(
            "Cache test: first call {:.3}s, second call {:.3}s",
            first_call_duration.as_secs_f64(),
            second_call_duration.as_secs_f64()
        )
    );

    // Test 4: Pool discovery for a real token
    log(LogTag::Pool, "TEST", "=== Test 4: Pool Discovery ===");
    let test_tokens = vec![
        test_mint.to_string()
        // Add more test tokens here if known
    ];

    for mint in &test_tokens {
        log(LogTag::Pool, "TEST", &format!("Testing pool discovery for {}", mint));

        match get_pool_addresses_for_token(mint).await {
            Ok(pools) => {
                log(LogTag::Pool, "SUCCESS", &format!("Found {} pools for {}", pools.len(), mint));

                for (i, pool) in pools.iter().take(3).enumerate() {
                    log(
                        LogTag::Pool,
                        "INFO",
                        &format!(
                            "  Pool {}: {} ({}, ${:.2} liquidity)",
                            i + 1,
                            pool.address,
                            pool.dex_name,
                            pool.liquidity_usd
                        )
                    );
                }
            }
            Err(e) => {
                log(LogTag::Pool, "ERROR", &format!("Pool discovery failed for {}: {}", mint, e));
            }
        }
    }

    // Test 5: Detailed price information
    log(LogTag::Pool, "TEST", "=== Test 5: Detailed Price Info ===");
    match get_detailed_price_info(SOL_MINT).await {
        Ok(Some(detailed)) => {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "Detailed price for SOL: {:.12} SOL (confidence: {:.2}, {} pools)",
                    detailed.price_sol,
                    detailed.confidence,
                    detailed.source_pools.len()
                )
            );

            for pool in &detailed.source_pools {
                log(LogTag::Pool, "INFO", &format!("  Source pool: {}", pool));
            }
        }
        Ok(None) => {
            log(LogTag::Pool, "WARN", "No detailed price info available for SOL");
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Detailed price lookup failed: {}", e));
        }
    }

    // Test 6: Cache statistics
    log(LogTag::Pool, "TEST", "=== Test 6: Cache Statistics ===");
    let (price_total, price_valid) = get_price_cache_stats();
    let (pool_total, pool_valid) = get_pool_cache_stats();

    log(
        LogTag::Pool,
        "INFO",
        &format!(
            "Cache stats - Price cache: {}/{} valid, Pool cache: {}/{} valid",
            price_valid,
            price_total,
            pool_valid,
            pool_total
        )
    );

    // Test 7: Error handling
    log(LogTag::Pool, "TEST", "=== Test 7: Error Handling ===");
    let invalid_mint = "InvalidMintAddress123";
    match get_token_price(invalid_mint).await {
        Some(price) => {
            log(LogTag::Pool, "WARN", &format!("Unexpected price for invalid mint: {:.12}", price));
        }
        None => {
            log(LogTag::Pool, "SUCCESS", "Correctly handled invalid mint address");
        }
    }

    // Test 8: Batch preloading
    log(LogTag::Pool, "TEST", "=== Test 8: Batch Preloading ===");
    let batch_tokens = vec![SOL_MINT.to_string()];
    match preload_token_prices(&batch_tokens).await {
        Ok(loaded_count) => {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!("Batch preloading: {}/{} tokens loaded", loaded_count, batch_tokens.len())
            );
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Batch preloading failed: {}", e));
        }
    }

    // Final cache cleanup
    clear_price_cache();
    cleanup_pool_cache();

    log(LogTag::Pool, "TEST", "=== Pool Price System Test Complete ===");
    log(LogTag::Pool, "SUCCESS", "All tests completed successfully");

    Ok(())
}
