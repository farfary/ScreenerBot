use screenerbot::global::*;
use screenerbot::profit_calculation::*;
use screenerbot::trader::{ Position, SAVED_POSITIONS };
use screenerbot::logger::{ log, LogTag };
use chrono::Utc;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging Profit Calculation Issues\n");

    // Load positions
    let positions = SAVED_POSITIONS.lock().unwrap().clone();
    println!("📊 Loaded {} positions from positions.json", positions.len());

    // Separate open and closed positions
    let closed_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.exit_time.is_some())
        .collect();

    let open_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.exit_time.is_none())
        .collect();

    println!("   └─ {} closed positions", closed_positions.len());
    println!("   └─ {} open positions\n", open_positions.len());

    // Initialize profit calculation system
    let mut profit_system = ProfitCalculationSystem::new();

    println!("🧮 Analyzing Closed Positions:");
    println!("═══════════════════════════════════════════════════════════════");

    let mut issues_found = 0;
    let mut correct_calculations = 0;

    for (i, position) in closed_positions.iter().enumerate() {
        println!("\n📍 Position #{}: {} ({})", i + 1, position.symbol, position.mint);
        println!("   Entry Time: {}", position.entry_time.format("%H:%M:%S"));
        if let Some(exit_time) = position.exit_time {
            println!("   Exit Time:  {}", exit_time.format("%H:%M:%S"));
            let duration = exit_time.signed_duration_since(position.entry_time);
            println!("   Duration:   {}m {}s", duration.num_minutes(), duration.num_seconds() % 60);
        }

        // Check for obvious issues
        let mut position_issues: Vec<String> = Vec::new();

        // Issue 1: Fixed P&L of -0.000250
        if let Some(pnl) = position.pnl_sol {
            if (pnl + 0.00025).abs() < 0.0000001 {
                position_issues.push("Fixed P&L of -0.000250 SOL (calculation bug)".to_string());
            }
        }

        // Issue 2: Exit signature shows NO_TOKENS_TO_SELL
        if let Some(ref exit_sig) = position.exit_transaction_signature {
            if exit_sig == "NO_TOKENS_TO_SELL" {
                position_issues.push("Exit signature shows NO_TOKENS_TO_SELL".to_string());
            }
        }

        // Issue 3: Effective exit price is 0.0
        if let Some(exit_price) = position.effective_exit_price {
            if exit_price == 0.0 {
                position_issues.push("Effective exit price is 0.0".to_string());
            }
        }

        // Issue 4: Missing P&L percentage
        if position.pnl_percent.is_none() {
            position_issues.push("Missing P&L percentage".to_string());
        }

        // Show current stored values
        println!("   📊 Stored Values:");
        println!("      Entry Price: {:.8} SOL", position.entry_price);
        if let Some(effective_entry) = position.effective_entry_price {
            println!("      Effective Entry: {:.8} SOL", effective_entry);
        }
        if let Some(exit_price) = position.exit_price {
            println!("      Exit Price: {:.8} SOL", exit_price);
        }
        if let Some(effective_exit) = position.effective_exit_price {
            println!("      Effective Exit: {:.8} SOL", effective_exit);
        }
        println!("      Trade Size: {:.6} SOL", position.entry_size_sol);
        if let Some(pnl) = position.pnl_sol {
            println!("      Stored P&L: {:.6} SOL", pnl);
        }
        if let Some(pnl_pct) = position.pnl_percent {
            println!("      Stored P&L %: {:.2}%", pnl_pct);
        }
        if let Some(token_amount) = position.token_amount {
            println!("      Token Amount: {} (raw)", token_amount);
        }

        // Calculate what P&L should be using new system
        if let Some(exit_price) = position.exit_price {
            let current_price = exit_price;

            // Get token decimals (we'll assume 9 if not found)
            let token_decimals = Some(9u8); // Most tokens use 9 decimals

            let accurate_pnl = profit_system.calculate_accurate_pnl(
                position,
                current_price,
                token_decimals
            );

            println!("   🎯 Corrected Calculation ({}):", accurate_pnl.calculation_method);
            println!("      Corrected P&L: {:.6} SOL", accurate_pnl.pnl_sol);
            println!("      Corrected P&L %: {:.2}%", accurate_pnl.pnl_percent);
            println!("      Total Fees: {:.6} SOL", accurate_pnl.total_fees_paid);

            // Compare with stored values
            if let Some(stored_pnl) = position.pnl_sol {
                let difference = (accurate_pnl.pnl_sol - stored_pnl).abs();
                if difference > 0.000001 {
                    let issue_msg = format!("P&L difference: {:.6} SOL", difference);
                    position_issues.push(issue_msg);
                }
            }
        }

        // Show transaction signatures
        println!("   📝 Transaction Info:");
        if let Some(ref entry_sig) = position.entry_transaction_signature {
            println!("      Entry TX: {}...{}", &entry_sig[..8], &entry_sig[entry_sig.len() - 8..]);
        }
        if let Some(ref exit_sig) = position.exit_transaction_signature {
            if exit_sig == "NO_TOKENS_TO_SELL" {
                println!("      Exit TX: ❌ NO_TOKENS_TO_SELL");
            } else {
                println!("      Exit TX: {}...{}", &exit_sig[..8], &exit_sig[exit_sig.len() - 8..]);
            }
        }

        // Show issues found
        if position_issues.is_empty() {
            println!("   ✅ No issues detected");
            correct_calculations += 1;
        } else {
            println!("   ⚠️  Issues found:");
            for issue in &position_issues {
                println!("      • {}", issue);
            }
            issues_found += 1;
        }
    }

    println!("\n\n🔍 Analyzing Open Positions:");
    println!("═══════════════════════════════════════════════════════════════");

    // Load tokens for current price lookup
    let tokens = LIST_TOKENS.read().unwrap();
    let mut price_map: HashMap<String, f64> = HashMap::new();
    for token in tokens.iter() {
        if
            let Some(price) = token.price_dexscreener_sol
                .or(token.price_geckoterminal_sol)
                .or(token.price_raydium_sol)
                .or(token.price_pool_sol)
        {
            price_map.insert(token.mint.clone(), price);
        }
    }
    drop(tokens);

    for (i, position) in open_positions.iter().enumerate() {
        println!("\n📍 Open Position #{}: {} ({})", i + 1, position.symbol, position.mint);

        let current_price = price_map.get(&position.mint).copied().unwrap_or(0.0);
        if current_price > 0.0 {
            let token_decimals = Some(9u8);
            let accurate_pnl = profit_system.calculate_accurate_pnl(
                position,
                current_price,
                token_decimals
            );

            println!("   Current Price: {:.8} SOL", current_price);
            println!(
                "   Current P&L: {:.6} SOL ({:.2}%)",
                accurate_pnl.pnl_sol,
                accurate_pnl.pnl_percent
            );

            // Test smart sell decision
            let (urgency, reason) = profit_system.should_sell_smart(
                position,
                current_price,
                Utc::now(),
                token_decimals
            );

            println!("   Sell Urgency: {:.2} - {}", urgency, reason);
        } else {
            println!("   ❌ No current price available");
        }
    }

    println!("\n\n📊 Summary:");
    println!("═══════════════════════════════════════════════════════════════");
    println!("Total positions analyzed: {}", positions.len());
    println!("Positions with issues: {}", issues_found);
    println!("Correct calculations: {}", correct_calculations);

    if issues_found > 0 {
        println!("\n🔧 Common Issues Identified:");
        println!("1. All closed positions show fixed P&L of -0.000250 SOL");
        println!("2. Exit transactions show 'NO_TOKENS_TO_SELL' instead of real signatures");
        println!("3. Effective exit prices are 0.0 instead of actual exit prices");
        println!("4. P&L percentages are missing (null values)");
        println!(
            "\n💡 This suggests the sell function is failing but positions are being marked as closed anyway."
        );
        println!("   The bot is likely trying to sell tokens but the transaction fails,");
        println!("   then incorrectly recording the position as closed with a fixed loss.");
    }

    // Display current profit system configuration
    let config = profit_system.get_config();
    println!("\n⚙️ Current Profit System Configuration:");
    println!("   Stop Loss: {:.1}%", config.stop_loss_percent);
    println!("   Profit Target: {:.1}%", config.profit_target_percent);
    println!("   Trailing Stop: {:.1}%", config.trailing_stop_percent);
    println!("   Time Decay Start: {:.0}s", config.time_decay_start_secs);
    println!("   Max Hold Time: {:.0}s", config.max_hold_time_secs);

    let (win_rate, avg_profit, avg_loss, trades_analyzed) = profit_system.get_performance_stats();
    if trades_analyzed > 0 {
        println!("   Historical Performance:");
        println!("     Win Rate: {:.1}%", win_rate * 100.0);
        println!("     Avg Profit: {:.1}%", avg_profit);
        println!("     Avg Loss: {:.1}%", avg_loss);
        println!("     Trades Analyzed: {}", trades_analyzed);
    }

    Ok(())
}
