use crate::trader::*;
use crate::positions::*;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::utils::*;
use crate::global::STARTUP_TIME;
use crate::ata_cleanup::{ get_ata_cleanup_statistics, get_failed_ata_count };
// TODO: Replace with new pool price system
// use crate::pool_price_manager::refresh_open_position_prices;

use chrono::{ Utc };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };

/// Display structure for closed positions with specific "Exit" column
#[derive(Tabled)]
pub struct ClosedPositionDisplay {
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "📈 Entry")]
    entry_price: String,
    #[tabled(rename = "🚪 Exit")]
    exit_price: String,
    #[tabled(rename = "💰 Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "💸 P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "📊 P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "⏱️ Duration")]
    duration: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for open positions with specific "Price" column
#[derive(Tabled)]
pub struct OpenPositionDisplay {
    #[tabled(rename = "🏷️ Symbol")]
    symbol: String,
    #[tabled(rename = "🔑 Mint")]
    mint: String,
    #[tabled(rename = "📈 Entry")]
    entry_price: String,
    #[tabled(rename = "💲 Price")]
    current_price: String,
    #[tabled(rename = "💰 Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "💸 P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "📊 P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "⏱️ Duration")]
    duration: String,
    #[tabled(rename = "🎯 Status")]
    status: String,
}

/// Display structure for bot summary overview
#[derive(Tabled)]
pub struct BotOverviewDisplay {
    #[tabled(rename = "💼 Wallet Balance")]
    wallet_balance: String,
    #[tabled(rename = "🔄 Open Positions")]
    open_positions: String,
    #[tabled(rename = "📊 Total Trades")]
    total_trades: usize,
    #[tabled(rename = "⏰ Bot Uptime")]
    bot_uptime: String,
    #[tabled(rename = "💸 Total P&L")]
    total_pnl: String,
}

/// Display structure for detailed trading statistics
#[derive(Tabled)]
pub struct TradingStatsDisplay {
    #[tabled(rename = "🎯 Win Rate")]
    win_rate: String,
    #[tabled(rename = "🏆 Winners")]
    winners: usize,
    #[tabled(rename = "❌ Losers")]
    losers: usize,
    #[tabled(rename = "⚖️ Break-even")]
    break_even: usize,
    #[tabled(rename = "📊 Avg P&L/Trade")]
    avg_pnl: String,
    #[tabled(rename = "💰 Trade Volume")]
    total_volume: String,
}

/// Display structure for performance metrics
#[derive(Tabled)]
pub struct PerformanceDisplay {
    #[tabled(rename = "🚀 Best Trade")]
    best_trade: String,
    #[tabled(rename = "💀 Worst Trade")]
    worst_trade: String,
    #[tabled(rename = "⚡ Profit Factor")]
    profit_factor: String,
    #[tabled(rename = "📉 Max Drawdown")]
    max_drawdown: String,
    #[tabled(rename = "🔥 Best Streak")]
    best_streak: String,
    #[tabled(rename = "🧊 Worst Streak")]
    worst_streak: String,
}

/// Display structure for ATA cleanup statistics
#[derive(Tabled)]
pub struct AtaCleanupDisplay {
    #[tabled(rename = "🧹 ATAs Closed")]
    atas_closed: String,
    #[tabled(rename = "💰 Rent Reclaimed")]
    rent_reclaimed: String,
    #[tabled(rename = "❌ Failed Cache")]
    failed_cache: String,
    #[tabled(rename = "⏰ Last Cleanup")]
    last_cleanup: String,
}

/// Background task to display positions table every 10 seconds
pub async fn monitor_positions_display(shutdown: Arc<Notify>) {
    loop {
        // Display the positions table
        display_positions_table().await;

        // Wait 10 seconds or until shutdown
        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(SUMMARY_DISPLAY_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "positions display monitor shutting down...");
            break;
        }
    }
}

