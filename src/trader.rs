/// Trading configuration constants
pub const PRICE_DROP_THRESHOLD_PERCENT: f64 = 2.5;
pub const PROFIT_THRESHOLD_PERCENT: f64 = 5.0;
// pub const DEFAULT_FEE: f64 = 0.0000025 + 0.000006 + 0.000001;
pub const DEFAULT_FEE: f64 = 0.0;
pub const DEFAULT_SLIPPAGE: f64 = 3.0; // 5% slippage

pub const TRADE_SIZE_SOL: f64 = 0.0001;
pub const STOP_LOSS_PERCENT: f64 = -99.0;
pub const PRICE_HISTORY_HOURS: i64 = 24;
pub const NEW_ENTRIES_CHECK_INTERVAL_SECS: u64 = 5;
pub const OPEN_POSITIONS_CHECK_INTERVAL_SECS: u64 = 5;
pub const MAX_OPEN_POSITIONS: usize = 3;

/// ATA (Associated Token Account) management configuration
pub const CLOSE_ATA_AFTER_SELL: bool = true; // Set to false to disable ATA closing

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::utils::*;
use crate::wallet::{ buy_token, sell_token };
use crate::profit_calculation::{ PROFIT_SYSTEM, AccuratePnL };

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use serde::{ Serialize, Deserialize };
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };
use colored::Colorize;

#[derive(Serialize, Deserialize, Clone)]
pub struct Position {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_price: Option<f64>,
    pub exit_time: Option<DateTime<Utc>>,
    pub pnl_sol: Option<f64>,
    pub pnl_percent: Option<f64>,
    pub position_type: String, // "buy" or "sell"
    pub entry_size_sol: f64,
    pub total_size_sol: f64,
    pub drawdown_percent: f64,
    pub price_highest: f64,
    pub price_lowest: f64,
    // Real swap tracking
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>, // Amount of tokens bought/sold
    pub effective_entry_price: Option<f64>, // Actual price from on-chain transaction
    pub effective_exit_price: Option<f64>, // Actual exit price from on-chain transaction
}

/// Display structure for position table formatting
#[derive(Tabled)]
pub struct PositionDisplay {
    #[tabled(rename = "Symbol")]
    symbol: String,
    #[tabled(rename = "Mint")]
    mint: String,
    #[tabled(rename = "Entry Price")]
    entry_price: String,
    #[tabled(rename = "Current/Exit")]
    current_or_exit: String,
    #[tabled(rename = "Size (SOL)")]
    size_sol: String,
    #[tabled(rename = "P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "Drawdown")]
    drawdown: String,
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

impl PositionDisplay {
    fn from_position(position: &Position, current_price: Option<f64>) -> Self {
        let current_or_exit = if let Some(exit_price) = position.exit_price {
            format!("{:.8}", exit_price)
        } else if let Some(price) = current_price {
            format!("{:.8}", price)
        } else {
            "N/A".to_string()
        };

        let pnl_sol_str = if let Some(pnl) = position.pnl_sol {
            if pnl >= 0.0 { format!("+{:.6}", pnl) } else { format!("{:.6}", pnl) }
        } else if let Some(price) = current_price {
            // Use new accurate profit calculation system
            if let Ok(profit_system) = PROFIT_SYSTEM.lock() {
                let token_decimals = {
                    let tokens = LIST_TOKENS.read().unwrap();
                    tokens
                        .iter()
                        .find(|t| t.mint == position.mint)
                        .map(|t| t.decimals)
                };

                let accurate_pnl = profit_system.calculate_accurate_pnl(
                    position,
                    price,
                    token_decimals
                );

                if accurate_pnl.pnl_sol >= 0.0 {
                    format!("+{:.6}", accurate_pnl.pnl_sol)
                } else {
                    format!("{:.6}", accurate_pnl.pnl_sol)
                }
            } else {
                // Fallback to old calculation
                let token_decimals = {
                    let tokens = LIST_TOKENS.read().unwrap();
                    tokens
                        .iter()
                        .find(|t| t.mint == position.mint)
                        .map(|t| t.decimals)
                };

                let (current_pnl, _) = calculate_position_pnl_from_swaps(
                    position,
                    price,
                    token_decimals
                );

                if current_pnl >= 0.0 {
                    format!("+{:.6}", current_pnl)
                } else {
                    format!("{:.6}", current_pnl)
                }
            }
        } else {
            "N/A".to_string()
        };

        let pnl_percent_str = if let Some(pnl_pct) = position.pnl_percent {
            if pnl_pct >= 0.0 { format!("+{:.2}%", pnl_pct) } else { format!("{:.2}%", pnl_pct) }
        } else if let Some(price) = current_price {
            // Use new accurate profit calculation system
            if let Ok(profit_system) = PROFIT_SYSTEM.lock() {
                let token_decimals = {
                    let tokens = LIST_TOKENS.read().unwrap();
                    tokens
                        .iter()
                        .find(|t| t.mint == position.mint)
                        .map(|t| t.decimals)
                };

                let accurate_pnl = profit_system.calculate_accurate_pnl(
                    position,
                    price,
                    token_decimals
                );

                if accurate_pnl.pnl_percent >= 0.0 {
                    format!("+{:.2}%", accurate_pnl.pnl_percent)
                } else {
                    format!("{:.2}%", accurate_pnl.pnl_percent)
                }
            } else {
                // Fallback to old calculation
                let token_decimals = {
                    let tokens = LIST_TOKENS.read().unwrap();
                    tokens
                        .iter()
                        .find(|t| t.mint == position.mint)
                        .map(|t| t.decimals)
                };

                let (_, current_pnl_percent) = calculate_position_pnl_from_swaps(
                    position,
                    price,
                    token_decimals
                );

                if current_pnl_percent >= 0.0 {
                    format!("+{:.2}%", current_pnl_percent)
                } else {
                    format!("{:.2}%", current_pnl_percent)
                }
            }
        } else {
            "N/A".to_string()
        };

        let duration = if let Some(exit_time) = position.exit_time {
            format_duration_compact(position.entry_time, exit_time)
        } else {
            format_duration_compact(position.entry_time, Utc::now())
        };

        let status = if position.exit_price.is_some() {
            if position.pnl_sol.unwrap_or(0.0) >= 0.0 {
                "✅ CLOSED".to_string()
            } else {
                "❌ CLOSED".to_string()
            }
        } else {
            "🔄 OPEN".to_string()
        };

        // Keep full mint address for readability
        let mint_display = position.mint.clone();

        Self {
            symbol: position.symbol.clone(),
            mint: mint_display,
            entry_price: if let Some(effective_price) = position.effective_entry_price {
                format!("{:.8}", effective_price)
            } else {
                format!("{:.8}", position.entry_price)
            },
            current_or_exit: current_or_exit,
            size_sol: format!("{:.6}", position.entry_size_sol),
            pnl_sol: pnl_sol_str,
            pnl_percent: pnl_percent_str,
            drawdown: format!("-{:.2}%", position.drawdown_percent),
            duration,
            status,
        }
    }
}

/// Helper function to format duration in a compact way
fn format_duration_compact(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}d{}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Static global: saved positions
pub static SAVED_POSITIONS: Lazy<StdArc<StdMutex<Vec<Position>>>> = Lazy::new(|| {
    let positions = load_positions_from_file();
    StdArc::new(StdMutex::new(positions))
});

/// Static global: price history for each token (mint), stores Vec<(timestamp, price)>
pub static PRICE_HISTORY_24H: Lazy<
    StdArc<StdMutex<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>
> = Lazy::new(|| StdArc::new(StdMutex::new(HashMap::new())));

/// Static global: last known prices for each token
pub static LAST_PRICES: Lazy<StdArc<StdMutex<HashMap<String, f64>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashMap::new()))
});

