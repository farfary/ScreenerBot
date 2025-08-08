use crate::trader::*;
use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::tokens::Token;
use crate::utils::*;
use crate::rpc::lamports_to_sol;
use crate::swaps::{ buy_token, sell_token };
use crate::swaps::transaction::{
    TransactionMonitoringService, register_position_transaction, 
    verify_position_entry_transaction, verify_position_exit_transaction,
    is_position_transaction_verified
};
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
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)?;
    
    if liquidity_usd < 0.0 {
        return Some("INVALID".to_string());
    }
    
    // Liquidity tier classification based on USD value
    let tier = match liquidity_usd {
        x if x < 1_000.0 => "MICRO",      // < $1K
        x if x < 10_000.0 => "SMALL",     // $1K - $10K  
        x if x < 50_000.0 => "MEDIUM",    // $10K - $50K
        x if x < 250_000.0 => "LARGE",    // $50K - $250K
        x if x < 1_000_000.0 => "XLARGE", // $250K - $1M
        _ => "MEGA",                      // > $1M
    };
    
    Some(tier.to_string())
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

        // Use actual transaction fees instead of hardcoded values
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee;
        
        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let safe_invested = if sol_invested < 0.00001 { 0.00001 } else { sol_invested };
        let net_pnl_percent = (net_pnl_sol / safe_invested) * 100.0;

        // Log detailed PnL calculation for debugging
        log(
            LogTag::Trader,
            "PNL_DETAILED",
            &format!(
                "💰 DETAILED PNL CALCULATION for {}:\n  Entry size: {:.9} SOL\n  SOL received: {:.9} SOL\n  Buy fee: {:.9} SOL\n  Sell fee: {:.9} SOL\n  Total fees: {:.9} SOL\n  Net P&L: {:.9} SOL ({:.2}%)",
                position.symbol,
                sol_invested,
                sol_received,
                buy_fee,
                sell_fee,
                total_fees,
                net_pnl_sol,
                net_pnl_percent
            )
        );

        return (net_pnl_percent, net_pnl_sol);
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

            // Account for actual buy + sell fees
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let total_fees = buy_fee + sell_fee;
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee;
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

            // Account for actual buy fee (already paid) + estimated sell fee
            let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
            let total_fees = buy_fee + estimated_sell_fee;
            let net_pnl_sol = current_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee| lamports_to_sol(fee));
        let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
        let total_fees = buy_fee + estimated_sell_fee;
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

            let transaction_signature = swap_result.transaction_signature.clone().unwrap_or_default();
            
            // Register transaction for monitoring
            if let Err(e) = register_position_transaction(
                &transaction_signature,
                &token.mint,
                "buy",
                crate::swaps::config::SOL_MINT,
                &token.mint,
            ).await {
                log(
                    LogTag::Trader,
                    "WARNING",
                    &format!("⚠️ Failed to register transaction for monitoring: {}", e)
                );
            }

            // Perform comprehensive transaction verification using instruction analysis
            match verify_position_entry_transaction(
                &transaction_signature,
                &token.mint,
                TRADE_SIZE_SOL
            ).await {
                Ok(verification) => {
                    if !verification.success {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "❌ Entry transaction verification failed for {}: {}",
                                token.symbol,
                                verification.error.unwrap_or_default()
                            )
                        );
                        return;
                    }

                    // CRITICAL FIX: Validate that we actually received tokens from verified transaction
                    if verification.token_amount_received == 0 {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "❌ Transaction verified but no tokens received for {}. TX: {}",
                                token.symbol,
                                transaction_signature
                            )
                        );
                        return;
                    }

                    log(
                        LogTag::Trader,
                        "VERIFIED",
                        &format!(
                            "✅ Entry verified: {} received {} tokens, spent {:.9} SOL, effective price: {:.12}",
                            token.symbol,
                            verification.token_amount_received,
                            verification.total_cost_sol,
                            verification.effective_entry_price
                        )
                    );

                    // Get simple profit targets for this token
                    let (profit_min, profit_max) = crate::entry::get_profit_target(token).await;

                    // Create new verified position using verification data
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
                        total_size_sol: verification.total_cost_sol,
                        price_highest: verification.effective_entry_price,
                        price_lowest: verification.effective_entry_price,
                        entry_transaction_signature: Some(transaction_signature.clone()),
                        exit_transaction_signature: None,
                        token_amount: Some(verification.token_amount_received),
                        effective_entry_price: Some(verification.effective_entry_price),
                        effective_exit_price: None,
                        sol_received: None,
                        profit_target_min: Some(profit_min),
                        profit_target_max: Some(profit_max),
                        liquidity_tier: calculate_liquidity_tier(token),
                        transaction_entry_verified: verification.entry_transaction_verified,
                        transaction_exit_verified: false,
                        entry_fee_lamports: Some(verification.transaction_fee),
                        exit_fee_lamports: None,
                    };

                    log(
                        LogTag::Trader,
                        "SUCCESS",
                        &format!(
                            "✅ POSITION CREATED: {} | TX: {} | Tokens: {} (verified) | Signal Price: {:.12} SOL | Effective Price: {:.12} SOL | Profit Target: {:.1}%-{:.1}%",
                            token.symbol,
                            transaction_signature,
                            verification.token_amount_received,
                            price,
                            verification.effective_entry_price,
                            profit_min,
                            profit_max
                        )
                    );

                    // Add verified position to saved positions
                    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        positions.push(new_position);
                        save_positions_to_file(&positions);

                        log(
                            LogTag::Trader,
                            "SAVED",
                            &format!("💾 Position saved to disk: {} (verified entry)", token.symbol)
                        );
                    }

                    // Record position for RL learning if enabled
                    let learner = get_trading_learner();
                    if learner.is_model_ready() {
                        if let Some(rugcheck_score) = get_rugcheck_score_for_token(&token.mint).await {
                            // Note: We'll record the complete trade when position is closed
                            log(
                                LogTag::Trader,
                                "RL_READY",
                                &format!("🤖 Position {} ready for RL learning when closed", token.symbol)
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("❌ Entry transaction verification failed for {}: {}", token.symbol, e)
                    );
                    return;
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

        let actual_balance = match
            crate::utils::get_token_balance(&wallet_address, &position.mint).await
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
                                &format!("🧊 Frozen account detected for {}, adding to cooldown", position.symbol)
                            );
                            add_mint_to_frozen_cooldown(&position.mint);
                        }
                    }
                    return false; // Failed to close
                }

                let transaction_signature = swap_result.transaction_signature.clone().unwrap_or_default();
                
                // Register transaction for monitoring
                if let Err(e) = register_position_transaction(
                    &transaction_signature,
                    &position.mint,
                    "sell",
                    &position.mint,
                    crate::swaps::config::SOL_MINT,
                ).await {
                    log(
                        LogTag::Trader,
                        "WARNING",
                        &format!("⚠️ Failed to register sell transaction for monitoring: {}", e)
                    );
                }

                // Perform comprehensive transaction verification using instruction analysis
                match verify_position_exit_transaction(
                    &transaction_signature,
                    &position.mint,
                    token_amount
                ).await {
                    Ok(verification) => {
                        if !verification.success {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "❌ Exit transaction verification failed for {}: {}",
                                    position.symbol,
                                    verification.error.unwrap_or_default()
                                )
                            );
                            return false; // Failed to close properly
                        }

                        // CRITICAL FIX: Validate that we actually received SOL from verified transaction
                        if verification.sol_received == 0 {
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

                        log(
                            LogTag::Trader,
                            "VERIFIED",
                            &format!(
                                "✅ Exit verified: {} sold {} tokens, received {:.9} SOL, effective price: {:.12}",
                                position.symbol,
                                verification.token_amount_sold,
                                verification.net_sol_received,
                                verification.effective_exit_price
                            )
                        );

                        // Update position with verified exit data
                        position.exit_price = Some(exit_price);
                        position.exit_time = Some(exit_time);
                        position.effective_exit_price = Some(verification.effective_exit_price);
                        position.sol_received = Some(verification.net_sol_received);
                        position.exit_transaction_signature = Some(transaction_signature.clone());
                        position.transaction_exit_verified = verification.exit_transaction_verified;
                        position.exit_fee_lamports = Some(verification.transaction_fee);

                        // Calculate actual P&L using unified function
                        let (net_pnl_sol, net_pnl_percent) = calculate_position_pnl(position, None);
                        let is_profitable = net_pnl_sol > 0.0;

                        log(
                            LogTag::Trader,
                            if is_profitable { "PROFIT" } else { "LOSS" },
                            &format!(
                                "{} POSITION CLOSED: {} | Exit TX: {} | Tokens sold: {} (verified) | SOL received: {:.9} | P&L: {:.1}% ({:+.9} SOL)",
                                if is_profitable { "💰" } else { "📉" },
                                position.symbol,
                                transaction_signature,
                                verification.token_amount_sold,
                                verification.net_sol_received,
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
                    }
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("❌ Exit transaction verification failed for {}: {}", position.symbol, e)
                        );
                        return false;
                    }
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
                        &format!("🧊 Frozen account detected for {}, adding to cooldown", position.symbol)
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