pub async fn display_positions_table() {
    // The new pool price system runs in background and continuously updates prices
    // for open positions, so we don't need to refresh them here

    let (open_positions, closed_positions, _open_count, _closed_count, total_invested, total_pnl) =
        {
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

    // Log position summary to file
    // log_positions_summary(&open_positions, &closed_positions, total_invested, total_pnl).await;

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
            println!("\n📋 Recently Closed Positions (Last 10):");
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

        let open_position_displays: Vec<_> = {
            // Collect all mints that need prices
            let mints: Vec<String> = sorted_open
                .iter()
                .map(|position| position.mint.clone())
                .collect();

            // Fetch all prices in one batch call (much faster!)
            let price_map = crate::tokens::get_current_token_prices_batch(&mints).await;

            // Build displays with fetched prices
            let mut displays = Vec::new();
            for position in &sorted_open {
                let current_price = price_map.get(&position.mint).copied().flatten();
                displays.push(OpenPositionDisplay::from_position(position, current_price));
            }
            displays
        };

        println!("\n🔄 Open Positions ({}):", open_positions.len());
        let mut open_table = Table::new(open_position_displays);
        open_table
            .with(Style::rounded())
            .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("{}", open_table);
        println!("");
    }
}

/// Convenience function to display bot summary using current positions
pub async fn display_current_bot_summary() {
    let closed_positions_refs: Vec<_> = {
        let positions = SAVED_POSITIONS.lock().unwrap();
        positions
            .iter()
            .filter(|p| p.exit_time.is_some())
            .cloned()
            .collect()
    };

    let refs: Vec<&_> = closed_positions_refs.iter().collect();
    display_bot_summary(&refs).await;
}

/// Displays comprehensive bot summary with detailed statistics and performance metrics
pub async fn display_bot_summary(closed_positions: &[&Position]) {
    // Get open positions count
    let open_count = {
        let all_positions = SAVED_POSITIONS.lock().unwrap();
        all_positions
            .iter()
            .filter(|p| p.exit_time.is_none())
            .count()
    };

    // Calculate comprehensive trading statistics
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
    let break_even_trades = total_trades - profitable_trades - losing_trades;

    let win_rate = if total_trades > 0 {
        ((profitable_trades as f64) / (total_trades as f64)) * 100.0
    } else {
        0.0
    };

    // Calculate P&L metrics
    let pnl_values: Vec<f64> = closed_positions
        .iter()
        .map(|p| {
            let (pnl_sol, _) = calculate_position_pnl(p, None);
            pnl_sol
        })
        .collect();

    let total_pnl: f64 = pnl_values.iter().sum();
    let avg_pnl_per_trade = if total_trades > 0 { total_pnl / (total_trades as f64) } else { 0.0 };

    let best_trade = pnl_values
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);

    let worst_trade = pnl_values
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .copied()
        .unwrap_or(0.0);

    // Calculate advanced metrics
    let total_volume = closed_positions
        .iter()
        .map(|p| p.entry_size_sol)
        .sum::<f64>();

    let total_gains: f64 = pnl_values
        .iter()
        .filter(|&&x| x > 0.0)
        .sum();
    let total_losses: f64 = pnl_values
        .iter()
        .filter(|&&x| x < 0.0)
        .sum::<f64>()
        .abs();
    let profit_factor = if total_losses > 0.0 { total_gains / total_losses } else { 0.0 };

    // Calculate streaks
    let (best_streak, worst_streak) = calculate_win_loss_streaks(&pnl_values);

    // Calculate maximum drawdown
    let max_drawdown = calculate_max_drawdown(&pnl_values);

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

    // Calculate bot uptime
    let uptime = format_duration_compact(*STARTUP_TIME, Utc::now());

    // Create display structures
    let overview = BotOverviewDisplay {
        wallet_balance,
        open_positions: format!("{}", open_count),
        total_trades,
        bot_uptime: uptime,
        total_pnl: format!("{:+.6} SOL", total_pnl),
    };

    let trading_stats = TradingStatsDisplay {
        win_rate: format!("{:.1}%", win_rate),
        winners: profitable_trades,
        losers: losing_trades,
        break_even: break_even_trades,
        avg_pnl: format!("{:+.6} SOL", avg_pnl_per_trade),
        total_volume: format!("{:.3} SOL", total_volume),
    };

    let performance = PerformanceDisplay {
        best_trade: format!("{:+.6} SOL", best_trade),
        worst_trade: format!("{:+.6} SOL", worst_trade),
        profit_factor: format!("{:.2}", profit_factor),
        max_drawdown: format!("{:.2}%", max_drawdown),
        best_streak: format!("{} wins", best_streak),
        worst_streak: format!("{} losses", worst_streak),
    };

    // Get ATA cleanup statistics
    let ata_stats = get_ata_cleanup_statistics();
    let failed_ata_count = get_failed_ata_count();

    let ata_cleanup = AtaCleanupDisplay {
        atas_closed: format!("{}", ata_stats.total_closed),
        rent_reclaimed: format!("{:.6} SOL", ata_stats.total_rent_reclaimed),
        failed_cache: format!("{} ATAs", failed_ata_count),
        last_cleanup: ata_stats.last_cleanup_time.unwrap_or_else(|| "Never".to_string()),
    };

    // Display all tables
    println!("\n📊 Bot Overview");
    let mut overview_table = Table::new(vec![overview]);
    overview_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    println!("{}", overview_table);

    println!("\n📈 Trading Statistics");
    let mut stats_table = Table::new(vec![trading_stats]);
    stats_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    println!("{}", stats_table);

    println!("\n🎯 Performance Metrics");
    let mut performance_table = Table::new(vec![performance]);
    performance_table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    println!("{}", performance_table);

    println!("\n🧹 ATA Cleanup Statistics");
    let mut ata_table = Table::new(vec![ata_cleanup]);
    ata_table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
    println!("{}", ata_table);

    // Display frozen account cooldowns if any exist
    let active_cooldowns = crate::positions::get_active_frozen_cooldowns();
    if !active_cooldowns.is_empty() {
        println!("\n❄️ Frozen Account Cooldowns");
        for (mint, remaining_minutes) in active_cooldowns {
            let short_mint = format!("{}...", &mint[..8]);
            println!("  {} - {} minutes remaining", short_mint, remaining_minutes);
        }
    }

    println!("");
}

