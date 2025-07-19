use anyhow::Result;
use chrono::Utc;
use screenerbot::{ config::Config, trader::{ TraderDatabase, Position, PositionStatus } };

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Fast Profit Position Monitor");
    println!("═══════════════════════════════════");

    // Load config
    let config = Config::load("configs.json").expect("Failed to load config");

    // Connect to database
    let database = TraderDatabase::new(&config.trader.database_path)?;

    println!("\n📊 Current Position Analysis:");
    println!("─────────────────────────────");

    // Get all active positions
    let active_positions = database.get_active_positions()?;
    let closed_positions = database.get_closed_positions(50)?; // Get last 50 closed positions

    let all_positions = [active_positions.clone(), closed_positions.clone()].concat();

    if all_positions.is_empty() {
        println!("   No positions found in database");
        return Ok(());
    }

    println!(
        "   Found {} total positions ({} active, {} closed)",
        all_positions.len(),
        active_positions.len(),
        closed_positions.len()
    );

    for (i, (id, position_summary)) in all_positions.iter().enumerate() {
        let position = Position::from_summary(*id, position_summary.clone());

        println!("\n   {}. Token: {} ({})", i + 1, position.token_address, position.token_symbol);
        println!("      Status: {:?}", position.status);
        println!(
            "      P&L: {:.2}% ({:.6} SOL)",
            position.unrealized_pnl_percent,
            position.unrealized_pnl_sol
        );
        println!(
            "      Invested: {:.6} SOL, Tokens: {:.2}",
            position.total_invested_sol,
            position.total_tokens
        );
        println!("      Peak Price: {:.10} SOL", position.peak_price);
        println!("      Current Price: {:.10} SOL", position.current_price);

        // Analyze potential profit-taking opportunities
        if matches!(position.status, PositionStatus::Active) {
            if position.unrealized_pnl_percent >= 10.0 {
                println!("      🎯 FAST PROFIT OPPORTUNITY! Consider selling 25% at 10% profit");
            }
            if position.unrealized_pnl_percent >= 25.0 {
                println!("      🚀 HIGH PROFIT! Consider selling 50% at 25% profit");
            }
            if position.unrealized_pnl_percent >= 50.0 {
                println!("      💎 VERY HIGH PROFIT! Consider selling 75% at 50% profit");
            }
            if position.unrealized_pnl_percent >= 100.0 {
                println!("      🌟 EXTREME PROFIT! Consider selling ALL at 100% profit");
            }

            // Check if position is very young (fast gains)
            let position_age_minutes = (Utc::now() - position.created_at).num_minutes();
            if position_age_minutes <= 10 && position.unrealized_pnl_percent > 5.0 {
                println!(
                    "      ⚡ FAST MOVER! {:.1}% gain in {} minutes",
                    position.unrealized_pnl_percent,
                    position_age_minutes
                );
            }
        }
    }

    println!("\n📈 Performance Summary:");
    println!("─────────────────────");

    println!("   Active Positions: {}", active_positions.len());
    println!("   Closed Positions: {}", closed_positions.len());

    // Calculate metrics for active positions
    if !active_positions.is_empty() {
        let total_invested: f64 = active_positions
            .iter()
            .map(|(_, pos)| pos.total_invested_sol)
            .sum();

        let total_unrealized_pnl: f64 = active_positions
            .iter()
            .map(|(_, pos)| pos.unrealized_pnl_sol)
            .sum();

        let profitable_count = active_positions
            .iter()
            .filter(|(_, pos)| pos.unrealized_pnl_sol > 0.0)
            .count();

        println!("   Total Invested (Active): {:.6} SOL", total_invested);
        println!("   Total Unrealized P&L: {:.6} SOL", total_unrealized_pnl);
        println!("   Profitable Active Positions: {}/{}", profitable_count, active_positions.len());

        if total_invested > 0.0 {
            let unrealized_roi = (total_unrealized_pnl / total_invested) * 100.0;
            println!("   Unrealized ROI: {:.2}%", unrealized_roi);
        }
    }

    // Calculate metrics for closed positions
    if !closed_positions.is_empty() {
        let total_realized_pnl: f64 = closed_positions
            .iter()
            .map(|(_, pos)| pos.realized_pnl_sol)
            .sum();

        let winning_positions = closed_positions
            .iter()
            .filter(|(_, pos)| pos.realized_pnl_sol > 0.0)
            .count();

        println!("   Total Realized P&L: {:.6} SOL", total_realized_pnl);
        println!("   Winning Closed Positions: {}/{}", winning_positions, closed_positions.len());

        if !closed_positions.is_empty() {
            let win_rate = ((winning_positions as f64) / (closed_positions.len() as f64)) * 100.0;
            println!("   Win Rate: {:.1}%", win_rate);
        }
    }

    println!("\n🎯 Fast Profit Recommendations:");
    println!("─────────────────────────────────");
    println!("   1. Enable fast monitoring: position_check_interval_seconds = 2");
    println!("   2. Enable fast price updates: price_check_interval_seconds = 1");
    println!("   3. Lower sell trigger: sell_trigger_percent = 10.0");
    println!("   4. Tighter stop loss: stop_loss_percent = -25.0");
    println!("   5. Disable DCA to avoid averaging down");

    println!("\n✅ Analysis Complete");
    println!("═══════════════════════════════════");

    Ok(())
}
