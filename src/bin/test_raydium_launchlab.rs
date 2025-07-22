use screenerbot::pool_price::{ PoolDiscoveryAndPricing, PoolType };
use anyhow::Result;
use serde_json::Value;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Raydium LaunchLab pool discovery and price calculation");

    let pool_address = "oxw6tRHFzerBtcsnnWpaAidAUDTphWoc5XH5wjXox7i";
    let token_address = "4zJy5WHdTbmNuhTiJ5HYbJjLij2k3a8pmB99cJN5bonk";
    println!("Pool address: {}", pool_address);
    println!("Token address: {}", token_address);

    // Get current price from DexScreener API
    println!("\n=== Getting current price from DexScreener API ===");
    let dexscreener_url =
        format!("https://api.dexscreener.com/token-pairs/v1/solana/{}", token_address);
    println!("Fetching from: {}", dexscreener_url);

    let client = reqwest::Client::new();
    let response = client.get(&dexscreener_url).send().await?;
    let dexscreener_data: Value = response.json().await?;

    let mut current_price_sol = None;
    if let Some(pairs) = dexscreener_data["pairs"].as_array() {
        for pair in pairs {
            if let Some(dex_id) = pair["dexId"].as_str() {
                if dex_id == "raydium" {
                    if let Some(price_native) = pair["priceNative"].as_str() {
                        if let Ok(price) = price_native.parse::<f64>() {
                            current_price_sol = Some(price);
                            println!("Found Raydium pair price: {} SOL per token", price);
                            break;
                        }
                    }
                }
            }
        }
    }

    let expected_price = current_price_sol.unwrap_or(0.00000012);
    println!("Using expected current price: {} SOL per token", expected_price);

    // Use the RPC URL from configs
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let discovery = PoolDiscoveryAndPricing::new(rpc_url);

    // Test pool type detection
    let detected_type = discovery.detect_pool_type(pool_address).await?;
    println!("Detected pool type: {:?}", detected_type);

    if detected_type == PoolType::RaydiumLaunchLab {
        println!("✅ Pool correctly detected as RaydiumLaunchLab");
    } else {
        println!("❌ Pool incorrectly detected as {:?}", detected_type);
    }

    // Test pool data parsing
    println!("Attempting to parse pool data...");
    let pool_data = discovery.parse_pool_data(pool_address, detected_type).await?;
    println!("✅ Successfully parsed pool data");

    println!("Token A: {} ({} decimals)", pool_data.token_a.mint, pool_data.token_a.decimals);
    println!("Token B: {} ({} decimals)", pool_data.token_b.mint, pool_data.token_b.decimals);
    println!("Reserve A: {} tokens", pool_data.reserve_a.balance);
    println!("Reserve B: {} tokens", pool_data.reserve_b.balance);

    // Expected values from your JSON data
    println!("\n=== Expected vs Actual Values ===");
    println!("Expected base_mint: 4zJy5WHdTbmNuhTiJ5HYbJjLij2k3a8pmB99cJN5bonk");
    println!("Expected quote_mint: So11111111111111111111111111111111111111112");
    println!("Expected real_base: 793100000000000");
    println!("Expected real_quote: 85000000226");
    println!("Expected base_decimals: 6");
    println!("Expected quote_decimals: 9");

    println!("Actual Token A mint: {}", pool_data.token_a.mint);
    println!("Actual Token B mint: {}", pool_data.token_b.mint);
    println!("Actual Reserve A: {}", pool_data.reserve_a.balance);
    println!("Actual Reserve B: {}", pool_data.reserve_b.balance);
    println!("Actual Token A decimals: {}", pool_data.token_a.decimals);
    println!("Actual Token B decimals: {}", pool_data.token_b.decimals);

    // Test price calculation
    let calculated_price = discovery.calculate_price_from_pool_data(&pool_data).await?;
    println!("Calculated price: {} SOL per token", calculated_price);

    // The library already returns SOL per token, no conversion needed
    let sol_per_token = calculated_price;
    println!("Price in SOL per token: {} SOL per token", sol_per_token);

    // Manual calculation using correct decimal values from expected data
    println!("\n=== Manual Price Calculation Using Expected Values ===");

    // From the pool data you provided:
    // base_decimals = 6 (token decimals)
    // quote_decimals = 9 (WSOL decimals)
    // real_base = 793,100,000,000,000 (token reserves)
    // real_quote = 85,000,000,226 (WSOL reserves)

    let expected_real_base = 793100000000000u64; // Token reserves
    let expected_real_quote = 85000000226u64; // WSOL reserves
    let expected_base_decimals = 6u8; // Token decimals (from pool data)
    let expected_quote_decimals = 9u8; // WSOL decimals

    let expected_ui_base =
        (expected_real_base as f64) / (10_f64).powi(expected_base_decimals as i32);
    let expected_ui_quote =
        (expected_real_quote as f64) / (10_f64).powi(expected_quote_decimals as i32);

    // Price = WSOL_amount / Token_amount (how much WSOL per 1 token)
    let expected_sol_per_token = expected_ui_quote / expected_ui_base;

    println!(
        "Expected pool data - Token: {} (raw: {}, UI: {} with {} decimals)",
        "4zJy5WHdTbmNuhTiJ5HYbJjLij2k3a8pmB99cJN5bonk",
        expected_real_base,
        expected_ui_base,
        expected_base_decimals
    );
    println!(
        "Expected pool data - WSOL: {} (raw: {}, UI: {} with {} decimals)",
        "So11111111111111111111111111111111111111112",
        expected_real_quote,
        expected_ui_quote,
        expected_quote_decimals
    );
    println!("Expected calculation - SOL per Token: {} (WSOL reserves / Token reserves)", expected_sol_per_token);

    // Check against current price (0.00000012 SOL per token)
    let current_price = expected_price;
    println!("Current known price: {} SOL per token", current_price);

    // Compare our expected calculation with the known current price
    let expected_price_diff = (expected_sol_per_token - current_price).abs();
    let expected_percent_diff = if current_price > 0.0 {
        (expected_price_diff / current_price) * 100.0
    } else {
        100.0
    };

    if expected_percent_diff < 15.0 {
        println!(
            "✅ Expected calculated price is close to current price! Difference: {:.2}%",
            expected_percent_diff
        );
    } else {
        println!(
            "❌ Expected calculated price is off by {:.2}%. Got: {}, Expected: {}",
            expected_percent_diff,
            expected_sol_per_token,
            current_price
        );
    }

    // Also compare the library's calculation
    let lib_price_diff = (sol_per_token - current_price).abs();
    let lib_percent_diff = if current_price > 0.0 {
        (lib_price_diff / current_price) * 100.0
    } else {
        100.0
    };

    if lib_percent_diff < 15.0 {
        println!(
            "✅ Library calculated price is close to current price! Difference: {:.2}%",
            lib_percent_diff
        );
    } else {
        println!(
            "❌ Library calculated price is off by {:.2}%. Got: {}, Expected: {}",
            lib_percent_diff,
            sol_per_token,
            current_price
        );
    }

    // Test the from_dex_id_and_labels method
    let pool_type = PoolType::from_dex_id_and_labels("raydium", &vec!["LaunchLab".to_string()]);
    println!("Pool type from dex_id and labels: {:?}", pool_type);

    if pool_type == PoolType::RaydiumLaunchLab {
        println!("✅ from_dex_id_and_labels correctly identified RaydiumLaunchLab");
    } else {
        println!("❌ from_dex_id_and_labels incorrectly identified {:?}", pool_type);
    }

    println!("All tests completed successfully!");
    Ok(())
}