/// Calculate consecutive win/loss streaks
fn calculate_win_loss_streaks(pnl_values: &[f64]) -> (usize, usize) {
    if pnl_values.is_empty() {
        return (0, 0);
    }

    let mut best_win_streak = 0;
    let mut worst_loss_streak = 0;
    let mut current_win_streak = 0;
    let mut current_loss_streak = 0;

    for &pnl in pnl_values {
        if pnl > 0.0 {
            current_win_streak += 1;
            current_loss_streak = 0;
            best_win_streak = best_win_streak.max(current_win_streak);
        } else if pnl < 0.0 {
            current_loss_streak += 1;
            current_win_streak = 0;
            worst_loss_streak = worst_loss_streak.max(current_loss_streak);
        } else {
            // Break even trades reset both streaks
            current_win_streak = 0;
            current_loss_streak = 0;
        }
    }

    (best_win_streak, worst_loss_streak)
}

/// Calculate maximum drawdown percentage
fn calculate_max_drawdown(pnl_values: &[f64]) -> f64 {
    if pnl_values.is_empty() {
        return 0.0;
    }

    let mut running_total = 0.0_f64;
    let mut peak = 0.0_f64;
    let mut max_drawdown = 0.0_f64;

    for &pnl in pnl_values {
        running_total += pnl;
        peak = peak.max(running_total);
        let drawdown = ((peak - running_total) / peak.max(0.001)) * 100.0; // Avoid division by zero
        max_drawdown = max_drawdown.max(drawdown);
    }

    max_drawdown
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

        let status = get_profit_status_emoji(pnl_sol, pnl_percent, true);

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.11}", effective_price)
            } else {
                format!("{:.11}", position.entry_price)
            },
            exit_price: format!("{:.11}", exit_price),
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
            format!("{:.11}", price)
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

        let status = if let Some(price) = current_price {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(price));
            get_profit_status_emoji(pnl_sol, pnl_percent, false)
        } else {
            "OPEN".to_string()
        };

        Self {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.11}", effective_price)
            } else {
                format!("{:.11}", position.entry_price)
            },
            current_price: current_price_str,
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            duration,
            status,
        }
    }
}

