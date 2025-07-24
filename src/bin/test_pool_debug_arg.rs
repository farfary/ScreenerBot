use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::global::{ read_configs, CMD_ARGS };
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate command line arguments
    let args: Vec<String> = env::args().collect();
    if let Ok(mut cmd_args) = CMD_ARGS.lock() {
        *cmd_args = args;
    }

    println!("🧪 Testing Pool Price Debug Argument System");
    println!("===========================================");

    // Load configurations
    let configs = read_configs("configs.json")?;

    // Create pool discovery service
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    // Test with a well-known token (BONK)
    let test_token = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

    println!("🔍 Testing pool discovery for token: {}", test_token);
    println!("📝 Check the log output to see debug vs summary mode:");

    if env::args().any(|arg| arg == "--debug-pool-price") {
        println!("   ✅ DEBUG MODE ENABLED - You should see detailed debug logs");
    } else {
        println!("   📊 SUMMARY MODE - You should see only summary information");
        println!("   💡 Run with --debug-pool-price to see detailed logs");
    }

    println!();

    // Test pool discovery (this will show different logging based on debug mode)
    match pool_service.discover_pools(test_token).await {
        Ok(pools) => {
            println!("✅ Discovery completed successfully");
            println!("📊 Found {} pools", pools.len());
        }
        Err(e) => {
            println!("❌ Discovery failed: {}", e);
        }
    }

    // Test pool price calculation (this will also show different logging)
    match pool_service.get_token_pool_prices(test_token).await {
        Ok(results) => {
            let successful = results
                .iter()
                .filter(|r| r.calculation_successful)
                .count();
            println!("✅ Price calculation completed");
            println!("📊 Processed {} pools, {} successful", results.len(), successful);
        }
        Err(e) => {
            println!("❌ Price calculation failed: {}", e);
        }
    }

    println!();
    println!("🎯 Test completed! Check the logs above to see the difference.");
    println!("   Without --debug-pool-price: Only summary logs");
    println!("   With --debug-pool-price: Detailed debug information");

    Ok(())
}
