use screenerbot::pool_price::{ PoolPriceDiscovery, PoolType };
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Raydium LaunchLab pool discovery and price calculation");

    let pool_address = "oxw6tRHFzerBtcsnnWpaAidAUDTphWoc5XH5wjXox7i";
    println!("Pool address: {}", pool_address);

    let discovery = PoolPriceDiscovery::new()?;

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
    let pool_data = discovery.parse_pool_data(pool_address, &detected_type).await?;
    println!("✅ Successfully parsed pool data");

    println!("Token A: {} ({} decimals)", pool_data.token_a.mint, pool_data.token_a.decimals);
    println!("Token B: {} ({} decimals)", pool_data.token_b.mint, pool_data.token_b.decimals);
    println!("Reserve A: {} tokens", pool_data.reserve_a.balance);
    println!("Reserve B: {} tokens", pool_data.reserve_b.balance);

    // Test price calculation
    let calculated_price = discovery.calculate_price_from_pool_data(&pool_data).await?;
    println!("Calculated price: {} SOL per token", calculated_price);

    // Manual calculation using expected JSON data values
    println!("\n=== Manual Price Calculation Using Expected JSON Values ===");
    let expected_real_base = 793100000000000u64; // base amount
    let expected_real_quote = 85000000226u64; // quote amount
    let expected_base_decimals = 6u8; // base_decimals from JSON
    let expected_quote_decimals = 9u8; // quote_decimals from JSON

    let ui_real_base = (expected_real_base as f64) / (10_f64).powi(expected_base_decimals as i32);
    let ui_real_quote =
        (expected_real_quote as f64) / (10_f64).powi(expected_quote_decimals as i32);
    let expected_price = ui_real_quote / ui_real_base;

    println!(
        "Expected real_base: {} (raw), {} (UI with {} decimals)",
        expected_real_base,
        ui_real_base,
        expected_base_decimals
    );
    println!(
        "Expected real_quote: {} (raw), {} (UI with {} decimals)",
        expected_real_quote,
        ui_real_quote,
        expected_quote_decimals
    );
    println!("Expected price: {} = {} / {}", expected_price, ui_real_quote, ui_real_base);
    println!("Target price should be near: 0.0000000903");

    // Check if our calculation is close to target
    let target_price = 0.0000000903;
    let price_diff = (expected_price - target_price).abs();
    let percent_diff = (price_diff / target_price) * 100.0;

    if percent_diff < 10.0 {
        println!(
            "✅ Expected price calculation is close to target! Difference: {:.2}%",
            percent_diff
        );
    } else {
        println!(
            "❌ Expected price calculation is off by {:.2}%. Got: {}, Expected: {}",
            percent_diff,
            expected_price,
            target_price
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