/// Generate profit-based status for positions
fn get_profit_status_emoji(_pnl_sol: f64, pnl_percent: f64, is_closed: bool) -> String {
    let base_status = if is_closed { "CLOSED" } else { "OPEN" };

    if pnl_percent >= 50.0 {
        format!("🚀 {}", base_status) // Moon shot gains
    } else if pnl_percent >= 20.0 {
        format!("🔥 {}", base_status) // Hot gains
    } else if pnl_percent >= 10.0 {
        format!("💰 {}", base_status) // Good profits
    } else if pnl_percent >= 5.0 {
        format!("📈 {}", base_status) // Modest gains
    } else if pnl_percent >= 0.0 {
        format!("✅ {}", base_status) // Small gains
    } else if pnl_percent >= -5.0 {
        format!("⚠️ {}", base_status) // Small loss
    } else if pnl_percent >= -10.0 {
        format!("📉 {}", base_status) // Moderate loss
    } else if pnl_percent >= -20.0 {
        format!("❌ {}", base_status) // Significant loss
    } else if pnl_percent >= -50.0 {
        format!("💀 {}", base_status) // Major loss
    } else {
        format!("🔴 {}", base_status) // Devastating loss
    }
}

/// Log positions summary to log file in simple format
async fn log_positions_summary(
    open_positions: &[Position],
    closed_positions: &[Position],
    total_invested: f64,
    total_pnl: f64
) {
    // Log overview
    log(
        LogTag::System,
        "POSITIONS",
        &format!(
            "Summary: {} open positions, {} closed positions, {:.6} SOL invested, {:+.6} SOL total P&L",
            open_positions.len(),
            closed_positions.len(),
            total_invested,
            total_pnl
        )
    );

    // Log open positions
    if !open_positions.is_empty() {
        log(LogTag::System, "OPEN_POS", &format!("Open positions ({})", open_positions.len()));

        // Get all prices in batch for efficiency
        let mints: Vec<String> = open_positions
            .iter()
            .map(|p| p.mint.clone())
            .collect();
        let price_map = crate::tokens::get_current_token_prices_batch(&mints).await;

        for position in open_positions {
            let current_price = price_map.get(&position.mint).copied().flatten();
            let (pnl_sol, pnl_percent) = if let Some(price) = current_price {
                calculate_position_pnl(position, Some(price))
            } else {
                (0.0, 0.0)
            };

            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            let current_price_str = current_price
                .map(|p| format!("{:.8}", p))
                .unwrap_or("N/A".to_string());
            let duration = format_duration_compact(position.entry_time, Utc::now());

            log(
                LogTag::System,
                "OPEN_POS",
                &format!(
                    "{} | Entry: {:.8} | Current: {} | Size: {:.6} SOL | P&L: {:+.6} SOL ({:+.2}%) | Duration: {}",
                    position.symbol,
                    entry_price,
                    current_price_str,
                    position.entry_size_sol,
                    pnl_sol,
                    pnl_percent,
                    duration
                )
            );
        }
    }

    // Log recent closed positions (last 5)
    if !closed_positions.is_empty() {
        let mut sorted_closed = closed_positions.to_vec();
        sorted_closed.sort_by_key(|p| p.exit_time.unwrap_or(Utc::now()));

        let recent_closed: Vec<_> = sorted_closed.iter().rev().take(5).collect();

        log(
            LogTag::System,
            "CLOSED_POS",
            &format!("Recent closed positions (last {})", recent_closed.len())
        );
        for position in recent_closed {
            let (pnl_sol, pnl_percent) = calculate_position_pnl(position, None);
            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            let exit_price = position.effective_exit_price.unwrap_or(
                position.exit_price.unwrap_or(0.0)
            );
            let duration = if let Some(exit_time) = position.exit_time {
                format_duration_compact(position.entry_time, exit_time)
            } else {
                "N/A".to_string()
            };

            log(
                LogTag::System,
                "CLOSED_POS",
                &format!(
                    "{} | Entry: {:.8} | Exit: {:.8} | Size: {:.6} SOL | P&L: {:+.6} SOL ({:+.2}%) | Duration: {}",
                    position.symbol,
                    entry_price,
                    exit_price,
                    position.entry_size_sol,
                    pnl_sol,
                    pnl_percent,
                    duration
                )
            );
        }
    }
}
