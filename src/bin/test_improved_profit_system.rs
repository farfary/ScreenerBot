use screenerbot::global::*;
use screenerbot::profit_calculation::*;
use screenerbot::trader::{ Position, SAVED_POSITIONS };
use screenerbot::logger::{ log, LogTag };
use chrono::{ Utc, DateTime };
use serde_json;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing Improved Profit Calculation System\n");

    // Test 1: Load and analyze existing positions
    println!("📊 Test 1: Analyzing Existing Positions");
    println!("════════════════════════════════════════");

    let positions = SAVED_POSITIONS.lock().unwrap().clone();
    println!("Loaded {} positions", positions.len());

    let mut profit_system = ProfitCalculationSystem::new();
    let config = profit_system.get_config();

    println!("Initial Configuration:");
    println!("  Stop Loss: {:.1}%", config.stop_loss_percent);
    println!("  Profit Target: {:.1}%", config.profit_target_percent);
    println!("  Trailing Stop: {:.1}%", config.trailing_stop_percent);
    println!("  Time Decay Start: {:.0}s", config.time_decay_start_secs);

    // Test 2: Validate profit calculations for closed positions
    println!("\n🧮 Test 2: Recalculating P&L for Closed Positions");
    println!("═══════════════════════════════════════════════════");

    let closed_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.exit_time.is_some())
        .collect();

    let mut corrected_positions = Vec::new();

    for (i, position) in closed_positions.iter().enumerate() {
        println!("\n#{}: {} ({})", i + 1, position.symbol, &position.mint[..8]);

        if let Some(exit_price) = position.exit_price {
            let token_decimals = Some(9u8); // Assume 9 decimals for most tokens

            let accurate_pnl = profit_system.calculate_accurate_pnl(
                position,
                exit_price,
                token_decimals
            );

            println!("  Original P&L: {:.6} SOL", position.pnl_sol.unwrap_or(0.0));
            println!(
                "  Corrected P&L: {:.6} SOL ({})",
                accurate_pnl.pnl_sol,
                accurate_pnl.calculation_method
            );
            println!("  Corrected %: {:.2}%", accurate_pnl.pnl_percent);
            println!("  Fees Estimated: {:.6} SOL", accurate_pnl.total_fees_paid);

            // Create corrected position
            let mut corrected_position = (*position).clone();
            corrected_position.pnl_sol = Some(accurate_pnl.pnl_sol);
            corrected_position.pnl_percent = Some(accurate_pnl.pnl_percent);

            corrected_positions.push(corrected_position);

            // Record performance for learning
            let exit_reason = if let Some(ref sig) = position.exit_transaction_signature {
                if sig == "NO_TOKENS_TO_SELL" {
                    "sell_failed".to_string()
                } else {
                    "normal_exit".to_string()
                }
            } else {
                "unknown".to_string()
            };

            profit_system.record_trade_performance(position, exit_reason);
        }
    }

    // Test 3: Smart sell decisions for open positions
    println!("\n🎯 Test 3: Smart Sell Decisions for Open Positions");
    println!("═══════════════════════════════════════════════════");

    let open_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.exit_time.is_none())
        .collect();

    // Load current prices
    let tokens = LIST_TOKENS.read().unwrap();
    let mut price_found = false;

    for position in &open_positions {
        println!("\n🔄 {} ({})", position.symbol, &position.mint[..8]);

        // Try to find current price
        let current_price = tokens
            .iter()
            .find(|t| t.mint == position.mint)
            .and_then(|t|
                t.price_dexscreener_sol
                    .or(t.price_geckoterminal_sol)
                    .or(t.price_raydium_sol)
                    .or(t.price_pool_sol)
            );

        if let Some(price) = current_price {
            price_found = true;
            let token_decimals = Some(9u8);

            // Calculate current P&L
            let accurate_pnl = profit_system.calculate_accurate_pnl(
                position,
                price,
                token_decimals
            );

            println!("  Current Price: {:.8} SOL", price);
            println!(
                "  Current P&L: {:.6} SOL ({:.2}%)",
                accurate_pnl.pnl_sol,
                accurate_pnl.pnl_percent
            );

            // Test smart sell decision
            let (urgency, reason) = profit_system.should_sell_smart(
                position,
                price,
                Utc::now(),
                token_decimals
            );

            println!("  Sell Urgency: {:.2}", urgency);
            println!("  Decision: {}", if urgency > 0.8 {
                "🔴 SELL NOW"
            } else if urgency > 0.5 {
                "🟡 CONSIDER SELLING"
            } else if urgency > 0.2 {
                "🟢 HOLD"
            } else {
                "💎 DIAMOND HANDS"
            });
            println!("  Reason: {}", reason);

            // Show time held
            let time_held = (Utc::now() - position.entry_time).num_seconds();
            println!("  Time Held: {}m {}s", time_held / 60, time_held % 60);
        } else {
            println!("  ❌ No current price available");
        }
    }

    drop(tokens);

    if !price_found {
        println!("⚠️  No current prices found. Run the discovery service first.");
    }

    // Test 4: Performance optimization
    println!("\n⚙️ Test 4: Performance Optimization");
    println!("═══════════════════════════════════════");

    let (win_rate, avg_profit, avg_loss, trades_analyzed) = profit_system.get_performance_stats();

    if trades_analyzed > 0 {
        println!("Performance Statistics:");
        println!("  Win Rate: {:.1}%", win_rate * 100.0);
        println!("  Average Profit: {:.2}%", avg_profit);
        println!("  Average Loss: {:.2}%", avg_loss);
        println!("  Trades Analyzed: {}", trades_analyzed);

        let updated_config = profit_system.get_config();
        println!("\nOptimized Configuration:");
        println!("  Stop Loss: {:.1}%", updated_config.stop_loss_percent);
        println!("  Profit Target: {:.1}%", updated_config.profit_target_percent);
        println!("  Trailing Stop: {:.1}%", updated_config.trailing_stop_percent);
    } else {
        println!("No performance data available yet.");
    }

    // Test 5: Save corrected positions (optional)
    println!("\n💾 Test 5: Save Corrected Positions?");
    println!("════════════════════════════════════");

    if !corrected_positions.is_empty() {
        println!("Found {} positions with corrected P&L calculations.", corrected_positions.len());
        println!("Would you like to save the corrections? (This would update positions.json)");
        println!("Note: This is a test run, so we won't actually save changes.");

        // Calculate the impact
        let original_total: f64 = closed_positions
            .iter()
            .filter_map(|p| p.pnl_sol)
            .sum();

        let corrected_total: f64 = corrected_positions
            .iter()
            .filter_map(|p| p.pnl_sol)
            .sum();

        println!("\nImpact Analysis:");
        println!("  Original Total P&L: {:.6} SOL", original_total);
        println!("  Corrected Total P&L: {:.6} SOL", corrected_total);
        println!("  Difference: {:.6} SOL", corrected_total - original_total);

        // Save to a test file instead
        let test_filename = "corrected_positions_test.json";
        match serde_json::to_string_pretty(&corrected_positions) {
            Ok(json_data) => {
                if let Err(e) = fs::write(test_filename, json_data) {
                    println!("❌ Failed to save test file: {}", e);
                } else {
                    println!("✅ Saved corrected positions to {}", test_filename);
                }
            }
            Err(e) => {
                println!("❌ Failed to serialize positions: {}", e);
            }
        }
    }

    // Test 6: Sell urgency simulation
    println!("\n🎲 Test 6: Sell Urgency Simulation");
    println!("═══════════════════════════════════");

    if let Some(test_position) = open_positions.first() {
        println!("Testing sell urgency at different P&L levels for {}:", test_position.symbol);

        let entry_price = test_position.effective_entry_price.unwrap_or(test_position.entry_price);
        let test_scenarios = vec![
            (-50.0, "Major Loss"),
            (-30.0, "Significant Loss"),
            (-15.0, "Moderate Loss"),
            (-5.0, "Small Loss"),
            (0.0, "Break Even"),
            (5.0, "Small Profit"),
            (15.0, "Good Profit"),
            (30.0, "Great Profit"),
            (50.0, "Excellent Profit")
        ];

        for (pnl_percent, description) in test_scenarios {
            let test_price = entry_price * (1.0 + pnl_percent / 100.0);
            let (urgency, reason) = profit_system.should_sell_smart(
                test_position,
                test_price,
                Utc::now(),
                Some(9u8)
            );

            println!(
                "  {:>15} ({:>+5.1}%): Urgency {:.2} - {}",
                description,
                pnl_percent,
                urgency,
                if reason.len() > 50 {
                    reason[..47].to_string() + "..."
                } else {
                    reason.to_string()
                }
            );
        }
    }

    println!("\n✅ Profit Calculation Tests Completed!");
    println!("🔧 The improved system should provide:");
    println!("   • More accurate P&L calculations");
    println!("   • Smarter sell decisions");
    println!("   • Auto-optimization based on performance");
    println!("   • Better handling of fees and slippage");
    println!("   • Recovery probability analysis");

    Ok(())
}
