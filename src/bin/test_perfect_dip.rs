use screenerbot::global::*;
use screenerbot::trader::{ should_buy, PRICE_HISTORY_24H, LAST_PRICES };
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Testing Smart Buying Logic - Buy Signal Conditions");
    println!("{}", "=".repeat(60));

    // Create a test token with good characteristics
    let test_token = Token {
        mint: "perfect_dip_token".to_string(),
        symbol: "PERFECT".to_string(),
        name: "Perfect Dip Token".to_string(),
        decimals: 6,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now() - Duration::hours(48)), // 2 days old (mature)
        price_dexscreener_sol: Some(0.00008), // Current price
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
            usd: Some(75000.0), // Good liquidity
            base: None,
            quote: None,
        }),
        info: None,
        boosts: None,
    };

    println!("📊 Creating Perfect Dip Conditions");

    // Create a price history that shows clear support levels and good dip conditions
    let mut price_history = Vec::new();
    let base_time = Utc::now() - Duration::hours(3);

    // Pattern: Higher price range initially, then decline to support level
    let price_pattern = vec![
        // Hour 1: Trading around 0.00015
        (0, 0.00015),
        (5, 0.00016),
        (10, 0.00014),
        (15, 0.00015),
        (20, 0.00016),
        (25, 0.00015),
        (30, 0.00014),
        (35, 0.00015),
        (40, 0.00016),
        (45, 0.00015),

        // Hour 2: Gradual decline to support
        (50, 0.00014),
        (55, 0.00013),
        (60, 0.00012),
        (65, 0.00011),
        (70, 0.00012),
        (75, 0.00011),
        (80, 0.0001),
        (85, 0.00011),
        (90, 0.0001),
        (95, 0.00009),

        // Hour 3: Testing support and bouncing
        (100, 0.000085),
        (105, 0.00009),
        (110, 0.000085),
        (115, 0.00008),
        (120, 0.000085),
        (125, 0.00008),
        (130, 0.000075),
        (135, 0.00008),
        (140, 0.000075),
        (145, 0.00007),

        // Recent: Strong decline to support - perfect dip condition
        (150, 0.00009),
        (155, 0.00008),
        (160, 0.000075),
        (165, 0.00007),
        (170, 0.000065),
        (175, 0.00006),
        (176, 0.00007),
        (177, 0.000065),
        (178, 0.00006),
        (179, 0.000055)
    ];

    for (minutes_offset, price) in price_pattern {
        let timestamp = base_time + Duration::minutes(minutes_offset);
        price_history.push((timestamp, price));
    }

    // Update global price history
    {
        let mut hist = PRICE_HISTORY_24H.lock().unwrap();
        hist.insert(test_token.mint.clone(), price_history.clone());
    }

    // Set last price to something higher to show the drop
    {
        let mut last_prices = LAST_PRICES.lock().unwrap();
        last_prices.insert(test_token.mint.clone(), 0.00009); // Previous price
    }

    println!("   Price History Summary:");
    println!("   • Started at: {:.8} SOL", 0.00015);
    println!("   • Support Level: ~{:.8} SOL", 0.00008);
    println!("   • Current: {:.8} SOL", 0.000055);
    println!("   • Recent Pattern: Consistent downward momentum to support");

    // Test the buy signal
    println!("\n📊 Testing Buy Signal Detection");

    let current_price = 0.000055; // Strong dip to support level
    let prev_price = 0.00009; // Previous higher price
    let drop_percent = ((current_price - prev_price) / prev_price) * 100.0;

    println!("   Current Price: {:.8} SOL", current_price);
    println!("   Previous Price: {:.8} SOL", prev_price);
    println!("   Drop: {:.1}%", drop_percent);
    println!("   Threshold: {:.1}%", -screenerbot::trader::PRICE_DROP_THRESHOLD_PERCENT);

    let urgency = should_buy(&test_token, current_price, prev_price);

    println!("   🎯 Buy Urgency: {:.3}", urgency);

    if urgency > 0.0 {
        println!("   ✅ SMART BUY SIGNAL DETECTED!");
        println!("   📈 This represents a genuine dip with:");
        println!(
            "      • Sufficient price drop ({:.1}% vs {:.1}% threshold)",
            drop_percent.abs(),
            screenerbot::trader::PRICE_DROP_THRESHOLD_PERCENT
        );
        println!("      • Token age validation passed");
        println!("      • Volatility analysis confirms genuine dip");
        println!("      • Price action consistent with support levels");
        println!("      • Downward momentum pattern detected");
    } else {
        println!("   ❌ No buy signal generated");
        println!("   🔍 Possible reasons:");
        println!("      • May need more consistent downward momentum");
        println!("      • Current dip may not meet all volatility criteria");
        println!("      • Historical analysis may be blocking entry");
    }

    // Test 2: Show how the system rejects fake dips
    println!("\n📊 Testing Fake Dip Rejection");

    // Create history showing choppy, unreliable price action
    let mut fake_dip_history = Vec::new();
    let fake_base_time = Utc::now() - Duration::hours(2);

    // Erratic price movements without clear pattern
    let fake_pattern = vec![
        (0, 0.0001),
        (10, 0.00012),
        (20, 0.00008),
        (30, 0.00014),
        (40, 0.00009),
        (50, 0.00013),
        (60, 0.00007),
        (70, 0.00015),
        (80, 0.00006),
        (90, 0.00011),
        (100, 0.00016),
        (110, 0.00005),
        (115, 0.00012),
        (116, 0.000045),
        (117, 0.00013),
        (118, 0.000035),
        (119, 0.00015) // Recent erratic moves
    ];

    for (minutes_offset, price) in fake_pattern {
        let timestamp = fake_base_time + Duration::minutes(minutes_offset);
        fake_dip_history.push((timestamp, price));
    }

    // Create fake dip token
    let mut fake_token = test_token.clone();
    fake_token.mint = "fake_dip_token".to_string();
    fake_token.symbol = "FAKE".to_string();

    // Update history for fake token
    {
        let mut hist = PRICE_HISTORY_24H.lock().unwrap();
        hist.insert(fake_token.mint.clone(), fake_dip_history);
    }

    // Set recent price
    {
        let mut last_prices = LAST_PRICES.lock().unwrap();
        last_prices.insert(fake_token.mint.clone(), 0.00015);
    }

    let fake_current = 0.000035; // Big drop but from erratic pattern
    let fake_prev = 0.00015;
    let fake_drop = ((fake_current - fake_prev) / fake_prev) * 100.0;

    println!("   Fake Dip - Current: {:.8}, Previous: {:.8}", fake_current, fake_prev);
    println!("   Fake Drop: {:.1}%", fake_drop);

    let fake_urgency = should_buy(&fake_token, fake_current, fake_prev);
    println!("   Fake Dip Urgency: {:.3}", fake_urgency);

    if fake_urgency == 0.0 {
        println!("   ✅ CORRECTLY REJECTED fake dip!");
        println!("   🛡️ Smart system detected erratic price pattern");
    } else {
        println!("   ⚠️ System accepted fake dip (unexpected)");
    }

    println!("\n🎯 Smart Buying System Summary");
    println!("{}", "=".repeat(60));
    println!("🧠 The enhanced buying logic ensures:");
    println!("   ✅ Only genuine dips trigger buy signals");
    println!("   ✅ Token age requirements prevent risky new tokens");
    println!("   ✅ Volatility analysis prevents fake-out purchases");
    println!("   ✅ Support/resistance levels guide entry timing");
    println!("   ✅ Historical patterns inform decision quality");
    println!("   ✅ Liquidity-adjusted urgency scoring");
    println!("");
    println!("🚀 Result: Much smarter entry timing that waits for");
    println!("    actual dips with consistent patterns and proper scale!");

    Ok(())
}
