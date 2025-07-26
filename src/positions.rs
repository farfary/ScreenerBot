use crate::trader::*;
use crate::logger::{ log, LogTag };
use crate::global::*;
use crate::tokens::Token;
use crate::utils::*;
use crate::wallet::{ buy_token, sell_token };

use once_cell::sync::Lazy;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, DateTime };
use serde::{ Serialize, Deserialize };
use colored::Colorize;

/// Static global: saved positions
pub static SAVED_POSITIONS: Lazy<StdArc<StdMutex<Vec<Position>>>> = Lazy::new(|| {
    let positions = load_positions_from_file();
    StdArc::new(StdMutex::new(positions))
});

/// Unified profit/loss calculation for both open and closed positions
/// Uses effective prices and actual token amounts when available
/// For closed positions with sol_received, uses actual SOL invested vs SOL received
/// NOTE: sol_received should contain ONLY the SOL from token sale, excluding ATA rent reclaim
pub fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    // Safety check: validate position has valid entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        // Invalid entry price - return neutral P&L to avoid triggering emergency exits
        return (0.0, 0.0);
    }

    // For open positions, validate current price if provided
    if let Some(current) = current_price {
        if current <= 0.0 || !current.is_finite() {
            // Invalid current price - return neutral P&L to avoid false emergency signals
            return (0.0, 0.0);
        }
    }

    // For closed positions, prioritize sol_received for most accurate P&L
    if let (Some(_), Some(sol_received)) = (position.exit_price, position.sol_received) {
        // Use actual SOL invested vs SOL received for closed positions
        let sol_invested = position.entry_size_sol;

        // Account for trading fees (buy + sell fees)
        // NOTE: sol_received should already be the net amount from token sale only
        // ATA rent reclaim (~0.002 SOL) is separate from trading P&L
        let total_fees = 2.0 * TRANSACTION_FEE_SOL;
        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let net_pnl_percent = (net_pnl_sol / sol_invested) * 100.0;

        return (net_pnl_sol, net_pnl_percent);
    }

    // Fallback for closed positions without sol_received (backward compatibility)
    if let Some(exit_price) = position.exit_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let effective_exit = position.effective_exit_price.unwrap_or(exit_price);

        // For closed positions: actual transaction-based calculation
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (synchronous)
            let token_decimals = crate::tokens::get_token_decimals_sync(&position.mint);

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let entry_cost = position.entry_size_sol;
            let exit_value = ui_token_amount * effective_exit;

            // Account for buy + sell fees
            let total_fees = 2.0 * TRANSACTION_FEE_SOL;
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let total_fees = 2.0 * TRANSACTION_FEE_SOL;
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // For open positions, use current price
    if let Some(current) = current_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);

        // For open positions: current value vs entry cost
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (synchronous)
            let token_decimals = crate::tokens::get_token_decimals_sync(&position.mint);

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let current_value = ui_token_amount * current;
            let entry_cost = position.entry_size_sol;

            // Account for buy fee (already paid) + estimated sell fee
            let total_fees = 2.0 * TRANSACTION_FEE_SOL;
            let net_pnl_sol = current_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let total_fees = 2.0 * TRANSACTION_FEE_SOL;
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // No price available
    (0.0, 0.0)
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Position {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub exit_price: Option<f64>,
    pub exit_time: Option<DateTime<Utc>>,
    pub position_type: String, // "buy" or "sell"
    pub entry_size_sol: f64,
    pub total_size_sol: f64,
    pub price_highest: f64,
    pub price_lowest: f64,
    // Real swap tracking
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>, // Amount of tokens bought/sold
    pub effective_entry_price: Option<f64>, // Actual price from on-chain transaction
    pub effective_exit_price: Option<f64>, // Actual exit price from on-chain transaction
    pub sol_received: Option<f64>, // Actual SOL received after sell (lamports converted to SOL)
}

