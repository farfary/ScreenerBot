use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::global::read_configs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Testing Pool Error Logging System");
    println!("===================================");

    // Load configuration
    let configs = read_configs("configs.json").map_err(|e| format!("Config error: {}", e))?;
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Test with various token mints to trigger different error scenarios
    let test_tokens = vec![
        ("BREADwHdS4F1kk38DLG1kZYDfJSrGQjYXr9m5PF2e4Zi", "Valid token with pools"), // Example valid token
        ("So11111111111111111111111111111111111111112", "Native SOL (might not have pools)"), // SOL
        ("INVALIDTOKENADDRESS123456789012345678901234", "Invalid token address"), // Invalid address
        ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC (might have pools)"), // USDC
        ("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", "BONK (might have pools)") // BONK
    ];

    for (token_mint, description) in test_tokens {
        println!("\n🔍 Testing: {} ({})", description, token_mint);
        println!("{}", "-".repeat(80));

        match pool_service.get_biggest_pool_cached(token_mint).await {
            Ok(Some(pool_result)) => {
                println!("✅ Successfully found pool:");
                println!("   Pool Address: {}", pool_result.pool_address);
                println!("   Pool Type: {:?}", pool_result.pool_type);
                println!("   DEX ID: {}", pool_result.dex_id);
                println!("   Price: {}", pool_result.calculated_price);
                println!("   Calculation Success: {}", pool_result.calculation_successful);
            }
            Ok(None) => {
                println!(
                    "⚠️ No valid pools found - check logs above for detailed error information"
                );
            }
            Err(e) => {
                println!("❌ Error occurred: {}", e);
                println!("   Check logs above for detailed error information");
            }
        }

        // Add a small delay between tests
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    println!("\n🎯 Test completed!");
    println!("Check the console output above for detailed error logging information.");
    println!("The error logs should include:");
    println!("- Token name, symbol, and mint");
    println!("- Pool address and DEX ID");
    println!("- Pool type and owner program ID");
    println!("- Specific error messages");

    Ok(())
}
