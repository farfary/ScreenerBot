use crate::trader::*;
use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::tokens::Token;
use crate::utils::*;
use crate::rpc::lamports_to_sol;
use crate::swaps::{ buy_token, sell_token, wait_for_swap_verification, wait_for_priority_swap_verification };
use crate::rl_learning::{ get_trading_learner, record_completed_trade };
use crate::entry::get_rugcheck_score_for_token;

use once_cell::sync::Lazy;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, DateTime };
use serde::{ Serialize, Deserialize };
use colored::Colorize;
use std::collections::HashMap;

/// Static global: saved positions
pub static SAVED_POSITIONS: Lazy<StdArc<StdMutex<Vec<Position>>>> = Lazy::new(|| {
    let positions = load_positions_from_file();
    StdArc::new(StdMutex::new(positions))
});

/// Static global: frozen account cooldown tracking
/// Maps mint address to timestamp when sell failed due to frozen account
static FROZEN_ACCOUNT_COOLDOWNS: Lazy<StdArc<StdMutex<HashMap<String, DateTime<Utc>>>>> = Lazy::new(
    || { StdArc::new(StdMutex::new(HashMap::new())) }
);

/// Cooldown duration for frozen account errors (15 minutes)
const FROZEN_ACCOUNT_COOLDOWN_MINUTES: i64 = 15;

/// Checks if a mint is currently in cooldown due to frozen account error
fn is_mint_in_frozen_cooldown(mint: &str) -> bool {
    if let Ok(cooldowns) = FROZEN_ACCOUNT_COOLDOWNS.lock() {
        if let Some(cooldown_time) = cooldowns.get(mint) {
            let now = Utc::now();
            let minutes_since_cooldown = (now - *cooldown_time).num_minutes();
            if minutes_since_cooldown < FROZEN_ACCOUNT_COOLDOWN_MINUTES {
                return true;
            }
        }
    }
    false
}

/// Adds a mint to frozen account cooldown tracking
fn add_mint_to_frozen_cooldown(mint: &str) {
    if let Ok(mut cooldowns) = FROZEN_ACCOUNT_COOLDOWNS.lock() {
        cooldowns.insert(mint.to_string(), Utc::now());
        log(
            LogTag::Trader,
            "COOLDOWN",
            &format!(
                "Added {} to frozen account cooldown for {} minutes",
                mint,
                FROZEN_ACCOUNT_COOLDOWN_MINUTES
            )
        );
    }
}

