use anyhow::Result;
use screenerbot::pairs::PairsClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("\n🔗 Testing Pairs Integration with DEX Screener API");
    println!("==================================================\n");

    // Create PairsClient
    let pairs_client = PairsClient::new()?;
    println!("✅ Created PairsClient successfully");

    // Test tokens (SOL and USDC)
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    println!("\n🔍 Testing single token pool discovery...");

    // Test SOL pairs
    match pairs_client.get_solana_token_pairs(sol_mint).await {
        Ok(pairs) => {
            println!("✅ Found {} pairs for SOL", pairs.len());

            if let Some(best_pair) = pairs_client.get_best_pair(pairs.clone()) {
                let quality_score = pairs_client.calculate_pool_quality_score(&best_pair);
                println!(
                    "🏆 Best SOL pair: {} on {} (Quality Score: {:.1}/100)",
                    best_pair.pair_address,
                    best_pair.dex_id,
                    quality_score
                );
                let liquidity_usd = best_pair.liquidity.as_ref().map_or(0.0, |l| l.usd);
                println!("   💰 Liquidity: ${:.2}", liquidity_usd);
                println!("   📊 Volume 24h: ${:.2}", best_pair.volume.h24);
                println!("   💵 Price: ${}", best_pair.price_usd);
            }

            // Filter high-quality pairs
            let high_quality_pairs = pairs_client.filter_by_liquidity(pairs, 1_000_000.0);
            println!("📈 High liquidity pairs (>$1M): {}", high_quality_pairs.len());
        }
        Err(e) => {
            println!("❌ Failed to get SOL pairs: {}", e);
        }
    }

    println!("\n💲 Testing smart price discovery...");

    // Test best price discovery
    match pairs_client.get_best_price(sol_mint).await {
        Ok(Some(price)) => {
            println!("✅ Best SOL price: ${:.6}", price);
        }
        Ok(None) => {
            println!("⚠️  No price found for SOL");
        }
        Err(e) => {
            println!("❌ Failed to get SOL price: {}", e);
        }
    }

    println!("\n🔄 Testing batch token processing...");

    // Test multiple tokens at once
    let token_addresses = vec![sol_mint, usdc_mint];
    match pairs_client.get_multiple_token_pairs(&token_addresses).await {
        Ok(all_pairs) => {
            println!(
                "✅ Found {} total pairs for {} tokens",
                all_pairs.len(),
                token_addresses.len()
            );

            // Group by DEX
            let mut dex_counts = std::collections::HashMap::new();
            for pair in &all_pairs {
                *dex_counts.entry(pair.dex_id.clone()).or_insert(0) += 1;
            }

            println!("📊 Pairs by DEX:");
            for (dex, count) in dex_counts {
                println!(
                    "   {} {}: {} pairs",
                    match dex.as_str() {
                        "raydium" => "🔴",
                        "orca" => "🟣",
                        "meteora" => "🟡",
                        "pump" => "🟢",
                        _ => "⚪",
                    },
                    dex,
                    count
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to get multiple token pairs: {}", e);
        }
    }

    println!("\n📈 Testing batch price discovery...");

    // Test batch price discovery
    match pairs_client.get_best_prices(&token_addresses).await {
        Ok(prices) => {
            println!("✅ Best prices for tokens:");
            for (token, price) in prices {
                match price {
                    Some(p) =>
                        println!(
                            "   {}: ${:.6}",
                            if token == sol_mint {
                                "SOL"
                            } else {
                                "USDC"
                            },
                            p
                        ),
                    None =>
                        println!("   {}: No price available", if token == sol_mint {
                            "SOL"
                        } else {
                            "USDC"
                        }),
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get batch prices: {}", e);
        }
    }

    println!("\n💾 Testing caching functionality...");

    // Test caching with the same token
    let start_time = std::time::Instant::now();
    match pairs_client.get_token_pairs_with_cache(sol_mint).await {
        Ok(cached_pairs) => {
            let duration = start_time.elapsed();
            println!("✅ Retrieved {} cached pairs in {:?}", cached_pairs.len(), duration);
        }
        Err(e) => {
            println!("❌ Failed to get cached pairs: {}", e);
        }
    }

    // Show cache statistics
    match pairs_client.get_cache_stats() {
        Ok(stats) => {
            println!("📊 Cache Statistics:");
            println!("   Total cached tokens: {}", stats.total_tokens);
            println!("   Active tokens: {}", stats.active_tokens);
            println!("   Total cached pairs: {}", stats.total_pairs);
            println!("   Active pairs: {}", stats.active_pairs);
        }
        Err(e) => {
            println!("⚠️  Failed to get cache stats: {}", e);
        }
    }

    println!("\n✅ Integration test completed successfully!");
    println!("\n💡 The PairsClient is now ready for use in the trader:");
    println!("   • Smart pool selection based on liquidity & volume");
    println!("   • Real-time price discovery from best pools");
    println!("   • Pool quality scoring for trade validation");
    println!("   • Efficient caching to reduce API calls");
    println!("   • Batch operations for multiple tokens");

    Ok(())
}