/// Gets the current count of open positions
fn get_open_positions_count() -> usize {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
            .count()
    } else {
        0
    }
}

pub fn should_sell(pos: &Position, current_price: f64, now: DateTime<Utc>) -> f64 {
    // Use the new smart profit calculation system
    if let Ok(mut profit_system) = PROFIT_SYSTEM.lock() {
        // Get token decimals from global token list
        let token_decimals = {
            let tokens = LIST_TOKENS.read().unwrap();
            tokens
                .iter()
                .find(|t| t.mint == pos.mint)
                .map(|t| t.decimals)
        };

        let (urgency, reason) = profit_system.should_sell_smart(
            pos,
            current_price,
            now,
            token_decimals
        );

        // Log the sell decision for debugging
        if urgency > 0.1 {
            log(
                LogTag::Trader,
                "SELL_DECISION",
                &format!("{} - Urgency: {:.2}, Reason: {}", pos.symbol, urgency, reason)
            );
        }

        return urgency;
    }

    // Fallback to original logic if profit system is unavailable
    let time_held_secs: f64 = (now - pos.entry_time).num_seconds() as f64;

    // More conservative fallback settings
    const MIN_HOLD_TIME_SECS: f64 = 180.0; // Hold for at least 3 minutes
    const STOP_LOSS_PERCENT: f64 = -30.0; // More conservative stop loss

    // Don't sell too early unless it's a major loss
    if time_held_secs < MIN_HOLD_TIME_SECS {
        let entry_price_to_use = pos.effective_entry_price.unwrap_or(pos.entry_price);
        let price_change_percent =
            ((current_price - entry_price_to_use) / entry_price_to_use) * 100.0;

        if price_change_percent <= STOP_LOSS_PERCENT {
            return 1.0; // Emergency exit for major losses
        } else {
            return 0.0; // Hold for minimum time
        }
    }

    // Use original logic for positions held longer than minimum time
    const MAX_HOLD_TIME_SECS: f64 = 3600.0;
    const PROFIT_TARGET_PERCENT: f64 = 25.0; // More realistic profit target
    const TRAILING_STOP_PERCENT: f64 = 8.0; // Wider trailing stop
    const TIME_DECAY_START_SECS: f64 = 1800.0;

    let entry_price_to_use = pos.effective_entry_price.unwrap_or(pos.entry_price);

    let current_pnl_percent: f64 = if entry_price_to_use > 0.0 {
        let price_change_percent =
            ((current_price - entry_price_to_use) / entry_price_to_use) * 100.0;

        // Use more accurate fee calculation
        let total_fee_cost = 2.0 * DEFAULT_FEE;
        let fee_percent = (total_fee_cost / pos.entry_size_sol) * 100.0;

        price_change_percent - fee_percent
    } else {
        0.0
    };

    // Decision logic
    let stop_loss_triggered: bool = current_pnl_percent <= STOP_LOSS_PERCENT;
    let profit_target_reached: bool = current_pnl_percent >= PROFIT_TARGET_PERCENT;

    // Trailing stop logic
    let peak_price: f64 = f64::max(pos.price_highest, current_price);
    let drawdown_percent: f64 = if peak_price > 0.0 {
        ((current_price - peak_price) / peak_price) * 100.0
    } else {
        0.0
    };
    let trailing_stop_triggered: bool =
        current_pnl_percent >= PROFIT_TARGET_PERCENT && drawdown_percent <= -TRAILING_STOP_PERCENT;

    // Time decay factor
    let time_decay_factor: f64 = if time_held_secs > TIME_DECAY_START_SECS {
        let decay_duration = MAX_HOLD_TIME_SECS - TIME_DECAY_START_SECS;
        let excess_time = time_held_secs - TIME_DECAY_START_SECS;
        let time_decay = excess_time / decay_duration;
        f64::min(time_decay, 1.0)
    } else {
        0.0
    };

    // Calculate urgency
    let mut urgency: f64 = 0.0;

    if stop_loss_triggered {
        urgency = 1.0;
    } else if trailing_stop_triggered {
        urgency = 0.9;
    } else if profit_target_reached {
        urgency = 0.8;
    } else {
        urgency = time_decay_factor * 0.4; // Reduced time pressure
    }

    // Less aggressive selling for positions with small losses
    if
        time_held_secs > TIME_DECAY_START_SECS &&
        current_pnl_percent <= 0.0 &&
        current_pnl_percent > -15.0
    {
        urgency = f64::max(urgency, 0.3); // Reduced urgency for small losses
    }

    urgency = f64::max(0.0, f64::min(urgency, 1.0));
    urgency
}

