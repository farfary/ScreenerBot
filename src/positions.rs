use crate::trader::*;
use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::tokens::Token;
use crate::utils::*;
use crate::rpc::lamports_to_sol;
use crate::swaps::{ buy_token, sell_token, wait_for_swap_verification, wait_for_priority_swap_verification };
use crate::rl_learning::{ get_trading_learner, record_completed_trade };
use crate::entry::get_rugcheck_score_for_token;
use crate::transactions::add_priority_transaction;

use once_cell::sync::Lazy;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use serde::{ Serialize, Deserialize };
use colored::Colorize;
use std::collections::HashMap;
use std::str::FromStr;

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

/// Global cooldown between opening positions (seconds)
const POSITION_OPEN_COOLDOWN_SECS: i64 = 0;

/// Static global: last time a position was opened (for global cooldown)
static LAST_OPEN_POSITION_AT: Lazy<StdArc<StdMutex<Option<DateTime<Utc>>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(None))
});

/// Centralized re-entry cooldown after closing a position for the same token (minutes)
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 15;

/// Track last close time per mint to enforce re-entry cooldown
static LAST_CLOSE_TIME_PER_MINT: Lazy<StdArc<StdMutex<HashMap<String, DateTime<Utc>>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashMap::new()))
});

/// Record a close time for a mint (called when closing a position)
fn record_close_time_for_mint(mint: &str, when: DateTime<Utc>) {
    if let Ok(mut map) = LAST_CLOSE_TIME_PER_MINT.lock() {
        map.insert(mint.to_string(), when);
    }
}

/// Returns remaining cooldown minutes for a mint if within re-entry cooldown, else None
pub fn get_remaining_reentry_cooldown_minutes(mint: &str) -> Option<i64> {
    if POSITION_CLOSE_COOLDOWN_MINUTES <= 0 { return None; }
    if let Ok(map) = LAST_CLOSE_TIME_PER_MINT.lock() {
        if let Some(last_close) = map.get(mint) {
            let now = Utc::now();
            let minutes = (now - *last_close).num_minutes();
            if minutes < POSITION_CLOSE_COOLDOWN_MINUTES {
                return Some(POSITION_CLOSE_COOLDOWN_MINUTES - minutes);
            }
        }
    }
    None
}