/// Removes expired cooldowns and returns remaining time for a mint
fn get_remaining_cooldown_minutes(mint: &str) -> Option<i64> {
    if let Ok(mut cooldowns) = FROZEN_ACCOUNT_COOLDOWNS.lock() {
        if let Some(cooldown_time) = cooldowns.get(mint) {
            let now = Utc::now();
            let minutes_since_cooldown = (now - *cooldown_time).num_minutes();
            if minutes_since_cooldown >= FROZEN_ACCOUNT_COOLDOWN_MINUTES {
                // Cooldown expired, remove it
                cooldowns.remove(mint);
                None
            } else {
                Some(FROZEN_ACCOUNT_COOLDOWN_MINUTES - minutes_since_cooldown)
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Checks if an error is a frozen account error (error code 0x11)
fn is_frozen_account_error(error_msg: &str) -> bool {
    error_msg.contains("custom program error: 0x11") ||
        error_msg.contains("Account is frozen") ||
        error_msg.contains("Error: Account is frozen")
}

/// Calculate liquidity tier based on USD liquidity amount
/// Returns tier classification for position tracking and analysis
pub fn calculate_liquidity_tier(token: &crate::tokens::types::Token) -> Option<String> {
    let liquidity_usd = token.liquidity.as_ref().and_then(|l| l.usd)?;

    if liquidity_usd < 0.0 {
        return Some("INVALID".to_string());
    }

    // Liquidity tier classification based on USD value
    let tier = match liquidity_usd {
        x if x < 1_000.0 => "MICRO", // < $1K
        x if x < 10_000.0 => "SMALL", // $1K - $10K
        x if x < 50_000.0 => "MEDIUM", // $10K - $50K
        x if x < 250_000.0 => "LARGE", // $50K - $250K
        x if x < 1_000_000.0 => "XLARGE", // $250K - $1M
        _ => "MEGA", // > $1M
    };

    Some(tier.to_string())
}

/// Calculate total fees for a position including entry fees and exit fees only
pub fn calculate_position_total_fees(position: &Position) -> f64 {
    let entry_fees_sol = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
    let exit_fees_sol = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));

    entry_fees_sol + exit_fees_sol
}

/// Calculate detailed breakdown of position fees for analysis
pub fn calculate_position_fees_breakdown(position: &Position) -> (f64, f64, f64) {
    let entry_fee_sol = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
    let exit_fee_sol = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
    let total_fees = entry_fee_sol + exit_fee_sol;

    (entry_fee_sol, exit_fee_sol, total_fees)
}

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
    if let (Some(exit_price), Some(sol_received)) = (position.exit_price, position.sol_received) {
        // Use actual SOL invested vs SOL received for closed positions
        let sol_invested = position.entry_size_sol;

        // Use actual transaction fees plus profit buffer for P&L calculation
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer in P&L calculation

        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let safe_invested = if sol_invested < 0.00001 { 0.00001 } else { sol_invested };
        let net_pnl_percent = (net_pnl_sol / safe_invested) * 100.0;

        if is_debug_profit_enabled() {
            // Log detailed PnL calculation for debugging
            log(
                LogTag::Trader,
                "PNL_DETAILED",
                &format!(
                    "💰 DETAILED PNL CALCULATION for {}:\n  Entry size: {:.9} SOL\n  SOL received: {:.9} SOL\n  Buy fee: {:.9} SOL\n  Sell fee: {:.9} SOL\n  Profit buffer: {:.9} SOL\n  Total fees + buffer: {:.9} SOL\n  Net P&L: {:.9} SOL ({:.2}%)",
                    position.symbol,
                    sol_invested,
                    sol_received,
                    buy_fee,
                    sell_fee,
                    PROFIT_EXTRA_NEEDED_SOL,
                    total_fees,
                    net_pnl_sol,
                    net_pnl_percent
                )
            );
        }

        return (net_pnl_sol, net_pnl_percent);
    }

    // Fallback for closed positions without sol_received (backward compatibility)
    if let Some(exit_price) = position.exit_price {
        let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
        let effective_exit = position.effective_exit_price.unwrap_or(exit_price);

        // For closed positions: actual transaction-based calculation
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (synchronous)
            let token_decimals_opt = crate::tokens::get_token_decimals_sync(&position.mint);

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let entry_cost = position.entry_size_sol;
            let exit_value = ui_token_amount * effective_exit;

            // Account for actual buy + sell fees plus profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
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
            let token_decimals_opt = crate::tokens::get_token_decimals_sync(&position.mint);

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!(
                            "Cannot calculate P&L for {} - decimals not available, skipping calculation",
                            position.mint
                        )
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let current_value = ui_token_amount * current;
            let entry_cost = position.entry_size_sol;

            // Account for actual buy fee (already paid) + estimated sell fee + profit buffer
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
            let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
            let net_pnl_sol = current_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
        let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL; // Include profit buffer
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
    // Smart profit targeting
    pub profit_target_min: Option<f64>, // Minimum profit target percentage
    pub profit_target_max: Option<f64>, // Maximum profit target percentage
    pub liquidity_tier: Option<String>, // Liquidity tier for reference
    // Transaction verification status
    pub transaction_entry_verified: bool, // Whether entry transaction is fully verified
    pub transaction_exit_verified: bool, // Whether exit transaction is fully verified
    // Actual transaction fees (in lamports)
    pub entry_fee_lamports: Option<u64>, // Actual entry transaction fee
    pub exit_fee_lamports: Option<u64>, // Actual exit transaction fee
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

    // Update running extremes
    if current_price > position.price_highest {
        position.price_highest = current_price;
    }
    if current_price < position.price_lowest {
        position.price_lowest = current_price;
    }

    // Track position extremes without logging
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

    // DRY-RUN MODE CHECK: Skip actual trading if dry-run is enabled
    if crate::arguments::is_dry_run_enabled() {
        let colored_percent = format!("\x1b[31m{:.2}%\x1b[0m", percent_change);
        let current_open_count = get_open_positions_count();
        log(
            LogTag::Trader,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would open position for {} ({}) at {:.6} SOL ({}) - Size: {:.6} SOL [{}/{}]",
                token.symbol,
                token.mint,
                price,
                colored_percent,
                TRADE_SIZE_SOL,
                current_open_count + 1,
                MAX_OPEN_POSITIONS
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

    // Execute real buy transaction with critical operation protection
    let _guard = crate::trader::CriticalOperationGuard::new(&format!("BUY {}", token.symbol));

    // Get wallet address for balance tracking
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("❌ Failed to get wallet address for {}: {}", token.symbol, e)
            );
            return;
        }
    };

    // Execute the token purchase using instruction-based analysis
    match buy_token(token, TRADE_SIZE_SOL, Some(price)).await {
        Ok(swap_result) => {
            // CRITICAL FIX: Check if the transaction was actually successful on-chain
            if !swap_result.success {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!(
                        "❌ Transaction failed on-chain for {}: {}",
                        token.symbol,
                        swap_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                    )
                );
                return;
            }

            let transaction_signature = swap_result.transaction_signature
                .clone()
                .unwrap_or_default();

            log(
                LogTag::Trader,
                "VERIFICATION_WAIT",
                &format!(
                    "⏳ Waiting for priority transaction verification before creating position for {}: {}",
                    token.symbol,
                    &transaction_signature[..8]
                )
            );

            // Wait for priority transaction verification with 5 second timeout
            let verification_result = wait_for_priority_swap_verification(&transaction_signature).await;
            let is_verified = match verification_result {
                Ok(true) => {
                    log(
                        LogTag::Trader,
                        "VERIFIED",
                        &format!(
                            "✅ Transaction verified successfully for {}: {}",
                            token.symbol,
                            &transaction_signature[..8]
                        )
                    );
                    true
                }
                Ok(false) => {
                    log(
                        LogTag::Trader,
                        "PRIORITY_TIMEOUT",
                        &format!(
                            "⏰ Priority transaction verification timeout for {}: {} - creating position with delayed verification",
                            token.symbol,
                            &transaction_signature[..8]
                        )
                    );
                    false // Timeout - create position but mark as unverified
                }
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "❌ Transaction verification failed for {}: {} - error: {} - creating position anyway to prevent money loss",
                            token.symbol,
                            &transaction_signature[..8],
                            e
                        )
                    );
                    false // Verification error - create position but mark as unverified to prevent money loss
                }
            };

            let (profit_min, profit_max) = crate::entry::get_profit_target(token).await;

            // Fetch transaction details to get actual token amount and prices
            let (token_amount, effective_entry_price, entry_fee, actual_sol_spent) = match
                crate::transactions_manager::get_transaction(&transaction_signature).await
            {
                Ok(Some(tx)) => {
                    log(
                        LogTag::Trader,
                        "DEBUG",
                        &format!(
                            "📄 Transaction {} fetched successfully for {} analysis",
                            &transaction_signature[..16],
                            token.symbol
                        )
                    );

                    let mut token_amount = None;
                    let mut effective_price = None;
                    let mut fee_lamports = None;
                    let mut sol_spent = TRADE_SIZE_SOL;

                    // Try to get token amount from swap analysis first
                    if let Some(ref swap) = tx.swap_analysis {
                        log(
                            LogTag::Trader,
                            "DEBUG",
                            &format!(
                                "Swap analysis found - Output token: '{}', Expected: '{}', Match: {}",
                                swap.output_token,
                                token.mint,
                                swap.output_token == token.mint
                            )
                        );
                        if swap.output_token == token.mint {
                            // Handle token amount with proper decimal consideration
                            // For display purposes, we store the actual received amount (with decimals)
                            // The token amount should represent the actual token units received
                            let token_decimals = crate::tokens
                                ::get_token_decimals(&token.mint).await
                                .unwrap_or(9);
                            let token_amount_units = (swap.output_amount *
                                (10_f64).powi(token_decimals as i32)) as u64;

                            log(
                                LogTag::Trader,
                                "DEBUG",
                                &format!(
                                    "🧮 TOKEN CONVERSION DEBUG for {}: 
                                        - swap.output_amount={} 
                                        - decimals={} 
                                        - calculation={}*10^{}={}
                                        - as_u64={}
                                        - This value will be stored as position.token_amount",
                                    token.symbol,
                                    swap.output_amount,
                                    token_decimals,
                                    swap.output_amount,
                                    token_decimals,
                                    swap.output_amount * (10_f64).powi(token_decimals as i32),
                                    token_amount_units
                                )
                            );

                            token_amount = Some(token_amount_units);
                            effective_price = Some(swap.effective_price);
                            sol_spent = swap.input_amount;
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!(
                                    "✅ Position data extracted from swap analysis for {}: {} tokens ({} units with {} decimals), {} SOL",
                                    token.symbol,
                                    swap.output_amount,
                                    token_amount_units,
                                    token_decimals,
                                    swap.input_amount
                                )
                            );
                        }
                    } else {
                        log(
                            LogTag::Trader,
                            "WARNING",
                            &format!(
                                "⚠️ No swap analysis found in transaction {} for {}",
                                &transaction_signature[..16],
                                token.symbol
                            )
                        );
                    }

                    // Get fee information
                    if let Some(ref fee_breakdown) = tx.fee_breakdown {
                        fee_lamports = Some((fee_breakdown.total_fees * 1_000_000_000.0) as u64);
                    }

                    (token_amount, effective_price, fee_lamports, sol_spent)
                }
                Ok(None) => {
                    log(
                        LogTag::Trader,
                        "WARNING",
                        &format!(
                            "⚠️ Transaction {} not found in cache or blockchain for {} - using defaults",
                            &transaction_signature[..16],
                            token.symbol
                        )
                    );
                    (None, None, None, TRADE_SIZE_SOL)
                }
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "❌ Failed to fetch transaction {} for {}: {} - using defaults",
                            &transaction_signature[..16],
                            token.symbol,
                            e
                        )
                    );
                    (None, None, None, TRADE_SIZE_SOL)
                }
            };

            let new_position = Position {
                mint: token.mint.clone(),
                symbol: token.symbol.clone(),
                name: token.name.clone(),
                entry_price: price,
                entry_time: Utc::now(),
                exit_price: None,
                exit_time: None,
                position_type: "buy".to_string(),
                entry_size_sol: TRADE_SIZE_SOL,
                total_size_sol: actual_sol_spent,
                price_highest: price,
                price_lowest: price,
                entry_transaction_signature: Some(transaction_signature.clone()),
                exit_transaction_signature: None,
                token_amount,
                effective_entry_price,
                effective_exit_price: None,
                sol_received: None,
                profit_target_min: Some(profit_min),
                profit_target_max: Some(profit_max),
                liquidity_tier: calculate_liquidity_tier(token),
                transaction_entry_verified: is_verified, // Use actual verification result
                transaction_exit_verified: false,
                entry_fee_lamports: entry_fee,
                exit_fee_lamports: None,
            };

            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "✅ POSITION CREATED{}: {} | TX: {} | Signal Price: {:.12} SOL | Token Amount: {} | Profit Target: {:.1}%-{:.1}% | Verified: {}",
                    if is_verified { " (VERIFIED)" } else { " (PENDING_VERIFICATION)" },
                    token.symbol,
                    &transaction_signature[..8],
                    price,
                    match token_amount {
                        Some(amount) => format!("{:.6}", amount),
                        None => "NOT_FOUND".to_string(),
                    },
                    profit_min,
                    profit_max,
                    is_verified
                )
            );

            // Add position to saved positions immediately to prevent money loss
            if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                positions.push(new_position);
                save_positions_to_file(&positions);

                log(
                    LogTag::Trader,
                    "SAVED",
                    &format!(
                        "💾 Position saved to disk: {} - {} for trading",
                        token.symbol,
                        if is_verified { "ready" } else { "pending verification" }
                    )
                );
            }

            // Schedule background verification for unverified positions
            if !is_verified {
                schedule_delayed_position_verification(transaction_signature.clone(), token.symbol.clone()).await;
            }

            // Simplified approach - no complex transaction monitoring
            log(
                LogTag::Trader,
                "TRANSACTION",
                &format!("📡 Position entry transaction completed: {}", &transaction_signature[..8])
            );

            // Record position for RL learning if enabled
            let learner = get_trading_learner();
            if learner.is_model_ready() {
                if let Some(rugcheck_score) = get_rugcheck_score_for_token(&token.mint).await {
                    // Note: We'll record the complete trade when position is closed
                    if is_debug_rl_learn_enabled() {
                        log(
                            LogTag::Trader,
                            "RL_READY",
                            &format!(
                                "🤖 Position {} ready for RL learning when closed",
                                token.symbol
                            )
                        );
                    }
                }
            }
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!(
                    "❌ Failed to execute buy swap for {} ({}): {}",
                    token.symbol,
                    token.mint,
                    e
                )
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
    // CRITICAL: Check if entry transaction is finalized before allowing position closure
    if !position.transaction_entry_verified {
        log(
            LogTag::Trader,
            "VERIFICATION_REQUIRED",
            &format!(
                "⏳ Cannot close position for {} - entry transaction not yet verified. Waiting for background verification to complete",
                position.symbol
            )
        );
        return false;
    }

    // Check if this mint is in frozen account cooldown
    if is_mint_in_frozen_cooldown(&position.mint) {
        if let Some(remaining_minutes) = get_remaining_cooldown_minutes(&position.mint) {
            log(
                LogTag::Trader,
                "COOLDOWN",
                &format!(
                    "Skipping sell for {} - frozen account cooldown active ({} minutes remaining)",
                    position.symbol,
                    remaining_minutes
                )
            );
            return false; // Skip this sell attempt
        }
    }

    // Only attempt to sell if we have tokens from the buy transaction
    if let Some(stored_token_amount) = position.token_amount {
        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "📊 Starting position close analysis for {} ({}): stored_amount={} token units",
                position.symbol,
                &position.mint[..8],
                stored_token_amount
            )
        );

        // Debug: Log position creation details if available
        if let Some(ref entry_tx) = position.entry_transaction_signature {
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "📝 Position created from transaction: {} (entry_price: {:.9} SOL)",
                    &entry_tx[..16],
                    position.entry_price
                )
            );
        }
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
        let wallet_address = match crate::utils::get_wallet_address() {
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

        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "🔍 Checking wallet balance for {} ({}): wallet={}, mint={}",
                position.symbol,
                position.mint,
                wallet_address,
                &position.mint[..8]
            )
        );

        let actual_balance = match
            crate::utils::get_token_balance(&wallet_address, &position.mint).await
        {
            Ok(balance) => {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!(
                        "✅ Wallet balance check successful for {}: {} tokens (raw units)",
                        position.symbol,
                        balance
                    )
                );
                balance
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("❌ Failed to check current {} balance: {}", position.symbol, e)
                );
                return false;
            }
        };

        // Use the minimum of stored amount and actual balance to avoid "insufficient balance" errors
        let token_amount = std::cmp::min(stored_token_amount, actual_balance);

        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "⚖️ Token amount comparison for {}: stored={}, actual={}, using_min={}",
                position.symbol,
                stored_token_amount,
                actual_balance,
                token_amount
            )
        );

        if token_amount == 0 {
            // Enhanced debugging for zero token scenario
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "🔍 ZERO TOKENS DETECTED for {}:
                     - Position stored: {} token units
                     - Wallet actual: {} token units
                     - Mint: {}
                     - Entry TX: {}
                     - Entry verified: {}",
                    position.symbol,
                    stored_token_amount,
                    actual_balance,
                    position.mint,
                    position.entry_transaction_signature.as_ref().unwrap_or(&"None".to_string()),
                    position.transaction_entry_verified
                )
            );
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

            // Call debug function to investigate this issue
            debug_position_token_mismatch(&position.mint).await;

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

        // DRY-RUN MODE CHECK: Skip actual selling if dry-run is enabled
        if crate::arguments::is_dry_run_enabled() {
            log(
                LogTag::Trader,
                "DRY-RUN",
                &format!(
                    "🚫 DRY-RUN: Would close position for {} ({}) - Would sell {} tokens at {:.6} SOL",
                    position.symbol,
                    position.mint,
                    token_amount,
                    exit_price
                )
            );
            return false; // Don't modify the position in dry-run mode
        }

        // Execute real sell transaction with critical operation protection using instruction-based analysis
        let _guard = crate::trader::CriticalOperationGuard::new(
            &format!("SELL {}", position.symbol)
        );
        match sell_token(token, token_amount, None).await {
            Ok(swap_result) => {
                // CRITICAL FIX: Check if the sell transaction was actually successful on-chain
                if !swap_result.success {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "❌ Sell transaction failed on-chain for {}: {}",
                            position.symbol,
                            swap_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                        )
                    );

                    // Check for frozen account error
                    if let Some(error_msg) = &swap_result.error {
                        if is_frozen_account_error(error_msg) {
                            log(
                                LogTag::Trader,
                                "FROZEN_ACCOUNT",
                                &format!(
                                    "🧊 Frozen account detected for {}, adding to cooldown",
                                    position.symbol
                                )
                            );
                            add_mint_to_frozen_cooldown(&position.mint);
                        }
                    }
                    return false; // Failed to close
                }

                let transaction_signature = swap_result.transaction_signature
                    .clone()
                    .unwrap_or_default();

                log(
                    LogTag::Trader,
                    "VERIFICATION_WAIT",
                    &format!(
                        "⏳ Waiting for priority exit transaction verification before closing position for {}: {}",
                        position.symbol,
                        &transaction_signature[..8]
                    )
                );

                // Wait for priority transaction verification with 5 second timeout
                match wait_for_priority_swap_verification(&transaction_signature).await {
                    Ok(true) => {
                        log(
                            LogTag::Trader,
                            "VERIFIED",
                            &format!(
                                "✅ Exit transaction verified successfully for {}: {}",
                                position.symbol,
                                &transaction_signature[..8]
                            )
                        );
                    }
                    Ok(false) => {
                        log(
                            LogTag::Trader,
                            "PRIORITY_TIMEOUT",
                            &format!(
                                "⏰ Priority exit transaction verification timeout for {}: {} - proceeding anyway",
                                position.symbol,
                                &transaction_signature[..8]
                            )
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "❌ Exit transaction verification failed for {}: {} - error: {}",
                                position.symbol,
                                &transaction_signature[..8],
                                e
                            )
                        );
                        return false; // Don't close position if verification failed
                    }
                }

                // Simplified verification - assume success if we have a transaction signature
                let verification_success = !transaction_signature.is_empty();

                if verification_success {
                    // CRITICAL: Calculate actual SOL received from swap result
                    let sol_received_str = swap_result.output_amount.clone();
                    let sol_received_lamports: u64 = sol_received_str.parse().unwrap_or(0);
                    let sol_received = lamports_to_sol(sol_received_lamports);

                    if sol_received == 0.0 {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "❌ Transaction verified but no SOL received for {}. TX: {}",
                                position.symbol,
                                transaction_signature
                            )
                        );
                        return false; // Failed to close properly
                    }

                    // Calculate effective exit price and other metrics
                    let token_amount_raw = position.token_amount.unwrap_or(0) as f64; // Raw units
                    
                    // Get token decimals to convert raw amount to UI units
                    let token_decimals_opt = crate::tokens::get_token_decimals_sync(&position.mint);
                    let effective_exit_price = if token_amount_raw > 0.0 {
                        match token_decimals_opt {
                            Some(decimals) => {
                                // Convert raw units to UI units before calculating price
                                let token_amount_ui = token_amount_raw / (10_f64).powi(decimals as i32);
                                sol_received / token_amount_ui
                            }
                            None => {
                                log(
                                    LogTag::Trader,
                                    "WARN",
                                    &format!(
                                        "⚠️ Cannot calculate effective exit price for {} - decimals not available",
                                        position.symbol
                                    )
                                );
                                0.0
                            }
                        }
                    } else {
                        0.0
                    };

                    log(
                        LogTag::Trader,
                        "VERIFIED",
                        &format!(
                            "✅ Exit verified: {} sold {:.6} tokens (UI), received {:.9} SOL, effective price: {:.12}",
                            position.symbol,
                            token_amount_raw / (10_f64).powi(token_decimals_opt.unwrap_or(6) as i32),
                            sol_received,
                            effective_exit_price
                        )
                    );

                    // Update position with verified exit data
                    position.exit_price = Some(exit_price);
                    position.exit_time = Some(exit_time);
                    position.effective_exit_price = Some(effective_exit_price);
                    position.sol_received = Some(sol_received);
                    position.exit_transaction_signature = Some(transaction_signature.clone());
                    position.transaction_exit_verified = true;
                    position.exit_fee_lamports = None;

                    // Calculate actual P&L using unified function
                    let (net_pnl_sol, net_pnl_percent) = calculate_position_pnl(position, None);
                    let is_profitable = net_pnl_sol > 0.0;

                    log(
                        LogTag::Trader,
                        if is_profitable {
                            "PROFIT"
                        } else {
                            "LOSS"
                        },
                        &format!(
                            "{} POSITION CLOSED: {} | Exit TX: {} | Tokens sold: {:.6} (UI verified) | SOL received: {:.9} | P&L: {:.1}% ({:+.9} SOL)",
                            if is_profitable {
                                "💰"
                            } else {
                                "📉"
                            },
                            position.symbol,
                            transaction_signature,
                            token_amount_raw / (10_f64).powi(token_decimals_opt.unwrap_or(6) as i32),
                            sol_received,
                            net_pnl_percent,
                            net_pnl_sol
                        )
                    );

                    // Record position for RL learning
                    if let Err(e) = record_position_for_learning(position).await {
                        log(
                            LogTag::Trader,
                            "WARNING",
                            &format!("Failed to record position for RL learning: {}", e)
                        );
                    }

                    return true; // Successfully closed and verified
                } else {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "❌ Exit transaction verification failed - no SOL received from verified transaction for {}",
                            position.symbol
                        )
                    );
                    return false;
                }
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("❌ Failed to execute sell transaction for {}: {}", position.symbol, e)
                );

                // Check for frozen account error
                let error_msg = format!("{}", e);
                if is_frozen_account_error(&error_msg) {
                    log(
                        LogTag::Trader,
                        "FROZEN_ACCOUNT",
                        &format!(
                            "🧊 Frozen account detected for {}, adding to cooldown",
                            position.symbol
                        )
                    );
                    add_mint_to_frozen_cooldown(&position.mint);
                }
                return false;
            }
        }
    } else {
        log(
            LogTag::Trader,
            "ERROR",
            &format!(
                "❌ Cannot close position for {} - no token_amount stored (position not properly opened)",
                position.symbol
            )
        );
        return false;
    }
}

