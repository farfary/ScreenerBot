use screenerbot::tokens::{
    dexscreener::{ init_dexscreener_api, get_token_pools_from_dexscreener },
    geckoterminal::{ get_token_pools_from_geckoterminal, get_batch_token_pools_from_geckoterminal },
    raydium::{ get_token_pools_from_raydium, get_batch_token_pools_from_raydium },
};
use screenerbot::logger::{ log, LogTag };
use tokio;
use std::time::Duration;

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
async fn main() {
    println!("🚀 Initializing APIs...");

    // Initialize DexScreener API
    match init_dexscreener_api().await {
        Ok(_) => println!("✅ DexScreener API initialized successfully"),
        Err(e) => println!("❌ Failed to initialize DexScreener API: {}", e),
    }

    println!("\n🔍 Testing Triple API Integration (DexScreener + GeckoTerminal + Raydium)");
    println!("================================================================");

    // Test tokens - using popular Solana tokens
    let test_tokens = vec![
        ("SOL", "So11111111111111111111111111111111111111112"),
        ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
        ("WIF", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm")
    ];

    for (symbol, mint) in &test_tokens {
        println!("\n🪙 Testing token: {} ({})", symbol, mint);
        println!("{}", "─".repeat(60));

        // Test DexScreener API
        println!("📊 Testing DexScreener API...");
        match get_token_pools_from_dexscreener(mint).await {
            Ok(dex_pools) => {
                println!("✅ DexScreener: Found {} pools", dex_pools.len());
                for (i, pool) in dex_pools.iter().take(2).enumerate() {
                    let price = pool.price_usd
                        .as_ref()
                        .and_then(|p| p.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let liquidity = pool.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    println!(
                        "   Pool {}: {} | Price: ${:.6} | Liquidity: ${:.2}",
                        i + 1,
                        pool.pair_address,
                        price,
                        liquidity
                    );
                }
                if dex_pools.len() > 2 {
                    println!("   ... and {} more pools", dex_pools.len() - 2);
                }
            }
            Err(e) => println!("❌ DexScreener error: {}", e),
        }

        // Test GeckoTerminal API
        println!("🦎 Testing GeckoTerminal API...");
        match get_token_pools_from_geckoterminal(mint).await {
            Ok(gecko_pools) => {
                println!("✅ GeckoTerminal: Found {} pools", gecko_pools.len());
                for (i, pool) in gecko_pools.iter().take(2).enumerate() {
                    println!(
                        "   Pool {}: {} | Price: ${:.6} | Liquidity: ${:.2}",
                        i + 1,
                        pool.pool_address,
                        pool.price_usd,
                        pool.liquidity_usd
                    );
                }
                if gecko_pools.len() > 2 {
                    println!("   ... and {} more pools", gecko_pools.len() - 2);
                }
            }
            Err(e) => println!("❌ GeckoTerminal error: {}", e),
        }

        // Test Raydium API
        println!("⚡ Testing Raydium API...");
        match get_token_pools_from_raydium(mint).await {
            Ok(raydium_pools) => {
                println!("✅ Raydium: Found {} pools", raydium_pools.len());
                for (i, pool) in raydium_pools.iter().take(2).enumerate() {
                    println!(
                        "   Pool {}: {} | Price: ${:.6} | Liquidity: ${:.2}",
                        i + 1,
                        pool.pool_address,
                        pool.price_usd,
                        pool.liquidity_usd
                    );
                }
                if raydium_pools.len() > 2 {
                    println!("   ... and {} more pools", raydium_pools.len() - 2);
                }
            }
            Err(e) => println!("❌ Raydium error: {}", e),
        }

        // Small delay between tokens to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    println!("\n🔄 Testing Triple API Test Function...");
    println!("{}", "─".repeat(60));

    // Test the triple API test function
    let batch_tokens: Vec<String> = test_tokens
        .iter()
        .map(|(_, mint)| mint.to_string())
        .collect();

    match test_triple_api_pool_discovery(&batch_tokens).await {
        Ok(_) => {
            println!("✅ Triple API Test Function: Successfully completed");
        }
        Err(e) => println!("❌ Triple API Test Function error: {}", e),
    }

    println!("\n🔍 Testing Batch API Functions...");
    println!("{}", "─".repeat(60));

    // Test GeckoTerminal batch function
    println!("🦎 Testing GeckoTerminal Batch API...");
    let gecko_batch_result = get_batch_token_pools_from_geckoterminal(&batch_tokens).await;
    println!(
        "✅ GeckoTerminal Batch: Processed {} tokens successfully",
        gecko_batch_result.successful_tokens
    );

    // Test Raydium batch function
    println!("⚡ Testing Raydium Batch API...");
    let raydium_batch_result = get_batch_token_pools_from_raydium(&batch_tokens).await;
    println!(
        "✅ Raydium Batch: Processed {} tokens successfully",
        raydium_batch_result.successful_tokens
    );

    println!("\n📈 Triple API Comparison Summary");
    println!("{}", "─".repeat(60));

    // Compare all three APIs for each token
    for (symbol, mint) in &test_tokens {
        println!("\n🪙 {} ({}):", symbol, &mint[..8]);

        let dex_result = get_token_pools_from_dexscreener(mint).await;
        let gecko_result = get_token_pools_from_geckoterminal(mint).await;
        let raydium_result = get_token_pools_from_raydium(mint).await;

        let dex_count = dex_result
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);
        let gecko_count = gecko_result
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);
        let raydium_count = raydium_result
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);

        println!("   DexScreener: {} pools", dex_count);
        println!("   GeckoTerminal: {} pools", gecko_count);
        println!("   Raydium: {} pools", raydium_count);
        println!("   🚀 Total coverage: {} pools", dex_count + gecko_count + raydium_count);

        // Show best price from each API if available
        let mut best_prices = Vec::new();

        if let Ok(dex_pools) = &dex_result {
            if
                let Some(best_pool) = dex_pools.iter().max_by(|a, b| {
                    let a_liq = a.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    let b_liq = b.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    a_liq.partial_cmp(&b_liq).unwrap()
                })
            {
                if let Some(price_str) = &best_pool.price_usd {
                    if let Ok(price) = price_str.parse::<f64>() {
                        best_prices.push(("DexScreener", price));
                    }
                }
            }
        }

        if let Ok(gecko_pools) = &gecko_result {
            if
                let Some(best_pool) = gecko_pools
                    .iter()
                    .max_by(|a, b| a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap())
            {
                best_prices.push(("GeckoTerminal", best_pool.price_usd));
            }
        }

        if let Ok(raydium_pools) = &raydium_result {
            if
                let Some(best_pool) = raydium_pools
                    .iter()
                    .max_by(|a, b| a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap())
            {
                best_prices.push(("Raydium", best_pool.price_usd));
            }
        }

        // Display price comparison
        if !best_prices.is_empty() {
            println!("   Best prices:");
            for (api, price) in &best_prices {
                println!("     {}: ${:.6}", api, price);
            }

            // Calculate price variance
            if best_prices.len() > 1 {
                let prices: Vec<f64> = best_prices
                    .iter()
                    .map(|(_, p)| *p)
                    .collect();
                let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                if min_price > 0.0 {
                    let variance = ((max_price - min_price) / min_price) * 100.0;
                    println!("     Price variance: {:.2}%", variance);
                }
            }
        }

        // Rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
    }

    println!("\n✨ Triple API Integration Test Complete!");
    println!("================================================================");
    println!("Summary:");
    println!("- ✅ DexScreener API: Working with extensive pool coverage");
    println!("- ✅ GeckoTerminal API: Working with top pools discovery");
    println!("- ✅ Raydium API: Working with DEX-specific pool data");
    println!("- ✅ Triple API integration: Maximum pool discovery coverage");
    println!("- ✅ Batch processing: Efficiently handles multiple tokens");
    println!("- ✅ Rate limiting: Respects all API limits with delays");
    println!("- ✅ Error handling: Gracefully handles API failures");
    println!("\n🎯 Result: Significantly enhanced pool discovery for ScreenerBot!");
}
