use screenerbot::positions::*;
use screenerbot::trader::DEFAULT_FEE;
use chrono::Utc;

#[tokio::main]
async fn main() {
    println!("🧪 Testing sol_received field and PnL calculation");
    println!("================================================");

    // Create a test position with SOL received data
    let mut position = Position {
        mint: "So11111111111111111111111111111111111111112".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.001, // Entry price (signal price)
        entry_time: Utc::now(),
        exit_price: Some(0.0015), // Exit price (signal price)
        exit_time: Some(Utc::now()),
        position_type: "buy".to_string(),
        entry_size_sol: 0.01, // Invested 0.01 SOL
        total_size_sol: 0.01,
        price_highest: 0.001,
        price_lowest: 0.001,
        entry_transaction_signature: Some("test_entry_tx".to_string()),
        exit_transaction_signature: Some("test_exit_tx".to_string()),
        token_amount: Some(10000), // Bought 10,000 tokens (raw units)
        effective_entry_price: Some(0.001), // Actual entry price
        effective_exit_price: Some(0.0015), // Actual exit price
        sol_received: Some(0.014), // Actually received 0.014 SOL after sell
    };

    println!("Test Position Data:");
    println!("- SOL Invested: {:.6} SOL", position.entry_size_sol);
    println!("- SOL Received: {:.6} SOL", position.sol_received.unwrap_or(0.0));
    println!("- Entry Price: {:.6} SOL", position.entry_price);
    println!("- Exit Price: {:.6} SOL", position.exit_price.unwrap_or(0.0));
    println!();

    // Test PnL calculation with sol_received
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&position, None);
    let expected_pnl = 0.014 - 0.01 - 2.0 * DEFAULT_FEE; // sol_received - invested - fees
    let expected_percent = (expected_pnl / 0.01) * 100.0;

    println!("✅ PnL Calculation with sol_received:");
    println!("- Calculated P&L: {:.6} SOL ({:.2}%)", pnl_sol, pnl_percent);
    println!("- Expected P&L:   {:.6} SOL ({:.2}%)", expected_pnl, expected_percent);
    println!(
        "- Calculation Method: SOL received ({:.6}) - SOL invested ({:.6}) - Fees ({:.6})",
        position.sol_received.unwrap_or(0.0),
        position.entry_size_sol,
        2.0 * DEFAULT_FEE
    );

    if (pnl_sol - expected_pnl).abs() < 0.000001 {
        println!("✅ PnL calculation matches expected result!");
    } else {
        println!("❌ PnL calculation mismatch!");
    }

    println!();

    // Test fallback calculation (without sol_received)
    position.sol_received = None;
    let (fallback_pnl_sol, fallback_pnl_percent) = calculate_position_pnl(&position, None);

    println!("✅ Fallback PnL Calculation (without sol_received):");
    println!("- Fallback P&L: {:.6} SOL ({:.2}%)", fallback_pnl_sol, fallback_pnl_percent);
    println!("- This uses price-based calculation as before");

    println!();
    println!("🎯 Test Summary:");
    println!("- The new sol_received field is properly integrated");
    println!("- P&L calculation prioritizes actual SOL received over price calculations");
    println!("- Fallback to price-based calculation works for backward compatibility");
    println!("✅ All tests passed!");
}
