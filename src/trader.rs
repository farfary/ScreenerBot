/// Trading configuration constants
pub const PRICE_DROP_THRESHOLD_PERCENT: f64 = 5.0;
pub const PROFIT_THRESHOLD_PERCENT: f64 = 5.0;
pub const DEFAULT_FEE: f64 = 0.00005;
pub const TRADE_SIZE_SOL: f64 = 0.001;
pub const STOP_LOSS_PERCENT: f64 = -20.0;
pub const PRICE_HISTORY_HOURS: i64 = 24;
pub const NEW_ENTRIES_CHECK_INTERVAL_SECS: u64 = 2;
pub const OPEN_POSITIONS_CHECK_INTERVAL_SECS: u64 = 5;
pub const MAX_OPEN_POSITIONS: usize = 10;

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::utils::*;

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use serde::{ Serialize, Deserialize };

#[derive(Serialize, Deserialize, Clone)]
pub struct Position {
    mint: String,
    symbol: String,
    name: String,
    entry_price: f64,
    entry_time: DateTime<Utc>,
    exit_price: Option<f64>,
    exit_time: Option<DateTime<Utc>>,
    pnl_sol: Option<f64>,
    pnl_percent: Option<f64>,
    position_type: String, // "buy" or "sell"
    entry_size_sol: f64,
    total_size_sol: f64,
    drawdown_percent: f64,
    price_highest: f64,
    price_lowest: f64,
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

/// Opens a new buy position for a token
fn open_position(token: &Token, price: f64, percent_change: f64) {
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
    };

    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
        positions.push(position);
        save_positions_to_file(&positions);
    }
}

/// Closes an existing position
fn close_position(position: &mut Position, exit_price: f64, exit_time: DateTime<Utc>) {
    let (is_profitable, net_pnl_sol, net_pnl_percent, total_value) = is_position_profitable(
        position.entry_price,
        exit_price,
        position.entry_size_sol
    );

    position.exit_price = Some(exit_price);
    position.exit_time = Some(exit_time);
    position.pnl_sol = Some(net_pnl_sol);
    position.pnl_percent = Some(net_pnl_percent);
    position.total_size_sol = total_value;

    let status_color = if is_profitable { "\x1b[32m" } else { "\x1b[31m" };
    let status_text = if is_profitable { "PROFIT" } else { "LOSS" };
    let remaining_open_count = get_open_positions_count() - 1; // -1 because we're about to close this one

    log(
        LogTag::Trader,
        status_text,
        &format!(
            "Closed position for {} ({}) at {:.6} SOL - Entry: {:.6} SOL, Exit Value: {:.6} SOL, Net P&L: {}{:.6} SOL ({:.2}%), Drawdown: {:.2}% [{}/{}]\x1b[0m",
            position.symbol,
            position.mint,
            exit_price,
            position.entry_size_sol,
            total_value,
            status_color,
            net_pnl_sol,
            net_pnl_percent,
            position.drawdown_percent,
            remaining_open_count,
            MAX_OPEN_POSITIONS
        )
    );
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

/// Background task to monitor new tokens for entry opportunities
async fn monitor_new_entries(shutdown: Arc<Notify>) {
    loop {
        let tokens: Vec<_> = {
            if let Ok(tokens_guard) = LIST_TOKENS.read() {
                tokens_guard.iter().cloned().collect()
            } else {
                Vec::new()
            }
        };

        let mut handles = Vec::with_capacity(tokens.len());

        for token in tokens {
            handles.push(
                tokio::spawn(async move {
                    if let Some(current_price) = token.price_dexscreener_sol {
                        if current_price <= 0.0 || !validate_token(&token) {
                            return;
                        }

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
                        let mut last_prices = LAST_PRICES.lock().unwrap();
                        if let Some(&prev_price) = last_prices.get(&token.mint) {
                            if prev_price > 0.0 {
                                let change = (current_price - prev_price) / prev_price;
                                let percent_change = change * 100.0;

                                if percent_change <= -PRICE_DROP_THRESHOLD_PERCENT {
                                    open_position(&token, current_price, percent_change);
                                }
                            }
                        }
                        last_prices.insert(token.mint.clone(), current_price);
                    }
                })
            );
        }

        // Wait for all token processing to complete
        for handle in handles {
            let _ = handle.await;
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
async fn monitor_open_positions(shutdown: Arc<Notify>) {
    loop {
        let mut positions_to_update = Vec::new();

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

                                        // Close position if profitable enough or stop loss
                                        if
                                            net_pnl_percent >= PROFIT_THRESHOLD_PERCENT ||
                                            net_pnl_percent <= STOP_LOSS_PERCENT
                                        {
                                            positions_to_update.push((
                                                index,
                                                current_price,
                                                Utc::now(),
                                            ));
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

        // Update positions that need to be closed
        if !positions_to_update.is_empty() {
            if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                for (index, exit_price, exit_time) in positions_to_update {
                    if let Some(position) = positions.get_mut(index) {
                        close_position(position, exit_price, exit_time);
                    }
                }
                save_positions_to_file(&positions);
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
    log(LogTag::Trader, "INFO", "Starting trader with two background tasks...");

    let shutdown_clone = shutdown.clone();
    let entries_task = tokio::spawn(async move {
        monitor_new_entries(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let positions_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    // Cancel both tasks
    entries_task.abort();
    positions_task.abort();

    log(LogTag::Trader, "INFO", "Trader shutting down...");
}
