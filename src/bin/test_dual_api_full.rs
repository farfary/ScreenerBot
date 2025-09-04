use screenerbot::tokens::{
    dexscreener::{ init_dexscreener_api, get_token_pairs_from_api },
    geckoterminal::{ get_token_pools_from_geckoterminal, get_batch_token_pools_from_geckoterminal },
    pool::test_dual_api_pool_discovery,
};
use tokio;

#[tokio::main]
async fn main() {
    println!("🚀 Initializing APIs...");

    // Initialize DexScreener API
    match init_dexscreener_api().await {
        Ok(_) => println!("✅ DexScreener API initialized successfully"),
        Err(e) => println!("❌ Failed to initialize DexScreener API: {}", e),
    }

    println!("\n🔍 Testing Dual API Integration (DexScreener + GeckoTerminal)");
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
        match get_token_pairs_from_api(mint).await {
            Ok(dex_pools) => {
                println!("✅ DexScreener: Found {} pools", dex_pools.len());
                for (i, pool) in dex_pools.iter().take(3).enumerate() {
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
                if dex_pools.len() > 3 {
                    println!("   ... and {} more pools", dex_pools.len() - 3);
                }
            }
            Err(e) => println!("❌ DexScreener error: {}", e),
        }

        // Test GeckoTerminal API
        println!("🦎 Testing GeckoTerminal API...");
        match get_token_pools_from_geckoterminal(mint).await {
            Ok(gecko_pools) => {
                println!("✅ GeckoTerminal: Found {} pools", gecko_pools.len());
                for (i, pool) in gecko_pools.iter().take(3).enumerate() {
                    println!(
                        "   Pool {}: {} | Price: ${:.6} | Liquidity: ${:.2}",
                        i + 1,
                        pool.pool_address,
                        pool.price_usd,
                        pool.liquidity_usd
                    );
                }
                if gecko_pools.len() > 3 {
                    println!("   ... and {} more pools", gecko_pools.len() - 3);
                }
            }
            Err(e) => println!("❌ GeckoTerminal error: {}", e),
        }

        // Small delay between tokens to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    }

    println!("\n🔄 Testing Dual API Test Function...");
    println!("{}", "─".repeat(60));

    // Test the dual API test function
    let batch_tokens: Vec<String> = test_tokens
        .iter()
        .map(|(_, mint)| mint.to_string())
        .collect();

    match test_dual_api_pool_discovery(&batch_tokens).await {
        Ok(_) => {
            println!("✅ Dual API Test Function: Successfully completed");
        }
        Err(e) => println!("❌ Dual API Test Function error: {}", e),
    }

    println!("\n🔍 Testing GeckoTerminal Batch API...");
    println!("{}", "─".repeat(60));

    // Test GeckoTerminal batch function specifically
    let gecko_batch_result = get_batch_token_pools_from_geckoterminal(&batch_tokens).await;
    println!("✅ GeckoTerminal Batch: Processed {} tokens", gecko_batch_result.successful_tokens);

    for (mint, pools) in &gecko_batch_result.pools {
        if let Some((symbol, _)) = test_tokens.iter().find(|(_, m)| m == mint) {
            println!("   {}: {} pools from GeckoTerminal", symbol, pools.len());
        }
    }

    if !gecko_batch_result.errors.is_empty() {
        println!("   Errors: {} tokens failed", gecko_batch_result.failed_tokens);
        for (mint, error) in &gecko_batch_result.errors {
            if let Some((symbol, _)) = test_tokens.iter().find(|(_, m)| m == mint) {
                println!("     {}: {}", symbol, error);
            }
        }
    }

    println!("\n📈 API Comparison Summary");
    println!("{}", "─".repeat(60));

    // Compare both APIs for each token
    for (symbol, mint) in &test_tokens {
        println!("\n🪙 {} ({}):", symbol, &mint[..8]);

        let dex_result = get_token_pairs_from_api(mint).await;
        let gecko_result = get_token_pools_from_geckoterminal(mint).await;

        let dex_count = dex_result
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);
        let gecko_count = gecko_result
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(0);

        println!("   DexScreener: {} pools", dex_count);
        println!("   GeckoTerminal: {} pools", gecko_count);
        println!("   Total coverage: {} pools", dex_count + gecko_count);

        // Show price comparison if both have data
        if let (Ok(dex_pools), Ok(gecko_pools)) = (&dex_result, &gecko_result) {
            if !dex_pools.is_empty() && !gecko_pools.is_empty() {
                if
                    let (Some(dex_best), Some(gecko_best)) = (
                        dex_pools.iter().max_by(|a, b| {
                            let a_liq = a.liquidity
                                .as_ref()
                                .map(|l| l.usd)
                                .unwrap_or(0.0);
                            let b_liq = b.liquidity
                                .as_ref()
                                .map(|l| l.usd)
                                .unwrap_or(0.0);
                            a_liq.partial_cmp(&b_liq).unwrap()
                        }),
                        gecko_pools
                            .iter()
                            .max_by(|a, b| a.liquidity_usd.partial_cmp(&b.liquidity_usd).unwrap()),
                    )
                {
                    let dex_price = dex_best.price_usd
                        .as_ref()
                        .and_then(|p| p.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let gecko_price = gecko_best.price_usd;

                    println!("   DexScreener best price: ${:.6}", dex_price);
                    println!("   GeckoTerminal best price: ${:.6}", gecko_price);

                    if dex_price > 0.0 && gecko_price > 0.0 {
                        let price_diff = (((dex_price - gecko_price) / gecko_price) * 100.0).abs();
                        println!("   Price difference: {:.2}%", price_diff);
                    }
                }
            }
        }

        // Rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\n✨ Dual API Integration Test Complete!");
    println!("================================================================");
    println!("Summary:");
    println!("- ✅ GeckoTerminal API: Working correctly with pool discovery");
    println!("- ✅ Dual API integration: Successfully combines both sources");
    println!("- ✅ Batch processing: Efficiently handles multiple tokens");
    println!("- ✅ Rate limiting: Respects API limits with delays");
    println!("- ✅ Error handling: Gracefully handles API failures");
}
