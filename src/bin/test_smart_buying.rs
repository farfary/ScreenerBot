use screenerbot::global::*;
use screenerbot::trader::{ should_buy, PRICE_HISTORY_24H, LAST_PRICES };
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Smart Buying Logic with Volatility Analysis");
    println!("{}", "=".repeat(60));

    // Create a test token with comprehensive data
    let test_token = Token {
        mint: "test_mint_123".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 6,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(48)), // 2 days old
        price_dexscreener_sol: Some(0.0001),
        price_dexscreener_usd: None,
        price_pool_sol: None,
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
            usd: Some(50000.0), // Medium liquidity
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    // Test 1: No price history - should not buy
    println!("\n📊 Test 1: No Price History");
    let current_price = 0.00008; // 20% drop
    let prev_price = 0.0001;

    let urgency = should_buy(&test_token, current_price, prev_price);
    println!("   Current: {:.8}, Previous: {:.8}", current_price, prev_price);
    println!("   Drop: {:.1}%", ((current_price - prev_price) / prev_price) * 100.0);
    println!("   Urgency: {:.2} (Expected: 0.0 - no history)", urgency);

    // Test 2: Create realistic price history showing volatility patterns
    println!("\n📊 Test 2: Adding Price History with Volatility Patterns");

    // Simulate 2 hours of price data with realistic volatility
    let mut price_history = Vec::new();
    let base_time = Utc::now() - Duration::hours(2);
    let mut price = 0.00012; // Starting higher

    // Create realistic price movements
    let movements = vec![
        -0.05,
        0.03,
        -0.08,
        0.02,
        -0.12,
        0.04,
        -0.06,
        0.08,
        -0.15,
        0.02,
        -0.1,
        0.06,
        -0.08,
        0.03,
        -0.2,
        0.05,
        -0.07,
        0.09,
        -0.11,
        0.03,
        -0.08,
        0.04,
        -0.18,
        0.02,
        -0.09,
        0.07,
        -0.12,
        0.04,
        -0.25,
        0.01
    ];

    for (i, movement) in movements.iter().enumerate() {
        let timestamp = base_time + Duration::minutes((i as i64) * 4);
        price = price * (1.0 + movement);
        price_history.push((timestamp, price));

        if i % 10 == 0 {
            println!("   Time +{}m: {:.8} ({:+.1}%)", i * 4, price, movement * 100.0);
        }
    }

    // Update global price history
    {
        let mut hist = PRICE_HISTORY_24H.lock().unwrap();
        hist.insert(test_token.mint.clone(), price_history);
    }

    // Update last prices
    {
        let mut last_prices = LAST_PRICES.lock().unwrap();
        last_prices.insert(test_token.mint.clone(), 0.00011);
    }

    // Test 3: Test buying with good dip conditions
    println!("\n📊 Test 3: Testing Smart Dip Detection");

    let current_price = 0.000085; // Significant drop from recent average
    let prev_price = 0.00011;
    let drop_percent = ((current_price - prev_price) / prev_price) * 100.0;

    let urgency = should_buy(&test_token, current_price, prev_price);
    println!("   Current: {:.8}, Previous: {:.8}", current_price, prev_price);
    println!("   Drop: {:.1}%", drop_percent);
    println!("   Urgency: {:.2}", urgency);

    if urgency > 0.0 {
        println!("   ✅ Smart buy signal detected!");
    } else {
        println!("   ❌ No buy signal (may not meet volatility criteria)");
    }

    // Test 4: Test with insufficient drop
    println!("\n📊 Test 4: Testing Insufficient Drop");

    let current_price = 0.000105; // Only 4.5% drop
    let prev_price = 0.00011;
    let drop_percent = ((current_price - prev_price) / prev_price) * 100.0;

    let urgency = should_buy(&test_token, current_price, prev_price);
    println!("   Current: {:.8}, Previous: {:.8}", current_price, prev_price);
    println!("   Drop: {:.1}%", drop_percent);
    println!("   Urgency: {:.2} (Expected: 0.0 - insufficient drop)", urgency);

    // Test 5: Test with high volatility token (different scale)
    println!("\n📊 Test 5: High Volatility Token");

    let mut high_vol_token = test_token.clone();
    high_vol_token.mint = "high_vol_mint".to_string();
    high_vol_token.symbol = "HVOL".to_string();

    // Create very volatile price history
    let mut high_vol_history = Vec::new();
    let mut vol_price = 0.0001;

    // Much larger movements for high volatility
    let vol_movements = vec![
        -0.25,
        0.2,
        -0.3,
        0.15,
        -0.35,
        0.28,
        -0.2,
        0.4,
        -0.45,
        0.18,
        -0.38,
        0.32,
        -0.28,
        0.22,
        -0.5,
        0.35,
        -0.33,
        0.45,
        -0.42,
        0.25
    ];

    for (i, movement) in vol_movements.iter().enumerate() {
        let timestamp = base_time + Duration::minutes((i as i64) * 6);
        vol_price = vol_price * (1.0 + movement);
        high_vol_history.push((timestamp, vol_price));
    }

    // Update global price history for high vol token
    {
        let mut hist = PRICE_HISTORY_24H.lock().unwrap();
        hist.insert(high_vol_token.mint.clone(), high_vol_history);
    }

    // Update last prices
    {
        let mut last_prices = LAST_PRICES.lock().unwrap();
        last_prices.insert(high_vol_token.mint.clone(), 0.00006);
    }

    let current_price = 0.00004; // 33% drop - big but normal for this token
    let prev_price = 0.00006;
    let drop_percent = ((current_price - prev_price) / prev_price) * 100.0;

    let urgency = should_buy(&high_vol_token, current_price, prev_price);
    println!("   Current: {:.8}, Previous: {:.8}", current_price, prev_price);
    println!("   Drop: {:.1}%", drop_percent);
    println!("   Urgency: {:.2}", urgency);

    if urgency > 0.0 {
        println!("   ✅ High volatility token buy signal (scale-adjusted)");
    } else {
        println!("   ❌ No buy signal for high volatility token");
    }

    // Test 6: Test with very young token (should be blocked)
    println!("\n📊 Test 6: Young Token (Age Filter)");

    let mut young_token = test_token.clone();
    young_token.mint = "young_mint".to_string();
    young_token.symbol = "YOUNG".to_string();
    young_token.created_at = Some(Utc::now() - Duration::hours(2)); // Only 2 hours old

    let current_price = 0.00007; // Big drop
    let prev_price = 0.0001;
    let drop_percent = ((current_price - prev_price) / prev_price) * 100.0;

    let urgency = should_buy(&young_token, current_price, prev_price);
    println!("   Current: {:.8}, Previous: {:.8}", current_price, prev_price);
    println!("   Drop: {:.1}%", drop_percent);
    println!("   Token Age: {} hours", (Utc::now() - young_token.created_at.unwrap()).num_hours());
    println!("   Urgency: {:.2} (Expected: 0.0 - too young)", urgency);

    println!("\n🎯 Smart Buying Analysis Summary");
    println!("{}", "=".repeat(60));
    println!("✅ Enhanced buying logic checks:");
    println!(
        "   • Token age validation (minimum {} hours)",
        screenerbot::trader::MIN_TOKEN_AGE_HOURS
    );
    println!("   • Volatility pattern analysis");
    println!("   • Genuine dip detection (vs fake-outs)");
    println!("   • Scale-adjusted urgency scoring");
    println!("   • Support/resistance level analysis");
    println!("   • Historical price pattern validation");
    println!("");
    println!("🧠 Key Improvements:");
    println!("   • Prevents buying during fake dips");
    println!("   • Scales thresholds to token's volatility");
    println!("   • Requires consistent downward momentum");
    println!("   • Validates dips against support levels");
    println!("   • Adjusts urgency based on pattern quality");

    Ok(())
}
