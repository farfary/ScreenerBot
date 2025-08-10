use screenerbot::positions::{calculate_position_total_fees, calculate_position_fees_breakdown, calculate_position_pnl, Position};
use screenerbot::trader::PROFIT_EXTRA_NEEDED_SOL;
use chrono::Utc;

fn main() {
    println!("🧪 TESTING PROFIT_EXTRA_NEEDED_SOL SEPARATION");
    println!("==============================================\n");

    // Create a test position with known fees
    let test_position = Position {
        mint: "test_mint".to_string(),
        symbol: "TEST".to_string(),
        name: "Test Token".to_string(),
        entry_price: 0.000001,
        entry_time: Utc::now(),
        exit_price: Some(0.000002),
        exit_time: Some(Utc::now()),
        position_type: "buy".to_string(),
        entry_size_sol: 0.004,
        total_size_sol: 0.004,
        price_highest: 0.000002,
        price_lowest: 0.000001,
        entry_transaction_signature: Some("test_entry".to_string()),
        exit_transaction_signature: Some("test_exit".to_string()),
        token_amount: Some(4000),
        effective_entry_price: Some(0.000001),
        effective_exit_price: Some(0.000002),
        sol_received: Some(0.008), // 2x return
        profit_target_min: Some(50.0),
        profit_target_max: Some(200.0),
        liquidity_tier: Some("MEDIUM".to_string()),
        transaction_entry_verified: true,
        transaction_exit_verified: true,
        entry_fee_lamports: Some(6000), // 0.000006 SOL
        exit_fee_lamports: Some(5000),  // 0.000005 SOL
    };

    println!("📊 TEST POSITION DATA:");
    println!("   Entry fee: {} lamports ({:.9} SOL)", 
             test_position.entry_fee_lamports.unwrap_or(0),
             test_position.entry_fee_lamports.unwrap_or(0) as f64 / 1_000_000_000.0);
    println!("   Exit fee: {} lamports ({:.9} SOL)", 
             test_position.exit_fee_lamports.unwrap_or(0),
             test_position.exit_fee_lamports.unwrap_or(0) as f64 / 1_000_000_000.0);
    println!("   PROFIT_EXTRA_NEEDED_SOL: {:.9} SOL", PROFIT_EXTRA_NEEDED_SOL);
    println!("   SOL invested: {:.9} SOL", test_position.entry_size_sol);
    println!("   SOL received: {:.9} SOL", test_position.sol_received.unwrap_or(0.0));
    println!();

    // Test fee calculation functions (should NOT include PROFIT_EXTRA_NEEDED_SOL)
    let total_fees = calculate_position_total_fees(&test_position);
    let (entry_fee, exit_fee, total_breakdown) = calculate_position_fees_breakdown(&test_position);

    println!("🔧 FEE CALCULATION RESULTS (should exclude profit buffer):");
    println!("   Total fees: {:.9} SOL", total_fees);
    println!("   Fee breakdown: Entry={:.9}, Exit={:.9}, Total={:.9}", 
             entry_fee, exit_fee, total_breakdown);

    let expected_total_fees = 0.000006 + 0.000005; // Entry + Exit fees only
    let total_fees_match = (total_fees - expected_total_fees).abs() < 0.000000001;
    
    if total_fees_match {
        println!("   ✅ Fee calculations correctly exclude PROFIT_EXTRA_NEEDED_SOL");
    } else {
        println!("   ❌ Fee calculations incorrectly include PROFIT_EXTRA_NEEDED_SOL");
        println!("      Expected: {:.9} SOL, Got: {:.9} SOL", expected_total_fees, total_fees);
    }
    println!();

    // Test P&L calculation (should INCLUDE PROFIT_EXTRA_NEEDED_SOL)
    let (pnl_sol, pnl_percent) = calculate_position_pnl(&test_position, None);

    println!("💰 P&L CALCULATION RESULTS (should include profit buffer):");
    println!("   P&L: {:.9} SOL ({:.2}%)", pnl_sol, pnl_percent);

    // Calculate expected P&L manually
    let sol_invested = test_position.entry_size_sol;
    let sol_received = test_position.sol_received.unwrap();
    let actual_fees = (test_position.entry_fee_lamports.unwrap() + test_position.exit_fee_lamports.unwrap()) as f64 / 1_000_000_000.0;
    let expected_pnl_with_buffer = sol_received - sol_invested - actual_fees - PROFIT_EXTRA_NEEDED_SOL;
    let expected_pnl_without_buffer = sol_received - sol_invested - actual_fees;

    println!("   Expected P&L with profit buffer: {:.9} SOL", expected_pnl_with_buffer);
    println!("   Expected P&L without profit buffer: {:.9} SOL", expected_pnl_without_buffer);

    let pnl_includes_buffer = (pnl_sol - expected_pnl_with_buffer).abs() < 0.000000001;
    let pnl_excludes_buffer = (pnl_sol - expected_pnl_without_buffer).abs() < 0.000000001;

    if pnl_includes_buffer {
        println!("   ✅ P&L calculations correctly include PROFIT_EXTRA_NEEDED_SOL");
    } else if pnl_excludes_buffer {
        println!("   ❌ P&L calculations incorrectly exclude PROFIT_EXTRA_NEEDED_SOL");
    } else {
        println!("   ❓ P&L calculation result doesn't match either expected value");
    }
    println!();

    // Summary
    println!("📋 SUMMARY:");
    if total_fees_match && pnl_includes_buffer {
        println!("   🎉 ALL TESTS PASSED!");
        println!("   • Fee calculations correctly exclude profit buffer");
        println!("   • P&L calculations correctly include profit buffer");
        println!("   • Stored position fees remain accurate");
    } else {
        println!("   ❌ SOME TESTS FAILED!");
        if !total_fees_match {
            println!("   • Fee calculations need fixing");
        }
        if !pnl_includes_buffer {
            println!("   • P&L calculations need fixing");
        }
    }
}
