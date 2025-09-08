/// Test Debug Force Buy Functionality
///
/// This tool demonstrates and tests the debug force buy feature in the trader module.
/// It shows how the force buy logic works when a token price drops by the configured threshold.

use screenerbot::trader::{
    should_debug_force_buy,
    DEBUG_FORCE_BUY_MODE,
    DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT,
};

fn main() {
    println!("🧪 Debug Force Buy Test Tool");
    println!("=============================");
    println!();

    // Show current configuration
    println!("📋 Current Configuration:");
    println!("  DEBUG_FORCE_BUY_MODE: {}", DEBUG_FORCE_BUY_MODE);
    println!("  DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT: {}%", DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT);
    println!();

    if !DEBUG_FORCE_BUY_MODE {
        println!("⚠️  DEBUG_FORCE_BUY_MODE is currently DISABLED");
        println!("   To enable, set DEBUG_FORCE_BUY_MODE = true in src/trader.rs");
        println!();
    }

    // Test scenarios
    println!("🔬 Testing Force Buy Scenarios:");
    println!();

    let test_cases = vec![
        ("No previous price", None, 0.001, "N/A"),
        ("No drop", Some(0.001), 0.001, "0.0%"),
        ("Small drop (1%)", Some(0.001), 0.00099, "1.0%"),
        ("Threshold drop (3%)", Some(0.001), 0.00097, "3.0%"),
        ("Large drop (5%)", Some(0.001), 0.00095, "5.0%"),
        ("Huge drop (10%)", Some(0.001), 0.0009, "10.0%"),
        ("Price increase", Some(0.001), 0.0011, "-10.0%")
    ];

    for (scenario, previous_price, current_price, expected_drop) in test_cases {
        let should_buy = should_debug_force_buy(current_price, previous_price, "TEST");
        let status = if should_buy { "✅ FORCE BUY" } else { "❌ No action" };

        println!(
            "  {:<20} | Prev: {:>12} | Curr: {:>10} | Drop: {:>6} | {}",
            scenario,
            previous_price.map(|p| format!("{:.6}", p)).unwrap_or_else(|| "None".to_string()),
            format!("{:.6}", current_price),
            expected_drop,
            status
        );
    }

    println!();
    println!("📝 Notes:");
    println!("  - Force buy only triggers when DEBUG_FORCE_BUY_MODE = true");
    println!("  - Requires available position slots (current positions < MAX_OPEN_POSITIONS)");
    println!("  - Drop threshold is configurable via DEBUG_FORCE_BUY_DROP_THRESHOLD_PERCENT");
    println!("  - In real trading, this overrides normal entry logic when conditions are met");
    println!();

    if DEBUG_FORCE_BUY_MODE {
        println!("🚨 WARNING: Debug force buy is currently ENABLED!");
        println!("   This will trigger automatic purchases when price drops are detected.");
        println!("   Make sure this is intended for your current environment.");
    }
}