/// Try to acquire the global "open position" cooldown window.
/// Returns Ok(()) if allowed now and sets the timestamp; Err(remaining_secs) if still cooling down.
fn try_acquire_open_cooldown() -> Result<(), i64> {
    if let Ok(mut last_ts) = LAST_OPEN_POSITION_AT.lock() {
        let now = Utc::now();
        if let Some(prev) = *last_ts {
            let elapsed = (now - prev).num_seconds();
            if elapsed < POSITION_OPEN_COOLDOWN_SECS {
                return Err(POSITION_OPEN_COOLDOWN_SECS - elapsed);
            }
        }
        // Set the new timestamp and allow
        *last_ts = Some(now);
        Ok(())
    } else {
        // If lock poisoned, fail-safe: block for full cooldown
        Err(POSITION_OPEN_COOLDOWN_SECS)
    }
}

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

    // RE-ENTRY COOLDOWN: Block re-entry for same mint shortly after closing
    if let Some(remaining) = get_remaining_reentry_cooldown_minutes(&token.mint) {
        log(
            LogTag::Trader,
            "COOLDOWN",
            &format!(
                "Re-entry cooldown active for {} ({}): wait {}m",
                token.symbol,
                &token.mint[..8],
                remaining
            )
        );
        return;
    }

    // GLOBAL COOLDOWN: Enforce delay between openings (regardless of token)
    match try_acquire_open_cooldown() {
        Ok(()) => { /* proceed */ }
        Err(remaining) => {
            log(
                LogTag::Trader,
                "COOLDOWN",
                &format!(
                    "Opening positions cooldown active: wait {}s before new position (requested: {} / {})",
                    remaining,
                    token.symbol,
                    &token.mint[..8]
                )
            );
            return;
        }
    }

    // DEADLOCK FIX: Check positions first, release lock, then check pending transactions
    let (already_has_position, open_positions_count) = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            let has_position = positions
                .iter()
                .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none());
            
            let count = positions
                .iter()
                .filter(|p| p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none())
                .count();
                
            (has_position, count)
        } else {
            (false, 0)
        }
    }; // Lock is released here

    if already_has_position {
        return; // Already have an open position for this token
    }

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

    // CRITICAL SAFETY CHECK: Check for pending transactions for this token
    // This prevents duplicate positions and transaction conflicts
    log(
        LogTag::Trader,
        "PENDING_TX_CHECK_START",
        &format!(
            "🔍 SAFETY CHECK: Checking for pending transactions before opening position for {} ({})",
            token.symbol, &token.mint[..8]
        )
    );

    match crate::transactions::has_pending_transactions_for_token(&token.mint).await {
        Ok(has_pending) => {
            if has_pending {
                log(
                    LogTag::Trader,
                    "PENDING_TX_BLOCKED",
                    &format!(
                        "� PENDING TRANSACTION DETECTED: Skipping new position for {} ({}) - transaction already in progress",
                        token.symbol, &token.mint[..8]
                    )
                );
                return;
            } else {
                log(
                    LogTag::Trader,
                    "PENDING_TX_CLEAR",
                    &format!(
                        "✅ SAFETY CHECK PASSED: No pending transactions found for {} ({}) - proceeding with position",
                        token.symbol, &token.mint[..8]
                    )
                );
            }
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "PENDING_TX_ERROR",
                &format!(
                    "❌ SAFETY CHECK FAILED: Failed to check pending transactions for {} ({}): {} - BLOCKING position for safety",
                    token.symbol, &token.mint[..8], e
                )
            );
            return; // Err on the side of caution - don't open position if we can't verify pending state
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
            let transaction_signature = swap_result.transaction_signature
                .clone()
                .unwrap_or_default();

            // NEW APPROACH: Create position optimistically even if initial confirmation failed
            // The background verification system will validate and update the position
            if !swap_result.success {
                log(
                    LogTag::Trader,
                    "WARNING",
                    &format!(
                        "⚠️ Initial transaction confirmation timed out for {}: {} - Creating position optimistically for background verification",
                        token.symbol,
                        &transaction_signature[..8]
                    )
                );
            } else {
                log(
                    LogTag::Trader,
                    "SUCCESS",
                    &format!(
                        "✅ Transaction confirmed for {}: {} - Creating verified position",
                        token.symbol,
                        &transaction_signature[..8]
                    )
                );
            }

            log(
                LogTag::Trader,
                "POSITION_CREATE",
                &format!(
                    "📝 Creating unverified position for {}: {} - verification will happen in background",
                    token.symbol,
                    &transaction_signature[..8]
                )
            );

            // Create position immediately without waiting for verification
            let is_verified = false;

            let (profit_min, profit_max) = crate::entry::get_profit_target(token).await;

            // Create position with minimal data - no transaction details fetching
            // All effective prices, token amounts, and fees will be filled during background verification
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
                total_size_sol: TRADE_SIZE_SOL,
                price_highest: price,
                price_lowest: price,
                entry_transaction_signature: Some(transaction_signature.clone()),
                exit_transaction_signature: None,
                token_amount: None, // Will be filled during verification
                effective_entry_price: None, // Will be filled during verification
                effective_exit_price: None,
                sol_received: None,
                profit_target_min: Some(profit_min),
                profit_target_max: Some(profit_max),
                liquidity_tier: calculate_liquidity_tier(token),
                transaction_entry_verified: false, // Always unverified initially
                transaction_exit_verified: false,
                entry_fee_lamports: None, // Will be filled during verification
                exit_fee_lamports: None,
            };

            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "✅ POSITION CREATED (UNVERIFIED): {} | TX: {} | Signal Price: {:.12} SOL | Profit Target: {:.1}%-{:.1}% | Verification: PENDING",
                    token.symbol,
                    &transaction_signature[..8],
                    price,
                    profit_min,
                    profit_max
                )
            );

            // DEADLOCK FIX: Add position and get snapshot for saving, then save outside lock
            let positions_snapshot = if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                positions.push(new_position);
                let snapshot = positions.clone();
                snapshot
            } else {
                log(LogTag::Trader, "ERROR", "Failed to lock positions for saving");
                return;
            }; // Lock is released here
            
            // Save to file outside of mutex lock to avoid blocking other threads
            save_positions_to_file(&positions_snapshot);

            log(
                LogTag::Trader,
                "SAVED",
                &format!(
                    "💾 Position saved to disk: {} - pending background verification",
                    token.symbol
                )
            );

            // Add transaction to transactions manager pending queue for background processing
            if let Err(e) = add_priority_transaction(transaction_signature.clone()).await {
                log(
                    LogTag::Trader,
                    "WARNING",
                    &format!(
                        "Failed to add entry transaction {} to priority queue: {}",
                        &transaction_signature[..8],
                        e
                    )
                );
            } else {
                log(
                    LogTag::Trader,
                    "PRIORITY_ADDED",
                    &format!(
                        "✅ Entry transaction {} added to priority processing queue",
                        &transaction_signature[..8]
                    )
                );
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
    exit_time: DateTime<Utc>,
    shutdown: Option<Arc<Notify>>
) -> bool {
    // CRITICAL CHECK: Don't close position if it already has an exit transaction
    if position.exit_transaction_signature.is_some() {
        log(
            LogTag::Trader,
            "ALREADY_CLOSED",
            &format!(
                "⚠️ Position for {} already has exit transaction signature: {}. Skipping close attempt.",
                position.symbol,
                position.exit_transaction_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );
        return true; // Position is already closed/being closed
    }

    // DRY-RUN MODE CHECK: Skip actual selling if dry-run is enabled
    if crate::arguments::is_dry_run_enabled() {
        log(
            LogTag::Trader,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would close position for {} ({}) at {:.6} SOL",
                position.symbol,
                position.mint,
                exit_price
            )
        );
        return false; // Don't modify the position in dry-run mode
    }

    // PHANTOM POSITION PREVENTION: Verify wallet balance before attempting sell
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to get wallet address for {}: {}", position.symbol, e)
            );
            return false;
        }
    };

    let wallet_balance = match crate::utils::get_token_balance(&wallet_address, &position.mint).await {
        Ok(balance) => balance,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to get wallet balance for {}: {}", position.symbol, e)
            );
            return false;
        }
    };

    // CRITICAL: Handle zero balance position (phantom position detection)
    if wallet_balance == 0 {
        log(
            LogTag::Trader,
            "PHANTOM_DETECTED",
            &format!(
                "🚨 PHANTOM POSITION DETECTED: {} - Expected {} tokens, wallet has 0. Investigating transaction history...",
                position.symbol,
                position.token_amount.unwrap_or(0)
            )
        );

        // Use transactions manager to investigate and resolve position state
        return verify_and_resolve_position_state(position, token, exit_price, exit_time).await;
    }

    // Log wallet balance vs position expectation for debugging
    let expected_amount = position.token_amount.unwrap_or(0);
    if wallet_balance != expected_amount {
        log(
            LogTag::Trader,
            "BALANCE_MISMATCH",
            &format!(
                "⚠️ Wallet balance mismatch for {}: Expected {} tokens, wallet has {} tokens",
                position.symbol,
                expected_amount,
                wallet_balance
            )
        );
    }

    // Execute real sell transaction with critical operation protection
    let _guard = crate::trader::CriticalOperationGuard::new(
        &format!("SELL {}", position.symbol)
    );

    log(
        LogTag::Trader,
        "SELL",
        &format!(
            "Closing position for {} ({}) at {:.6} SOL - Wallet balance: {} tokens",
            position.symbol,
            position.mint,
            exit_price,
            wallet_balance
        )
    );

    // Execute the token sale (shutdown-aware to avoid retries during shutdown)
    match sell_token(token, position.token_amount.unwrap_or(0), None, shutdown.clone()).await {
        Ok(swap_result) => {
            // Check if the transaction was successful
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
                return false;
            }

            let transaction_signature = swap_result.transaction_signature
                .clone()
                .unwrap_or_default();

            // Simply set the exit transaction signature and save
            position.exit_transaction_signature = Some(transaction_signature.clone());

            log(
                LogTag::Trader,
                "SUCCESS",
                &format!(
                    "✅ POSITION EXIT SIGNATURE SAVED: {} | Exit TX: {} | Details will be filled by background verification",
                    position.symbol,
                    &transaction_signature[..8]
                )
            );

            // Record close time for cooldown tracking
            record_close_time_for_mint(&position.mint, Utc::now());

            // DEADLOCK FIX: Get positions snapshot, then save outside lock
            let positions_snapshot = if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions.clone()
            } else {
                log(LogTag::Trader, "ERROR", "Failed to lock positions for saving");
                return false;
            }; // Lock is released here
            
            // Save to file outside of mutex lock to avoid blocking other threads
            save_positions_to_file(&positions_snapshot);
            log(
                LogTag::Trader,
                "SAVED",
                &format!(
                    "💾 Position exit signature saved to disk: {} - pending background verification",
                    position.symbol
                )
            );

            // Add transaction to transactions manager pending queue for background processing
            if let Err(e) = add_priority_transaction(transaction_signature.clone()).await {
                log(
                    LogTag::Trader,
                    "WARNING",
                    &format!(
                        "Failed to add exit transaction {} to priority queue: {}",
                        &transaction_signature[..8],
                        e
                    )
                );
            } else {
                log(
                    LogTag::Trader,
                    "PRIORITY_ADDED",
                    &format!(
                        "✅ Exit transaction {} added to priority processing queue",
                        &transaction_signature[..8]
                    )
                );
            }

            return true;
        }
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("❌ Failed to execute sell transaction for {}: {}", position.symbol, e)
            );
            return false;
        }
    }
}