/// Checks recent transactions to see if position was already closed
/// Enhanced version with strict validation to prevent phantom sells
async fn check_recent_transactions_for_position(position: &mut Position) -> bool {
    // Get wallet address
    let wallet_address = match crate::wallet::get_wallet_address() {
        Ok(addr) => addr,
        Err(_) => {
            log(LogTag::Trader, "ERROR", "Failed to get wallet address for transaction check");
            return false;
        }
    };

    // Don't auto-close positions that are too new - they need time for balance to settle
    let min_age_for_auto_close = chrono::Duration::seconds(30);
    let position_age = Utc::now() - position.entry_time;

    if position_age < min_age_for_auto_close {
        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "Position {} too new ({:.1}s) for auto-close detection - skipping",
                position.symbol,
                position_age.num_seconds()
            )
        );
        return false;
    }

    // Perform multiple balance checks with delays to ensure consistency
    let mut balance_checks = Vec::new();
    let check_count = 3;

    for attempt in 1..=check_count {
        match crate::wallet::get_token_balance(&wallet_address, &position.mint).await {
            Ok(balance) => {
                balance_checks.push(balance);
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Balance check {}/{} for {}: {} tokens",
                        attempt,
                        check_count,
                        position.symbol,
                        balance
                    )
                );
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "Balance check {}/{} failed for {}: {}",
                        attempt,
                        check_count,
                        position.symbol,
                        e
                    )
                );
            }
        }

        // Add delay between checks (except for the last one)
        if attempt < check_count {
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        }
    }

    // Require at least 2 successful balance checks
    if balance_checks.len() < 2 {
        log(
            LogTag::Trader,
            "WARN",
            &format!(
                "Insufficient balance checks for {} - cannot determine auto-close",
                position.symbol
            )
        );
        return false;
    }

    // Check if all balance checks consistently show 0 tokens
    let all_zero = balance_checks.iter().all(|&balance| balance == 0);
    let consistent = balance_checks.windows(2).all(|w| w[0] == w[1]);

    if !consistent {
        log(
            LogTag::Trader,
            "WARN",
            &format!(
                "Inconsistent balance checks for {} - results: {:?}",
                position.symbol,
                balance_checks
            )
        );
        return false;
    }

    let stored_amount = position.token_amount.unwrap_or(0);

    // Only proceed if we consistently have 0 tokens but position shows we should have tokens
    if all_zero && stored_amount > 0 {
        log(
            LogTag::Trader,
            "WARNING",
            &format!(
                "Consistent zero balance detected for {} (stored: {}) - investigating external sell",
                position.symbol,
                stored_amount
            )
        );

        // TODO: In a more complete implementation, we would search recent transaction history
        // to find the actual sell transaction signature. For now, we mark it as external sell.

        // Mark position as closed but with proper exit transaction signature indicating external sell
        let now = Utc::now();
        position.exit_time = Some(now);
        position.exit_transaction_signature = Some("EXTERNAL_SELL_DETECTED".to_string());

        // Use the last known price as exit price if not set
        if position.exit_price.is_none() {
            // Try to get current price from token list
            if let Ok(tokens_guard) = LIST_TOKENS.read() {
                if let Some(token) = tokens_guard.iter().find(|t| t.mint == position.mint) {
                    if let Some(current_price) = token.price_dexscreener_sol {
                        position.exit_price = Some(current_price);
                        position.effective_exit_price = Some(current_price);
                    }
                }
            }

            // Fallback to entry price if no current price available
            if position.exit_price.is_none() {
                position.exit_price = Some(position.entry_price);
                position.effective_exit_price = Some(position.entry_price);
            }
        }

        // Calculate P&L using effective prices when available
        if let Some(exit_price) = position.exit_price {
            // Use effective entry price if available, otherwise fallback to original entry price
            let entry_price_to_use = position.effective_entry_price.unwrap_or(position.entry_price);

            let (is_profitable, net_pnl_sol, net_pnl_percent, _) = is_position_profitable(
                entry_price_to_use,
                exit_price,
                position.entry_size_sol
            );

            position.pnl_sol = Some(net_pnl_sol);
            position.pnl_percent = Some(net_pnl_percent);

            log(
                LogTag::Trader,
                if is_profitable {
                    "PROFIT"
                } else {
                    "LOSS"
                },
                &format!(
                    "External sell detected for {} - P&L: {:.6} SOL ({:.2}%)",
                    position.symbol,
                    net_pnl_sol,
                    net_pnl_percent
                )
            );

            // Do NOT attempt to close ATA for external sells - we don't control the transaction
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "Skipping ATA close for external sell of {} - not our transaction",
                    position.symbol
                )
            );

            return true;
        }
    }

    false
}

/// Displays all positions in a beautifully formatted table
async fn display_bot_summary(closed_positions: &[&Position]) {
    // Calculate trading statistics
    let total_trades = closed_positions.len();
    let profitable_trades = closed_positions
        .iter()
        .filter(|p| p.pnl_sol.unwrap_or(0.0) > 0.0)
        .count();
    let losing_trades = closed_positions
        .iter()
        .filter(|p| p.pnl_sol.unwrap_or(0.0) < 0.0)
        .count();
    let win_rate = if total_trades > 0 {
        ((profitable_trades as f64) / (total_trades as f64)) * 100.0
    } else {
        0.0
    };

    let total_pnl: f64 = closed_positions
        .iter()
        .filter_map(|p| p.pnl_sol)
        .sum();
    let avg_pnl_per_trade = if total_trades > 0 { total_pnl / (total_trades as f64) } else { 0.0 };

    let best_trade = closed_positions
        .iter()
        .filter_map(|p| p.pnl_sol)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0);

    let worst_trade = closed_positions
        .iter()
        .filter_map(|p| p.pnl_sol)
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

/// Get current price for a token from the global token list
fn get_current_token_price(mint: &str) -> Option<f64> {
    let tokens = LIST_TOKENS.read().unwrap();

    // Find the token by mint address
    for token in tokens.iter() {
        if token.mint == mint {
            // Try to get the best available price (prioritize DexScreener SOL price)
            if let Some(price) = token.price_dexscreener_sol {
                return Some(price);
            }
            // Fallback to other price sources
            if let Some(price) = token.price_geckoterminal_sol {
                return Some(price);
            }
            if let Some(price) = token.price_raydium_sol {
                return Some(price);
            }
            if let Some(price) = token.price_pool_sol {
                return Some(price);
            }
        }
    }

    None
}

async fn display_positions_table() {
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
            .filter_map(|p| p.pnl_sol)
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
            .map(|position| PositionDisplay::from_position(position, None))
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
                // Get current price for this position
                let current_price = get_current_token_price(&position.mint);
                PositionDisplay::from_position(position, current_price)
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

/// Validates if a token has all required metadata for trading
fn validate_token(token: &Token) -> bool {
    if token.symbol.is_empty() || token.name.is_empty() {
        return false;
    }

    let has_url = token.logo_url
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_website = token.website
        .as_ref()
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // Require either logo_url or website
    has_url || has_website
}

/// Calculates if position is in profit considering fees and trade size
/// For open positions, uses actual token amounts and effective prices when available
fn is_position_profitable(
    entry_price: f64,
    current_price: f64,
    trade_size_sol: f64
) -> (bool, f64, f64, f64) {
    // Simple price-based calculation for general use (fallback only)
    // This is used for display purposes when we don't have actual token amounts
    let price_change_percent = ((current_price - entry_price) / entry_price) * 100.0;

    // Account for fixed swap fees using the hardcoded DEFAULT_FEE (buy + sell = 2 * DEFAULT_FEE)
    let total_fee_cost = 2.0 * DEFAULT_FEE; // Total cost for both buy and sell transactions
    let fee_percent = (total_fee_cost / trade_size_sol) * 100.0;
    let net_pnl_percent = price_change_percent - fee_percent;
    let net_pnl_sol = (net_pnl_percent / 100.0) * trade_size_sol;

    // Total value (initial investment + profit/loss)
    let total_value = trade_size_sol + net_pnl_sol;

    (net_pnl_sol > 0.0, net_pnl_sol, net_pnl_percent, total_value)
}

/// Updates position with current price to track extremes and drawdown
/// Drawdown % = (price_highest – current_price) / price_highest * 100
fn update_position_tracking(position: &mut Position, current_price: f64) {
    if current_price == 0.0 {
        log(
            LogTag::Trader,
            "WARN",
            &format!(
                "Skipping position tracking update for {}: current_price is zero",
                position.symbol
            )
            .yellow()
            .dimmed()
            .to_string()
        );
        return;
    }

    // On first update, set both high/low to the actual entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if position.price_highest == 0.0 {
        position.price_highest = entry_price;
        position.price_lowest  = entry_price;
    }

    let old_high = position.price_highest;
    let old_low  = position.price_lowest;
    let old_dd   = position.drawdown_percent;

    // Update running extremes
    if current_price > position.price_highest {
        position.price_highest = current_price;
    }
    if current_price < position.price_lowest {
        position.price_lowest = current_price;
    }

    // Calculate drawdown % from the peak seen so far
    let dd_pct = if position.price_highest > 0.0 {
        ((position.price_highest - current_price) / position.price_highest) * 100.0
    } else {
        0.0
    };
    position.drawdown_percent = dd_pct.max(0.0);

    // Log the transition
    log(
        LogTag::Trader,
        "DEBUG",
        &format!(
            "Track {}: entry={:.6}, current={:.6}, high={:.6}->{:.6}, low={:.6}->{:.6}, drawdown={:.2}%->{:.2}%",
            position.symbol,
            entry_price,
            current_price,
            old_high,
            position.price_highest,
            old_low,
            position.price_lowest,
            old_dd,
            position.drawdown_percent
        )
        .dimmed()
        .to_string(),
    );
}


