use chrono::{ Utc, Duration };
use screenerbot::global::*;
use screenerbot::positions::*;
use screenerbot::profit::*;
use screenerbot::logger::{ log, LogTag };

fn main() {
    println!("🎯 Testing IMPROVED Duration-Based Profit System");
    println!("==================================================");

    // Create a mock token with some price changes
    let mock_token = Token {
        mint: "TestMint123".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        decimals: 9,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![], // Vec<String>, not Option
        is_verified: false, // bool, not Option<bool>
        created_at: None,
        price_dexscreener_sol: Some(0.001),
        price_dexscreener_usd: Some(0.1),
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![], // Vec<Pool>
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![], // Vec<String>
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: Some(PriceChangeStats {
            m5: Some(5.0), // 5% gain in 5 minutes
            h1: Some(10.0), // 10% gain in 1 hour
            h6: Some(15.0),
            h24: Some(20.0),
        }),
        liquidity: Some(LiquidityInfo {
            usd: Some(100000.0),
            base: Some(1000000.0),
            quote: Some(100000.0),
        }),
        info: None,
        boosts: None,
    };

    // Test scenarios with different time durations and profit levels
    let test_scenarios = vec![
        // Short duration tests (< 2 hours)
        ("1 hour, 7% profit", 1.0, 7.0, "Should sell - above 5% target for short duration"),
        ("1 hour, 3% profit", 1.0, 3.0, "Should hold - below 5% target"),
        ("0.5 hour, 60% profit", 0.5, 60.0, "Should sell immediately - high profit"),

        // Medium duration tests (2-6 hours)
        ("3 hours, 10% profit", 3.0, 10.0, "Should sell - above 8% target for medium duration"),
        ("4 hours, 6% profit", 4.0, 6.0, "Should hold - below 8% target"),
        ("5 hours, 120% profit", 5.0, 120.0, "Should sell immediately - very high profit"),

        // Long duration tests (6-24 hours)
        ("12 hours, 15% profit", 12.0, 15.0, "Should sell - above 12% target for long duration"),
        ("18 hours, 8% profit", 18.0, 8.0, "Should hold - below 12% target"),
        ("20 hours, 600% profit", 20.0, 600.0, "Should sell immediately - extreme profit"),

        // Very long duration tests (> 24 hours)
        (
            "30 hours, 25% profit",
            30.0,
            25.0,
            "Should sell - above 20% target for very long duration",
        ),
        ("48 hours, 15% profit", 48.0, 15.0, "Should hold - below 20% target but time pressure"),
        ("72 hours, 10% profit", 72.0, 10.0, "Should sell - time pressure after 72 hours")
    ];

    println!("\n📊 Testing Duration-Based Profit Targets:");
    println!("─────────────────────────────────────────");

    for (scenario, hours_held, profit_percent, expected) in test_scenarios {
        // Create a position with specific profit
        let entry_price = 1.0;
        let current_price = entry_price * (1.0 + profit_percent / 100.0);
        let time_held_seconds = hours_held * 3600.0;

        let position = Position {
            mint: "TestMint123".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            entry_price,
            entry_time: Utc::now() - Duration::seconds(time_held_seconds as i64),
            exit_price: None,
            exit_time: None,
            position_type: "buy".to_string(),
            entry_size_sol: 0.001,
            total_size_sol: 0.001,
            price_highest: current_price,
            price_lowest: entry_price * 0.9,
            entry_transaction_signature: Some("test_signature".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(1000000),
            effective_entry_price: Some(entry_price),
            effective_exit_price: None,
            sol_received: None,
        };

        // Test the improved profit system
        let (urgency, reason) = should_sell_smart_system(
            &position,
            &mock_token,
            current_price,
            time_held_seconds
        );

        // Determine the required profit target for this duration
        let required_profit = get_profit_target_for_duration(hours_held);

        // Format output with decision
        let decision = if urgency > 0.5 {
            "💰 SELL"
        } else if urgency > 0.2 {
            "⚠️  CONSIDER"
        } else {
            "⏳ HOLD"
        };

        println!("🔹 {}", scenario);
        println!(
            "   Duration: {:.1}h | Profit: {:.1}% | Target: {:.1}% | Urgency: {:.2}",
            hours_held,
            profit_percent,
            required_profit,
            urgency
        );
        println!("   Decision: {} | Reason: {}", decision, reason);
        println!("   Expected: {}", expected);
        println!();
    }

    println!("🎯 Testing High Profit Protection:");
    println!("───────────────────────────────────");

    // Test extreme profit scenarios
    let extreme_scenarios = vec![
        (50.0, "High Profit - Quick Exit"),
        (100.0, "Very High Profit - High Urgency"),
        (500.0, "Extreme Profit - Maximum Urgency"),
        (1000.0, "Moonshot - Immediate Exit")
    ];

    for (profit_percent, description) in extreme_scenarios {
        let entry_price = 1.0;
        let current_price = entry_price * (1.0 + profit_percent / 100.0);
        let time_held_seconds = 3600.0; // 1 hour

        let position = Position {
            mint: "TestMint123".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            entry_price,
            entry_time: Utc::now() - Duration::seconds(time_held_seconds as i64),
            exit_price: None,
            exit_time: None,
            position_type: "buy".to_string(),
            entry_size_sol: 0.001,
            total_size_sol: 0.001,
            price_highest: current_price,
            price_lowest: entry_price * 0.9,
            entry_transaction_signature: Some("test_signature".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(1000000),
            effective_entry_price: Some(entry_price),
            effective_exit_price: None,
            sol_received: None,
        };

        let (urgency, reason) = should_sell_smart_system(
            &position,
            &mock_token,
            current_price,
            time_held_seconds
        );

        println!(
            "💎 {}: {:.0}% profit → Urgency: {:.2} → {}",
            description,
            profit_percent,
            urgency,
            reason
        );
    }

    println!("\n🎯 Testing Momentum Analysis:");
    println!("──────────────────────────────");

    // Test different momentum scenarios
    let momentum_scenarios = vec![
        ("Strong momentum", 20.0, 15.0, "Strong upward momentum"),
        ("Weak momentum", 2.0, 1.0, "Weak momentum"),
        ("Fading momentum", -1.0, 0.5, "Momentum fading"),
        ("No momentum", 0.0, 0.0, "No momentum")
    ];

    for (description, hourly_change, recent_change, expected) in momentum_scenarios {
        let position = Position {
            mint: "TestMint123".to_string(),
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            entry_price: 1.0,
            entry_time: Utc::now() - Duration::seconds(7200), // 2 hours ago
            exit_price: None,
            exit_time: None,
            position_type: "buy".to_string(),
            entry_size_sol: 0.001,
            total_size_sol: 0.001,
            price_highest: 1.1,
            price_lowest: 0.95,
            entry_transaction_signature: Some("test_signature".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(1000000),
            effective_entry_price: Some(1.0),
            effective_exit_price: None,
            sol_received: None,
        };

        let mut test_token = mock_token.clone();
        test_token.price_change = Some(PriceChangeStats {
            m5: Some(recent_change),
            h1: Some(hourly_change),
            h6: Some(hourly_change),
            h24: Some(hourly_change),
        });

        let momentum = analyze_simple_momentum(&test_token, 1.1, &position, 2.0);

        println!(
            "📈 {}: H1: {:.1}% | Recent: {:.1}% | Strong: {} | Weak: {} | Fading: {}",
            description,
            hourly_change,
            recent_change,
            momentum.is_momentum_strong,
            momentum.is_momentum_weak,
            momentum.is_momentum_fading
        );
        println!("   Expected: {}", expected);
    }

    println!("\n✅ IMPROVED PROFIT SYSTEM TESTED!");
    println!("=====================================");
    println!("🎯 Key improvements:");
    println!("• Duration-based profit targets (5% → 8% → 12% → 20%)");
    println!("• High profit protection (50% → 100% → 500%+)");
    println!("• Time pressure for very long positions (72h+)");
    println!("• Simplified momentum analysis");
    println!("• Better hold patience for longer-term gains");
}