/// Schedule delayed verification for unverified positions
async fn schedule_delayed_position_verification(transaction_signature: String, symbol: String) {
    tokio::spawn(async move {
        // Wait 30 seconds to allow transaction to be finalized
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        
        log(
            LogTag::Trader,
            "DELAYED_VERIFY",
            &format!(
                "🔄 Starting delayed verification for position {}: {}",
                symbol,
                &transaction_signature[..8]
            )
        );
        
        // Try to verify the transaction
        match crate::transactions_manager::is_transaction_verified(&transaction_signature).await {
            true => {
                log(
                    LogTag::Trader,
                    "DELAYED_SUCCESS",
                    &format!(
                        "✅ Delayed verification successful for {}: {} - updating position",
                        symbol,
                        &transaction_signature[..8]
                    )
                );
                
                // Update position verification status
                if let Err(e) = update_position_verification_status(&transaction_signature, true).await {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!(
                            "❌ Failed to update verification status for {}: {}",
                            symbol,
                            e
                        )
                    );
                }
            }
            false => {
                log(
                    LogTag::Trader,
                    "DELAYED_FAILED",
                    &format!(
                        "❌ Delayed verification failed for {}: {} - transaction may have failed",
                        symbol,
                        &transaction_signature[..8]
                    )
                );
                
                // Additional investigation - check if transaction actually succeeded
                match crate::transactions_manager::get_transaction(&transaction_signature).await {
                    Ok(Some(tx)) => {
                        if tx.success {
                            log(
                                LogTag::Trader,
                                "WARNING",
                                &format!(
                                    "⚠️ Transaction {} succeeded but verification failed - manual review needed",
                                    &transaction_signature[..8]
                                )
                            );
                        } else {
                            log(
                                LogTag::Trader,
                                "CONFIRM_FAILED",
                                &format!(
                                    "✓ Transaction {} confirmed failed - position status correct",
                                    &transaction_signature[..8]
                                )
                            );
                        }
                    }
                    Ok(None) => {
                        log(
                            LogTag::Trader,
                            "WARNING",
                            &format!(
                                "⚠️ Transaction {} not found in cache - may need blockchain verification",
                                &transaction_signature[..8]
                            )
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "❌ Error checking transaction {}: {}",
                                &transaction_signature[..8],
                                e
                            )
                        );
                    }
                }
            }
        }
    });
}

