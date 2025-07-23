use screenerbot::loss_prevention::*;
use screenerbot::positions::{ Position, SAVED_POSITIONS };
use screenerbot::global::{ read_configs };
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Loss Prevention System");

    // Load configurations
    let _configs = read_configs("configs.json")?;
    println!("✅ Loaded configurations");

    // Load positions from file to populate SAVED_POSITIONS
    if let Ok(data) = std::fs::read_to_string("positions.json") {
        if let Ok(positions) = serde_json::from_str::<Vec<Position>>(&data) {
            if let Ok(mut saved_positions) = SAVED_POSITIONS.lock() {
                *saved_positions = positions;
                println!("✅ Loaded {} positions from file", saved_positions.len());
            }
        }
    }

    // Display configuration
    println!("\n⚙️ Loss Prevention Configuration:");
    println!("   Enabled: {}", LOSS_PREVENTION_ENABLED);
    println!("   Min Positions for Analysis: {}", MIN_CLOSED_POSITIONS_FOR_ANALYSIS);
    println!("   Max Loss Rate: {:.1}%", MAX_LOSS_RATE_PERCENT);
    println!("   Max Average Loss: {:.1}%", MAX_AVERAGE_LOSS_PERCENT);
    println!("   Lookback Period: {} hours", LOOKBACK_HOURS);

    // Test with actual mints from positions - limited sample
    println!("\n🔍 Testing with sample position data:");
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        let mut tested_mints = std::collections::HashSet::new();
        let mut test_count = 0;

        for position in positions.iter() {
            if test_count >= 5 || tested_mints.contains(&position.mint) {
                continue;
            }

            // Only test positions that are closed (have exit_price)
            if position.exit_price.is_none() {
                continue;
            }

            tested_mints.insert(position.mint.clone());
            test_count += 1;

            let allowed = should_allow_token_purchase(&position.mint, &position.symbol);
            let stats = analyze_token_loss_history(&position.mint, &position.symbol);

            println!("\n🪙 {} ({}):", position.symbol, &position.mint[..8]);
            println!("   Status: {}", if allowed { "✅ ALLOWED" } else { "❌ BLOCKED" });
            println!("   Closed Positions: {}", stats.total_closed_positions);
            if stats.total_closed_positions > 0 {
                println!("   Loss Rate: {:.1}%", stats.loss_rate_percent);
                println!("   Average P&L: {:.1}%", stats.average_pnl_percent);
                println!("   Total P&L: {:.6} SOL", stats.total_pnl_sol);
                println!(
                    "   Range: {:.1}% to {:.1}%",
                    stats.worst_loss_percent,
                    stats.best_gain_percent
                );
            }
        }

        // Show overall statistics
        let total_positions = positions.len();
        let closed_positions = positions
            .iter()
            .filter(|p| p.exit_price.is_some())
            .count();
        let unique_tokens = positions
            .iter()
            .map(|p| &p.mint)
            .collect::<std::collections::HashSet<_>>()
            .len();

        println!("\n📈 Overall Statistics:");
        println!("   Total Positions: {}", total_positions);
        println!("   Closed Positions: {}", closed_positions);
        println!("   Unique Tokens: {}", unique_tokens);

        if closed_positions > 0 {
            // Quick analysis of all closed positions
            let mut total_pnl_sol = 0.0;
            let mut losing_count = 0;

            for position in positions.iter().filter(|p| p.exit_price.is_some()) {
                let (pnl_sol, pnl_percent) = screenerbot::positions::calculate_position_pnl(
                    position,
                    None
                );
                total_pnl_sol += pnl_sol;
                if pnl_percent < 0.0 {
                    losing_count += 1;
                }
            }

            let loss_rate = ((losing_count as f64) / (closed_positions as f64)) * 100.0;
            println!("   Overall Loss Rate: {:.1}%", loss_rate);
            println!("   Total P&L: {:.6} SOL", total_pnl_sol);
        }
    }

    println!("\n✅ Loss prevention testing completed!");
    Ok(())
}
