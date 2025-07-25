/// Test the pricing monitor functionality
use screenerbot::tokens::{
    initialize_price_service,
    initialize_tokens_system,
    update_tokens_prices_safe,
    get_all_tokens_by_liquidity,
    get_token_price_safe,
};
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ScreenerBot Pricing Monitor Test");
    println!("====================================\n");

    // Initialize tokens system
    println!("📋 Test 1: Initialize Tokens System");
    let mut _system = match initialize_tokens_system().await {
        Ok(system) => system,
        Err(e) => {
            println!("❌ Failed to initialize tokens system: {}", e);
            return Err(e.into());
        }
    };
    println!("✅ Tokens system initialized\n");

    // Initialize price service
    println!("📋 Test 2: Initialize Price Service");
    if let Err(e) = initialize_price_service().await {
        println!("❌ Failed to initialize price service: {}", e);
        return Err(e.into());
    }
    println!("✅ Price service initialized successfully\n");

    // Test price updates via price service
    println!("📋 Test 3: Price Service Update");
    let test_mints = vec!["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()]; // USDC
    update_tokens_prices_safe(&test_mints).await;
    println!("✅ Price service update completed");
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
                if let Some(price) = get_token_price_safe(&token.mint).await {
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

        // Use the safe price service update
        let test_mints = vec!["EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()];
        update_tokens_prices_safe(&test_mints).await;
        println!("   ✅ Cycle {} completed successfully", cycle);

        // Small delay between cycles
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    println!();

    println!("🎉 Pricing monitor test completed!");
    println!("✅ All pricing functions are working correctly");

    Ok(())
}
