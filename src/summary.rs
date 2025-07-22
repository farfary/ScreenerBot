use crate::trader::*;
use crate::positions::*;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::utils::*;

use chrono::{ Utc };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };

/// Display structure for closed positions with specific "Exit" column
#[derive(Tabled)]
pub struct ClosedPositionDisplay {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Mint")]
    mint: String,
    #[tabled(rename = "Entry")]
    entry_price: String,
    #[tabled(rename = "Exit")]
    exit_price: String,
    #[tabled(rename = "Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "Duration")]
    duration: String,
    #[tabled(rename = "Status")]
    status: String,
}

/// Display structure for open positions with specific "Price" column
#[derive(Tabled)]
pub struct OpenPositionDisplay {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Mint")]
    mint: String,
    #[tabled(rename = "Entry")]
    entry_price: String,
    #[tabled(rename = "Price")]
    current_price: String,
    #[tabled(rename = "Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "Duration")]
    duration: String,
    #[tabled(rename = "Status")]
    status: String,
}

/// Display structure for bot summary table formatting
#[derive(Tabled)]
pub struct BotSummaryDisplay {
    #[tabled(rename = "💼 Wallet Balance")]
    wallet_balance: String,
    #[tabled(rename = "📊 Total Trades")]
    total_trades: usize,
    #[tabled(rename = "🏆 Win Rate")]
    win_rate: String,
    #[tabled(rename = "💰 Total P&L")]
    total_pnl: String,
    #[tabled(rename = "✅ Winners")]
    winners: usize,
    #[tabled(rename = "❌ Losers")]
    losers: usize,
    #[tabled(rename = "📈 Avg P&L/Trade")]
    avg_pnl: String,
    #[tabled(rename = "🚀 Best Trade")]
    best_trade: String,
    #[tabled(rename = "📉 Worst Trade")]
    worst_trade: String,
}

/// Background task to display positions table every 10 seconds
pub async fn monitor_positions_display(shutdown: Arc<Notify>) {
    loop {
        // Display the positions table
        display_positions_table().await;

        // Wait 10 seconds or until shutdown
        if check_shutdown_or_delay(&shutdown, Duration::from_secs(5)).await {
            log(LogTag::Trader, "INFO", "positions display monitor shutting down...");
            break;
        }
    }
}

pub async fn display_positions_table() {
    let (open_positions, closed_positions, open_count, closed_count, total_invested, total_pnl) = {
        let all_positions = SAVED_POSITIONS.lock().unwrap();

        // Separate open and closed positions
        let open_positions: Vec<Position> = all_positions
            .iter()
            .filter(|p| p.exit_time.is_none())
            .cloned()
            .collect();
        let closed_positions: Vec<Position> = all_positions
            .iter()
            .filter(|p| p.exit_time.is_some())
            .cloned()
            .collect();

        let open_count = open_positions.len();
        let closed_count = closed_positions.len();
        let total_invested: f64 = open_positions
            .iter()
            .map(|p| p.entry_size_sol)
            .sum();
        let total_pnl: f64 = closed_positions
            .iter()
            .map(|p| {
                let (pnl_sol, _) = calculate_position_pnl(p, None);
                pnl_sol
            })
            .sum();

        (open_positions, closed_positions, open_count, closed_count, total_invested, total_pnl)
    }; // Lock is released here

    // Display bot summary section (now with owned data)
    let closed_refs: Vec<&Position> = closed_positions.iter().collect();
    display_bot_summary(&closed_refs).await;

    // Display closed positions first (last 10, sorted by close time)
    if !closed_positions.is_empty() {
        let mut sorted_closed = closed_positions.clone();
        sorted_closed.sort_by_key(|p| p.exit_time.unwrap_or(Utc::now()));

        let recent_closed: Vec<_> = sorted_closed
            .iter()
            .rev() // Most recent first
            .take(10) // Take last 10
            .rev() // Reverse back so oldest of the 10 is first
            .map(|position| ClosedPositionDisplay::from_position(position))
            .collect();

        if !recent_closed.is_empty() {
            println!("\n🔒 Recently Closed Positions (Last 10):");
            let mut closed_table = Table::new(recent_closed);
            closed_table
                .with(Style::rounded())
                .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
            println!("{}", closed_table);
            println!("");
        }
    }

    // Display open positions (sorted by entry time, latest at bottom)
    if !open_positions.is_empty() {
        let mut sorted_open = open_positions.clone();
        sorted_open.sort_by_key(|p| p.entry_time);

        let open_position_displays: Vec<_> = sorted_open
            .iter()
            .map(|position| {
                // Get current price for this position (use pool price for open positions)
                let current_price = get_current_token_price(&position.mint, true);
                OpenPositionDisplay::from_position(position, current_price)
            })
            .collect();

        println!("\n🔄 Open Positions:");
        let mut open_table = Table::new(open_position_displays);
        open_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("{}", open_table);
        println!("");
    }
}