/// Update position verification status
async fn update_position_verification_status(transaction_signature: &str, verified: bool) -> Result<(), String> {
    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
        for position in positions.iter_mut() {
            if let Some(ref entry_sig) = position.entry_transaction_signature {
                if entry_sig == transaction_signature {
                    position.transaction_entry_verified = verified;
                    
                    // Save updated positions to file
                    drop(positions); // Release lock before file operation
                    if let Ok(positions) = SAVED_POSITIONS.lock() {
                        save_positions_to_file(&positions);
                    }
                    
                    return Ok(());
                }
            }
        }
        Err("Position not found".to_string())
    } else {
        Err("Failed to lock positions".to_string())
    }
}
pub async fn debug_position_token_mismatch(mint: &str) {
    use crate::logger::{ log, LogTag };

    log(
        LogTag::Trader,
        "DEBUG",
        &format!("🔬 DEBUGGING POSITION TOKEN MISMATCH for mint: {}", mint)
    );

    // Extract position data without holding the lock across async operations
    let position_data = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .find(|p| p.mint == mint)
                .cloned()
        } else {
            None
        }
    }; // Mutex guard is dropped here

    if let Some(pos) = position_data {
        log(
            LogTag::Trader,
            "DEBUG",
            &format!(
                "📊 Position found: 
            - Symbol: {}
            - Stored token_amount: {:?}
            - Entry price: {:.9}
            - Entry transaction: {:?}
            - Entry verified: {}
            - Exit price: {:?}
            - Created: {}",
                pos.symbol,
                pos.token_amount,
                pos.entry_price,
                pos.entry_transaction_signature,
                pos.transaction_entry_verified,
                pos.exit_price,
                pos.entry_time
            )
        );

        // Check current wallet balance
        if let Ok(wallet_address) = crate::utils::get_wallet_address() {
            match crate::utils::get_token_balance(&wallet_address, mint).await {
                Ok(balance) => {
                    log(
                        LogTag::Trader,
                        "DEBUG",
                        &format!("💰 Current wallet balance: {} tokens", balance)
                    );

                    if let Some(stored) = pos.token_amount {
                        let difference = (balance as i64) - (stored as i64);
                        log(
                            LogTag::Trader,
                            "DEBUG",
                            &format!("📉 Balance difference: {} tokens (negative = missing tokens)", difference)
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("❌ Failed to check wallet balance: {}", e)
                    );
                }
            }

            // Check if we can find the entry transaction
            if let Some(ref entry_tx) = pos.entry_transaction_signature {
                log(
                    LogTag::Trader,
                    "DEBUG",
                    &format!("🔍 Checking entry transaction: {}", entry_tx)
                );

                match crate::transactions_manager::get_transaction(entry_tx).await {
                    Ok(Some(tx)) => {
                        log(
                            LogTag::Trader,
                            "DEBUG",
                            &format!(
                                "✅ Entry transaction found: 
                            - Success: {}
                            - Finalized: {}
                            - Transaction type: {:?}
                            - SOL balance change: {:.9}
                            - Token transfers: {}
                            - Has swap analysis: {}",
                                tx.success,
                                tx.finalized,
                                tx.transaction_type,
                                tx.sol_balance_change,
                                tx.token_transfers.len(),
                                tx.swap_analysis.is_some()
                            )
                        );

                        if let Some(swap) = &tx.swap_analysis {
                            log(
                                LogTag::Trader,
                                "DEBUG",
                                &format!(
                                    "🔄 Swap analysis details:
                                - Input token: {}
                                - Output token: {}  
                                - Input amount: {:.9}
                                - Output amount: {:.9}
                                - Effective price: {:.9}",
                                    &swap.input_token[..8],
                                    &swap.output_token[..8],
                                    swap.input_amount,
                                    swap.output_amount,
                                    swap.effective_price
                                )
                            );
                        }
                    }
                    Ok(None) => {
                        log(LogTag::Trader, "WARNING", "⚠️ Entry transaction not found in cache");
                    }
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("❌ Error fetching entry transaction: {}", e)
                        );
                    }
                }
            }
        }
    } else {
        log(LogTag::Trader, "WARNING", "⚠️ Position not found for the given mint");
    }

    log(LogTag::Trader, "DEBUG", "🏁 Position debug analysis complete");
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

