/// Trading configuration constants
pub const PRICE_DROP_THRESHOLD_PERCENT: f64 = 5.0;
pub const PROFIT_THRESHOLD_PERCENT: f64 = 5.0;
pub const DEFAULT_FEE: f64 = 0.00005;
pub const TRADE_SIZE_SOL: f64 = 0.0005;
pub const STOP_LOSS_PERCENT: f64 = -30.0;
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

                                        // Close position if profitable enough or stop loss
                                        if
                                            net_pnl_percent >= PROFIT_THRESHOLD_PERCENT ||
                                            net_pnl_percent <= STOP_LOSS_PERCENT
                                        {
                                            positions_to_close.push((
                                                index,
                                                token.clone(),
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

    log(LogTag::Trader, "INFO", "Trader shutting down...");

    // Give tasks a chance to shutdown gracefully
    let graceful_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = tokio::try_join!(entries_task, positions_task);
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
        }
    }
}
