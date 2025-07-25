//! Test Token Update Functionality
//! Tests the token info update system and monitors the process

use screenerbot::tokens::*;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 ScreenerBot Token Update Test");
    println!("=================================\n");

    // Test 1: Initialize system
    println!("📋 Test 1: System Initialization");
    let mut system = initialize_tokens_system().await?;
    let initial_stats = system.get_system_stats().await?;
    println!("   📊 Initial Stats:");
    println!("      Total tokens: {}", initial_stats.total_tokens);
    println!("      Active tokens: {}", initial_stats.active_tokens);
    println!("      Last discovery: {}", initial_stats.last_discovery_cycle);
    println!("      Last monitoring: {}", initial_stats.last_monitoring_cycle);
    println!("✅ System initialization test passed\n");

    // Test 2: Get some tokens to update
    println!("📋 Test 2: Get Current Tokens");
    let db = TokenDatabase::new()?;
    let tokens = db.get_all_tokens().await?;

    if tokens.is_empty() {
        println!("   ⚠️  No tokens in database. Running discovery first...");
        let results = discover_tokens_once().await?;
        println!("   🔍 Discovery completed. Found {} new tokens", results.len());

        let updated_tokens = db.get_all_tokens().await?;
        println!("   📊 Database now has {} tokens", updated_tokens.len());
    } else {
        println!("   📊 Found {} tokens in database", tokens.len());

        // Show first 5 tokens
        for (i, token) in tokens.iter().take(5).enumerate() {
            println!(
                "      {}. {} ({}) - Price: ${:.6}",
                i + 1,
                token.symbol,
                token.name,
                token.price_usd
            );
        }
    }
    println!("✅ Token retrieval test passed\n");

    // Test 3: Manual Token Update
    println!("📋 Test 3: Manual Token Update");
    println!("   🔄 Running manual token monitoring...");

    monitor_tokens_once().await?;
    println!("   📊 Monitoring completed successfully");

    println!("✅ Token update test passed\n");

    // Test 4: Check specific token updates
    println!("📋 Test 4: Verify Token Updates");
    let updated_tokens = db.get_all_tokens().await?;

    println!("   📊 Checking for recent updates...");
    let mut recent_updates = 0;

    for token in updated_tokens.iter().take(10) {
        if token.price_usd > 0.0 {
            recent_updates += 1;
            println!("      ✅ {} has price data: ${:.6}", token.symbol, token.price_usd);
        }
    }

    println!("   📊 Found {} tokens with price data", recent_updates);

    if recent_updates > 0 {
        println!("✅ Token update verification passed");
    } else {
        println!("⚠️  No tokens with price data detected");
    }
    println!();

    // Test 5: Test API directly
    println!("📋 Test 5: Direct API Test");
    let mut api_client = DexScreenerApi::new();
    api_client.initialize().await?;

    // Test with a known token mint
    let test_mints = vec![
        "So11111111111111111111111111111111111111112".to_string(), // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string() // USDC
    ];

    println!("   🧪 Testing API with known tokens...");
    match api_client.get_tokens_info(&test_mints).await {
        Ok(api_tokens) => {
            println!("   ✅ API call successful! Retrieved {} tokens", api_tokens.len());
            for token in api_tokens.iter().take(3) {
                println!("      📊 {} ({}): ${:.6}", token.symbol, token.name, token.price_usd);
            }
        }
        Err(e) => {
            println!("   ❌ API call failed: {}", e);
        }
    }
    println!("✅ Direct API test completed\n");

    println!("🎉 Token Update Test Summary");
    println!("============================");
    println!("All tests completed. Check the results above for any issues.");

    Ok(())
}