/// Calculates accurate P&L using actual swap transaction data
pub fn calculate_position_pnl_from_swaps(
    position: &Position,
    current_price: f64,
    token_decimals: Option<u8>
) -> (f64, f64) {
    // Use actual transaction data when available for maximum accuracy
    if
        let (Some(token_amount), Some(effective_entry_price)) = (
            position.token_amount,
            position.effective_entry_price,
        )
    {
        if let Some(decimals) = token_decimals {
            // Convert raw token amount to UI amount using correct decimals
            let ui_token_amount = (token_amount as f64) / (10_f64).powi(decimals as i32);

            // Current value of tokens at current price
            let current_value_sol = ui_token_amount * current_price;

            // Net P&L = current value - initial investment - fees
            // Account for buy fee (already paid) and estimated sell fee
            let total_fee_cost = 2.0 * DEFAULT_FEE; // Buy + sell fees
            let net_pnl_sol = current_value_sol - position.entry_size_sol - total_fee_cost;
            let net_pnl_percent = (net_pnl_sol / position.entry_size_sol) * 100.0;

            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "P&L calc for {}: token_amount={}, decimals={}, ui_amount={:.6}, current_price={:.8}, current_value={:.6}, entry_size={:.6}, fees={:.6}, pnl_sol={:.6}, pnl_percent={:.2}%",
                    position.symbol,
                    token_amount,
                    decimals,
                    ui_token_amount,
                    current_price,
                    current_value_sol,
                    position.entry_size_sol,
                    total_fee_cost,
                    net_pnl_sol,
                    net_pnl_percent
                )
                    .dimmed()
                    .to_string()
            );

            return (net_pnl_sol, net_pnl_percent);
        }
    }

    // Fallback to effective entry price if available, otherwise use original entry price
    let price_to_use = position.effective_entry_price.unwrap_or(position.entry_price);
    let (_, net_pnl_sol, net_pnl_percent, _) = is_position_profitable(
        price_to_use,
        current_price,
        position.entry_size_sol
    );

    log(
        LogTag::Trader,
        "DEBUG",
        &format!(
            "P&L fallback calc for {}: entry_price={:.8}, current_price={:.8}, entry_size={:.6}, pnl_sol={:.6}, pnl_percent={:.2}%",
            position.symbol,
            price_to_use,
            current_price,
            position.entry_size_sol,
            net_pnl_sol,
            net_pnl_percent
        )
            .dimmed()
            .to_string()
    );

    (net_pnl_sol, net_pnl_percent)
}

/// Opens a new buy position for a token with real swap execution
async fn open_position(token: &Token, price: f64, percent_change: f64) {
    // Check if we already have an open position for this token and count open positions
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        if
            positions
                .iter()
                .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_price.is_none())
        {
            return; // Already have an open position for this token
        }

        // Check if we've reached the maximum open positions limit
        let open_positions_count = positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
            .count();

        if open_positions_count >= MAX_OPEN_POSITIONS {
            log(
                LogTag::Trader,
                "LIMIT",
                &format!(
                    "Maximum open positions reached ({}/{}). Skipping new position for {} ({})",
                    open_positions_count,
                    MAX_OPEN_POSITIONS,
                    token.symbol,
                    token.mint
                )
            );
            return;
        }
    }

    let colored_percent = format!("\x1b[31m{:.2}%\x1b[0m", percent_change);
    let current_open_count = get_open_positions_count();
    log(
        LogTag::Trader,
        "BUY",
        &format!(
            "Opening position for {} ({}) at {:.6} SOL ({}) - Size: {:.6} SOL [{}/{}]",
            token.symbol,
            token.mint,
            price,
            colored_percent,
            TRADE_SIZE_SOL,
            current_open_count + 1,
            MAX_OPEN_POSITIONS
        )
    );

    // Execute real buy transaction
    match buy_token(token, TRADE_SIZE_SOL, Some(price)).await {
        Ok(swap_result) => {
            // Check if the transaction was actually successful on-chain
            if !swap_result.success {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "Transaction failed on-chain for {}: {}",
                        token.symbol,
                        swap_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                    )
                );
                return;
            }

            let effective_entry_price = swap_result.effective_price.unwrap_or(price);
            let token_amount = swap_result.actual_output_change.unwrap_or(0);

            // Validate that we actually received tokens
            if token_amount == 0 {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "Transaction successful but no tokens received for {}. TX: {}",
                        token.symbol,
                        swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                    )
                );
                return;
            }

            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "Real swap executed for {}: TX: {}, Tokens: {}, Signal Price: {:.12} SOL, Effective Price: {:.12} SOL",
                    token.symbol,
                    swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                    token_amount,
                    price,
                    effective_entry_price
                )
            );

            let position = Position {
                mint: token.mint.clone(),
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                entry_price: price, // Keep original signal price
                entry_time: Utc::now(),
                exit_price: None,
                exit_time: None,
                pnl_sol: None,
                pnl_percent: None,
                position_type: "buy".to_string(),
                entry_size_sol: TRADE_SIZE_SOL,
                total_size_sol: TRADE_SIZE_SOL,
                drawdown_percent: 0.0,
                price_highest: effective_entry_price, // Use effective price for tracking
                price_lowest: effective_entry_price, // Use effective price for tracking
                entry_transaction_signature: swap_result.transaction_signature,
                exit_transaction_signature: None,
                token_amount: Some(token_amount),
                effective_entry_price: Some(effective_entry_price), // Actual transaction price
                effective_exit_price: None,
            };

            if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                positions.push(position);
                save_positions_to_file(&positions);
            }
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to execute buy swap for {} ({}): {}", token.symbol, token.mint, e)
            );
        }
    }
}