/// Verify and resolve position state using transactions manager
/// This function investigates phantom positions and resolves their state based on blockchain data
async fn verify_and_resolve_position_state(
    position: &mut Position,
    token: &Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> bool {
    log(
        LogTag::Trader,
        "VERIFICATION",
        &format!("🔍 Verifying position state for {} using transactions manager", position.symbol)
    );

    // Get all swap transactions from transactions manager
    let swap_transactions = match crate::transactions::get_global_swap_transactions().await {
        Ok(transactions) => transactions,
        Err(e) => {
            log(
                LogTag::Trader,
                "ERROR",
                &format!("Failed to get swap transactions from manager: {}", e)
            );
            return false;
        }
    };

    // Filter transactions for this token mint
    let token_transactions: Vec<_> = swap_transactions
        .iter()
        .filter(|tx| tx.token_mint == position.mint)
        .collect();

    log(
        LogTag::Trader,
        "VERIFICATION",
        &format!("Found {} transactions for token {}", token_transactions.len(), position.symbol)
    );

    // Check for entry transaction verification if we have entry signature
    if let Some(ref entry_sig) = position.entry_transaction_signature {
        match crate::transactions::get_transaction(entry_sig).await {
            Ok(Some(tx)) => {
                if !tx.success {
                    log(
                        LogTag::Trader,
                        "PHANTOM_RESOLVED",
                        &format!(
                            "🚨 ENTRY TRANSACTION FAILED: {} - Entry transaction {} failed on blockchain. Marking position as closed.",
                            position.symbol,
                            &entry_sig[..8]
                        )
                    );
                    
                    // Mark phantom position as closed with zero values
                    position.exit_price = Some(0.0);
                    position.exit_time = Some(exit_time);
                    position.transaction_exit_verified = true;
                    position.sol_received = Some(0.0);
                    
                    // RACE CONDITION FIX: Update position in-place within locked context
                    // Save the corrected position - we need to find and update the position in the vector
                    let positions_snapshot = if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        // Find the position by mint and update it
                        if let Some(pos) = positions.iter_mut().find(|p| p.mint == position.mint) {
                            pos.exit_price = Some(0.0);
                            pos.exit_time = Some(exit_time);
                            pos.transaction_exit_verified = true;
                            pos.sol_received = Some(0.0);
                        }
                        positions.clone()
                    } else {
                        log(LogTag::Trader, "ERROR", "Failed to lock positions for phantom resolution");
                        return false;
                    }; // Lock is released here
                    
                    // Save to file outside of mutex lock
                    save_positions_to_file(&positions_snapshot);
                    
                    return true; // Position resolved as phantom
                }
            }
            Ok(None) => {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!("Entry transaction {} not found in transactions manager", &entry_sig[..8])
                );
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Failed to verify entry transaction {}: {}", &entry_sig[..8], e)
                );
            }
        }
    }

    // Look for any untracked sell transactions that match this position
    let sell_transactions: Vec<_> = token_transactions
        .iter()
        .filter(|tx| tx.swap_type == "Sell")
        .collect();

    if !sell_transactions.is_empty() {
        log(
            LogTag::Trader,
            "UNTRACKED_SELL",
            &format!("Found {} untracked sell transactions for {}", sell_transactions.len(), position.symbol)
        );
        
        // Find the most recent sell transaction that could match this position
        if let Some(latest_sell) = sell_transactions.iter().max_by_key(|tx| tx.slot.unwrap_or(0)) {
            log(
                LogTag::Trader,
                "POSITION_RESOLVED",
                &format!(
                    "🔍 UNTRACKED SELL DETECTED: {} - Found sell transaction: {:.6} SOL received for {:.0} tokens at {:.9} SOL/token",
                    position.symbol,
                    latest_sell.sol_amount,
                    latest_sell.token_amount,
                    latest_sell.calculated_price_sol
                )
            );

            // Update position with the untracked sell data
            position.exit_price = Some(latest_sell.calculated_price_sol);
            position.exit_time = Some(exit_time);
            position.effective_exit_price = Some(latest_sell.calculated_price_sol);
            position.sol_received = Some(latest_sell.sol_amount);
            position.transaction_exit_verified = true;
            position.exit_fee_lamports = Some((latest_sell.fee_sol * 1_000_000_000.0) as u64);

            // RACE CONDITION FIX: Update position in-place within locked context  
            // Save the updated position - we need to find and update the position in the vector
            let positions_snapshot = if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                // Find the position by mint and update it
                if let Some(pos) = positions.iter_mut().find(|p| p.mint == position.mint) {
                    pos.exit_price = Some(latest_sell.calculated_price_sol);
                    pos.exit_time = Some(exit_time);
                    pos.effective_exit_price = Some(latest_sell.calculated_price_sol);
                    pos.sol_received = Some(latest_sell.sol_amount);
                    pos.transaction_exit_verified = true;
                    pos.exit_fee_lamports = Some((latest_sell.fee_sol * 1_000_000_000.0) as u64);
                }
                positions.clone()
            } else {
                log(LogTag::Trader, "ERROR", "Failed to lock positions for untracked sell update");
                return false;
            }; // Lock is released here
            
            // Save to file outside of mutex lock
            save_positions_to_file(&positions_snapshot);
            log(
                LogTag::Trader,
                "POSITION_UPDATED",
                &format!("💾 Position {} updated with untracked sell data and saved", position.symbol)
            );

            return true; // Position resolved with historical data
        }
    }

    // If we reach here, position might be genuinely phantom or have other issues
    log(
        LogTag::Trader,
        "PHANTOM_UNRESOLVED",
        &format!(
            "❌ PHANTOM POSITION UNRESOLVED: {} - No matching sell transactions found. Position may be invalid. Consider manual review.",
            position.symbol
        )
    );

    // Don't attempt to sell - position is in an invalid state
    return false;
}