/// Checks recent transactions to see if position was already closed
/// Enhanced version with strict validation to prevent phantom sells
pub async fn check_recent_transactions_for_position(position: &mut Position) -> bool {
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
            // Fallback to entry price since LIST_TOKENS was moved to tokens module
            // TODO: Implement proper async price lookup from tokens database
            position.exit_price = Some(position.entry_price);
            position.effective_exit_price = Some(position.entry_price);
        }

        // Calculate P&L using unified function
        if let Some(exit_price) = position.exit_price {
            let (net_pnl_sol, net_pnl_percent) = calculate_position_pnl(position, None);

            log(
                LogTag::Trader,
                if net_pnl_sol > 0.0 {
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

/// Updates position with current price to track extremes
pub fn update_position_tracking(position: &mut Position, current_price: f64) {
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
        position.price_lowest = entry_price;
    }

    let old_high = position.price_highest;
    let old_low = position.price_lowest;

    // Update running extremes
    if current_price > position.price_highest {
        position.price_highest = current_price;
    }
    if current_price < position.price_lowest {
        position.price_lowest = current_price;
    }

    // Log the transition
    log(
        LogTag::Trader,
        "DEBUG",
        &format!(
            "Track {}: entry={:.6}, current={:.6}, high={:.6}->{:.6}, low={:.6}->{:.6}",
            position.symbol,
            entry_price,
            current_price,
            old_high,
            position.price_highest,
            old_low,
            position.price_lowest
        )
            .dimmed()
            .to_string()
    );
}

/// Opens a new buy position for a token with real swap execution
pub async fn open_position(token: &Token, price: f64, percent_change: f64) {
    // CRITICAL SAFETY CHECK: Validate price before any trading operations
    if price <= 0.0 || !price.is_finite() {
        log(
            LogTag::Trader,
            "ERROR",
            &format!(
                "REFUSING TO TRADE: Invalid price for {} ({}). Price = {:.10}",
                token.symbol,
                token.mint,
                price
            )
        );
        return;
    }

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
                position_type: "buy".to_string(),
                entry_size_sol: TRADE_SIZE_SOL,
                total_size_sol: TRADE_SIZE_SOL,
                price_highest: effective_entry_price, // Use effective price for tracking
                price_lowest: effective_entry_price, // Use effective price for tracking
                entry_transaction_signature: swap_result.transaction_signature,
                exit_transaction_signature: None,
                token_amount: Some(token_amount),
                effective_entry_price: Some(effective_entry_price), // Actual transaction price
                effective_exit_price: None,
                sol_received: None, // Will be set when position is closed
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
pub async fn close_position(
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

                // Use ATA-cleaned SOL amount for accurate P&L calculation
                let clean_sol_received = if swap_result.ata_close_detected {
                    // Use the ATA-cleaned amount (trading proceeds only)
                    let ata_cleaned_lamports =
                        swap_result.sol_from_trade_only.unwrap_or(sol_received);
                    let ata_rent_lamports = swap_result.ata_rent_reclaimed.unwrap_or(0);

                    log(
                        LogTag::Trader,
                        "ATA_SEPARATION",
                        &format!(
                            "ATA detected for {} - Total: {:.6} SOL, Trade only: {:.6} SOL, ATA rent: {:.6} SOL",
                            position.symbol,
                            crate::wallet::lamports_to_sol(sol_received),
                            crate::wallet::lamports_to_sol(ata_cleaned_lamports),
                            crate::wallet::lamports_to_sol(ata_rent_lamports)
                        )
                    );

                    ata_cleaned_lamports
                } else {
                    sol_received
                };

                // Calculate actual P&L using unified function
                position.exit_price = Some(exit_price);
                position.effective_exit_price = Some(effective_exit_price);
                position.sol_received = Some(crate::wallet::lamports_to_sol(clean_sol_received)); // Store ATA-cleaned SOL

                let (net_pnl_sol, net_pnl_percent) = calculate_position_pnl(position, None);
                let is_profitable = net_pnl_sol > 0.0;

                position.exit_price = Some(exit_price);
                position.exit_time = Some(exit_time);
                position.total_size_sol = crate::wallet::lamports_to_sol(sol_received);
                position.exit_transaction_signature = transaction_signature.clone();
                position.effective_exit_price = Some(effective_exit_price);

                let status_color = if is_profitable { "\x1b[32m" } else { "\x1b[31m" };
                let status_text = if is_profitable { "PROFIT" } else { "LOSS" };

                let actual_sol_received = crate::wallet::lamports_to_sol(clean_sol_received);
                let total_sol_received = crate::wallet::lamports_to_sol(sol_received);

                let log_message = if swap_result.ata_close_detected {
                    format!(
                        "Closed position for {} ({}) - TX: {}, Total SOL: {:.6} (Trade: {:.6}, ATA rent: {:.6}), Net Trading P&L: {}{:.6} SOL ({:.2}%)\x1b[0m",
                        position.symbol,
                        position.mint,
                        transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        total_sol_received,
                        actual_sol_received,
                        crate::wallet::lamports_to_sol(swap_result.ata_rent_reclaimed.unwrap_or(0)),
                        status_color,
                        net_pnl_sol,
                        net_pnl_percent
                    )
                } else {
                    format!(
                        "Closed position for {} ({}) - TX: {}, SOL From Sale: {:.6}, Net Trading P&L: {}{:.6} SOL ({:.2}%)\x1b[0m",
                        position.symbol,
                        position.mint,
                        transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                        actual_sol_received,
                        status_color,
                        net_pnl_sol,
                        net_pnl_percent
                    )
                };

                log(LogTag::Trader, status_text, &log_message);

                // Attempt to close the Associated Token Account (ATA) if enabled
                if AUTO_CLOSE_ATA_AFTER_SELL {
                    log(
                        LogTag::Trader,
                        "ATA",
                        &format!(
                            "Attempting to close ATA for {} after successful sell (will reclaim ~0.002 SOL rent separately from trading P&L)",
                            position.symbol
                        )
                    );

                    match crate::wallet::close_token_account(&position.mint, &wallet_address).await {
                        Ok(close_tx) => {
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!(
                                    "Successfully closed ATA for {} - Rent reclaimed: ~0.002 SOL (separate from trading P&L). TX: {}",
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
                            "ATA closing disabled for {} (AUTO_CLOSE_ATA_AFTER_SELL = false)",
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

/// Gets the current count of open positions
pub fn get_open_positions_count() -> usize {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
            .count()
    } else {
        0
    }
}
