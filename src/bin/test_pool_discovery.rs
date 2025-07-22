use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;

#[tokio::main]
async fn main() -> Result<()> {
    // Load RPC URL from configs
    let configs_content = std::fs::read_to_string("configs.json")?;
    let configs: serde_json::Value = serde_json::from_str(&configs_content)?;
    let rpc_url = configs["rpc_url"].as_str().unwrap_or("https://api.mainnet-beta.solana.com");

    println!("🚀 ScreenerBot Pool Discovery & Price Calculator");
    println!("═════════════════════════════════════════════════");
    println!("Using RPC: {}\n", rpc_url);

    let calculator = PoolDiscoveryAndPricing::new(rpc_url);

    // Test tokens
    let test_tokens = vec![
        ("Jupiter (JUP)", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"),
        ("Altman Token", "32H8ZLmaXQMQ57NNMB2KeW2yewzdFGMKwXLcgfc9bonk")
    ];

    for (name, token_mint) in test_tokens {
        println!("\n🎯 Processing: {} ({})", name, &token_mint[0..8]);
        println!("═══════════════════════════════════════════════════");

        match calculator.generate_pool_price_report(token_mint).await {
            Ok(report) => {
                println!("{}", report);
            }
            Err(e) => {
                println!("❌ Failed to generate report for {}: {}", name, e);
            }
        }

        println!("\n{}", "═".repeat(60));
    }

    println!("\n✅ Pool Discovery & Pricing Analysis Complete!");

    // Show example of how to use individual functions
    println!("\n📚 INDIVIDUAL FUNCTION EXAMPLES");
    println!("═════════════════════════════════════════════════");

    // Example 1: Discover pools only
    println!("\n1. Discover Pools Only:");
    match calculator.discover_pools("JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN").await {
        Ok(pools) => {
            println!("   Found {} pools", pools.len());
            for (i, pool) in pools.iter().take(3).enumerate() {
                println!(
                    "   {}. {} - {} ({}/{}) - Liquidity: ${:.0}",
                    i + 1,
                    pool.dex_id,
                    pool.pair_address[0..8].to_string(),
                    pool.base_token.symbol,
                    pool.quote_token.symbol,
                    pool.liquidity_usd
                );
            }
            if pools.len() > 3 {
                println!("   ... and {} more", pools.len() - 3);
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Example 2: Calculate specific pool price
    println!("\n2. Calculate Specific Pool Price:");
    let example_pool = "C1MgLojNLWBKADvu9BHdtgzz1oZX4dZ5zGdGcgvvW8Wz"; // JUP/SOL Orca pool
    match calculator.calculate_pool_price(example_pool).await {
        Ok((price, token_a, token_b, pool_type)) => {
            println!("   ✅ Pool: {} - Type: {:?}", example_pool[0..8].to_string(), pool_type);
            println!(
                "   ✅ Price: {:.12} (Token A: {}..{}, Token B: {}..{})",
                price,
                &token_a[0..4],
                &token_a[token_a.len() - 4..],
                &token_b[0..4],
                &token_b[token_b.len() - 4..]
            );
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Example 3: Get all pool prices for a token
    println!("\n3. Get All Pool Prices for a Token:");
    match calculator.get_token_pool_prices("32H8ZLmaXQMQ57NNMB2KeW2yewzdFGMKwXLcgfc9bonk").await {
        Ok(results) => {
            let successful = results
                .iter()
                .filter(|r| r.calculation_successful)
                .count();
            println!("   ✅ Found {} pools, {} successful calculations", results.len(), successful);

            for result in results.iter().take(2) {
                if result.calculation_successful {
                    println!(
                        "   ✅ {} - Price: {:.12}, Diff: {:.1}%",
                        result.dex_id,
                        result.calculated_price,
                        result.price_difference_percent
                    );
                } else {
                    println!(
                        "   ❌ {} - Failed: {}",
                        result.dex_id,
                        result.error_message.as_ref().unwrap_or(&"Unknown".to_string())
                    );
                }
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    println!("\n🎯 Integration Tips:");
    println!("════════════════════");
    println!("• Use discover_pools() to find all pools for a token");
    println!("• Use get_token_pool_prices() for complete analysis with our calculations");
    println!("• Filter by liquidity_usd for better trading opportunities");
    println!("• Monitor price_difference_percent for arbitrage opportunities");
    println!("• Focus on is_sol_pair=true pools for SOL-based trading");

    Ok(())
}