/// Gets active frozen account cooldowns for display
pub fn get_active_frozen_cooldowns() -> Vec<(String, i64)> {
    let mut active_cooldowns = Vec::new();

    if let Ok(mut cooldowns) = FROZEN_ACCOUNT_COOLDOWNS.lock() {
        let now = Utc::now();
        let mut expired_mints = Vec::new();

        for (mint, cooldown_time) in cooldowns.iter() {
            let minutes_since_cooldown = (now - *cooldown_time).num_minutes();
            if minutes_since_cooldown >= FROZEN_ACCOUNT_COOLDOWN_MINUTES {
                expired_mints.push(mint.clone());
            } else {
                let remaining_minutes = FROZEN_ACCOUNT_COOLDOWN_MINUTES - minutes_since_cooldown;
                active_cooldowns.push((mint.clone(), remaining_minutes));
            }
        }

        // Remove expired cooldowns
        for mint in expired_mints {
            cooldowns.remove(&mint);
        }
    }

    active_cooldowns
}

/// Records a completed trade for RL learning
async fn record_position_for_learning(position: &Position) -> Result<(), String> {
    // Only record if we have exit data (entry data is always available)
    if position.exit_price.is_none() || position.exit_time.is_none() {
        return Err("Incomplete position data".to_string());
    }

    let entry_price = position.entry_price;
    let exit_price = position.exit_price.unwrap();
    let entry_time = position.entry_time;
    let exit_time = position.exit_time.unwrap();

    // Get additional data needed for RL learning
    // For now, use placeholder values - in a real implementation, we'd store these at entry time
    let liquidity_usd = 1000.0; // Default liquidity estimate
    let volume_24h = 50000.0; // Default volume estimate
    let market_cap = None; // Unknown market cap
    let rugcheck_score = get_rugcheck_score_for_token(&position.mint).await;

    // Record the trade using the RL system
    record_completed_trade(
        &position.mint,
        &position.symbol,
        entry_price,
        exit_price,
        entry_time,
        exit_time,
        liquidity_usd,
        volume_24h,
        market_cap,
        rugcheck_score
    ).await;

    Ok(())
}

