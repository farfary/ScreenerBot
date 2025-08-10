use screenerbot::{
    positions::{Position, calculate_position_total_fees},
    summary::{ClosedPositionDisplay, OpenPositionDisplay},
    trader::PROFIT_EXTRA_NEEDED_SOL,
};
use chrono::Utc;

/// Test to verify that summary tables include profit buffer in fee calculations
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Summary Fee Calculations with Profit Buffer");
    println!("═══════════════════════════════════════════════════════════════");

    // Create a test position with known fees
    let test_position = Position {
        mint: "test_mint".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        position_type: "buy".to_string(),
        entry_price: 0.000000005,
        entry_size_sol: 0.004,
        total_size_sol: 0.004,
        entry_time: Utc::now(),
        entry_fee_lamports: Some(10000), // 0.00001 SOL
        exit_price: Some(0.000000006),
        exit_time: Some(Utc::now()),
        exit_fee_lamports: Some(5000), // 0.000005 SOL
        transaction_entry_verified: true,
        transaction_exit_verified: true,
        entry_transaction_signature: Some("test_entry_sig".to_string()),
        exit_transaction_signature: Some("test_exit_sig".to_string()),
        token_amount: Some(800000000), // 800M tokens
        effective_entry_price: Some(0.000000005),
        effective_exit_price: Some(0.000000006),
        sol_received: Some(0.0048),
        profit_target_min: Some(10.0),
        profit_target_max: Some(30.0),
        liquidity_tier: Some("medium".to_string()),
        price_highest: 0.000000007,
        price_lowest: 0.000000004,
    };

    // Test 1: Check actual stored fees (without buffer)
    let stored_fees = calculate_position_total_fees(&test_position);
    println!("📊 Test 1: Stored Fee Calculation");
    println!("   Entry fee: {} lamports ({:.9} SOL)", 10000, 0.00001);
    println!("   Exit fee: {} lamports ({:.9} SOL)", 5000, 0.000005);
    println!("   Total stored fees: {:.9} SOL", stored_fees);
    println!("   Expected: {:.9} SOL", 0.000015);
    
    if (stored_fees - 0.000015).abs() < 0.0000001 {
        println!("   ✅ Stored fees calculation correct");
    } else {
        println!("   ❌ Stored fees calculation incorrect");
    }

    // Test 2: Check closed position display fees (should include buffer)
    println!("\n📊 Test 2: Closed Position Display Fees");
    let closed_display = ClosedPositionDisplay::from_position(&test_position);
    
    // Extract fee value from display string
    let display_fees_str = &closed_display.fees_sol;
    let display_fees = display_fees_str.parse::<f64>().unwrap_or(0.0);
    let expected_display_fees = stored_fees + PROFIT_EXTRA_NEEDED_SOL;
    
    println!("   Stored fees: {:.9} SOL", stored_fees);
    println!("   Profit buffer: {:.9} SOL", PROFIT_EXTRA_NEEDED_SOL);
    println!("   Expected display fees: {:.9} SOL", expected_display_fees);
    println!("   Actual display fees: {:.9} SOL", display_fees);
    println!("   Display string: '{}'", display_fees_str);
    
    if (display_fees - expected_display_fees).abs() < 0.0000001 {
        println!("   ✅ Closed position display fees include profit buffer correctly");
    } else {
        println!("   ❌ Closed position display fees incorrect");
    }

    // Test 3: Check open position display fees (should include buffer)
    println!("\n📊 Test 3: Open Position Display Fees");
    
    // Create an open position (no exit data)
    let mut open_position = test_position.clone();
    open_position.exit_price = None;
    open_position.exit_time = None;
    open_position.exit_fee_lamports = None;
    open_position.transaction_exit_verified = false;
    open_position.exit_transaction_signature = None;
    open_position.effective_exit_price = None;
    
    let open_stored_fees = calculate_position_total_fees(&open_position);
    let open_display = OpenPositionDisplay::from_position(&open_position, Some(0.000000006));
    
    // Extract fee value from display string
    let open_display_fees_str = &open_display.fees_sol;
    let open_display_fees = open_display_fees_str.parse::<f64>().unwrap_or(0.0);
    let expected_open_display_fees = open_stored_fees + PROFIT_EXTRA_NEEDED_SOL;
    
    println!("   Stored fees (open): {:.9} SOL", open_stored_fees);
    println!("   Profit buffer: {:.9} SOL", PROFIT_EXTRA_NEEDED_SOL);
    println!("   Expected display fees: {:.9} SOL", expected_open_display_fees);
    println!("   Actual display fees: {:.9} SOL", open_display_fees);
    println!("   Display string: '{}'", open_display_fees_str);
    
    if (open_display_fees - expected_open_display_fees).abs() < 0.0000001 {
        println!("   ✅ Open position display fees include profit buffer correctly");
    } else {
        println!("   ❌ Open position display fees incorrect");
    }

    // Test 4: Verify separation - stored vs display
    println!("\n📊 Test 4: Separation Verification");
    println!("   Stored fees exclude profit buffer: {:.9} SOL", stored_fees);
    println!("   Display fees include profit buffer: {:.9} SOL", display_fees);
    println!("   Difference: {:.9} SOL", display_fees - stored_fees);
    println!("   Expected difference (profit buffer): {:.9} SOL", PROFIT_EXTRA_NEEDED_SOL);
    
    if (display_fees - stored_fees - PROFIT_EXTRA_NEEDED_SOL).abs() < 0.0000001 {
        println!("   ✅ Proper separation: stored data pure, display includes buffer");
    } else {
        println!("   ❌ Separation incorrect");
    }

    println!("\n🎉 Summary Fee Buffer Integration Test Complete!");
    println!("   • Stored position fees remain accurate blockchain data");
    println!("   • Display tables include profit buffer for user visibility");
    println!("   • Proper separation maintained between data storage and presentation");

    Ok(())
}
