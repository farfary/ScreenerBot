use anyhow::Result;
use screenerbot::pool_price::PoolDiscoveryAndPricing;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Testing Pump.fun AMM Pool Price Calculation");
    println!("==============================================");

    // Load configuration
    let configs_json = std::fs
        ::read_to_string("configs.json")
        .expect("Failed to read configs.json");
    let configs: serde_json::Value = serde_json
        ::from_str(&configs_json)
        .expect("Failed to parse configs.json");

    let rpc_url = configs["rpc_url"].as_str().expect("rpc_url not found in configs.json");

    let pool_discovery = PoolDiscoveryAndPricing::new(rpc_url);

    // Test the specific Pump.fun AMM pool you provided
    let test_pool_address = "8koRLicQQcFn7cvqSC1gRZ5AJ6YieP5gv1ksSVkPGyou";
    let test_token_mint = "BDNPD38erhzRmu5qYLTLFAwmyyW5UvGryUu6TsJFpump";

    println!("\n🔍 Testing CRONG/WSOL Pump.fun AMM Pool:");
    println!("Pool Address: {}", test_pool_address);
    println!("Token Mint: {}", test_token_mint);
    println!("Expected Program ID: pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");

    // Test pool type detection
    match pool_discovery.detect_pool_type(test_pool_address).await {
        Ok(pool_type) => {
            println!("✅ Pool type detected: {:?}", pool_type);

            // Test price calculation with detected type
            println!("\n💰 Testing On-Chain Price Calculation:");
            match pool_discovery.calculate_pool_price_with_type(test_pool_address, pool_type).await {
                Ok((price, token_a, token_b, detected_type)) => {
                    println!("✅ Price calculation successful!");
                    println!("   Calculated Price: {} SOL per token", price);
                    println!("   Token A: {}", token_a);
                    println!("   Token B: {}", token_b);
                    println!("   Pool Type: {:?}", detected_type);

                    // Get DexScreener price for comparison
                    println!("\n📊 Comparing with DexScreener API:");
                    let api_price = get_dexscreener_price(test_token_mint).await;

                    if let Some(api_price) = api_price {
                        println!("   DexScreener Price: {} SOL", api_price);
                        let difference = ((price - api_price).abs() / api_price) * 100.0;
                        println!("   Price Difference: {:.2}%", difference);

                        if difference < 1.0 {
                            println!("✅ Excellent accuracy! (within 1%)");
                        } else if difference < 5.0 {
                            println!("✅ Good accuracy (within 5%)");
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

                            if
                                let screenerbot::pool_price::PoolSpecificData::PumpfunAmm {
                                    pool_bump,
                                    index,
                                    creator,
                                    lp_mint,
                                    lp_supply,
                                    coin_creator,
                                } = &pool_data.specific_data
                            {
                                println!("   Pump.fun Specific Data:");
                                println!("     Pool Bump: {}", pool_bump);
                                println!("     Index: {}", index);
                                println!("     Creator: {}", creator);
                                println!("     LP Mint: {}", lp_mint);
                                println!("     LP Supply: {}", lp_supply);
                                println!("     Coin Creator: {}", coin_creator);
                            }
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

    // Test pool discovery via API for multiple Pump.fun pools
    println!("\n🌐 Testing Pool Discovery via DexScreener API:");
    match pool_discovery.discover_pools(test_token_mint).await {
        Ok(pools) => {
            println!("✅ Discovered {} pools", pools.len());

            let mut pumpfun_pools = 0;
            for (i, pool) in pools.iter().enumerate() {
                println!(
                    "   Pool {}: {} ({}) - Liquidity: ${:.2}",
                    i + 1,
                    pool.pair_address,
                    pool.dex_id,
                    pool.liquidity_usd
                );

                if pool.dex_id.contains("pump") {
                    pumpfun_pools += 1;
                }
            }

            println!("   Found {} Pump.fun related pools", pumpfun_pools);

            // Test price calculation for all pools
            println!("\n💹 Testing Price Calculation for All Pools:");
            match pool_discovery.get_token_pool_prices(test_token_mint).await {
                Ok(results) => {
                    println!("✅ Results for {} pools:\n", results.len());

                    let mut successful_calcs = 0;
                    let mut pumpfun_success = 0;

                    for result in &results {
                        let is_pumpfun = result.dex_id.contains("pump");
                        let status_icon = if result.calculation_successful { "✅" } else { "❌" };

                        println!(
                            "{}  Pool: {} ({})",
                            status_icon,
                            result.pool_address,
                            result.dex_id
                        );

                        if result.calculation_successful {
                            successful_calcs += 1;
                            if is_pumpfun {
                                pumpfun_success += 1;
                            }

                            println!("     🎯 Calculated Price: {} SOL", result.calculated_price);
                            println!("     📊 DexScreener Price: {} SOL", result.dexscreener_price);
                            println!("     📈 Difference: {:.2}%", result.price_difference_percent);
                            println!("     💧 Liquidity: ${:.2}", result.liquidity_usd);
                            println!("     🏷️  Pool Type: {:?}", result.pool_type);
                        } else {
                            println!(
                                "     ❌ Error: {}",
                                result.error_message
                                    .as_ref()
                                    .unwrap_or(&"Unknown error".to_string())
                            );
                        }
                        println!();
                    }

                    println!("📈 Summary:");
                    println!("   Successful calculations: {}/{}", successful_calcs, results.len());
                    println!("   Pump.fun pools working: {}", pumpfun_success);

                    if pumpfun_success > 0 {
                        println!("🎉 Pump.fun AMM integration working successfully!");
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

    println!("\n🏁 Pump.fun AMM testing completed!");
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