/// Gets all open position mints
pub fn get_open_positions_mints() -> Vec<String> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
            .map(|p| p.mint.clone())
            .collect()
    } else {
        Vec::new()
    }
}

/// Gets all open positions
pub fn get_open_positions() -> Vec<Position> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

/// Gets all closed positions
pub fn get_closed_positions() -> Vec<Position> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_some())
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

/// Checks if a mint is an open position
pub fn is_open_position(mint: &str) -> bool {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .any(|p| p.mint == mint && p.position_type == "buy" && p.exit_price.is_none())
    } else {
        false
    }
}

/// Recovery function to detect successful transactions without positions
pub async fn recover_missing_positions() -> usize {
    log(
        LogTag::Trader,
        "RECOVERY",
        "🔍 Starting recovery scan for successful transactions without positions"
    );
    
    let mut recovered_count = 0;
    
    // Get all recent successful buy transactions
    let recent_transactions = match crate::transactions_manager::get_recent_successful_buy_transactions(24).await {
        Ok(transactions) => transactions,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("❌ Failed to fetch recent transactions for recovery: {}", e)
            );
            return 0;
        }
    };
    
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        for tx in recent_transactions {
            // Check if this transaction already has a position
            let has_position = positions.iter().any(|pos| {
                pos.entry_transaction_signature.as_ref() == Some(&tx.signature)
            });
            
            if !has_position && tx.success {
                log(
                    LogTag::Trader,
                    "RECOVERY_FOUND",
                    &format!(
                        "🚨 Found successful transaction without position: {} - attempting recovery",
                        &tx.signature[..8]
                    )
                );
                
                // Drop the lock before async operations
                drop(positions);
                
                // Attempt to create missing position
                if let Err(e) = create_recovery_position(&tx).await {
                    log(
                        LogTag::Trader,
                        "RECOVERY_FAILED",
                        &format!(
                            "❌ Failed to recover position for {}: {}",
                            &tx.signature[..8],
                            e
                        )
                    );
                } else {
                    recovered_count += 1;
                    log(
                        LogTag::Trader,
                        "RECOVERY_SUCCESS",
                        &format!(
                            "✅ Successfully recovered position for transaction: {}",
                            &tx.signature[..8]
                        )
                    );
                }
                
                // Re-acquire lock for next iteration
                if let Ok(pos) = SAVED_POSITIONS.lock() {
                    let positions = pos;
                    break; // Exit loop to avoid borrowing issues
                } else {
                    break;
                }
            }
        }
    }
    
    if recovered_count > 0 {
        log(
            LogTag::Trader,
            "RECOVERY_COMPLETE",
            &format!("🎯 Recovery complete: {} positions recovered", recovered_count)
        );
    } else {
        log(
            LogTag::Trader,
            "RECOVERY_COMPLETE",
            "✓ No missing positions found - all transactions accounted for"
        );
    }
    
    recovered_count
}

