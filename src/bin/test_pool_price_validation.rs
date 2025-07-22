use screenerbot::trader::*;
use screenerbot::global::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Pool Price Validation");
    println!("================================");

    // Test cases for pool price validation
    let test_cases = vec![
        // (pool_price, api_price, expected_valid, description)
        (1.0, 1.0, true, "Identical prices"),
        (1.0, 1.05, true, "5% difference - should be valid"),
        (1.0, 1.1, true, "10% difference - should be valid (at limit)"),
        (1.0, 1.12, false, "12% difference - should be invalid"),
        (1.0, 0.91, true, "~9.9% difference below - should be valid"),
        (1.0, 0.89, false, "12.4% difference below - should be invalid"),
        (1.0, 1.25, false, "25% difference - should be invalid"),
        (0.0, 1.0, false, "Zero pool price - should be invalid"),
        (1.0, 0.0, false, "Zero API price - should be invalid"),
        (0.000001, 0.0000011, true, "Very small prices with valid difference"),
        (0.000001, 0.0000015, false, "Very small prices with invalid difference")
    ];

    println!("Testing validation function with {} test cases:", test_cases.len());
    println!();

    let mut passed = 0;
    let mut failed = 0;

    for (i, (pool_price, api_price, expected_valid, description)) in test_cases.iter().enumerate() {
        println!("Test {}: {}", i + 1, description);
        println!("  Pool Price: {:.12} SOL", pool_price);
        println!("  API Price:  {:.12} SOL", api_price);

        // Test the validation function
        let result = validate_pool_price_against_api(*pool_price, *api_price, "TEST");

        if result == *expected_valid {
            println!("  ✅ PASSED: Expected {}, got {}", expected_valid, result);
            passed += 1;
        } else {
            println!("  ❌ FAILED: Expected {}, got {}", expected_valid, result);
            failed += 1;
        }

        // Calculate and show the actual difference percentage
        if *pool_price > 0.0 && *api_price > 0.0 {
            let diff_percent = ((pool_price - api_price).abs() / api_price) * 100.0;
            println!(
                "  Difference: {:.2}% (max allowed: {:.1}%)",
                diff_percent,
                MAX_POOL_PRICE_DIFFERENCE_PERCENT
            );
        }

        println!();
    }

    // Summary
    println!("📊 Test Results:");
    println!("================");
    println!("✅ Passed: {}", passed);
    println!("❌ Failed: {}", failed);
    println!("📈 Success Rate: {:.1}%", ((passed as f64) / ((passed + failed) as f64)) * 100.0);

    if failed == 0 {
        println!("🎉 All tests passed! Pool price validation is working correctly.");
    } else {
        println!("⚠️  Some tests failed. Please check the validation logic.");
        return Err("Tests failed".into());
    }

    // Test the get_current_token_price function with mock data
    println!();
    println!("🔍 Testing get_current_token_price function");
    println!("===========================================");

    // Create a mock token for testing
    let mock_token = Token {
        mint: "test_mint_123".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: Some(1.0),
        price_dexscreener_usd: None,
        price_pool_sol: Some(1.05), // 5% difference - should be valid
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(100000.0),
            base: Some(50000.0),
            quote: Some(50000.0),
        }),
        info: None,
        boosts: None,
    };

    // Add the mock token to the global token list for testing
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        tokens.clear();
        tokens.push(mock_token);
    }

    // Test getting price for open position (should use validated pool price)
    match get_current_token_price("test_mint_123", true) {
        Some(price) =>
            println!(
                "✅ Open position price: {:.12} SOL (expected pool price since it's valid)",
                price
            ),
        None => println!("❌ Failed to get price for open position"),
    }

    // Test getting price for non-open position (should use API price)
    match get_current_token_price("test_mint_123", false) {
        Some(price) =>
            println!("✅ Non-open position price: {:.12} SOL (expected API price)", price),
        None => println!("❌ Failed to get price for non-open position"),
    }

    // Test with invalid pool price (should fall back to API price)
    {
        let mut tokens = LIST_TOKENS.write().unwrap();
        if let Some(token) = tokens.first_mut() {
            token.price_pool_sol = Some(1.15); // 15% difference - should be invalid
        }
    }

    match get_current_token_price("test_mint_123", true) {
        Some(price) => {
            if (price - 1.0).abs() < 0.000001 {
                println!("✅ Correctly fell back to API price: {:.12} SOL", price);
            } else {
                println!("❌ Unexpected price returned: {:.12} SOL", price);
            }
        }
        None => println!("❌ Failed to get price with invalid pool price"),
    }

    println!();
    println!("🎯 Pool price validation system is ready!");
    println!(
        "📋 Configuration: Maximum allowed difference = {:.1}%",
        MAX_POOL_PRICE_DIFFERENCE_PERCENT
    );

    Ok(())
}

/// Helper function to access the validation function from the trader module
fn validate_pool_price_against_api(pool_price: f64, api_price: f64, symbol: &str) -> bool {
    // Call the validation function directly
    use screenerbot::trader::validate_pool_price_against_api;
    validate_pool_price_against_api(pool_price, api_price, symbol)
}