/// Closes an existing position with real sell transaction
async fn close_position(
    position: &mut Position,
    token: &Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> bool {
    // Only attempt to sell if we have tokens from the buy transaction
    if let Some(stored_token_amount) = position.token_amount {
        // Check if we actually have tokens to sell
        if stored_token_amount == 0 {
            log(
                LogTag::Trader,
                "WARNING",
                &format!(
                    "Cannot close position for {} ({}) - No tokens to sell (stored amount: 0)",
                    position.symbol,
                    position.mint
                )
            );

            // DO NOT mark position as sold when stored amount is 0
            // This indicates the position was never properly opened or already closed
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "Position {} has stored amount 0 - cannot execute sell. Position remains as-is",
                    position.symbol
                )
            );
            return false; // Don't corrupt the position
        }

        // Check actual current wallet balance before attempting to sell
        let wallet_address = match crate::wallet::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "Failed to get wallet address for {} balance check: {}",
                        position.symbol,
                        e
                    )
                );
                return false;
            }
        };

        let actual_balance = match
            crate::wallet::get_token_balance(&wallet_address, &position.mint).await
        {
            Ok(balance) => balance,
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Failed to check current {} balance: {}", position.symbol, e)
                );
                return false;
            }
        };

        // Use the minimum of stored amount and actual balance to avoid "insufficient balance" errors
        let token_amount = std::cmp::min(stored_token_amount, actual_balance);

        if token_amount == 0 {
            log(
                LogTag::Trader,
                "WARNING",
                &format!(
                    "Cannot close position for {} ({}) - No tokens in wallet (stored: {}, actual: {})",
                    position.symbol,
                    position.mint,
                    stored_token_amount,
                    actual_balance
                )
            );

            // Before marking as total loss, check if transaction might have already completed
            if check_recent_transactions_for_position(position).await {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "Successfully detected and updated completed transaction for {}",
                        position.symbol
                    )
                );
                return true;
            }

            // DO NOT mark position as sold if we can't actually execute a sell transaction
            // This prevents phantom sells and P&L corruption
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "Cannot close position for {} - insufficient tokens. Position remains OPEN",
                    position.symbol
                )
            );
            return false; // Keep position open, don't corrupt it
        }

        if actual_balance < stored_token_amount {
            log(
                LogTag::Trader,
                "WARNING",
                &format!(
                    "Balance mismatch for {} - Position stored: {}, Wallet actual: {}, Selling: {}",
                    position.symbol,
                    stored_token_amount,
                    actual_balance,
                    token_amount
                )
            );
        }

        log(
            LogTag::Trader,
            "SELL",
            &format!(
                "Closing position for {} ({}) - Selling {} tokens at {:.6} SOL",
                position.symbol,
                position.mint,
                token_amount,
                exit_price
            )
        );

        // Execute real sell transaction
        match sell_token(token, token_amount, None).await {
            Ok(swap_result) => {
                // Check if the sell transaction was actually successful on-chain
                if !swap_result.success {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "Sell transaction failed on-chain for {}: {}",
                            position.symbol,
                            swap_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                        )
                    );
                    return false; // Failed to close
                }

                let effective_exit_price = swap_result.effective_price.unwrap_or(exit_price);
                let sol_received = swap_result.actual_output_change.unwrap_or(0);
                let transaction_signature = swap_result.transaction_signature.clone();

                // Validate that we actually received SOL
                if sol_received == 0 {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "Sell transaction successful but no SOL received for {}. TX: {}",
                            position.symbol,
                            transaction_signature.as_ref().unwrap_or(&"None".to_string())
                        )
                    );
                    return false; // Failed to close properly
                }

                // Calculate actual P&L using the new profit calculation system
                let net_pnl_sol: f64;
                let net_pnl_percent: f64;

                if let Ok(profit_system) = PROFIT_SYSTEM.lock() {
                    // Use token decimals for accurate calculation
                    let token_decimals = Some(token.decimals);

                    // Create a temporary position with the exit data to calculate final P&L
                    let mut temp_position = position.clone();
                    temp_position.exit_price = Some(exit_price);
                    temp_position.effective_exit_price = Some(effective_exit_price);

                    let accurate_pnl = profit_system.calculate_accurate_pnl(
                        &temp_position,
                        effective_exit_price,
                        token_decimals
                    );

                    net_pnl_sol = accurate_pnl.pnl_sol;
                    net_pnl_percent = accurate_pnl.pnl_percent;

                    log(
                        LogTag::Trader,
                        "PNL_CALC",
                        &format!(
                            "P&L calculation for {}: Method={}, SOL={:.6}, %={:.2}%",
                            position.symbol,
                            accurate_pnl.calculation_method,
                            net_pnl_sol,
                            net_pnl_percent
                        )
                    );
                } else {
                    // Fallback calculation if profit system is unavailable
                    let actual_sol_received = crate::wallet::lamports_to_sol(sol_received);
                    net_pnl_sol = actual_sol_received - position.entry_size_sol;
                    net_pnl_percent = (net_pnl_sol / position.entry_size_sol) * 100.0;

                    log(
                        LogTag::Trader,
                        "PNL_CALC",
                        &format!(
                            "P&L calculation for {} (fallback): SOL={:.6}, %={:.2}%",
                            position.symbol,
                            net_pnl_sol,
                            net_pnl_percent
                        )
                    );
                }
                let is_profitable = net_pnl_sol > 0.0;

                position.exit_price = Some(exit_price);
                position.exit_time = Some(exit_time);
                position.pnl_sol = Some(net_pnl_sol);
                position.pnl_percent = Some(net_pnl_percent);
                position.total_size_sol = crate::wallet::lamports_to_sol(sol_received);
                position.exit_transaction_signature = transaction_signature.clone();
                position.effective_exit_price = Some(effective_exit_price);

                // Record trade performance for learning (profit system optimization)
                if let Ok(mut profit_system) = PROFIT_SYSTEM.lock() {
                    profit_system.record_trade_performance(position, "successful_sell".to_string());
                }

                let status_color = if is_profitable { "\x1b[32m" } else { "\x1b[31m" };
                let status_text = if is_profitable { "PROFIT" } else { "LOSS" };

                let actual_sol_received = crate::wallet::lamports_to_sol(sol_received);

                log(
                    LogTag::Trader,
                    status_text,
                    &format!(
                        "Closed position for {} ({}) - TX: {}, SOL Received: {:.6}, Net P&L: {}{:.6} SOL ({:.2}%), Drawdown: {:.2}%\x1b[0m",
                        position.symbol,
                        position.mint,
                        transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        actual_sol_received,
                        status_color,
                        net_pnl_sol,
                        net_pnl_percent,
                        position.drawdown_percent
                    )
                );

                // Attempt to close the Associated Token Account (ATA) if enabled
                if CLOSE_ATA_AFTER_SELL {
                    log(
                        LogTag::Trader,
                        "ATA",
                        &format!(
                            "Attempting to close ATA for {} after successful sell",
                            position.symbol
                        )
                    );

                    match crate::wallet::close_token_account(&position.mint, &wallet_address).await {
                        Ok(close_tx) => {
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!(
                                    "Successfully closed ATA for {} - Rent reclaimed. TX: {}",
                                    position.symbol,
                                    close_tx
                                )
                            );
                        }
                        Err(e) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                &format!(
                                    "Failed to close ATA for {} (this is not critical): {}",
                                    position.symbol,
                                    e
                                )
                            );
                            // Don't fail the position close if ATA close fails
                        }
                    }
                } else {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!(
                            "ATA closing disabled for {} (CLOSE_ATA_AFTER_SELL = false)",
                            position.symbol
                        )
                    );
                }

                return true; // Successfully closed
            }
            Err(e) => {
                // Check if this is an insufficient balance error
                let error_msg = format!("{}", e);
                if error_msg.contains("Insufficient") && error_msg.contains("balance") {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!(
                            "Insufficient balance error for {} - checking if transaction already completed",
                            position.symbol
                        )
                    );

                    // Check if the position was already closed via a recent transaction
                    if check_recent_transactions_for_position(position).await {
                        return true; // Position was successfully closed
                    }
                }

                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "Failed to execute sell swap for {} ({}): {}",
                        position.symbol,
                        position.mint,
                        e
                    )
                );
                return false; // Failed to close
            }
        }
    } else {
        log(
            LogTag::Trader,
            "ERROR",
            &format!("Cannot close position for {} - no token amount recorded", position.symbol)
        );
        return false;
    }
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

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    loop {
        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        let mut tokens: Vec<_> = {
            if let Ok(tokens_guard) = LIST_TOKENS.read() {
                // Log total tokens available
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Total tokens in LIST_TOKENS: {}", tokens_guard.len())
                        .dimmed()
                        .to_string()
                );

                // Include all tokens - we want to trade on existing tokens with updated info
                // The discovery system ensures tokens are updated with fresh data before trading
                let all_tokens: Vec<_> = tokens_guard.iter().cloned().collect();

                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Using all {} tokens for trading (startup filter removed)",
                        all_tokens.len()
                    )
                        .dimmed()
                        .to_string()
                );

                // Count tokens with liquidity data
                let with_liquidity = all_tokens
                    .iter()
                    .filter(|token| {
                        token.liquidity
                            .as_ref()
                            .and_then(|l| l.usd)
                            .unwrap_or(0.0) > 0.0
                    })
                    .count();

                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("Tokens with non-zero liquidity: {}", with_liquidity)
                        .dimmed()
                        .to_string()
                );

                all_tokens
            } else {
                log(LogTag::Trader, "ERROR", "Failed to acquire read lock on LIST_TOKENS");
                Vec::new()
            }
        };

        // Sort tokens by liquidity in descending order (highest liquidity first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Safety check - if processing is taking too long, log it
        if cycle_start.elapsed() > Duration::from_secs(5) {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token sorting took too long: {:?}", cycle_start.elapsed())
            );
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
                .dimmed()
                .to_string()
        );

        // Count tokens with zero liquidity before filtering
        let zero_liquidity_count = tokens
            .iter()
            .filter(|token| {
                let liquidity_usd = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                liquidity_usd == 0.0
            })
            .count();

        if zero_liquidity_count > 0 {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Found {} tokens with zero liquidity USD", zero_liquidity_count)
                    .dimmed()
                    .to_string()
            );
        }

        // Filter out zero-liquidity tokens first
        tokens.retain(|token| {
            let liquidity_usd = token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_usd > 0.0
        });

        log(
            LogTag::Trader,
            "INFO",
            &format!("Processing {} tokens with non-zero liquidity", tokens.len())
                .dimmed()
                .to_string()
        );

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(LogTag::Trader, "INFO", "No tokens to process, skipping token checking cycle");

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if
                cycle_duration >= Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS)
            {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        // Use a semaphore to limit the number of concurrent token checks
        // This balances between parallelism and not overwhelming external APIs
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        log(
            LogTag::Trader,
            "INFO",
            &format!("Starting to spawn {} token checking tasks", tokens.len()).dimmed().to_string()
        );

        // Process all tokens in parallel with concurrent tasks
        let mut handles = Vec::new();

        // Get the total token count before starting the loop
        let total_tokens = tokens.len();

        // Note: tokens are still sorted by liquidity from highest to lowest
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown before spawning tasks
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                return;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match
                tokio::time::timeout(
                    Duration::from_secs(120),
                    semaphore.clone().acquire_owned()
                ).await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e)
                    );
                    continue;
                }
                Err(_) => {
                    log(LogTag::Trader, "WARN", "Semaphore acquire timed out after 10 seconds");
                    continue;
                }
            };

            // Clone necessary variables for the task
            let token = token.clone();
            let index = index; // Capture the index
            let total = total_tokens; // Capture the total

            // Spawn a new task for this token with overall timeout
            let handle = tokio::spawn(async move {
                // Keep the permit alive for the duration of this task
                let _permit = permit; // This will be automatically dropped when the task completes

                // Clone the symbol for error logging before moving token into timeout
                let token_symbol = token.symbol.clone();

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(30), async {
                        if let Some(current_price) = token.price_dexscreener_sol {
                            if current_price <= 0.0 || !validate_token(&token) {
                                return None;
                            }

                            let liquidity_usd = token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);

                            // log(
                            //     LogTag::Trader,
                            //     "DEBUG",
                            //     &format!(
                            //         "Checking token {}/{}: {} ({}) - Price: {:.12} SOL, Liquidity: ${:.2}",
                            //         index + 1,
                            //         total,
                            //         token.symbol,
                            //         token.mint,
                            //         current_price,
                            //         liquidity_usd
                            //     )
                            //         .dimmed()
                            //         .to_string()
                            // );

                            // Update price history with proper error handling and timeout
                            let now = Utc::now();
                            match
                                tokio::time::timeout(Duration::from_millis(500), async {
                                    PRICE_HISTORY_24H.try_lock()
                                }).await
                            {
                                Ok(Ok(mut hist)) => {
                                    let entry = hist
                                        .entry(token.mint.clone())
                                        .or_insert_with(Vec::new);
                                    entry.push((now, current_price));

                                    // Retain only last 24h
                                    let cutoff = now - ChronoDuration::hours(PRICE_HISTORY_HOURS);
                                    entry.retain(|(ts, _)| *ts >= cutoff);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire price history lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Check for entry opportunity with timeout
                            let mut should_open_position = false;
                            let mut percent_change = 0.0;

                            // Use timeout for last prices mutex as well
                            match
                                tokio::time::timeout(Duration::from_millis(500), async {
                                    LAST_PRICES.try_lock()
                                }).await
                            {
                                Ok(Ok(mut last_prices)) => {
                                    if let Some(&prev_price) = last_prices.get(&token.mint) {
                                        if prev_price > 0.0 {
                                            let change = (current_price - prev_price) / prev_price;
                                            percent_change = change * 100.0;

                                            if percent_change <= -PRICE_DROP_THRESHOLD_PERCENT {
                                                should_open_position = true;
                                                log(
                                                    LogTag::Trader,
                                                    "OPPORTUNITY",
                                                    &format!(
                                                        "Entry opportunity detected for {} ({}): {:.2}% price drop, Liquidity: ${:.2}",
                                                        token.symbol,
                                                        token.mint,
                                                        percent_change,
                                                        liquidity_usd
                                                    )
                                                );
                                            }
                                        }
                                    }
                                    last_prices.insert(token.mint.clone(), current_price);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire last_prices lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Return the token, price, and percent change if it's an opportunity
                            if should_open_position {
                                return Some((token, current_price, percent_change));
                            }
                        }
                        None
                    }).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Task timed out
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Token check task for {} timed out after 10 seconds", token_symbol)
                        );
                        None
                    }
                }
            });

            handles.push(handle);
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Successfully spawned {} token checking tasks", handles.len())
                .dimmed()
                .to_string()
        );

        // Process the results of all tasks with overall timeout
        let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
            // This maintains the priority of processing high-liquidity tokens first
            log(
                LogTag::Trader,
                "INFO",
                &format!("Waiting for {} token checks to complete", handles.len())
                    .dimmed()
                    .to_string()
            );

            let mut opportunities = Vec::new();

            // Collect all opportunities in the order they complete
            let mut completed = 0;
            let total_handles = handles.len();

            for handle in handles {
                // Skip any tasks that failed or if shutdown signal received
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "new entries monitor shutting down during result collection..."
                    );
                    return opportunities; // Return what we have so far
                }

                // Add timeout for each handle to prevent getting stuck on a single task
                match tokio::time::timeout(Duration::from_secs(120), handle).await {
                    Ok(task_result) => {
                        match task_result {
                            Ok(Some((token, price, percent_change))) => {
                                opportunities.push((token, price, percent_change));
                            }
                            Ok(None) => {
                                // No opportunity found for this token, continue
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Token check task failed: {}", e)
                                );
                            }
                        }
                    }
                    Err(_) => {
                        // Task timed out after 5 seconds
                        log(LogTag::Trader, "WARN", "Token check task timed out after 5 seconds");
                    }
                }

                completed += 1;
                if completed % 10 == 0 || completed == total_handles {
                    log(
                        LogTag::Trader,
                        "INFO",
                        &format!("Completed {}/{} token checks", completed, total_handles)
                            .dimmed()
                            .to_string()
                    );
                }
            }

            opportunities
        }).await;

        let mut opportunities = match collection_result {
            Ok(opportunities) => opportunities,
            Err(_) => {
                log(LogTag::Trader, "ERROR", "Token check collection timed out after 60 seconds");
                Vec::new() // Return empty if timeout
            }
        };

        // Sort opportunities by liquidity again to ensure priority
        opportunities.sort_by(|(a, _, _), (b, _, _)| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} potential entry opportunities", opportunities.len())
        );

        // Log the total time taken for the token checking cycle
        log(
            LogTag::Trader,
            "INFO",
            &format!("Token checking cycle completed in {:?}", cycle_start.elapsed())
                .dimmed()
                .to_string()
        );

        // Process opportunities concurrently while respecting position limits
        if !opportunities.is_empty() {
            let current_open_count = get_open_positions_count();
            let available_slots = MAX_OPEN_POSITIONS.saturating_sub(current_open_count);

            if available_slots == 0 {
                log(
                    LogTag::Trader,
                    "LIMIT",
                    &format!(
                        "Maximum open positions already reached ({}/{}). Skipping all opportunities.",
                        current_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );
            } else {
                // Limit opportunities to available slots
                let opportunities_to_process = opportunities
                    .into_iter()
                    .take(available_slots)
                    .collect::<Vec<_>>();

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Processing {} opportunities concurrently (available slots: {}, current open: {})",
                        opportunities_to_process.len(),
                        available_slots,
                        current_open_count
                    )
                );

                // Use a semaphore to limit concurrent buy transactions
                use tokio::sync::Semaphore;
                let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent buys

                let mut handles = Vec::new();

                // Process all buy orders concurrently
                for (token, price, percent_change) in opportunities_to_process {
                    // Check for shutdown before spawning tasks
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during buy processing..."
                        );
                        break;
                    }

                    // Get permit from semaphore to limit concurrency with timeout
                    let permit = match
                        tokio::time::timeout(
                            Duration::from_secs(120),
                            semaphore.clone().acquire_owned()
                        ).await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore permit for buy: {}", e)
                            );
                            continue;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                "Semaphore acquire timed out for buy operation"
                            );
                            continue;
                        }
                    };

                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Keep permit alive for duration of task

                        let token_symbol = token.symbol.clone();

                        // Wrap the buy operation in a timeout
                        match
                            tokio::time::timeout(Duration::from_secs(120), async {
                                open_position(&token, price, percent_change).await
                            }).await
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Completed buy operation for {} in concurrent task", token_symbol)
                                );
                                true
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Buy operation for {} timed out after 20 seconds", token_symbol)
                                );
                                false
                            }
                        }
                    });

                    handles.push(handle);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Spawned {} concurrent buy tasks", handles.len()).dimmed().to_string()
                );

                // Collect results from all concurrent buy operations with overall timeout
                let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                    let mut completed = 0;
                    let mut successful = 0;
                    let total_handles = handles.len();

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                            log(
                                LogTag::Trader,
                                "INFO",
                                "new entries monitor shutting down during buy result collection..."
                            );
                            break;
                        }

                        // Add timeout for each handle to prevent getting stuck
                        match tokio::time::timeout(Duration::from_secs(120), handle).await {
                            Ok(task_result) => {
                                match task_result {
                                    Ok(success) => {
                                        if success {
                                            successful += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Buy task failed: {}", e)
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log(LogTag::Trader, "WARN", "Buy task timed out after 5 seconds");
                            }
                        }

                        completed += 1;
                        if completed % 2 == 0 || completed == total_handles {
                            log(
                                LogTag::Trader,
                                "INFO",
                                &format!("Completed {}/{} buy operations", completed, total_handles)
                                    .dimmed()
                                    .to_string()
                            );
                        }
                    }

                    (completed, successful)
                }).await;

                match collection_result {
                    Ok((completed, successful)) => {
                        let new_open_count = get_open_positions_count();
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!(
                                "Concurrent buy operations completed: {}/{} successful, new open positions: {}",
                                successful,
                                completed,
                                new_open_count
                            )
                        );
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            "Buy operations collection timed out after 30 seconds"
                        );
                    }
                }
            }
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token checking cycle took longer than interval: {:?}", cycle_duration)
            );
            Duration::from_millis(100)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    loop {
        let mut positions_to_close = Vec::new();

        // Find open positions and check if they should be closed
        {
            if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                for (index, position) in positions.iter_mut().enumerate() {
                    if position.position_type == "buy" && position.exit_price.is_none() {
                        // Find current price for this token
                        if let Ok(tokens_guard) = LIST_TOKENS.read() {
                            if
                                let Some(token) = tokens_guard
                                    .iter()
                                    .find(|t| t.mint == position.mint)
                            {
                                if let Some(current_price) = token.price_dexscreener_sol {
                                    if current_price > 0.0 {
                                        // Update position tracking (extremes and drawdown)
                                        update_position_tracking(position, current_price);

                                        let (
                                            _is_profitable,
                                            _net_pnl_sol,
                                            net_pnl_percent,
                                            _total_value,
                                        ) = is_position_profitable(
                                            position.entry_price,
                                            current_price,
                                            position.entry_size_sol
                                        );

                                        let now = Utc::now();

                                        // Update drawdown calculation - calculate from effective entry price
                                        let old_drawdown = position.drawdown_percent;
                                        let entry_price = position.effective_entry_price.unwrap_or(
                                            position.entry_price
                                        );
                                        if entry_price > 0.0 {
                                            position.drawdown_percent =
                                                ((entry_price - current_price) / entry_price) *
                                                100.0;

                                            log(
                                                LogTag::Trader,
                                                "DEBUG",
                                                &format!(
                                                    "Drawdown calc for {}: entry_price={:.8}, current_price={:.8}, drawdown={:.2}%->{:.2}%",
                                                    position.symbol,
                                                    entry_price,
                                                    current_price,
                                                    old_drawdown,
                                                    position.drawdown_percent
                                                )
                                                    .dimmed()
                                                    .to_string()
                                            );
                                        } // Calculate sell urgency using the advanced mathematical model
                                        let sell_urgency = should_sell(
                                            position,
                                            current_price,
                                            now
                                        );

                                        // Emergency exit conditions (keep original logic for safety)
                                        let emergency_exit = net_pnl_percent <= STOP_LOSS_PERCENT;

                                        // Urgency-based exit (sell if urgency > 70% or emergency)
                                        let should_exit = emergency_exit || sell_urgency > 0.7;

                                        if should_exit {
                                            log(
                                                LogTag::Trader,
                                                "SELL",
                                                &format!(
                                                    "Sell signal for {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Emergency: {}",
                                                    position.symbol,
                                                    position.mint,
                                                    sell_urgency,
                                                    net_pnl_percent,
                                                    emergency_exit
                                                )
                                            );

                                            positions_to_close.push((
                                                index,
                                                position.clone(), // Include the full position data
                                                token.clone(),
                                                current_price,
                                                now,
                                            ));
                                        } else {
                                            log(
                                                LogTag::Trader,
                                                "HOLD",
                                                &format!(
                                                    "Holding {} ({}) - Urgency: {:.2}, P&L: {:.2}%",
                                                    position.symbol,
                                                    position.mint,
                                                    sell_urgency,
                                                    net_pnl_percent
                                                )
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // Save updated positions with tracking data
                save_positions_to_file(&positions);
            }
        }

        // Close positions that need to be closed concurrently (outside of lock to avoid deadlock)
        if !positions_to_close.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} positions for concurrent closing", positions_to_close.len())
            );

            // Use a semaphore to limit concurrent sell transactions to avoid overwhelming the network
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (index, position, token, exit_price, exit_time) in positions_to_close {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing..."
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(e)) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to acquire semaphore permit for sell: {}", e)
                        );
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            "Semaphore acquire timed out for sell operation"
                        );
                        continue;
                    }
                };

                // We already have the position from the analysis phase, no need to look it up
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(Duration::from_secs(120), async {
                            close_position(&mut position, &token, exit_price, exit_time).await
                        }).await
                    {
                        Ok(success) => {
                            if success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Successfully closed position for {} in concurrent task", token_symbol)
                                );
                                Some((index, position))
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Failed to close position for {} in concurrent task", token_symbol)
                                );
                                None
                            }
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out after 15 seconds", token_symbol)
                            );
                            None
                        }
                    }
                });

                handles.push(handle);
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Spawned {} concurrent sell tasks", handles.len()).dimmed().to_string()
            );

            // Collect results from all concurrent sell operations with overall timeout
            // Increased timeout to 60 seconds to accommodate multiple 15-second sell operations
            let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                let mut completed_positions = Vec::new();
                let mut completed = 0;
                let total_handles = handles.len();

                for handle in handles {
                    // Skip if shutdown signal received
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "open positions monitor shutting down during sell result collection..."
                        );
                        break;
                    }

                    // Add timeout for each handle to prevent getting stuck
                    // Increased timeout to 15 seconds to allow for transaction verification and ATA closing
                    match tokio::time::timeout(Duration::from_secs(120), handle).await {
                        Ok(task_result) => {
                            match task_result {
                                Ok(Some((index, updated_position))) => {
                                    completed_positions.push((index, updated_position));
                                }
                                Ok(None) => {
                                    // Position failed to close, continue
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Sell task failed: {}", e)
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            log(LogTag::Trader, "WARN", "Sell task timed out after 60 seconds");
                        }
                    }

                    completed += 1;
                    if completed % 2 == 0 || completed == total_handles {
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!("Completed {}/{} sell operations", completed, total_handles)
                                .dimmed()
                                .to_string()
                        );
                    }
                }

                completed_positions
            }).await;

            let completed_positions = match collection_result {
                Ok(positions) => positions,
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        "Sell operations collection timed out after 60 seconds"
                    );
                    Vec::new()
                }
            };

            // Update all successfully closed positions in the saved positions
            if !completed_positions.is_empty() {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    for (index, updated_position) in &completed_positions {
                        if let Some(saved_position) = positions.get_mut(*index) {
                            *saved_position = updated_position.clone();
                        }
                    }
                    save_positions_to_file(&positions);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Updated {} positions after concurrent sell operations",
                        completed_positions.len()
                    )
                );
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(OPEN_POSITIONS_CHECK_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "open positions monitor shutting down...");
            break;
        }
    }
}

/// Main trader function that spawns both monitoring tasks
pub async fn trader(shutdown: Arc<Notify>) {
    log(LogTag::Trader, "INFO", "Starting trader with background tasks...");

    let shutdown_clone = shutdown.clone();
    let entries_task = tokio::spawn(async move {
        monitor_new_entries(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let positions_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let display_task = tokio::spawn(async move {
        monitor_positions_display(shutdown_clone).await;
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    log(LogTag::Trader, "INFO", "Trader shutting down...");

    // Give tasks a chance to shutdown gracefully
    let graceful_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = tokio::try_join!(entries_task, positions_task, display_task);
    });

    match graceful_timeout.await {
        Ok(_) => {
            log(LogTag::Trader, "INFO", "Trader tasks finished gracefully");
        }
        Err(_) => {
            log(LogTag::Trader, "WARN", "Trader tasks did not finish gracefully, aborting");
            // Force abort if graceful shutdown fails
            // entries_task.abort(); // These might already be finished
            // positions_task.abort();
            // display_task.abort();
        }
    }
}
