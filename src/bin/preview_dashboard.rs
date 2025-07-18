use anyhow::Result;
use screenerbot::config::Config;
use screenerbot::trader::database::TraderDatabase;
use tabled::{ Table, settings::Style };
use colored::*;

#[derive(tabled::Tabled)]
struct StatsRow {
    metric: String,
    value: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 {}", "ScreenerBot Trading Dashboard (Preview)".bold().bright_cyan());
    println!();

    // Load config and get stats
    let _config = Config::load("configs.json")?;
    let database = TraderDatabase::new("trader.db")?;
    let stats = database.get_trader_stats()?;

    // Calculate additional metrics
    let total_pnl = stats.total_realized_pnl_sol + stats.total_unrealized_pnl_sol;
    let roi_percentage = if stats.total_invested_sol > 0.0 {
        (total_pnl / stats.total_invested_sol) * 100.0
    } else {
        0.0
    };

    let avg_win = if stats.largest_win_sol > 0.0 { stats.largest_win_sol } else { 0.0 };
    let avg_loss = if stats.largest_loss_sol < 0.0 { stats.largest_loss_sol.abs() } else { 0.0 };
    let profit_factor = if avg_loss > 0.0 { avg_win / avg_loss } else { 0.0 };

    let execution_success_rate = if stats.total_trades > 0 {
        ((stats.successful_trades as f64) / (stats.total_trades as f64)) * 100.0
    } else {
        0.0
    };

    // Create comprehensive stats table
    let stats_data = vec![
        StatsRow {
            metric: "🎯 Total Trades".to_string(),
            value: format!("{}", stats.total_trades),
        },
        StatsRow {
            metric: "📈 Win Rate (P&L)".to_string(),
            value: if stats.win_rate >= 50.0 {
                format!("{:.1}% ✅", stats.win_rate)
            } else if stats.win_rate >= 30.0 {
                format!("{:.1}% ⚠️", stats.win_rate)
            } else {
                format!("{:.1}% ❌", stats.win_rate)
            },
        },
        StatsRow {
            metric: "⚡ Execution Rate".to_string(),
            value: format!("{:.1}%", execution_success_rate),
        },
        StatsRow {
            metric: "💰 Total Invested".to_string(),
            value: format!("{:.4} SOL", stats.total_invested_sol),
        },
        StatsRow {
            metric: "📊 Realized P&L".to_string(),
            value: if stats.total_realized_pnl_sol >= 0.0 {
                format!("{:.4} SOL 📈", stats.total_realized_pnl_sol)
            } else {
                format!("{:.4} SOL 📉", stats.total_realized_pnl_sol)
            },
        },
        StatsRow {
            metric: "🔄 Unrealized P&L".to_string(),
            value: if stats.total_unrealized_pnl_sol >= 0.0 {
                format!("{:.4} SOL 📈", stats.total_unrealized_pnl_sol)
            } else {
                format!("{:.4} SOL 📉", stats.total_unrealized_pnl_sol)
            },
        },
        StatsRow {
            metric: "🎖️ Total P&L".to_string(),
            value: if total_pnl >= 0.0 {
                format!("{:.4} SOL 🚀", total_pnl)
            } else {
                format!("{:.4} SOL 💥", total_pnl)
            },
        },
        StatsRow {
            metric: "📊 ROI".to_string(),
            value: if roi_percentage >= 0.0 {
                format!("{:.1}% 📈", roi_percentage)
            } else {
                format!("{:.1}% 📉", roi_percentage)
            },
        },
        StatsRow {
            metric: "💎 Largest Win".to_string(),
            value: format!("{:.4} SOL", stats.largest_win_sol),
        },
        StatsRow {
            metric: "💸 Largest Loss".to_string(),
            value: format!("{:.4} SOL", stats.largest_loss_sol),
        },
        StatsRow {
            metric: "⚖️ Profit Factor".to_string(),
            value: if profit_factor >= 2.0 {
                format!("{:.2}x 🔥", profit_factor)
            } else if profit_factor >= 1.0 {
                format!("{:.2}x ✅", profit_factor)
            } else {
                format!("{:.2}x ⚠️", profit_factor)
            },
        },
        StatsRow {
            metric: "💼 Active Positions".to_string(),
            value: format!("{}", stats.active_positions),
        },
        StatsRow {
            metric: "📁 Closed Positions".to_string(),
            value: format!("{}", stats.closed_positions),
        },
        StatsRow {
            metric: "💱 Avg Trade Size".to_string(),
            value: format!("{:.4} SOL", stats.average_trade_size_sol),
        }
    ];

    let mut stats_table = Table::new(stats_data);
    let styled_stats_table = stats_table.with(Style::modern());
    println!("📊 {}", "Trading Performance Analytics".bold().bright_yellow());
    println!("{}", styled_stats_table);
    println!();

    // Add performance summary
    if stats.closed_positions > 0 {
        let winning_positions = ((stats.win_rate / 100.0) * (stats.closed_positions as f64)) as u32;
        let losing_positions = stats.closed_positions - winning_positions;

        println!("🏆 {}", "Performance Summary".bold().bright_green());
        println!(
            "   └─ {} winning trades • {} losing trades • {} active",
            winning_positions,
            losing_positions,
            stats.active_positions
        );
        if roi_percentage >= 10.0 {
            println!("   └─ 🚀 Strong performance with {:.1}% ROI", roi_percentage);
        } else if roi_percentage >= 0.0 {
            println!("   └─ 📈 Positive performance with {:.1}% ROI", roi_percentage);
        } else {
            println!("   └─ 📉 Needs improvement: {:.1}% ROI", roi_percentage);
        }
        println!();
    }

    println!("✅ {}", "Statistics fixed! Win rate now shows actual profitability.".bright_green());
    Ok(())
}
