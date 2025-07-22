use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::global::read_configs;

#[tokio::main]
async fn main() -> Result<()> {
    // Test the specific Orca Whirlpool pool provided by the user
    let test_pool_address = "C9U2Ksk6KKWvLEeo5yUQ7Xu46X7NzeBJtd9PBfuXaUSM";

    // Load configs for RPC URL
    let configs = read_configs("configs.json").map_err(|e|
        anyhow::anyhow!("Failed to read configs: {}", e)
    )?;

    // Create pool service
    let pool_service = PoolDiscoveryAndPricing::new(&configs.rpc_url);

    println!("🧪 Testing Orca Whirlpool Support");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Pool Address: {}", test_pool_address);

    // Test pool type detection
    println!("\n🔍 Step 1: Detecting Pool Type");
    match pool_service.detect_pool_type(test_pool_address).await {
        Ok(pool_type) => {
            println!("✅ Pool type detected: {:?}", pool_type);

            // Test price calculation
            println!("\n💰 Step 2: Calculating Pool Price");
            match pool_service.calculate_pool_price(test_pool_address).await {
                Ok((price, token_a, token_b, detected_type)) => {
                    println!("✅ Price calculation successful!");
                    println!("   Price: {} SOL", price);
                    println!("   Token A: {}", token_a);
                    println!("   Token B: {}", token_b);
                    println!("   Detected Type: {:?}", detected_type);

                    // Check if it's a SOL pair
                    let is_sol_pair =
                        token_a == "So11111111111111111111111111111111111111112" ||
                        token_b == "So11111111111111111111111111111111111111112";
                    println!("   SOL Pair: {}", if is_sol_pair { "Yes" } else { "No" });
                }
                Err(e) => {
                    println!("❌ Price calculation failed: {}", e);
                }
            }

            // Test with explicit pool type
            println!("\n🎯 Step 3: Testing with Explicit Pool Type");
            match pool_service.calculate_pool_price_with_type(test_pool_address, pool_type).await {
                Ok((price, token_a, token_b, _)) => {
                    println!("✅ Explicit type calculation successful!");
                    println!("   Price: {} SOL", price);
                    println!("   Token A: {}", token_a);
                    println!("   Token B: {}", token_b);
                }
                Err(e) => {
                    println!("❌ Explicit type calculation failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Pool type detection failed: {}", e);
        }
    }

    println!("\n📊 Step 4: Testing Token Pool Discovery");
    // Try to discover pools for the token in the whirlpool
    // Based on the JSON data, tokenMintB is the pump token
    let test_token_mint = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump";

    match pool_service.get_token_pool_prices(test_token_mint).await {
        Ok(pool_results) => {
            println!("✅ Found {} pools for token {}", pool_results.len(), &test_token_mint[0..8]);

            for result in &pool_results {
                if result.pool_address == test_pool_address {
                    println!("🎯 Found our test pool in discovery results!");
                    println!("   Pool Type: {:?}", result.pool_type);
                    println!("   Calculated Price: {}", result.calculated_price);
                    println!("   DexScreener Price: {}", result.dexscreener_price);
                    println!("   Price Difference: {:.2}%", result.price_difference_percent);
                    println!("   Calculation Success: {}", result.calculation_successful);
                    if let Some(error) = &result.error_message {
                        println!("   Error: {}", error);
                    }
                    break;
                }
            }
        }
        Err(e) => {
            println!("❌ Token pool discovery failed: {}", e);
        }
    }

    println!("\n🏁 Test completed!");
    Ok(())
}