/// Create a recovery position from transaction data
async fn create_recovery_position(tx: &crate::transactions_manager::Transaction) -> Result<(), String> {
    // Extract swap analysis for position data
    let swap = tx.swap_analysis.as_ref().ok_or("No swap analysis available")?;
    
    // Get token information
    let token = crate::tokens::get_token_from_db(&swap.output_token).await
        .ok_or("Token not found in database")?;
    
    // Calculate token amount with decimals
    let token_decimals = crate::tokens::get_token_decimals(&token.mint).await.unwrap_or(9);
    let token_amount_units = (swap.output_amount * (10_f64).powi(token_decimals as i32)) as u64;
    
    // Create recovery position
    let recovery_position = Position {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        entry_price: swap.effective_price,
        entry_time: tx.timestamp,
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: swap.input_amount,
        total_size_sol: swap.input_amount,
        price_highest: swap.effective_price,
        price_lowest: swap.effective_price,
        entry_transaction_signature: Some(tx.signature.clone()),
        exit_transaction_signature: None,
        token_amount: Some(token_amount_units),
        effective_entry_price: Some(swap.effective_price),
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: None,
        profit_target_max: None,
        liquidity_tier: calculate_liquidity_tier(&token),
        transaction_entry_verified: true, // Mark as verified since we're recovering from confirmed transaction
        transaction_exit_verified: false,
        entry_fee_lamports: tx.fee_breakdown.as_ref().map(|f| (f.total_fees * 1_000_000_000.0) as u64),
        exit_fee_lamports: None,
    };
    
    // Add to positions
    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
        positions.push(recovery_position);
        save_positions_to_file(&positions);
        Ok(())
    } else {
        Err("Failed to lock positions for recovery".to_string())
    }
}

/// Comprehensive wallet reconciliation system - runs at startup
/// Compares actual wallet token balances with recorded positions to detect discrepancies
/// This is the CRITICAL function that prevents double-purchases and position tracking errors
pub async fn reconcile_wallet_positions_at_startup() -> Result<(), String> {
    log(
        LogTag::Position,
        "STARTUP_RECONCILE",
        "🚀 Starting comprehensive wallet reconciliation at startup..."
    );
    
    // Step 1: Run existing recovery for known missing positions
    let recovered_from_transactions = recover_missing_positions().await;
    
    // Step 2: Get actual wallet token balances
    let wallet_balances = get_wallet_token_balances().await?;
    log(
        LogTag::Position,
        "STARTUP_RECONCILE", 
        &format!("📊 Found {} tokens with non-zero balances in wallet", wallet_balances.len())
    );
    
    // Step 3: Get current recorded positions
    let open_positions = {
        let positions = SAVED_POSITIONS.lock().map_err(|e| format!("Failed to lock positions: {}", e))?;
        positions.iter()
            .filter(|p| p.exit_time.is_none())
            .cloned()
            .collect::<Vec<_>>()
    };
    
    // Create position map for quick lookup
    let position_map: HashMap<String, Position> = open_positions.iter()
        .map(|p| (p.mint.clone(), p.clone()))
        .collect();
    
    log(
        LogTag::Position,
        "STARTUP_RECONCILE",
        &format!("📋 Found {} recorded open positions", open_positions.len())
    );
    
    let mut fixes_applied = 0;
    let mut critical_issues = Vec::new();
    
    // Step 4: Check each wallet balance against positions
    for (mint, actual_balance) in &wallet_balances {
        if *actual_balance == 0 {
            continue; // Skip empty balances
        }
        
        match position_map.get(mint) {
            Some(position) => {
                // Position exists - verify amount matches
                if let Some(recorded_amount) = position.token_amount {
                    if recorded_amount != *actual_balance {
                        log(
                            LogTag::Position,
                            "STARTUP_RECONCILE",
                            &format!(
                                "⚠️ AMOUNT MISMATCH for {}: recorded={}, actual={}",
                                position.symbol, recorded_amount, actual_balance
                            )
                        );
                        
                        // Fix the position amount
                        if let Err(e) = update_position_token_amount(mint, *actual_balance).await {
                            critical_issues.push(format!("Failed to update {} amount: {}", position.symbol, e));
                        } else {
                            fixes_applied += 1;
                            log(
                                LogTag::Position,
                                "STARTUP_RECONCILE",
                                &format!("✅ Updated {} position amount: {} → {}", position.symbol, recorded_amount, actual_balance)
                            );
                        }
                    }
                } else {
                    // Position exists but has no recorded amount
                    if let Err(e) = update_position_token_amount(mint, *actual_balance).await {
                        critical_issues.push(format!("Failed to set {} amount: {}", position.symbol, e));
                    } else {
                        fixes_applied += 1;
                        log(
                            LogTag::Position,
                            "STARTUP_RECONCILE",
                            &format!("✅ Set missing amount for {} position: {}", position.symbol, actual_balance)
                        );
                    }
                }
            }
            None => {
                // Critical: We have tokens but no position!
                log(
                    LogTag::Position,
                    "STARTUP_RECONCILE",
                    &format!(
                        "🚨 CRITICAL: Found tokens without position! Mint: {}, Amount: {}",
                        mint, actual_balance
                    )
                );
                
                if let Err(e) = create_position_from_wallet_balance(mint, *actual_balance).await {
                    critical_issues.push(format!("Failed to create position for unknown tokens {}: {}", mint, e));
                } else {
                    fixes_applied += 1;
                    log(
                        LogTag::Position,
                        "STARTUP_RECONCILE",
                        &format!("✅ Created missing position for {} tokens", actual_balance)
                    );
                }
            }
        }
    }
    
    // Step 5: Check for positions without corresponding wallet balance (phantom positions)
    for position in &open_positions {
        if !wallet_balances.contains_key(&position.mint) {
            log(
                LogTag::Position,
                "STARTUP_RECONCILE",
                &format!(
                    "👻 PHANTOM POSITION: {} ({}) - position exists but no wallet balance",
                    position.symbol, position.mint
                )
            );
            
            // This could mean:
            // 1. Tokens were sold outside the bot
            // 2. Position tracking error
            // 3. Wallet was compromised
            critical_issues.push(format!("Phantom position detected: {} - manual review required", position.symbol));
        }
    }
    
    // Step 6: Report results
    if critical_issues.is_empty() {
        log(
            LogTag::Position,
            "STARTUP_RECONCILE",
            &format!(
                "✅ Wallet reconciliation complete: {} fixes applied, {} critical issues resolved, {} transactions recovered",
                fixes_applied, critical_issues.len(), recovered_from_transactions
            )
        );
    } else {
        log(
            LogTag::Position,
            "STARTUP_RECONCILE",
            &format!(
                "⚠️ Wallet reconciliation complete with issues: {} fixes applied, {} critical issues detected, {} transactions recovered",
                fixes_applied, critical_issues.len(), recovered_from_transactions
            )
        );
        
        for issue in &critical_issues {
            log(LogTag::Position, "CRITICAL_ISSUE", issue);
        }
    }
    
    // Step 7: Double-check by running another quick scan
    let post_reconcile_balance_count = get_wallet_token_balances().await?.len();
    let post_reconcile_position_count = {
        let positions = SAVED_POSITIONS.lock().map_err(|e| format!("Failed to lock positions: {}", e))?;
        positions.iter().filter(|p| p.exit_time.is_none()).count()
    };
    
    log(
        LogTag::Position,
        "STARTUP_RECONCILE",
        &format!(
            "📊 Post-reconciliation status: {} wallet tokens, {} open positions",
            post_reconcile_balance_count, post_reconcile_position_count
        )
    );
    
    if !critical_issues.is_empty() {
        return Err(format!("Wallet reconciliation completed with {} critical issues - manual review required", critical_issues.len()));
    }
    
    Ok(())
}

