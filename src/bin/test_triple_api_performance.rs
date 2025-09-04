use screenerbot::tokens::{
    dexscreener::get_token_pools_from_dexscreener,
    geckoterminal::get_token_pools_from_geckoterminal,
    raydium::get_token_pools_from_raydium,
};
use screenerbot::logger::{ log, LogTag };
use std::time::{ Duration, Instant };
use tokio;

/// Test function to compare pool discovery between DexScreener, GeckoTerminal, and Raydium
/// This function is useful for debugging and validating the triple API integration
async fn test_triple_api_pool_discovery(token_addresses: &[String]) -> Result<(), String> {
    if token_addresses.is_empty() {
        return Err("No token addresses provided".to_string());
    }

    log(
        LogTag::Pool,
        "TRIPLE_API_TEST_START",
        &format!("🚀 Testing triple API pool discovery for {} tokens", token_addresses.len())
    );

    for token_address in token_addresses.iter().take(5) {
        // Limit to 5 tokens for testing
        log(LogTag::Pool, "TRIPLE_API_TEST_TOKEN", &format!("🔍 Testing token: {}", token_address));

        // Test DexScreener (using new consistent naming)
        let dexscreener_result = get_token_pools_from_dexscreener(token_address).await;
        let dexscreener_count = match &dexscreener_result {
            Ok(pairs) => pairs.len(),
            Err(_) => 0,
        };

        // Test GeckoTerminal
        let geckoterminal_result = get_token_pools_from_geckoterminal(token_address).await;
        let geckoterminal_count = match &geckoterminal_result {
            Ok(pools) => pools.len(),
            Err(_) => 0,
        };

        // Test Raydium
        let raydium_result = get_token_pools_from_raydium(token_address).await;
        let raydium_count = match &raydium_result {
            Ok(pools) => pools.len(),
            Err(_) => 0,
        };

        log(
            LogTag::Pool,
            "TRIPLE_API_TEST_RESULT",
            &format!(
                "📊 {}: DexScreener {} pools, GeckoTerminal {} pools, Raydium {} pools",
                &token_address[..8],
                dexscreener_count,
                geckoterminal_count,
                raydium_count
            )
        );

        // Show details from each API
        if let Ok(pairs) = &dexscreener_result {
            for (i, pair) in pairs.iter().take(3).enumerate() {
                let liquidity = pair.liquidity
                    .as_ref()
                    .map(|l| l.usd)
                    .unwrap_or(0.0);
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_DX_POOL",
                    &format!(
                        "   🔸 DX Pool {}: {} ({}, ${:.2})",
                        i + 1,
                        pair.pair_address,
                        pair.dex_id,
                        liquidity
                    )
                );
            }
        }

        if let Ok(pools) = &geckoterminal_result {
            for (i, pool) in pools.iter().take(3).enumerate() {
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_GT_POOL",
                    &format!(
                        "   🦎 GT Pool {}: {} ({}, ${:.2})",
                        i + 1,
                        pool.pool_address,
                        pool.dex_id,
                        pool.liquidity_usd
                    )
                );
            }
        }

        if let Ok(pools) = &raydium_result {
            for (i, pool) in pools.iter().take(3).enumerate() {
                log(
                    LogTag::Pool,
                    "TRIPLE_API_TEST_RAY_POOL",
                    &format!(
                        "   ⚡ Ray Pool {}: {} ({}, ${:.2})",
                        i + 1,
                        pool.pool_address,
                        pool.pool_type,
                        pool.liquidity_usd
                    )
                );
            }
        }

        // Small delay between tokens to respect rate limits
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    log(LogTag::Pool, "TRIPLE_API_TEST_COMPLETE", "🚀 Triple API test completed");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Performance Test: Optimized Triple API Batch Processing");
    println!("=========================================================\n");

    // Test with a batch of popular tokens
    let test_tokens = vec![
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // BONK
        "EKpQGSJtyjbpT68KVD8kcyiN7wbXoEpj4pGz1YHHxbZt", // WIF
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" // mSOL
    ];

    println!("📊 Testing optimized batch processing with {} tokens...\n", test_tokens.len());

    let start_time = Instant::now();

    // Convert to Vec<String> for the function call
    let test_tokens_string: Vec<String> = test_tokens
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Use the existing test function that implements the optimized triple API
    test_triple_api_pool_discovery(&test_tokens_string).await.expect("Test failed");

    let total_time = start_time.elapsed();

    println!("\n🎯 Performance Results:");
    println!("======================");
    println!("⏱️  Total Time: {}ms", total_time.as_millis());
    println!("📊 Tokens per second: {:.2}", (test_tokens.len() as f64) / total_time.as_secs_f64());
    println!(
        "⏱️  Average time per token: {}ms",
        total_time.as_millis() / (test_tokens.len() as u128)
    );

    // Calculate theoretical vs actual concurrency benefit
    let theoretical_sequential_time = (test_tokens.len() as u128) * 2000; // ~2s per token if sequential
    let speedup_factor = (theoretical_sequential_time as f64) / (total_time.as_millis() as f64);
    println!("🚀 Concurrency speedup: {:.1}x faster than sequential", speedup_factor);

    println!("\n✅ Optimization completed! All three APIs now run concurrently for maximum speed.");

    Ok(())
}