/// Gets the current count of open positions
pub fn get_open_positions_count() -> usize {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none())
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
    let entry_time = position.entry_time;
    // PANIC PREVENTION: Safe unwrapping with early return on invalid data
    let exit_price = match position.exit_price {
        Some(price) => price,
        None => return Err("Position has no exit price".to_string()),
    };
    let exit_time = match position.exit_time {
        Some(time) => time,
        None => return Err("Position has no exit time".to_string()),
    };

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
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none())
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
            .filter(|p| p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none())
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
            .filter(|p| p.position_type == "buy" && (p.exit_price.is_some() || p.exit_transaction_signature.is_some()))
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
            .any(|p| p.mint == mint && p.position_type == "buy" && p.exit_price.is_none() && p.exit_transaction_signature.is_none())
    } else {
        false
    }
}

/// Helper enum to categorize position states
#[derive(Debug, Clone, PartialEq)]
pub enum PositionState {
    Open,          // No exit transaction, actively trading
    Closing,       // Exit transaction submitted but not yet verified
    Closed,        // Exit transaction verified and exit_price set
}

/// Get the current state of a position
pub fn get_position_state(position: &Position) -> PositionState {
    if position.exit_price.is_some() {
        PositionState::Closed
    } else if position.exit_transaction_signature.is_some() {
        PositionState::Closing
    } else {
        PositionState::Open
    }
}

/// Gets positions by state
pub fn get_positions_by_state(state: PositionState) -> Vec<Position> {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        positions
            .iter()
            .filter(|p| p.position_type == "buy" && get_position_state(p) == state)
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}