/// Get all non-zero token balances in wallet
async fn get_wallet_token_balances() -> Result<HashMap<String, u64>, String> {
    let rpc_client = crate::rpc::get_rpc_client();
    let wallet_pubkey = crate::utils::get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    
    let mut balances = HashMap::new();
    
    // Get all token accounts for our wallet
    match rpc_client.get_all_token_accounts(&wallet_pubkey.to_string()).await {
        Ok(token_accounts) => {
            for account in token_accounts {
                // Use the correct field names from TokenAccountInfo
                if account.balance > 0 {
                    balances.insert(account.mint, account.balance);
                }
            }
        }
        Err(e) => {
            return Err(format!("Failed to get token accounts: {}", e));
        }
    }
    
    log(
        LogTag::Position,
        "WALLET_SCAN",
        &format!("📊 Retrieved {} non-zero token balances from wallet", balances.len())
    );
    
    Ok(balances)
}

/// Update position token amount for reconciliation
async fn update_position_token_amount(mint: &str, new_amount: u64) -> Result<(), String> {
    let mut updated = false;
    
    {
        let mut positions = SAVED_POSITIONS.lock()
            .map_err(|e| format!("Failed to lock positions: {}", e))?;
        
        if let Some(position) = positions.iter_mut().find(|p| p.mint == *mint && p.exit_time.is_none()) {
            position.token_amount = Some(new_amount);
            updated = true;
        }
    }
    
    if updated {
        // Save to file
        let positions = SAVED_POSITIONS.lock()
            .map_err(|e| format!("Failed to lock positions for save: {}", e))?;
        save_positions_to_file(&positions);
        Ok(())
    } else {
        Err(format!("No open position found for mint: {}", mint))
    }
}

/// Create position from wallet balance when we have tokens but no position
async fn create_position_from_wallet_balance(mint: &str, amount: u64) -> Result<(), String> {
    // Try to find the most recent buy transaction for this mint
    let recent_transactions = crate::transactions_manager::get_recent_successful_buy_transactions(168) // 1 week
        .await
        .unwrap_or_default();
    
    let matching_tx = recent_transactions
        .iter()
        .filter(|tx| {
            tx.swap_analysis.as_ref()
                .map(|swap| swap.output_token == *mint)
                .unwrap_or(false)
        })
        .max_by_key(|tx| tx.timestamp);
    
    if let Some(tx) = matching_tx {
        // Create position from transaction data
        log(
            LogTag::Position,
            "WALLET_RECOVERY",
            &format!("📋 Found matching transaction {} for mint {} - creating position", &tx.signature[..8], mint)
        );
        
        create_recovery_position(tx).await?;
        
        // Update the amount to match actual wallet balance
        update_position_token_amount(mint, amount).await?;
        
        log(
            LogTag::Position,
            "WALLET_RECOVERY", 
            &format!("✅ Created position from transaction {} and updated amount to {}", &tx.signature[..8], amount)
        );
    } else {
        // No transaction found - create minimal position that needs manual review
        let (symbol, name) = get_token_info_by_mint(mint).await.unwrap_or_else(|_| {
            (format!("UNK_{}", &mint[..8]), "Unknown Token".to_string())
        });
        
        let minimal_position = Position {
            mint: mint.to_string(),
            symbol: symbol.clone(),
            name: name.clone(),
            entry_price: 0.0, // Will need manual correction
            entry_time: Utc::now(),
            exit_price: None,
            exit_time: None,
            position_type: "buy".to_string(),
            entry_size_sol: 0.0, // Will need manual correction
            total_size_sol: 0.0,
            price_highest: 0.0,
            price_lowest: f64::MAX,
            entry_transaction_signature: None,
            exit_transaction_signature: None,
            token_amount: Some(amount),
            effective_entry_price: None,
            effective_exit_price: None,
            sol_received: None,
            profit_target_min: None,
            profit_target_max: None,
            liquidity_tier: None,
            transaction_entry_verified: false, // Mark as unverified
            transaction_exit_verified: false,
            entry_fee_lamports: None,
            exit_fee_lamports: None,
        };
        
        {
            let mut positions = SAVED_POSITIONS.lock()
                .map_err(|e| format!("Failed to lock positions: {}", e))?;
            positions.push(minimal_position);
        }
        
        // Save to file
        let positions = SAVED_POSITIONS.lock()
            .map_err(|e| format!("Failed to lock positions for save: {}", e))?;
        save_positions_to_file(&positions);
        
        log(
            LogTag::Position,
            "WALLET_RECOVERY",
            &format!("⚠️ Created minimal position for unknown wallet balance: {} {} - MANUAL REVIEW REQUIRED", symbol, amount)
        );
    }
    
    Ok(())
}

/// Get token info by mint address
async fn get_token_info_by_mint(mint: &str) -> Result<(String, String), String> {
    // Try to get from token database
    if let Some(token) = crate::tokens::get_token_from_db(mint).await {
        return Ok((token.symbol.clone(), token.name.clone()));
    }
    
    // Fallback to minimal info
    Ok((format!("UNK_{}", &mint[..8]), "Unknown Token".to_string()))
}
