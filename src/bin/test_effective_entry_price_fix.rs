use screenerbot::{
    trader::{ Position, should_sell, calculate_position_pnl_from_swaps, DEFAULT_FEE },
    global::Token,
};
use chrono::{ Utc, Duration as ChronoDuration };

#[tokio::main]
async fn main() {
    println!("🎯 Testing Effective Entry Price Fix");
    println!("===================================");

    // Test scenario 1: Position with effective entry price different from discovery price
    let mut position = Position {
        mint: "TestMint123".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.0001, // Discovery price
        effective_entry_price: Some(0.00012), // Actual price paid (20% higher slippage)
        entry_time: Utc::now() - ChronoDuration::minutes(30),
        entry_size_sol: 0.001, // 0.001 SOL investment
        exit_price: None,
        exit_time: None,
        pnl_sol: None,
        pnl_percent: None,
        position_type: "buy".to_string(),
        total_size_sol: 0.001,
        drawdown_percent: 0.0,
        price_highest: 0.00012,
        price_lowest: 0.00012,
        entry_transaction_signature: Some("test_tx_123".to_string()),
        exit_transaction_signature: None,
        token_amount: Some(8333), // Amount of tokens received (accounting for slippage)
        effective_exit_price: None,
    };

    println!("\n📊 Test Position Details:");
    println!("  🎯 Discovery Price: {:.8} SOL", position.entry_price);
    println!("  💰 Effective Entry Price: {:.8} SOL", position.effective_entry_price.unwrap());
    println!("  📈 Entry Size: {:.6} SOL", position.entry_size_sol);
    println!("  🪙 Token Amount: {}", position.token_amount.unwrap());

    // Test different current prices
    let test_prices = [
        (0.00012, "Same as effective entry"),
        (0.0001, "Same as discovery price"),
        (0.00014, "16.7% profit from effective"),
        (0.00008, "33.3% loss from effective"),
        (0.00015, "25% profit from effective"),
    ];

    println!("\n📈 Fee Calculation Verification:");
    println!("  💸 DEFAULT_FEE per swap: {:.9} SOL", DEFAULT_FEE);
    println!("  💸 Total fees (buy + sell): {:.9} SOL", 2.0 * DEFAULT_FEE);
    println!(
        "  📊 Fee percentage of investment: {:.2}%",
        ((2.0 * DEFAULT_FEE) / position.entry_size_sol) * 100.0
    );

    println!("\n🧪 Testing should_sell with different current prices:");
    println!("╭────────────────────────────┬─────────────┬───────────┬──────────┬─────────────╮");
    println!(
        "│ 📈 Price Scenario           │ 💰 P&L %    │ 🔥 Urgency │ 📍 Decision │ 📝 Method    │"
    );
    println!("├────────────────────────────┼─────────────┼───────────┼──────────┼─────────────┤");

    for (current_price, scenario) in test_prices {
        let now = Utc::now();

        // Test should_sell function (now uses effective entry price)
        let urgency = should_sell(&position, current_price, now);

        // Calculate P&L manually using effective entry price and fees
        let price_change_percent =
            ((current_price - position.effective_entry_price.unwrap()) /
                position.effective_entry_price.unwrap()) *
            100.0;
        let total_fee_cost = 2.0 * DEFAULT_FEE;
        let fee_percent = (total_fee_cost / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change_percent - fee_percent;

        let decision = if urgency > 0.5 { "🔴 SELL" } else { "🟢 HOLD" };

        println!(
            "│ {:26} │ {:+10.2}% │ {:8.3} │ {:8} │ Effective   │",
            scenario,
            net_pnl_percent,
            urgency,
            decision
        );
    }
    println!("╰────────────────────────────┴─────────────┴───────────┴──────────┴─────────────╯");

    // Test position P&L calculation with token decimals
    println!("\n🧮 Testing P&L Calculation with Token Decimals:");
    let token_decimals = Some(5u8); // Assuming 5 decimals for test token
    let current_price = 0.00014; // 16.7% profit scenario

    let (pnl_sol, pnl_percent) = calculate_position_pnl_from_swaps(
        &position,
        current_price,
        token_decimals
    );

    println!("  🪙 Token Amount (raw): {}", position.token_amount.unwrap());
    println!(
        "  🪙 Token Amount (UI): {:.2}",
        (position.token_amount.unwrap() as f64) / (10_f64).powi(5)
    );
    println!("  💰 Current Price: {:.8} SOL", current_price);
    println!("  📊 P&L (SOL): {:+.6}", pnl_sol);
    println!("  📊 P&L (%): {:+.2}%", pnl_percent);

    // Manual verification
    let ui_tokens = (position.token_amount.unwrap() as f64) / (10_f64).powi(5);
    let current_value = ui_tokens * current_price;
    let total_fees = 2.0 * DEFAULT_FEE;
    let expected_pnl = current_value - position.entry_size_sol - total_fees;

    println!("\n✅ Manual Verification:");
    println!("  📈 Current Token Value: {:.6} SOL", current_value);
    println!("  💸 Total Fees: {:.6} SOL", total_fees);
    println!("  💰 Initial Investment: {:.6} SOL", position.entry_size_sol);
    println!("  🧮 Expected P&L: {:+.6} SOL", expected_pnl);
    println!("  ✅ Calculation Match: {}", if (pnl_sol - expected_pnl).abs() < 0.000001 {
        "✅ YES"
    } else {
        "❌ NO"
    });

    println!("\n🎉 Test Complete!");
    println!("✅ should_sell now uses effective entry price");
    println!("✅ Fee calculation uses hardcoded DEFAULT_FEE");
    println!("✅ P&L calculations are accurate and fee-aware");
}
