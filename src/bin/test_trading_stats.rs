use anyhow::Result;
use screenerbot::config::Config;
use screenerbot::trader::database::TraderDatabase;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🧪 Testing Trading Statistics Calculation");
    println!("═══════════════════════════════════════════");

    // Load config
    let _config = Config::load("configs.json")?;

    // Connect to trader database
    let database = TraderDatabase::new("trader.db")?;

    // Get raw position data for verification
    println!("\n📊 Raw Position Data:");
    println!("─────────────────────────");

    let stats = database.get_trader_stats()?;

    println!("📈 Trading Statistics:");
    println!("   Total Trades: {}", stats.total_trades);
    println!("   Successful Executions: {}", stats.successful_trades);
    println!("   Failed Executions: {}", stats.failed_trades);
    println!("   🎯 Win Rate (P&L): {:.1}%", stats.win_rate);
    println!("   💰 Total Invested: {:.4} SOL", stats.total_invested_sol);
    println!("   📊 Realized P&L: {:.4} SOL", stats.total_realized_pnl_sol);
    println!("   🔄 Unrealized P&L: {:.4} SOL", stats.total_unrealized_pnl_sol);
    println!("   📁 Active Positions: {}", stats.active_positions);
    println!("   📁 Closed Positions: {}", stats.closed_positions);
    println!("   💎 Largest Win: {:.4} SOL", stats.largest_win_sol);
    println!("   💸 Largest Loss: {:.4} SOL", stats.largest_loss_sol);
    println!("   💱 Avg Trade Size: {:.4} SOL", stats.average_trade_size_sol);

    // Calculate additional metrics
    let total_pnl = stats.total_realized_pnl_sol + stats.total_unrealized_pnl_sol;
    let roi_percentage = if stats.total_invested_sol > 0.0 {
        (total_pnl / stats.total_invested_sol) * 100.0
    } else {
        0.0
    };

    println!("\n🔍 Calculated Metrics:");
    println!("   🎖️ Total P&L: {:.4} SOL", total_pnl);
    println!("   📊 ROI: {:.1}%", roi_percentage);

    // Verify with manual calculation
    let winning_positions = ((stats.win_rate / 100.0) * (stats.closed_positions as f64)) as u32;
    let losing_positions = stats.closed_positions - winning_positions;

    println!("\n✅ Verification:");
    println!("   Winning Positions: {}", winning_positions);
    println!("   Losing Positions: {}", losing_positions);
    println!(
        "   Expected Win Rate: {:.1}%",
        ((winning_positions as f64) / (stats.closed_positions as f64)) * 100.0
    );

    if stats.win_rate < 50.0 {
        println!(
            "\n⚠️ Analysis: Win rate is {:.1}%, indicating room for strategy improvement",
            stats.win_rate
        );
    } else {
        println!("\n✅ Analysis: Good win rate of {:.1}%", stats.win_rate);
    }

    Ok(())
}
