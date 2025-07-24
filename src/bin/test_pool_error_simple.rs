use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::global::read_configs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Pool Error Logging for Failed Cases");
    println!("=============================================\n");

    // Read configs to get RPC URL
    let configs = read_configs("configs.json")?;
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    println!("🔍 Testing: Invalid token that will fail pool discovery");
    println!("Token: INVALID_TOKEN_ADDRESS");
    println!("Expected: Should show detailed error logs with token info\n");

    // Test with clearly invalid token - should trigger comprehensive error logging
    match pool_service.get_token_pool_prices("INVALID_TOKEN_ADDRESS").await {
        Ok(results) => println!("✅ Unexpected success: Found {} pools", results.len()),
        Err(e) => println!("❌ Expected failure: {}\n", e),
    }

    println!("🔍 Testing: Real token that might have no valid pools");
    println!("Token: BREADwHdS4F1kk38DLG1kZYDfJSrGQjYXr9m5PF2e4Zi (BREAD)");
    println!("Expected: Should show 'NO POOLS DISCOVERED' error with details\n");

    // Test with a token that has pools but calculations might fail
    match pool_service.get_token_pool_prices("BREADwHdS4F1kk38DLG1kZYDfJSrGQjYXr9m5PF2e4Zi").await {
        Ok(results) => {
            if results.is_empty() {
                println!("❌ Expected: No pools found for BREAD token");
            } else {
                println!("✅ Success: Found {} pools for BREAD token", results.len());
                for result in &results {
                    println!("  - Pool: {} ({:?})", result.pool_address, result.pool_type);
                }
            }
        }
        Err(e) => println!("❌ Expected: {}\n", e),
    }

    println!("\n🎯 Error Logging Test Complete!");
    println!("Check the logs above for:");
    println!("- ⚠️ NO PAIRS FROM DEXSCREENER with token mint and API URL");
    println!("- ⚠️ NO POOLS DISCOVERED with token mint and reason");
    println!("- ❌ PRICE CALCULATION FAILED with full token and pool details");
    println!("- All requested information: token name, mint, pool address, program ID");

    Ok(())
}
