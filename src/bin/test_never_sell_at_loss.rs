/// Test to verify the "NEVER SELL AT LOSS" protection system
/// This test validates that all sell decision functions properly enforce the no-loss rule

use screenerbot::trader::{ should_sell, STOP_LOSS_PERCENT };
use screenerbot::profit::should_sell_smart_system;
use screenerbot::positions::Position;
use screenerbot::global::Token;
use chrono::{ Utc, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔒 Testing NEVER SELL AT LOSS Protection System");
    println!("==================================================");

    // Create a test position with realistic values
    let entry_time = Utc::now() - Duration::minutes(30); // 30 minutes ago
    let test_position = Position {
        mint: "TestToken123".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_time,
        entry_price: 0.00001, // Entered at 0.00001 SOL per token
        entry_size_sol: 0.1, // 0.1 SOL invested (realistic amount)
        effective_entry_price: Some(0.00001),
        token_amount: Some(10000000000), // 10B tokens (realistic with decimals)
        price_highest: 0.00001,
        price_lowest: 0.00001,
        exit_time: None,
        exit_price: None,
        position_type: "buy".to_string(),
        total_size_sol: 0.1,
        entry_transaction_signature: Some("test_sig".to_string()),
        exit_transaction_signature: None,
        effective_exit_price: None,
        sol_received: None,
    };

    // Create a test token
    let test_token = Token {
        mint: "TestToken123".to_string(),
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
        price_dexscreener_sol: None,
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
        liquidity: None,
        info: None,
        boosts: None,
    };

    let now = Utc::now();

    // Test scenarios with different price levels (avoiding extreme values that trigger edge cases)
    let test_scenarios = vec![
        (0.000008, -20.0, "20% loss"), // Should not sell
        (0.0000095, -5.0, "5% loss"), // Should not sell
        (0.0000099, -1.0, "1% loss"), // Should not sell
        (0.00001, 0.0, "break even"), // Should not sell
        (0.0000105, 5.0, "5% profit"), // May sell for profit
        (0.000012, 20.0, "20% profit"), // Should sell for profit
        (0.000015, 50.0, "50% profit") // Should sell for profit
    ];

    println!("Testing should_sell() function:");
    println!("--------------------------------");

    for (current_price, expected_pnl, description) in &test_scenarios {
        let sell_urgency = should_sell(&test_position, *current_price, now);

        let should_sell_result = if *expected_pnl <= STOP_LOSS_PERCENT {
            "🚨 EMERGENCY SELL"
        } else if *expected_pnl < 0.0 {
            "🔒 HOLD (NEVER SELL AT LOSS)"
        } else if sell_urgency > 0.0 {
            "💰 SELL (PROFIT)"
        } else {
            "⏳ HOLD"
        };

        println!(
            "Price: ${:.3} ({}) → Urgency: {:.2} → {}",
            current_price,
            description,
            sell_urgency,
            should_sell_result
        );

        // Validate the logic - the key test is that we never sell at loss
        if *expected_pnl < 0.0 && sell_urgency > 0.0 {
            // Calculate the actual P&L to see what the system calculated
            let (_, actual_pnl) = screenerbot::positions::calculate_position_pnl(
                &test_position,
                Some(*current_price)
            );
            panic!(
                "❌ CRITICAL ERROR: should_sell() would sell at {:.1}% expected loss! (System calculated: {:.2}% P&L)",
                expected_pnl,
                actual_pnl
            );
        }
    }

    println!("\nTesting should_sell_smart_system() function:");
    println!("--------------------------------------------");

    for (current_price, expected_pnl, description) in &test_scenarios {
        let (sell_urgency, reason) = should_sell_smart_system(
            &test_position,
            &test_token,
            *current_price,
            1800.0 // 30 minutes held
        );

        let should_sell_result = if *expected_pnl <= -99.9 {
            "🚨 EMERGENCY SELL"
        } else if *expected_pnl < 0.0 {
            "🔒 HOLD (NEVER SELL AT LOSS)"
        } else if sell_urgency > 0.0 {
            "💰 SELL (PROFIT)"
        } else {
            "⏳ HOLD"
        };

        println!(
            "Price: ${:.3} ({}) → Urgency: {:.2} → {} | {}",
            current_price,
            description,
            sell_urgency,
            should_sell_result,
            reason
        );

        // Validate the logic - the key test is that we never sell at loss
        if *expected_pnl < 0.0 && sell_urgency > 0.0 {
            // Calculate the actual P&L to see what the system calculated
            let (_, actual_pnl) = screenerbot::positions::calculate_position_pnl(
                &test_position,
                Some(*current_price)
            );
            panic!(
                "❌ CRITICAL ERROR: should_sell_smart_system() would sell at {:.1}% expected loss! (System calculated: {:.2}% P&L)",
                expected_pnl,
                actual_pnl
            );
        }
    }

    println!("\n✅ ALL TESTS PASSED!");
    println!("====================================");
    println!("🔒 NEVER SELL AT LOSS system is working correctly");
    println!("🚨 Emergency exit only at {:.1}% loss", STOP_LOSS_PERCENT);
    println!("💰 All other exits are profit-only");
    println!("⏳ Positions at loss are held indefinitely for recovery");

    Ok(())
}
