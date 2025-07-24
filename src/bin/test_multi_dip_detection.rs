use screenerbot::trader::*;
use screenerbot::global::*;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Multi-Strategy Dip Detection System");
    println!("================================================");

    // Create a test token
    let test_token = Token {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "SOL".to_string(),
        name: "Solana".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: Vec::new(),
        is_verified: true,
        created_at: Some(Utc::now() - chrono::Duration::hours(24)),
        price_dexscreener_sol: Some(0.01),
        price_dexscreener_usd: Some(100.0),
        price_pool_sol: Some(0.01),
        price_pool_usd: Some(100.0),
        pools: Vec::new(),
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: Vec::new(),
        fdv: None,
        market_cap: Some(50000000.0),
        txns: None,
        volume: Some(VolumeStats {
            h24: Some(1000000.0),
            h6: Some(250000.0),
            h1: Some(50000.0),
            m5: Some(10000.0),
        }),
        price_change: None,
        liquidity: Some(LiquidityInfo {
            usd: Some(500000.0),
            base: Some(5000.0),
            quote: Some(500000.0),
        }),
        info: None,
        boosts: None,
    };

    println!("Test Token: {} ({})", test_token.symbol, test_token.mint);

    // Test different dip scenarios
    let test_scenarios = vec![
        // (current_price, prev_price, expected_result, description)
        (0.0095, 0.01, true, "5% immediate drop - should trigger immediate drop strategy"),
        (0.009, 0.01, true, "10% immediate drop - should trigger multiple strategies"),
        (0.0085, 0.01, true, "15% immediate drop - should trigger strong signal"),
        (0.008, 0.01, true, "20% immediate drop - should trigger very strong signal"),
        (0.007, 0.01, true, "30% immediate drop - should trigger maximum signal"),
        (0.0097, 0.01, false, "3% drop - should not trigger (below 5% threshold)"),
        (0.01, 0.0095, false, "Price increase - should not trigger"),
        (0.0099, 0.01, false, "1% drop - too small should not trigger")
    ];

    println!("\nTesting various dip scenarios:");
    println!("------------------------------");

    for (i, (current_price, prev_price, expected_trigger, description)) in test_scenarios
        .iter()
        .enumerate() {
        let urgency = should_buy(&test_token, *current_price, *prev_price);
        let triggered = urgency > 0.0;
        let percent_change = ((*current_price - *prev_price) / *prev_price) * 100.0;

        let status = if triggered == *expected_trigger { "✅ PASS" } else { "❌ FAIL" };

        println!(
            "{} Test {}: {} | Change: {:.1}% | Urgency: {:.3} | {}",
            status,
            i + 1,
            description,
            percent_change,
            urgency,
            if triggered {
                "TRIGGERED"
            } else {
                "NOT TRIGGERED"
            }
        );
    }

    println!("\n🎯 Multi-Strategy Dip Detection Results:");
    println!("----------------------------------------");

    // Test a strong dip scenario with detailed logging
    let strong_dip_current = 0.008; // 20% drop
    let strong_dip_prev = 0.01;
    let urgency = should_buy(&test_token, strong_dip_current, strong_dip_prev);

    println!("Strong Dip Test (20% drop):");
    println!("  Current Price: {:.6}", strong_dip_current);
    println!("  Previous Price: {:.6}", strong_dip_prev);
    println!(
        "  Change: {:.1}%",
        ((strong_dip_current - strong_dip_prev) / strong_dip_prev) * 100.0
    );
    println!("  Final Urgency: {:.3}", urgency);
    println!("  Result: {}", if urgency > 0.0 { "BUY SIGNAL GENERATED" } else { "NO SIGNAL" });

    if urgency > 1.0 {
        println!("  🚀 HIGH URGENCY SIGNAL (Multi-strategy consensus!)");
    } else if urgency > 0.5 {
        println!("  📈 MEDIUM URGENCY SIGNAL");
    } else if urgency > 0.0 {
        println!("  📊 LOW URGENCY SIGNAL");
    }

    println!("\n✅ Multi-Strategy Dip Detection Test Complete!");

    Ok(())
}
