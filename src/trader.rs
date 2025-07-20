/// Trading configuration constants
pub const PRICE_DROP_THRESHOLD_PERCENT: f64 = 5.0;
pub const PROFIT_THRESHOLD_PERCENT: f64 = 5.0;
pub const DEFAULT_FEE: f64 = 0.0000015;
pub const TRADE_SIZE_SOL: f64 = 0.0005;
pub const STOP_LOSS_PERCENT: f64 = -70.0;
pub const PRICE_HISTORY_HOURS: i64 = 24;
pub const NEW_ENTRIES_CHECK_INTERVAL_SECS: u64 = 2;
pub const OPEN_POSITIONS_CHECK_INTERVAL_SECS: u64 = 5;
pub const MAX_OPEN_POSITIONS: usize = 1;

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::utils::*;
use crate::wallet::{ buy_token, sell_token };

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use serde::{ Serialize, Deserialize };
use tabled::{ Tabled, Table, settings::{ Style, Alignment, object::Rows, Modify } };

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
            // Calculate current P&L if position is still open
            let (_, _, net_pnl_percent, _) = is_position_profitable(
                position.entry_price,
                price,
                position.entry_size_sol
            );
            let current_pnl = (net_pnl_percent / 100.0) * position.entry_size_sol;
            if current_pnl >= 0.0 {
                format!("+{:.6}", current_pnl)
            } else {
                format!("{:.6}", current_pnl)
            }
        } else {
            "N/A".to_string()
        };

        let pnl_percent_str = if let Some(pnl_pct) = position.pnl_percent {
            if pnl_pct >= 0.0 { format!("+{:.2}%", pnl_pct) } else { format!("{:.2}%", pnl_pct) }
        } else if let Some(price) = current_price {
            // Calculate current P&L percentage if position is still open
            let (_, _, net_pnl_percent, _) = is_position_profitable(
                position.entry_price,
                price,
                position.entry_size_sol
            );
            if net_pnl_percent >= 0.0 {
                format!("+{:.2}%", net_pnl_percent)
            } else {
                format!("{:.2}%", net_pnl_percent)
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
            entry_price: format!("{:.8}", position.entry_price),
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

/// Calculates sell urgency based on time held and profit
pub fn calculate_sell_urgency(pos: &Position, now: DateTime<Utc>) -> f64 {
    // === PARAMETERS ===
    let min_time_secs = 10.0;
    let max_time_secs = 600.0; // 10 minutes
    let min_profit = 0.0;
    let max_profit = 500.0;

    // === TIME CALC ===
    let time_secs = (pos.exit_time.unwrap_or(now) - pos.entry_time)
        .num_seconds()
        .max(min_time_secs as i64) as f64;

    // === PROFIT CALC ===
    let profit = pos.pnl_percent.unwrap_or_else(|| {
        if pos.entry_price > 0.0 {
            ((pos.price_highest - pos.entry_price) / pos.entry_price) * 100.0
        } else {
            0.0
        }
    });

    // === NORMALIZE ===
    let norm_time = ((time_secs - min_time_secs) / (max_time_secs - min_time_secs)).clamp(0.0, 1.0);
    let norm_profit = ((profit - min_profit) / (max_profit - min_profit)).clamp(0.0, 1.0);

    // === URGENCY LOGIC ===
    let urgency = norm_profit * (1.0 - norm_time) + norm_time * (1.0 - norm_profit);
    urgency.clamp(0.0, 1.0)
}

/// Checks recent transactions to see if position was already closed
async fn check_recent_transactions_for_position(position: &mut Position) -> bool {
    // Get wallet address
    let wallet_address = match crate::wallet::get_wallet_address() {
        Ok(addr) => addr,
        Err(_) => {
            log(LogTag::Trader, "ERROR", "Failed to get wallet address for transaction check");
            return false;
        }
    };

    // Check if we can find a recent sell transaction for this token with the expected amount
    // This would involve checking on-chain transaction history
    // For now, we'll implement a simple approach that checks if token balance is 0
    // and position was trying to sell, indicating transaction likely succeeded

    match crate::wallet::get_token_balance(&wallet_address, &position.mint).await {
        Ok(current_balance) => {
            // If we have 0 tokens and position shows we should have tokens,
            // it likely means the sell transaction succeeded but we missed the confirmation
            if current_balance == 0 && position.token_amount.unwrap_or(0) > 0 {
                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Detected completed sell transaction for {} - Balance is 0, updating position as closed",
                        position.symbol
                    )
                );

                // Mark position as closed with estimated exit price
                let now = Utc::now();
                position.exit_time = Some(now);

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

                // Calculate P&L
                if let Some(exit_price) = position.exit_price {
                    let (is_profitable, net_pnl_sol, net_pnl_percent, _) = is_position_profitable(
                        position.entry_price,
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
                            "Auto-closed position for {} - P&L: {:.6} SOL ({:.2}%)",
                            position.symbol,
                            net_pnl_sol,
                            net_pnl_percent
                        )
                    );
                }

                return true;
            }
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to check token balance for {}: {}", position.symbol, e)
            );
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

    let now = Utc::now();
    println!("\n📊 Positions Dashboard - {} UTC", now.format("%H:%M:%S"));
    println!(
        "📈 Open: {} | 📋 Closed: {} | 💰 Invested: {:.6} SOL | P&L: {:+.6} SOL",
        open_count,
        closed_count,
        total_invested,
        total_pnl
    );

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
fn is_position_profitable(
    entry_price: f64,
    current_price: f64,
    trade_size_sol: f64
) -> (bool, f64, f64, f64) {
    let gross_pnl_sol = (current_price - entry_price) * (trade_size_sol / entry_price);
    let net_pnl_sol = gross_pnl_sol - 2.0 * DEFAULT_FEE; // Entry and exit fees
    let net_pnl_percent = (net_pnl_sol / trade_size_sol) * 100.0;
    let total_value = trade_size_sol + net_pnl_sol;

    (net_pnl_sol > 0.0, net_pnl_sol, net_pnl_percent, total_value)
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
                    "Real swap executed for {}: TX: {}, Tokens: {}, Effective Price: {:.12} SOL",
                    token.symbol,
                    swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                    token_amount,
                    effective_entry_price
                )
            );

            let position = Position {
                mint: token.mint.clone(),
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                entry_price: price,
                entry_time: Utc::now(),
                exit_price: None,
                exit_time: None,
                pnl_sol: None,
                pnl_percent: None,
                position_type: "buy".to_string(),
                entry_size_sol: TRADE_SIZE_SOL,
                total_size_sol: TRADE_SIZE_SOL,
                drawdown_percent: 0.0,
                price_highest: price,
                price_lowest: price,
                entry_transaction_signature: swap_result.transaction_signature,
                exit_transaction_signature: None,
                token_amount: Some(token_amount),
                effective_entry_price: Some(effective_entry_price),
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
    if let Some(token_amount) = position.token_amount {
        // Check if we actually have tokens to sell
        if token_amount == 0 {
            log(
                LogTag::Trader,
                "WARNING",
                &format!(
                    "Cannot close position for {} ({}) - No tokens to sell (amount: 0)",
                    position.symbol,
                    position.mint
                )
            );

            // Mark position as closed with zero values
            position.exit_time = Some(exit_time);
            position.exit_price = Some(exit_price);
            position.effective_exit_price = Some(0.0);
            position.pnl_sol = Some(-position.entry_size_sol); // Loss = entry amount
            position.exit_transaction_signature = Some("NO_TOKENS_TO_SELL".to_string());
            return true;
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

                // Calculate actual P&L based on real transaction amounts
                let actual_sol_received = crate::wallet::lamports_to_sol(sol_received);
                let net_pnl_sol = actual_sol_received - position.entry_size_sol;
                let net_pnl_percent = (net_pnl_sol / position.entry_size_sol) * 100.0;
                let is_profitable = net_pnl_sol > 0.0;

                position.exit_price = Some(exit_price);
                position.exit_time = Some(exit_time);
                position.pnl_sol = Some(net_pnl_sol);
                position.pnl_percent = Some(net_pnl_percent);
                position.total_size_sol = actual_sol_received;
                position.exit_transaction_signature = transaction_signature.clone();
                position.effective_exit_price = Some(effective_exit_price);

                let status_color = if is_profitable { "\x1b[32m" } else { "\x1b[31m" };
                let status_text = if is_profitable { "PROFIT" } else { "LOSS" };
                let remaining_open_count = get_open_positions_count() - 1;

                log(
                    LogTag::Trader,
                    status_text,
                    &format!(
                        "Closed position for {} ({}) - TX: {}, SOL Received: {:.6}, Net P&L: {}{:.6} SOL ({:.2}%), Drawdown: {:.2}% [{}/{}]\x1b[0m",
                        position.symbol,
                        position.mint,
                        transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        actual_sol_received,
                        status_color,
                        net_pnl_sol,
                        net_pnl_percent,
                        position.drawdown_percent,
                        remaining_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );

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

/// Updates position with current price to track extremes and drawdown
fn update_position_tracking(position: &mut Position, current_price: f64) {
    // Update price extremes
    if current_price > position.price_highest {
        position.price_highest = current_price;
    }
    if current_price < position.price_lowest {
        position.price_lowest = current_price;
    }

    // Calculate drawdown from highest price
    let drawdown = ((position.price_highest - current_price) / position.price_highest) * 100.0;
    if drawdown > position.drawdown_percent {
        position.drawdown_percent = drawdown;
    }
}

/// Background task to display positions table every 10 seconds
pub async fn monitor_positions_display(shutdown: Arc<Notify>) {
    loop {
        // Display the positions table
        display_positions_table().await;

        // Wait 10 seconds or until shutdown
        if check_shutdown_or_delay(&shutdown, Duration::from_secs(10)).await {
            log(LogTag::Trader, "INFO", "positions display monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    loop {
        let mut tokens: Vec<_> = {
            if let Ok(tokens_guard) = LIST_TOKENS.read() {
                tokens_guard.iter().cloned().collect()
            } else {
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

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
        );

        // Process tokens one by one instead of in parallel
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown between each token
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                return;
            }

            if let Some(current_price) = token.price_dexscreener_sol {
                if current_price <= 0.0 || !validate_token(&token) {
                    continue;
                }

                let liquidity_usd = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);

                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "Checking token {}/{}: {} ({}) - Price: {:.12} SOL, Liquidity: ${:.2}",
                        index + 1,
                        tokens.len(),
                        token.symbol,
                        token.mint,
                        current_price,
                        liquidity_usd
                    )
                );

                // Update price history
                let now = Utc::now();
                {
                    let mut hist = PRICE_HISTORY_24H.lock().unwrap();
                    let entry = hist.entry(token.mint.clone()).or_insert_with(Vec::new);
                    entry.push((now, current_price));

                    // Retain only last 24h
                    let cutoff = now - ChronoDuration::hours(PRICE_HISTORY_HOURS);
                    entry.retain(|(ts, _)| *ts >= cutoff);
                }

                // Check for entry opportunity
                let mut should_open_position = false;
                let mut percent_change = 0.0;

                {
                    let mut last_prices = LAST_PRICES.lock().unwrap();
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

                if should_open_position {
                    open_position(&token, current_price, percent_change).await;

                    // If we've reached max positions, we can break early to avoid unnecessary processing
                    if get_open_positions_count() >= MAX_OPEN_POSITIONS {
                        log(
                            LogTag::Trader,
                            "LIMIT",
                            &format!(
                                "Maximum open positions reached ({}/{}). Stopping token scanning.",
                                get_open_positions_count(),
                                MAX_OPEN_POSITIONS
                            )
                        );
                        break;
                    }
                }
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(NEW_ENTRIES_CHECK_INTERVAL_SECS)
            ).await
        {
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

                                        // Calculate sell urgency using the new function
                                        let sell_urgency = calculate_sell_urgency(position, now);

                                        // Emergency exit conditions (keep original logic for safety)
                                        let emergency_exit = net_pnl_percent <= STOP_LOSS_PERCENT;

                                        // Urgency-based exit (sell if urgency > 70% or emergency)
                                        let should_sell = emergency_exit || sell_urgency > 0.7;

                                        if should_sell {
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

        // Close positions that need to be closed (outside of lock to avoid deadlock)
        for (index, token, exit_price, exit_time) in positions_to_close {
            let mut position_clone = None;

            // Get a clone of the position to work with
            {
                if let Ok(positions) = SAVED_POSITIONS.lock() {
                    if let Some(position) = positions.get(index) {
                        position_clone = Some(position.clone());
                    }
                }
            }

            if let Some(mut position) = position_clone {
                if close_position(&mut position, &token, exit_price, exit_time).await {
                    // Update the position in the saved positions
                    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        if let Some(saved_position) = positions.get_mut(index) {
                            *saved_position = position;
                        }
                        save_positions_to_file(&positions);
                    }
                }
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
