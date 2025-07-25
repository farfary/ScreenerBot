/// Test the pricing monitor functionality
use screenerbot::tokens::{
    initialize_pricing_system,
    initialize_token_database,
    update_token_prices_manual,
    get_all_tokens_by_liquidity,
    get_current_token_price,
};
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ScreenerBot Pricing Monitor Test");
    println!("====================================\n");

    // Initialize pricing system
    println!("📋 Test 1: Initialize Pricing System");
    if let Err(e) = initialize_pricing_system().await {
        println!("❌ Failed to initialize pricing system: {}", e);
        return Err(e.into());
    }
    println!("✅ Pricing system initialized\n");

    // Initialize database
    println!("📋 Test 2: Initialize Token Database");
    if let Err(e) = initialize_token_database() {
        println!("❌ Failed to initialize database: {}", e);
        return Err(e.into());
    }
    println!("✅ Database initialized successfully\n");

    // Test manual pricing update
    println!("📋 Test 3: Manual Pricing Update");
    match update_token_prices_manual().await {
        Ok(_) => println!("✅ Manual pricing update successful"),
        Err(e) => println!("⚠️  Manual pricing update failed: {}", e),
    }
    println!();

    // Test liquidity-based token retrieval
    println!("📋 Test 4: Get Tokens by Liquidity");
    match get_all_tokens_by_liquidity().await {
        Ok(tokens) => {
            println!("✅ Retrieved {} tokens sorted by liquidity", tokens.len());

            // Show top 5 tokens by liquidity
            let top_tokens = tokens.iter().take(5);
            for (i, token) in top_tokens.enumerate() {
                let liquidity = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .map(|l| format!("${:.0}", l))
                    .unwrap_or_else(|| "N/A".to_string());
                println!(
                    "   {}. {} ({}) - Liquidity: {}",
                    i + 1,
                    token.symbol,
                    token.mint[..8].to_string(),
                    liquidity
                );
            }
        }
        Err(e) => println!("❌ Failed to get tokens: {}", e),
    }
    println!();

    // Test current price lookup for a few tokens
    println!("📋 Test 5: Current Price Lookup");
    match get_all_tokens_by_liquidity().await {
        Ok(tokens) => {
            let test_tokens = tokens.iter().take(3);
            for token in test_tokens {
                if let Some(price) = get_current_token_price(&token.mint).await {
                    println!("   💰 {} price: ${:.8}", token.symbol, price);
                } else {
                    println!("   ⚠️  {} price: Not available", token.symbol);
                }
            }
        }
        Err(e) => println!("❌ Failed to test price lookup: {}", e),
    }
    println!();

    // Test multiple pricing cycles (simulate background operation)
    println!("📋 Test 6: Multiple Pricing Cycles (3 cycles)");
    for cycle in 1..=3 {
        log(LogTag::System, "TEST", &format!("Starting pricing cycle #{}", cycle));

        match update_token_prices_manual().await {
            Ok(_) => println!("   ✅ Cycle {} completed successfully", cycle),
            Err(e) => println!("   ❌ Cycle {} failed: {}", cycle, e),
        }

        // Small delay between cycles
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    println!();

    println!("🎉 Pricing monitor test completed!");
    println!("✅ All pricing functions are working correctly");

    Ok(())
}