/// Displays all positions in a beautifully formatted table
pub async fn display_bot_summary(closed_positions: &[&Position]) {
    // Calculate trading statistics
    let total_trades = closed_positions.len();
    let profitable_trades = closed_positions
        .iter()
        .filter(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol > 0.0
        })
        .count();
    let losing_trades = closed_positions
        .iter()
        .filter(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol < 0.0
        })
        .count();
    let win_rate = if total_trades > 0 {
        ((profitable_trades as f64) / (total_trades as f64)) * 100.0
    } else {
        0.0
    };

    let total_pnl: f64 = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .sum();
    let avg_pnl_per_trade = if total_trades > 0 { total_pnl / (total_trades as f64) } else { 0.0 };

    let best_trade = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let worst_trade = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    // Get wallet balance
    let wallet_balance = match crate::wallet::get_wallet_address() {
        Ok(address) => {
            match crate::wallet::get_sol_balance(&address).await {
                Ok(balance) => format!("{:.6} SOL", balance),
                Err(_) => "Error fetching".to_string(),
            }
        }
        Err(_) => "Error getting address".to_string(),
    };

    // Create bot summary display data
    let summary = BotSummaryDisplay {
        wallet_balance,
        total_trades,
        win_rate: format!("{:.1}%", win_rate),
        total_pnl: format!("{:+.6} SOL", total_pnl),
        winners: profitable_trades,
        losers: losing_trades,
        avg_pnl: format!("{:+.6} SOL", avg_pnl_per_trade),
        best_trade: format!("{:+.6} SOL", best_trade),
        worst_trade: format!("{:+.6} SOL", worst_trade),
    };

    println!("\n🤖 Bot Summary");
    let mut summary_table = Table::new(vec![summary]);
    summary_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    println!("{}", summary_table);
    println!("");
}

impl ClosedPositionDisplay {
    fn from_position(position: &Position) -> Self {
        // For closed positions, prioritize effective exit price over regular exit price
        let exit_price = position.effective_exit_price.unwrap_or(
            position.exit_price.unwrap_or(0.0)
        );

        let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);

        let pnl_sol_str = if pnl_sol >= 0.0 {
            format!("+{:.6}", pnl_sol)
        } else {
            format!("{:.6}", pnl_sol)
        };

        let pnl_percent_str = if pnl_percent >= 0.0 {
            format!("+{:.2}%", pnl_percent)
        } else {
            format!("{:.2}%", pnl_percent)
        };

        let duration = if let Some(exit_time) = position.exit_time {
            format_duration_compact(position.entry_time, exit_time)
        } else {
            format_duration_compact(position.entry_time, Utc::now())
        };

        let status = if pnl_sol >= 0.0 { "✅ CLOSED".to_string() } else { "❌ CLOSED".to_string() };

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.8}", effective_price)
            } else {
                format!("{:.8}", position.entry_price)
            },
            exit_price: format!("{:.8}", exit_price),
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            duration,
            status,
        }
    }
}

impl OpenPositionDisplay {
    fn from_position(position: &Position, current_price: Option<f64>) -> Self {
        let current_price_str = if let Some(price) = current_price {
            format!("{:.8}", price)
        } else {
            "N/A".to_string()
        };

        let (pnl_sol_str, pnl_percent_str) = if let Some(price) = current_price {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price));
            let sol_str = if pnl_sol >= 0.0 {
                format!("+{:.6}", pnl_sol)
            } else {
                format!("{:.6}", pnl_sol)
            };
            let percent_str = if pnl_percent >= 0.0 {
                format!("+{:.2}%", pnl_percent)
            } else {
                format!("{:.2}%", pnl_percent)
            };
            (sol_str, percent_str)
        } else {
            ("N/A".to_string(), "N/A".to_string())
        };

        let duration = format_duration_compact(position.entry_time, Utc::now());

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.8}", effective_price)
            } else {
                format!("{:.8}", position.entry_price)
            },
            current_price: current_price_str,
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            duration,
            status: "🔄 OPEN".to_string(),
        }
    }
}
