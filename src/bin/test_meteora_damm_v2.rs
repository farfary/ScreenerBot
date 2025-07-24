use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Meteora DAMM v2 Pool Price Calculation");
    println!("================================================");

    // Load configuration
    let configs_json = std::fs
        ::read_to_string("configs.json")
        .expect("Failed to read configs.json");
    let configs: serde_json::Value = serde_json
        ::from_str(&configs_json)
        .expect("Failed to parse configs.json");

    let rpc_url = configs["rpc_url"].as_str().expect("rpc_url not found in configs.json");

    let pool_discovery = PoolDiscoveryAndPricing::new(rpc_url);

    // Test the specific pool mentioned in the error
    let test_pool_address = "GAAxbSVm3sLKjFnAhQCmFjDPXkmbWWYP1F3xrN5LQs4n";
    let test_token_mint = "BDNPD38erhzRmu5qYLTLFAwmyyW5UvGryUu6TsJFpump";

    println!("\n🔍 Testing Pool Detection:");
    println!("Pool Address: {}", test_pool_address);
    println!("Token Mint: {}", test_token_mint);

    // Test pool type detection
    match pool_discovery.detect_pool_type(test_pool_address).await {
        Ok(pool_type) => {
            println!("✅ Pool type detected: {:?}", pool_type);

            // Test price calculation with detected type
            println!("\n💰 Testing Price Calculation:");
            match pool_discovery.calculate_pool_price_with_type(test_pool_address, pool_type).await {
                Ok((price, token_a, token_b, detected_type)) => {
                    println!("✅ Price calculation successful!");
                    println!("   Price: {} SOL per token", price);
                    println!("   Token A: {}", token_a);
                    println!("   Token B: {}", token_b);
                    println!("   Pool Type: {:?}", detected_type);

                    // Get DexScreener price for comparison
                    println!("\n📊 Fetching DexScreener API price for comparison:");
                    let api_price = get_dexscreener_price(test_token_mint).await;

                    if let Some(api_price) = api_price {
                        println!("   DexScreener Price: {} SOL", api_price);
                        let difference = ((price - api_price).abs() / api_price) * 100.0;
                        println!("   Price Difference: {:.2}%", difference);

                        if difference < 5.0 {
                            println!("✅ Price calculation is accurate (within 5%)");
                        } else {
                            println!("⚠️  Price difference is significant (>5%)");
                        }
                    } else {
                        println!("❌ Failed to get DexScreener price for comparison");
                    }
                }
                Err(e) => {
                    println!("❌ Price calculation failed: {}", e);

                    // Additional debugging - try to parse pool data manually
                    println!("\n🔧 Debug: Manual Pool Data Parsing:");
                    match pool_discovery.parse_pool_data(test_pool_address, pool_type).await {
                        Ok(pool_data) => {
                            println!("✅ Pool data parsed successfully:");
                            println!(
                                "   Token A: {} (decimals: {})",
                                pool_data.token_a.mint,
                                pool_data.token_a.decimals
                            );
                            println!(
                                "   Token B: {} (decimals: {})",
                                pool_data.token_b.mint,
                                pool_data.token_b.decimals
                            );
                            println!(
                                "   Reserve A: {} (vault: {})",
                                pool_data.reserve_a.balance,
                                pool_data.reserve_a.vault_address
                            );
                            println!(
                                "   Reserve B: {} (vault: {})",
                                pool_data.reserve_b.balance,
                                pool_data.reserve_b.vault_address
                            );
                        }
                        Err(parse_err) => {
                            println!("❌ Pool data parsing failed: {}", parse_err);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Pool type detection failed: {}", e);
        }
    }

    // Test pool discovery via API
    println!("\n🌐 Testing Pool Discovery via DexScreener API:");
    match pool_discovery.discover_pools(test_token_mint).await {
        Ok(pools) => {
            println!("✅ Discovered {} pools", pools.len());
            for (i, pool) in pools.iter().enumerate() {
                println!(
                    "   Pool {}: {} ({}) - Liquidity: ${:.2}",
                    i + 1,
                    pool.pair_address,
                    pool.dex_id,
                    pool.liquidity_usd
                );
            }

            // Test price calculation for all discovered pools
            println!("\n💹 Testing Price Calculation for All Pools:");
            match pool_discovery.get_token_pool_prices(test_token_mint).await {
                Ok(results) => {
                    println!("✅ Calculated prices for {} pools", results.len());
                    for result in &results {
                        println!("   Pool: {} ({})", result.pool_address, result.dex_id);
                        if result.calculation_successful {
                            println!("     ✅ Calculated Price: {} SOL", result.calculated_price);
                            println!("     📊 DexScreener Price: {} SOL", result.dexscreener_price);
                            println!("     📈 Difference: {:.2}%", result.price_difference_percent);
                            println!("     💧 Liquidity: ${:.2}", result.liquidity_usd);
                        } else {
                            println!(
                                "     ❌ Calculation failed: {}",
                                result.error_message
                                    .as_ref()
                                    .unwrap_or(&"Unknown error".to_string())
                            );
                        }
                        println!();
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get pool prices: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Pool discovery failed: {}", e);
        }
    }

    println!("\n🏁 Test completed!");
    Ok(())
}

async fn get_dexscreener_price(token_mint: &str) -> Option<f64> {
    let url = format!("https://api.dexscreener.com/token-pairs/v1/solana/{}", token_mint);

    match reqwest::get(&url).await {
        Ok(response) => {
            if response.status().is_success() {
                if let Ok(pairs) = response.json::<Vec<serde_json::Value>>().await {
                    // Find SOL pair and get price
                    for pair in pairs {
                        if let Some(quote_token) = pair.get("quoteToken") {
                            if let Some(quote_address) = quote_token.get("address") {
                                if
                                    quote_address.as_str() ==
                                    Some("So11111111111111111111111111111111111111112")
                                {
                                    // This is a SOL pair
                                    if let Some(price_native) = pair.get("priceNative") {
                                        if let Some(price_str) = price_native.as_str() {
                                            if let Ok(price) = price_str.parse::<f64>() {
                                                return Some(price);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to fetch DexScreener price: {}", e);
        }
    }
    None
}
