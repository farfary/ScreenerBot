use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::rpc::{ lamports_to_sol, get_rpc_client, SwapError, sol_to_lamports };
use crate::swaps::{ get_best_quote, execute_best_swap, RouterType, SwapResult };
use crate::swaps::types::SwapData;
use crate::swaps::config::{ SOL_MINT, QUOTE_SLIPPAGE_PERCENT, SELL_RETRY_SLIPPAGES };
use crate::tokens::Token;
use crate::arguments::is_debug_positions_enabled;
use crate::trader::*;
use crate::transactions::{
    get_transaction,
    is_transaction_verified,
    Transaction,
    SwapPnLInfo,
    TransactionStatus,
};
use crate::utils::*;

use chrono::{ DateTime, Utc };
use colored::Colorize;
use once_cell::sync::Lazy;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{ mpsc, oneshot, Notify, Mutex as AsyncMutex };
use tokio::fs;
use tokio::time::{ interval, Duration };

/// Unified profit/loss calculation for both open and closed positions
/// Uses effective prices and actual token amounts when available
/// For closed positions with sol_received, uses actual SOL invested vs SOL received
/// NOTE: sol_received should contain ONLY the SOL from token sale, excluding ATA rent reclaim
pub async fn calculate_position_pnl_async(
    position: &Position,
    current_price: Option<f64>
) -> (f64, f64) {
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🧮 Calculating P&L for {} - entry: {:.8}, exit: {:?}, current: {:?}",
                position.symbol,
                position.effective_entry_price.unwrap_or(position.entry_price),
                position.exit_price,
                current_price
            )
        );
    }

    // Safety check: validate position has valid entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("❌ Invalid entry price for {}: {}", position.symbol, entry_price)
            );
        }
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
            // Get token decimals from cache (async)
            let token_decimals_opt = crate::tokens::get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
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
            // Get token decimals from cache (async)
            let token_decimals_opt = crate::tokens::get_token_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    log(
                        LogTag::Positions,
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

/// Synchronous wrapper for calculate_position_pnl for backward compatibility
/// This function will use the sync decimals function and may block briefly
/// For new code, prefer calculate_position_pnl_async when possible
pub fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    if is_debug_positions_enabled() {
        log(
            LogTag::Positions,
            "DEBUG",
            &format!(
                "🧮 Calculating P&L (SYNC) for {} - entry: {:.8}, exit: {:?}, current: {:?}",
                position.symbol,
                position.effective_entry_price.unwrap_or(position.entry_price),
                position.exit_price,
                current_price
            )
        );
    }

    // Safety check: validate position has valid entry price
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("❌ Invalid entry price for {}: {}", position.symbol, entry_price)
            );
        }
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
            // Get token decimals from cache (synchronous - may block briefly)
            let token_decimals_opt = crate::tokens::get_token_decimals_sync(&position.mint);

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "❌ No decimals available for {} - using price change fallback",
                                position.symbol
                            )
                        );
                    }
                    // Fall back to price-change calculation
                    let price_change = (effective_exit - entry_price) / entry_price;
                    let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee|
                        lamports_to_sol(fee)
                    );
                    let sell_fee = position.exit_fee_lamports.map_or(0.0, |fee|
                        lamports_to_sol(fee)
                    );
                    let total_fees = buy_fee + sell_fee + PROFIT_EXTRA_NEEDED_SOL;
                    let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
                    let net_pnl_percent = price_change * 100.0 - fee_percent;
                    let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;
                    return (net_pnl_sol, net_pnl_percent);
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
            // Get token decimals from cache (synchronous - may block briefly)
            let token_decimals_opt = crate::tokens::get_token_decimals_sync(&position.mint);

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "❌ No decimals available for {} - using price change fallback",
                                position.symbol
                            )
                        );
                    }
                    // Fall back to price-change calculation
                    let price_change = (current - entry_price) / entry_price;
                    let buy_fee = position.entry_fee_lamports.map_or(0.0, |fee|
                        lamports_to_sol(fee)
                    );
                    let estimated_sell_fee = buy_fee; // Estimate sell fee same as buy fee
                    let total_fees = buy_fee + estimated_sell_fee + PROFIT_EXTRA_NEEDED_SOL;
                    let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
                    let net_pnl_percent = price_change * 100.0 - fee_percent;
                    let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;
                    return (net_pnl_sol, net_pnl_percent);
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

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    // Current price tracking
    pub current_price: Option<f64>, // Current market price (updated by monitoring system)
    pub current_price_updated: Option<DateTime<Utc>>, // When current_price was last updated
    // Phantom position cleanup flag (temporary, not persisted)
    #[serde(skip)]
    pub phantom_remove: bool,
}

// =============================================================================
// DEADLOCK PREVENTION RULES FOR GLOBAL LOCKS
// =============================================================================
//
// This module uses multiple global locks that can create deadlock scenarios.
//
// LOCK HIERARCHY (must be acquired in this order to prevent deadlocks):
// 1. RECENT_SWAP_ATTEMPTS
// 2. GLOBAL_TRANSACTION_MANAGER
// 3. GLOBAL_POSITIONS_HANDLE
//
// RULES:
// - NEVER hold multiple locks simultaneously unless following the hierarchy above
// - NEVER perform async operations (await) while holding any global lock
// - Use timeouts on all lock acquisitions to prevent indefinite blocking
// - Keep lock scopes as minimal as possible
// - Pre-calculate data before acquiring locks when possible
//
// =============================================================================

// =============================================================================
// DUPLICATE SWAP PROTECTION
// =============================================================================

/// Recent swap attempts tracking to prevent duplicate transactions
#[derive(Debug, Clone)]
struct SwapAttempt {
    timestamp: DateTime<Utc>,
    mint: String,
    amount_sol: f64,
    operation_type: String, // "BUY" or "SELL"
}

/// Global cache for recent swap attempts (prevents duplicate swaps during network delays)
static RECENT_SWAP_ATTEMPTS: Lazy<Arc<AsyncMutex<HashMap<String, SwapAttempt>>>> = Lazy::new(||
    Arc::new(AsyncMutex::new(HashMap::new()))
);

/// Duration to prevent duplicate swaps (30 seconds)
const DUPLICATE_SWAP_PREVENTION_SECS: i64 = 30;

/// Check if a similar swap was recently attempted for the same token
/// NOTE: This function must NOT call other functions that might acquire global locks
/// to prevent deadlocks. Keep lock scope minimal and avoid async operations while holding the lock.
async fn is_duplicate_swap_attempt(mint: &str, amount_sol: f64, operation: &str) -> bool {
    // Use timeout to prevent indefinite blocking
    let lock_result = tokio::time::timeout(
        Duration::from_secs(2),
        RECENT_SWAP_ATTEMPTS.lock()
    ).await;

    let mut recent_attempts = match lock_result {
        Ok(guard) => guard,
        Err(_) => {
            log(
                LogTag::Positions,
                "WARN",
                "🔒 Timeout acquiring RECENT_SWAP_ATTEMPTS lock, assuming no duplicate"
            );
            return false;
        }
    };

    let now = Utc::now();

    // Clean old attempts (older than prevention window)
    recent_attempts.retain(|_, attempt| {
        now.signed_duration_since(attempt.timestamp).num_seconds() < DUPLICATE_SWAP_PREVENTION_SECS
    });

    // Check for recent similar attempts
    let key = format!("{}_{}", mint, operation);
    if let Some(recent_attempt) = recent_attempts.get(&key) {
        let time_since = now.signed_duration_since(recent_attempt.timestamp).num_seconds();
        if time_since < DUPLICATE_SWAP_PREVENTION_SECS {
            // Similar amount check (within 10% tolerance)
            let amount_diff =
                (amount_sol - recent_attempt.amount_sol).abs() / recent_attempt.amount_sol;
            if amount_diff < 0.1 {
                log(
                    LogTag::Swap,
                    "DUPLICATE_PREVENTED",
                    &format!(
                        "🚫 DUPLICATE SWAP PREVENTED: {} {} for {} (last attempt {:.1}s ago)",
                        operation,
                        mint,
                        amount_sol,
                        time_since
                    )
                );
                return true;
            }
        }
    }

    // Record this attempt
    recent_attempts.insert(key, SwapAttempt {
        timestamp: now,
        mint: mint.to_string(),
        amount_sol,
        operation_type: operation.to_string(),
    });

    false
}

// =============================================================================
// POSITIONS MANAGER - CENTRALIZED POSITION HANDLING
// =============================================================================

/// Enhanced position state enum with comprehensive lifecycle tracking
#[derive(Debug, Clone, PartialEq)]
pub enum PositionState {
    Open, // No exit transaction, actively trading
    Closing, // Exit transaction submitted but not yet verified
    Closed, // Exit transaction verified and exit_price set
    ExitPending, // Exit transaction in verification queue (similar to Closing but more explicit)
    ExitFailed, // Exit transaction failed and needs retry
    Phantom, // Position exists but wallet has zero tokens (needs reconciliation)
    Reconciling, // Auto-healing in progress for phantom positions
}

/// PositionsManager handles all position operations in a centralized service
pub struct PositionsManager {
    shutdown: Arc<Notify>,
    pending_verifications: HashMap<String, DateTime<Utc>>, // signature -> created_at
    retry_queue: HashMap<String, (DateTime<Utc>, u32)>, // mint -> (next_retry, attempt_count)
    positions: Vec<Position>, // Internal positions storage (in-memory only)
    frozen_cooldowns: HashMap<String, DateTime<Utc>>, // mint -> cooldown_time
    last_close_time_per_mint: HashMap<String, DateTime<Utc>>, // mint -> last_close_time
    last_open_position_at: Option<DateTime<Utc>>, // global open cooldown
    applied_exit_signatures: HashMap<String, DateTime<Utc>>, // signature -> applied_at (prevents double-processing)
    verification_deadlines: HashMap<String, DateTime<Utc>>, // signature -> deadline (guards against premature reset)
}

/// Constants for cooldowns
const FROZEN_ACCOUNT_COOLDOWN_MINUTES: i64 = 15;
const POSITION_OPEN_COOLDOWN_SECS: i64 = 0;
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 15;

impl PositionsManager {
    /// Create new PositionsManager and load positions from disk
    pub fn new(shutdown: Arc<Notify>) -> Self {
        if is_debug_positions_enabled() {
            log(LogTag::Positions, "DEBUG", "🏗️ Creating new PositionsManager instance");
        }

        let manager = Self {
            shutdown,
            pending_verifications: HashMap::new(),
            retry_queue: HashMap::new(),
            positions: Vec::new(),
            frozen_cooldowns: HashMap::new(),
            last_close_time_per_mint: HashMap::new(),
            last_open_position_at: None,
            applied_exit_signatures: HashMap::new(),
            verification_deadlines: HashMap::new(),
        };

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                "📊 PositionsManager instance created, async initialization pending"
            );
        }

        manager
    }

    /// Async initialization after construction
    pub async fn initialize(&mut self) {
        // Load positions from disk on startup
        self.load_positions_from_disk().await;

        // Re-queue unverified transactions for comprehensive verification
        self.requeue_unverified_transactions();

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "📊 PositionsManager initialized with {} positions loaded from disk, {} pending verifications queued",
                    self.positions.len(),
                    self.pending_verifications.len()
                )
            );
        }
    }

    /// Run actor loop: handle incoming requests and periodic background tasks
    pub async fn run_actor(mut self, mut rx: mpsc::Receiver<PositionsRequest>) {
        log(LogTag::Positions, "INFO", "PositionsManager actor starting...");

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🎬 Actor started with {} open positions, {} pending verifications, {} retry queue items",
                    self.get_open_positions_count(),
                    self.pending_verifications.len(),
                    self.retry_queue.len()
                )
            );
        }

        let mut verification_interval = interval(Duration::from_secs(10));
        let mut retry_interval = interval(Duration::from_secs(30));
        let mut cleanup_interval = interval(Duration::from_secs(60));
        let mut reconciliation_interval = interval(Duration::from_secs(1800)); // Much less frequent: 30 minutes

        // Add reconciliation state tracking to prevent overlaps
        let mut reconciliation_in_progress = false;

        loop {
            tokio::select! {
                _ = self.shutdown.notified() => {
                    log(LogTag::Positions, "INFO", "PositionsManager shutting down gracefully");
                    break;
                }
                _ = verification_interval.tick() => { 
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "⏰ Running verification tick");
                    }
                    self.check_pending_verifications().await; 
                }
                _ = retry_interval.tick() => { 
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🔄 Running retry tick");
                    }
                    self.process_retry_queue().await; 
                }
                _ = cleanup_interval.tick() => { 
                    if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🧹 Running cleanup tick");
                    }
                    self.cleanup_phantom_positions().await; 
                }
                _ = reconciliation_interval.tick() => {
                    // Skip if reconciliation is already in progress to prevent overlaps
                    if reconciliation_in_progress {
                        if is_debug_positions_enabled() {
                            log(LogTag::Positions, "DEBUG", "⏭️ Skipping reconciliation - already in progress");
                        }
                        continue;
                    }
                    
                    reconciliation_in_progress = true;
                    
                    // Only run reconciliation on positions with missing fields or inconsistent state
                    let positions_needing_reconciliation: Vec<_> = self.positions.iter().enumerate()
                        .filter_map(|(index, p)| {
                            // Check for specific missing field conditions that indicate reconciliation is needed
                            let needs_reconciliation = 
                                // Case 1: Has exit signature but not verified (failed verification)
                                (p.exit_transaction_signature.is_some() && !p.transaction_exit_verified) ||
                                
                                // Case 2: Has token amount but missing exit data (potential phantom)
                                (p.token_amount.unwrap_or(0) > 0 && 
                                 p.exit_transaction_signature.is_none() && 
                                 p.exit_price.is_none() && 
                                 !p.phantom_remove) ||
                                
                                // Case 3: Marked for phantom removal but still has exit data inconsistency
                                (p.phantom_remove && p.exit_transaction_signature.is_some() && !p.transaction_exit_verified) ||
                                
                                // Case 4: Has sol_received but no exit_price (incomplete exit data)
                                (p.sol_received.is_some() && p.exit_price.is_none() && p.token_amount.unwrap_or(0) > 0);
                                
                            if needs_reconciliation {
                                Some((index, p.mint.clone(), p.symbol.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    if !positions_needing_reconciliation.is_empty() {
                        // Limit reconciliation to max 3 positions per cycle to prevent blocking
                        let limited_positions: Vec<_> = positions_needing_reconciliation.into_iter().take(3).collect();
                        
                        if is_debug_positions_enabled() {
                            log(LogTag::Positions, "DEBUG", &format!(
                                "🩺 Running targeted reconciliation on {} positions with missing fields: {} (limited from full list)",
                                limited_positions.len(),
                                limited_positions.iter()
                                    .map(|(_, _, symbol)| symbol.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                        self.run_targeted_reconciliation(limited_positions).await;
                    } else if is_debug_positions_enabled() {
                        log(LogTag::Positions, "DEBUG", "🩺 Skipping reconciliation - no positions with missing fields");
                    }
                    
                    reconciliation_in_progress = false;
                }
                maybe_msg = rx.recv() => {
                    match maybe_msg {
                        Some(msg) => {
                            self.handle_request(msg).await;
                        }
                        None => {
                            log(LogTag::Positions, "WARN", "PositionsManager channel closed; exiting actor");
                            break;
                        }
                    }
                }
            }
        }

        log(LogTag::Positions, "INFO", "PositionsManager actor stopped");
    }

    async fn handle_request(&mut self, msg: PositionsRequest) {
        match msg {
            PositionsRequest::OpenPosition { token, price, percent_change, size_sol, reply } => {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "📈 Received OpenPosition request for {} at price {} ({}% change) with size {} SOL",
                            token.symbol,
                            price,
                            percent_change,
                            size_sol
                        )
                    );
                }
                let _ = reply.send(
                    self.open_position(&token, price, percent_change, size_sol).await
                );
            }
            PositionsRequest::ClosePosition { mint, token, exit_price, exit_time, reply } => {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "📉 Spawning background task for ClosePosition request for {} at price {}",
                            token.symbol,
                            exit_price
                        )
                    );
                }

                // Spawn background task to prevent blocking the actor
                let mint_clone = mint.clone();
                let token_clone = token.clone();

                tokio::spawn(async move {
                    let result = execute_close_position_background(
                        mint_clone,
                        token_clone,
                        exit_price,
                        exit_time
                    ).await;
                    let _ = reply.send(result);
                });
            }
            PositionsRequest::AddVerification { signature } => {
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "🔍 Adding signature {} to verification queue",
                            get_signature_prefix(&signature)
                        )
                    );
                }
                self.add_verification(signature);
            }
            PositionsRequest::AddRetryFailedSell { mint } => {
                self.add_retry_failed_sell(mint);
            }
            PositionsRequest::UpdateTracking { mint, current_price, reply } => {
                let _ = reply.send(self.update_position_tracking(&mint, current_price));
            }
            PositionsRequest::GetOpenPositionsCount { reply } => {
                let _ = reply.send(self.get_open_positions_count());
            }
            PositionsRequest::GetOpenPositions { reply } => {
                let _ = reply.send(self.get_open_positions());
            }
            PositionsRequest::GetClosedPositions { reply } => {
                let _ = reply.send(self.get_closed_positions());
            }
            PositionsRequest::GetOpenMints { reply } => {
                let _ = reply.send(self.get_open_positions_mints());
            }
            PositionsRequest::IsOpen { mint, reply } => {
                let _ = reply.send(self.is_open_position(&mint));
            }
            PositionsRequest::GetByState { state, reply } => {
                let _ = reply.send(self.get_positions_by_state(&state));
            }
            PositionsRequest::RemoveByEntrySignature { signature, reason, reply } => {
                let _ = reply.send(
                    self.remove_position_by_entry_signature(&signature, &reason).await
                );
            }
            PositionsRequest::GetActiveFrozenCooldowns { reply } => {
                let _ = reply.send(self.get_active_frozen_cooldowns());
            }
            PositionsRequest::ForceReverifyAll { reply } => {
                if is_debug_positions_enabled() {
                    log(LogTag::Positions, "DEBUG", "🔄 Received ForceReverifyAll request");
                }
                let _ = reply.send(self.force_reverify_all_positions());
            }
        }
    }

    /// Get open positions count (includes Open and Closing states)
    fn get_open_positions_count(&self) -> usize {
        self.positions
            .iter()
            .filter(|p| {
                p.position_type == "buy" &&
                    ({
                        let state = self.get_position_state(p);
                        state == PositionState::Open || state == PositionState::Closing
                    })
            })
            .count()
    }

    /// Get open positions (includes Open, Closing, and ExitPending states)
    fn get_open_positions(&self) -> Vec<Position> {
        self.positions
            .iter()
            .filter(|p| {
                p.position_type == "buy" &&
                    ({
                        let state = self.get_position_state(p);
                        matches!(
                            state,
                            PositionState::Open |
                                PositionState::Closing |
                                PositionState::ExitPending
                        )
                    })
            })
            .cloned()
            .collect()
    }

    /// Get closed positions (only Closed state)
    fn get_closed_positions(&self) -> Vec<Position> {
        self.positions
            .iter()
            .filter(|p| {
                p.position_type == "buy" && self.get_position_state(p) == PositionState::Closed
            })
            .cloned()
            .collect()
    }

    /// Get open positions mints (includes Open, Closing, and ExitPending states)
    fn get_open_positions_mints(&self) -> Vec<String> {
        self.positions
            .iter()
            .filter(|p| {
                p.position_type == "buy" &&
                    ({
                        let state = self.get_position_state(p);
                        matches!(
                            state,
                            PositionState::Open |
                                PositionState::Closing |
                                PositionState::ExitPending
                        )
                    })
            })
            .map(|p| p.mint.clone())
            .collect()
    }

    /// Check if mint is an open position (includes Open, Closing, and ExitPending states)
    fn is_open_position(&self, mint: &str) -> bool {
        self.positions.iter().any(|p| {
            p.mint == mint &&
                p.position_type == "buy" &&
                ({
                    let state = self.get_position_state(p);
                    matches!(
                        state,
                        PositionState::Open | PositionState::Closing | PositionState::ExitPending
                    )
                })
        })
    }

    /// Get positions by state
    fn get_positions_by_state(&self, state: &PositionState) -> Vec<Position> {
        self.positions
            .iter()
            .filter(|p| p.position_type == "buy" && self.get_position_state(p) == *state)
            .cloned()
            .collect()
    }

    /// Get position state with enhanced phantom detection
    pub fn get_position_state(&self, position: &Position) -> PositionState {
        // Check for phantom state first (most critical)
        if position.phantom_remove {
            return PositionState::Phantom;
        }

        // Fully closed: entry verified, exit verified, and has exit price
        if
            position.transaction_entry_verified &&
            position.transaction_exit_verified &&
            position.exit_price.is_some()
        {
            return PositionState::Closed;
        }

        // Exit submitted but verification failed - needs retry
        if
            position.exit_transaction_signature.is_some() &&
            position.exit_price.is_some() &&
            !position.transaction_exit_verified
        {
            return PositionState::ExitFailed;
        }

        // Exit transaction submitted and pending verification
        if position.exit_transaction_signature.is_some() {
            // Check if signature is in pending verification queue
            if let Some(signature) = &position.exit_transaction_signature {
                if self.pending_verifications.contains_key(signature) {
                    return PositionState::ExitPending;
                }
            }
            return PositionState::Closing;
        }

        // Default to open state
        PositionState::Open
    }

    /// Targeted reconciliation - only processes positions with missing fields or inconsistent state
    /// This is much more efficient than checking all positions
    async fn run_targeted_reconciliation(
        &mut self,
        positions_to_check: Vec<(usize, String, String)>
    ) {
        let now = Utc::now();
        let mut healed_positions = 0;
        let mut positions_to_heal: Vec<(usize, String)> = Vec::new(); // (index, signature)

        log(
            LogTag::Positions,
            "RECONCILE",
            &format!(
                "🎯 Targeted reconciliation: checking {} specific positions",
                positions_to_check.len()
            )
        );

        // Get wallet address once for all checks
        let wallet_address = match crate::utils::get_wallet_address() {
            Ok(addr) => addr,
            Err(_) => {
                log(LogTag::Positions, "RECONCILE_ERROR", "Failed to get wallet address");
                return;
            }
        };

        // Process each position that needs reconciliation
        let mut positions_to_heal = Vec::new();
        let mut failed_signatures_to_clear = Vec::new(); // Track failed transaction signatures to clear

        for (index, mint, symbol) in positions_to_check {
            // Longer delay between checks to respect rate limits (500ms instead of 150ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            let position = &self.positions[index];

            log(
                LogTag::Positions,
                "RECONCILE",
                &format!("🔍 Checking position {} for missing fields", symbol)
            );

            // Case 1: Unverified exit transaction - check if it actually succeeded
            if let Some(ref exit_sig) = position.exit_transaction_signature {
                if !position.transaction_exit_verified {
                    log(
                        LogTag::Positions,
                        "RECONCILE",
                        &format!(
                            "🔄 Checking unverified exit transaction {} for {}",
                            get_signature_prefix(exit_sig),
                            symbol
                        )
                    );

                    // Add timeout to prevent hanging on slow RPC calls
                    match
                        tokio::time::timeout(
                            Duration::from_secs(60),
                            get_transaction(exit_sig)
                        ).await
                    {
                        Ok(Ok(Some(tx))) => {
                            // Check transaction status and success
                            match tx.status {
                                TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                                    if
                                        tx.success &&
                                        !self.applied_exit_signatures.contains_key(exit_sig)
                                    {
                                        log(
                                            LogTag::Positions,
                                            "RECONCILE_FOUND",
                                            &format!(
                                                "✅ Found successful verified exit transaction {} for {}",
                                                get_signature_prefix(exit_sig),
                                                symbol
                                            )
                                        );
                                        positions_to_heal.push((index, exit_sig.clone()));
                                        continue; // Move to next position
                                    } else if !tx.success {
                                        log(
                                            LogTag::Positions,
                                            "RECONCILE_FAILED_TX",
                                            &format!(
                                                "❌ Exit transaction {} failed for {} - marking for signature clearing",
                                                get_signature_prefix(exit_sig),
                                                symbol
                                            )
                                        );
                                        // Mark failed transaction signature for clearing
                                        failed_signatures_to_clear.push((
                                            index,
                                            "exit".to_string(),
                                        ));
                                    }
                                }
                                TransactionStatus::Pending => {
                                    log(
                                        LogTag::Positions,
                                        "RECONCILE_STILL_PENDING",
                                        &format!(
                                            "⏳ Exit transaction {} still pending for {} - will retry later",
                                            get_signature_prefix(exit_sig),
                                            symbol
                                        )
                                    );
                                }
                                TransactionStatus::Failed(ref error) => {
                                    log(
                                        LogTag::Positions,
                                        "RECONCILE_CONFIRMED_FAILED",
                                        &format!(
                                            "❌ Exit transaction {} confirmed failed for {}: {} - marking for signature clearing",
                                            get_signature_prefix(exit_sig),
                                            symbol,
                                            error
                                        )
                                    );
                                    // Mark confirmed failed transaction signature for clearing
                                    failed_signatures_to_clear.push((index, "exit".to_string()));
                                }
                            }
                        }
                        Ok(Ok(None)) => {
                            log(
                                LogTag::Positions,
                                "RECONCILE_TX_NOT_FOUND",
                                &format!(
                                    "📄 Exit transaction {} not found or still pending for {} - will retry",
                                    get_signature_prefix(exit_sig),
                                    symbol
                                )
                            );
                        }
                        Ok(Err(e)) => {
                            log(
                                LogTag::Positions,
                                "RECONCILE_TX_ERROR",
                                &format!(
                                    "⚠️ Error fetching exit transaction {} for {}: {}",
                                    get_signature_prefix(exit_sig),
                                    symbol,
                                    e
                                )
                            );
                        }
                        Err(_) => {
                            log(
                                LogTag::Positions,
                                "RECONCILE_TX_TIMEOUT",
                                &format!(
                                    "⏰ Timeout fetching exit transaction {} for {} - will retry later",
                                    get_signature_prefix(exit_sig),
                                    symbol
                                )
                            );
                        }
                    }
                }
            }

            // Case 2: Potential phantom - check wallet balance and search for missing exit
            if
                position.token_amount.unwrap_or(0) > 0 &&
                position.exit_transaction_signature.is_none() &&
                position.exit_price.is_none()
            {
                // Add timeout to wallet balance check to prevent hanging
                if
                    let Ok(Ok(wallet_balance)) = tokio::time::timeout(
                        Duration::from_secs(45),
                        crate::utils::get_token_balance(&wallet_address, &mint)
                    ).await
                {
                    if wallet_balance == 0 {
                        log(
                            LogTag::Positions,
                            "RECONCILE",
                            &format!("👻 Confirmed phantom position {} - searching for missing exit transaction", symbol)
                        );

                        // Search for missing exit transaction
                        if
                            let Some(exit_signature) =
                                self.find_missing_exit_transaction_targeted(position).await
                        {
                            if !self.applied_exit_signatures.contains_key(&exit_signature) {
                                positions_to_heal.push((index, exit_signature));
                            }
                        }
                    } else {
                        log(
                            LogTag::Positions,
                            "RECONCILE",
                            &format!(
                                "✅ Position {} has wallet balance {}, not phantom",
                                symbol,
                                wallet_balance
                            )
                        );
                    }
                } else {
                    log(
                        LogTag::Positions,
                        "RECONCILE_ERROR",
                        &format!("Failed to get wallet balance for {} (timeout or error)", symbol)
                    );
                }
            }

            // Case 3: Incomplete exit data - has sol_received but missing exit_price
            if
                position.sol_received.is_some() &&
                position.exit_price.is_none() &&
                position.token_amount.unwrap_or(0) > 0
            {
                log(
                    LogTag::Positions,
                    "RECONCILE",
                    &format!("🔧 Position {} has sol_received but missing exit_price - calculating", symbol)
                );

                // This will be handled in the healing phase if we have the necessary data
                if let Some(ref exit_sig) = position.exit_transaction_signature {
                    positions_to_heal.push((index, exit_sig.clone()));
                }
            }
        }

        // Clear failed transaction signatures (separate phase to avoid borrowing conflicts)
        for (index, signature_type) in failed_signatures_to_clear {
            if let Some(pos) = self.positions.get_mut(index) {
                match signature_type.as_str() {
                    "exit" => {
                        pos.exit_transaction_signature = None;
                        pos.transaction_exit_verified = false;
                        log(
                            LogTag::Positions,
                            "RECONCILE_CLEARED",
                            &format!("🧹 Cleared failed exit signature for position {}", pos.symbol)
                        );
                    }
                    "entry" => {
                        pos.entry_transaction_signature = None;
                        pos.transaction_entry_verified = false;
                        log(
                            LogTag::Positions,
                            "RECONCILE_CLEARED",
                            &format!(
                                "🧹 Cleared failed entry signature for position {}",
                                pos.symbol
                            )
                        );
                    }
                    _ => {}
                }
            }
        }

        // Apply healing to identified positions
        for (index, exit_signature) in positions_to_heal {
            // Shorter delay between healing operations (100ms instead of 200ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            log(
                LogTag::Positions,
                "RECONCILE_HEAL",
                &format!(
                    "✨ Auto-healing position with found exit tx {}",
                    get_signature_prefix(&exit_signature)
                )
            );

            // Get transaction details first (outside the mutable borrow)
            let healing_result = {
                // Get transaction details
                match get_transaction(&exit_signature).await {
                    Ok(Some(transaction)) => {
                        // Check transaction status first
                        match transaction.status {
                            TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                                if transaction.success {
                                    // Convert to swap info
                                    let empty_cache = std::collections::HashMap::new();
                                    match
                                        self.convert_to_swap_pnl_info(
                                            &transaction,
                                            &empty_cache,
                                            true
                                        ).await
                                    {
                                        Some(swap_info) => {
                                            // Calculate exit time
                                            let exit_time = DateTime::from_timestamp(
                                                transaction.block_time.unwrap_or(0) as i64,
                                                0
                                            ).unwrap_or_else(|| Utc::now());

                                            // Get fee from transaction (use fee_sol field converted to lamports)
                                            let fee = if transaction.fee_sol > 0.0 {
                                                Some(
                                                    crate::rpc::sol_to_lamports(transaction.fee_sol)
                                                )
                                            } else {
                                                None
                                            };

                                            Some((swap_info, exit_time, fee))
                                        }
                                        None => {
                                            log(
                                                LogTag::Positions,
                                                "RECONCILE_HEAL_NO_SWAP",
                                                &format!(
                                                    "⚠️ Transaction {} is not a valid swap - cannot heal position",
                                                    get_signature_prefix(&exit_signature)
                                                )
                                            );
                                            None
                                        }
                                    }
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "RECONCILE_HEAL_FAILED",
                                        &format!(
                                            "❌ Transaction {} failed - cannot use for healing: {}",
                                            get_signature_prefix(&exit_signature),
                                            transaction.error_message.unwrap_or(
                                                "Unknown error".to_string()
                                            )
                                        )
                                    );
                                    None
                                }
                            }
                            TransactionStatus::Pending => {
                                log(
                                    LogTag::Positions,
                                    "RECONCILE_HEAL_PENDING",
                                    &format!(
                                        "⏳ Transaction {} still pending - healing will retry later",
                                        get_signature_prefix(&exit_signature)
                                    )
                                );
                                None
                            }
                            TransactionStatus::Failed(ref error) => {
                                log(
                                    LogTag::Positions,
                                    "RECONCILE_HEAL_CONFIRMED_FAILED",
                                    &format!(
                                        "❌ Transaction {} confirmed failed - cannot use for healing: {}",
                                        get_signature_prefix(&exit_signature),
                                        error
                                    )
                                );
                                None
                            }
                        }
                    }
                    Ok(None) => {
                        log(
                            LogTag::Positions,
                            "RECONCILE_HEAL_NOT_FOUND",
                            &format!(
                                "📄 Transaction {} not found or pending - healing will retry",
                                get_signature_prefix(&exit_signature)
                            )
                        );
                        None
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "RECONCILE_HEAL_ERROR",
                            &format!(
                                "⚠️ Error fetching transaction {} for healing: {}",
                                get_signature_prefix(&exit_signature),
                                e
                            )
                        );
                        None
                    }
                }
            };

            if let Some((swap_info, exit_time, fee)) = healing_result {
                // Now apply the healing with all the data we need
                if let Some(position) = self.positions.get_mut(index) {
                    // Apply exit data to position
                    position.exit_transaction_signature = Some(exit_signature.clone());
                    position.transaction_exit_verified = true;
                    position.exit_time = Some(exit_time);
                    position.sol_received = Some(swap_info.sol_amount);
                    position.exit_fee_lamports = fee;

                    // Calculate effective exit price from actual transaction
                    if let Some(token_amount) = position.token_amount {
                        if
                            let Some(decimals) = crate::tokens::get_token_decimals(
                                &position.mint
                            ).await
                        {
                            let ui_token_amount =
                                (token_amount as f64) / (10_f64).powi(decimals as i32);
                            if ui_token_amount > 0.0 {
                                position.effective_exit_price = Some(
                                    swap_info.sol_amount / ui_token_amount
                                );
                                if position.exit_price.is_none() {
                                    position.exit_price = position.effective_exit_price;
                                }
                            }
                        }
                    }

                    log(
                        LogTag::Positions,
                        "RECONCILE_SUCCESS",
                        &format!(
                            "✅ Successfully applied retroactive exit for {} - SOL received: {:.6}, effective price: {:.8}",
                            position.symbol,
                            swap_info.sol_amount,
                            position.effective_exit_price.unwrap_or(0.0)
                        )
                    );

                    healed_positions += 1;
                    self.applied_exit_signatures.insert(exit_signature, now);
                } else {
                    log(
                        LogTag::Positions,
                        "RECONCILE_ERROR",
                        &format!("❌ Position index {} no longer valid during healing", index)
                    );
                }
            } else {
                log(
                    LogTag::Positions,
                    "RECONCILE_ERROR",
                    &format!(
                        "❌ Failed to get transaction details for exit signature {}",
                        get_signature_prefix(&exit_signature)
                    )
                );
            }
        }

        if healed_positions > 0 {
            log(
                LogTag::Positions,
                "RECONCILE_COMPLETE",
                &format!("🎯 Targeted reconciliation healed {} positions", healed_positions)
            );
            self.save_positions_to_disk().await;
        } else {
            log(
                LogTag::Positions,
                "RECONCILE_COMPLETE",
                "🎯 Targeted reconciliation completed - no healing needed"
            );
        }
    }

    /// Targeted search for missing exit transaction - only searches for specific position
    async fn find_missing_exit_transaction_targeted(&self, position: &Position) -> Option<String> {
        let search_start = position.entry_time;

        // Pre-calculate expected amount WITHOUT holding any locks
        let (expected_amount, _token_decimals) = if
            let Some(position_token_amount) = position.token_amount
        {
            if let Some(decimals) = crate::tokens::get_token_decimals(&position.mint).await {
                ((position_token_amount as f64) / (10_f64).powi(decimals as i32), decimals)
            } else {
                log(
                    LogTag::Positions,
                    "RECONCILE_ERROR",
                    &format!("Cannot get decimals for {} during targeted search", position.symbol)
                );
                return None;
            }
        } else {
            log(
                LogTag::Positions,
                "RECONCILE_ERROR",
                &format!("Position {} has no token_amount for targeted search", position.symbol)
            );
            return None;
        };

        // Only hold the lock for the minimum time needed
        use crate::transactions::GLOBAL_TRANSACTION_MANAGER;
        let manager_guard = GLOBAL_TRANSACTION_MANAGER.lock().await;

        if let Some(ref manager) = *manager_guard {
            // Targeted search: only look through recent transactions (limited scope)
            if let Ok(recent_transactions) = manager.get_recent_transactions(50).await {
                // Even more limited for targeted search
                for transaction in recent_transactions {
                    // Time filter
                    if let Some(block_time) = transaction.block_time {
                        let tx_time = chrono::DateTime
                            ::from_timestamp(block_time as i64, 0)
                            .unwrap_or_else(|| Utc::now());
                        if tx_time <= search_start {
                            continue;
                        }
                    }

                    // Quick filters before expensive analysis
                    if
                        !transaction.success ||
                        !manager.involves_token(&transaction, &position.mint)
                    {
                        continue;
                    }

                    // Analyze transaction
                    let empty_cache = std::collections::HashMap::new();
                    if
                        let Some(swap_info) = manager.convert_to_swap_pnl_info(
                            &transaction,
                            &empty_cache,
                            true
                        )
                    {
                        if swap_info.swap_type == "Sell" && swap_info.token_mint == position.mint {
                            // Check amount match (within 10% tolerance)
                            let amount_difference =
                                (swap_info.token_amount.abs() - expected_amount).abs() /
                                expected_amount;
                            if amount_difference <= 0.1 {
                                log(
                                    LogTag::Positions,
                                    "RECONCILE_FOUND",
                                    &format!(
                                        "🎯 Targeted search found exit transaction {} for {} - amount match: {:.2}% difference",
                                        get_signature_prefix(&transaction.signature),
                                        position.symbol,
                                        amount_difference * 100.0
                                    )
                                );
                                return Some(transaction.signature);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn update_position_tracking_direct(&mut self, position: &mut Position, current_price: f64) {
        if current_price == 0.0 {
            log(
                LogTag::Positions,
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

        // Update current price (always)
        position.current_price = Some(current_price);
    }

    /// Open a new position
    pub async fn open_position(
        &mut self,
        token: &Token,
        price: f64,
        percent_change: f64,
        size_sol: f64
    ) -> Result<(String, String), String> {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🎯 Starting open_position for {} at price {:.8} SOL ({}% change) with size {} SOL",
                    token.symbol,
                    price,
                    percent_change,
                    size_sol
                )
            );
        }

        // CRITICAL SAFETY CHECK: Validate price
        if price <= 0.0 || !price.is_finite() {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("❌ Invalid price validation failed: {}", price)
                );
            }
            return Err(format!("Invalid price: {}", price));
        }

        // DRY-RUN MODE CHECK
        if crate::arguments::is_dry_run_enabled() {
            log(
                LogTag::Positions,
                "DRY-RUN",
                &format!(
                    "🚫 DRY-RUN: Would open position for {} ({}) at {:.6} SOL ({})",
                    token.symbol,
                    get_mint_prefix(&token.mint),
                    price,
                    percent_change
                )
            );
            return Err("DRY-RUN: Position would be opened".to_string());
        }

        // RE-ENTRY COOLDOWN CHECK
        if let Some(remaining) = self.get_remaining_reentry_cooldown_minutes(&token.mint) {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "⏳ Re-entry cooldown active for {} - {} minutes remaining",
                        token.symbol,
                        remaining
                    )
                );
            }
            return Err(
                format!(
                    "Re-entry cooldown active for {} ({}): wait {}m",
                    token.symbol,
                    get_mint_prefix(&token.mint),
                    remaining
                )
            );
        }

        // GLOBAL COOLDOWN CHECK
        if let Err(remaining) = self.try_acquire_open_cooldown() {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!("⏳ Global open cooldown active - {} seconds remaining", remaining)
                );
            }
            return Err(format!("Opening positions cooldown active: wait {}s", remaining));
        }

        // CHECK EXISTING POSITION
        let (already_has_position, open_positions_count) = {
            let has_position = self.positions
                .iter()
                .any(|p| {
                    p.mint == token.mint &&
                        p.position_type == "buy" &&
                        p.exit_price.is_none() &&
                        p.exit_transaction_signature.is_none()
                });

            let count = self.positions
                .iter()
                .filter(|p| {
                    p.position_type == "buy" &&
                        p.exit_price.is_none() &&
                        p.exit_transaction_signature.is_none()
                })
                .count();

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "📊 Position check - existing: {}, open count: {}/{}",
                        has_position,
                        count,
                        MAX_OPEN_POSITIONS
                    )
                );
            }

            (has_position, count)
        };

        if already_has_position {
            return Err("Already have open position for this token".to_string());
        }

        if open_positions_count >= MAX_OPEN_POSITIONS {
            return Err(
                format!(
                    "Maximum open positions reached ({}/{})",
                    open_positions_count,
                    MAX_OPEN_POSITIONS
                )
            );
        }

        // Execute the buy transaction
        let _guard = crate::trader::CriticalOperationGuard::new(&format!("BUY {}", token.symbol));

        // DUPLICATE SWAP PREVENTION: Check if similar swap was recently attempted
        if is_duplicate_swap_attempt(&token.mint, size_sol, "BUY").await {
            return Err(
                format!(
                    "Duplicate swap prevented for {} - similar buy attempted within last {}s",
                    token.symbol,
                    DUPLICATE_SWAP_PREVENTION_SECS
                )
            );
        }

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "💸 Executing swap for {} with {} SOL at price {:.8}",
                    token.symbol,
                    size_sol,
                    price
                )
            );
        }

        // Validate expected price if provided
        if let Some(price) = Some(price) {
            if price <= 0.0 || !price.is_finite() {
                log(
                    LogTag::Swap,
                    "ERROR",
                    &format!(
                        "❌ REFUSING TO BUY: Invalid expected_price for {} ({}). Price = {:.10}",
                        token.symbol,
                        token.mint,
                        price
                    )
                );
                return Err(format!("Invalid expected price: {:.10}", price));
            }
        }

        log(
            LogTag::Swap,
            "BUY_START",
            &format!(
                "🟢 BUYING {} SOL worth of {} tokens (mint: {})",
                size_sol,
                token.symbol,
                token.mint
            )
        );

        let wallet_address = get_wallet_address().map_err(|e|
            format!("Failed to get wallet address: {}", e)
        )?;

        let best_quote = get_best_quote(
            SOL_MINT,
            &token.mint,
            sol_to_lamports(size_sol),
            &wallet_address,
            QUOTE_SLIPPAGE_PERCENT
        ).await.map_err(|e| format!("Failed to get quote: {}", e))?;

        if is_debug_swaps_enabled() {
            log(
                LogTag::Swap,
                "QUOTE",
                &format!(
                    "📊 Best quote from {:?}: {} SOL -> {} tokens",
                    best_quote.router,
                    lamports_to_sol(best_quote.input_amount),
                    best_quote.output_amount
                )
            );
        }

        log(
            LogTag::Swap,
            "SWAP",
            &format!("🚀 Executing swap with best quote via {:?}...", best_quote.router)
        );

        let swap_result = execute_best_swap(
            token,
            SOL_MINT,
            &token.mint,
            sol_to_lamports(size_sol),
            best_quote
        ).await.map_err(|e| format!("Failed to execute swap: {}", e))?;

        if let Some(ref signature) = swap_result.transaction_signature {
            log(
                LogTag::Swap,
                "TRANSACTION",
                &format!("Transaction {} will be monitored by positions manager", &signature[..8])
            );
        }

        if is_debug_swaps_enabled() {
            log(
                LogTag::Swap,
                "BUY_COMPLETE",
                &format!(
                    "🟢 BUY operation completed for {} - Success: {} | TX: {}",
                    token.symbol,
                    swap_result.success,
                    swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                )
            );
        }

        match swap_result {
            swap_result => {
                let transaction_signature = swap_result.transaction_signature
                    .clone()
                    .unwrap_or_default();

                // CRITICAL VALIDATION: Verify transaction signature is valid before creating position
                if transaction_signature.is_empty() || transaction_signature.len() < 32 {
                    return Err("Invalid transaction signature - swap may have failed".to_string());
                }

                // Additional validation: Check if signature is valid base58
                if bs58::decode(&transaction_signature).into_vec().is_err() {
                    return Err(
                        format!(
                            "Invalid base58 transaction signature: {}",
                            get_signature_prefix(&transaction_signature)
                        )
                    );
                }

                // Log swap execution details for debugging
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "✅ Swap executed via {:?} - signature: {}, success: {}",
                            swap_result.router_used
                                .as_ref()
                                .map(|r| format!("{:?}", r))
                                .unwrap_or_else(|| "Unknown".to_string()),
                            get_signature_prefix(&transaction_signature),
                            swap_result.success
                        )
                    );
                }

                // Create position optimistically
                let (profit_min, profit_max) = crate::entry::get_profit_target(token).await;

                let new_position = Position {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: token.name.clone(),
                    entry_price: price,
                    entry_time: Utc::now(),
                    exit_price: None,
                    exit_time: None,
                    position_type: "buy".to_string(),
                    entry_size_sol: size_sol,
                    total_size_sol: size_sol,
                    price_highest: price,
                    price_lowest: price,
                    entry_transaction_signature: Some(transaction_signature.clone()),
                    exit_transaction_signature: None,
                    token_amount: None,
                    effective_entry_price: None,
                    effective_exit_price: None,
                    sol_received: None,
                    profit_target_min: Some(profit_min),
                    profit_target_max: Some(profit_max),
                    liquidity_tier: calculate_liquidity_tier(token),
                    transaction_entry_verified: false,
                    transaction_exit_verified: false,
                    entry_fee_lamports: None,
                    exit_fee_lamports: None,
                    current_price: Some(price), // Initialize with entry price
                    current_price_updated: Some(Utc::now()),
                    phantom_remove: false,
                };

                // Add position to in-memory list
                self.positions.push(new_position);

                // Save positions to disk after adding new position
                self.save_positions_to_disk().await;

                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "✅ Position created for {} with signature {} - profit targets: {:.2}%-{:.2}%",
                            token.symbol,
                            get_signature_prefix(&transaction_signature),
                            profit_min,
                            profit_max
                        )
                    );
                }

                // Log entry transaction with comprehensive verification
                log(
                    LogTag::Positions,
                    "POSITION_ENTRY",
                    &format!(
                        "📝 Entry transaction {} added to comprehensive verification queue (RPC + transaction analysis)",
                        get_signature_prefix(&transaction_signature)
                    )
                );

                // Track for comprehensive verification using RPC and transaction analysis
                self.pending_verifications.insert(transaction_signature.clone(), Utc::now());

                // Immediately attempt to fetch transaction to accelerate verification (fire-and-forget)
                let sig_for_fetch = transaction_signature.clone();
                tokio::spawn(async move {
                    let _ = crate::transactions::get_transaction(&sig_for_fetch).await;
                });

                log(
                    LogTag::Positions,
                    "SUCCESS",
                    &format!(
                        "✅ POSITION CREATED: {} | TX: {} | Signal Price: {:.12} SOL | Verification: Pending",
                        token.symbol,
                        get_signature_prefix(&transaction_signature),
                        price
                    )
                );

                Ok((token.mint.clone(), transaction_signature))
            }
        }
    }

    /// Close a position
    pub async fn close_position(
        &mut self,
        mint: &str,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(String, String), String> {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🎯 Starting close_position for {} at price {:.8} SOL",
                    token.symbol,
                    exit_price
                )
            );
        }

        // Find the position to close
        let mut position_opt = None;

        if
            let Some(pos) = self.positions.iter_mut().find(|p| {
                let matches_mint = p.mint == mint;
                let no_exit_sig = p.exit_transaction_signature.is_none();
                let failed_exit =
                    p.exit_transaction_signature.is_some() && !p.transaction_exit_verified;
                let can_close = no_exit_sig || failed_exit;

                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!(
                            "🎯 Position check: mint_match={}, no_exit_sig={}, failed_exit={}, can_close={}",
                            matches_mint,
                            no_exit_sig,
                            failed_exit,
                            can_close
                        )
                    );
                }

                matches_mint && can_close
            })
        {
            // Allow retry if previous exit transaction failed
            if pos.exit_transaction_signature.is_some() && !pos.transaction_exit_verified {
                log(
                    LogTag::Positions,
                    "RETRY_EXIT",
                    &format!(
                        "🔄 Previous exit transaction failed for {} - clearing failed exit data and retrying",
                        pos.symbol
                    )
                );
                // Clear failed exit transaction data
                pos.exit_transaction_signature = None;
                pos.exit_price = None;
                pos.exit_time = None;
                pos.transaction_exit_verified = false;
                pos.sol_received = None;
                pos.effective_exit_price = None;
                pos.exit_fee_lamports = None;
            }
            position_opt = Some(pos.clone());
        }

        let mut position = match position_opt {
            Some(pos) => pos,
            None => {
                return Err("Position not found or already closed".to_string());
            }
        };

        // DRY-RUN MODE CHECK
        if crate::arguments::is_dry_run_enabled() {
            log(
                LogTag::Positions,
                "DRY-RUN",
                &format!(
                    "🚫 DRY-RUN: Would close position for {} at {:.6} SOL",
                    position.symbol,
                    exit_price
                )
            );
            return Err("DRY-RUN: Position would be closed".to_string());
        }

        // Check wallet balance
        let wallet_address = match crate::utils::get_wallet_address() {
            Ok(addr) => addr,
            Err(e) => {
                return Err(format!("Failed to get wallet address: {}", e));
            }
        };

        let wallet_balance = match
            crate::utils::get_token_balance(&wallet_address, &position.mint).await
        {
            Ok(balance) => balance,
            Err(e) => {
                return Err(format!("Failed to get token balance: {}", e));
            }
        };

        if wallet_balance == 0 {
            // Handle phantom position
            self.handle_phantom_position(&mut position, token, exit_price, exit_time).await;
            return Err("Phantom position resolved".to_string());
        }

        // Execute sell transaction with retry logic
        self.execute_sell_with_retry(&mut position, token, exit_price, exit_time).await
    }

    async fn execute_sell_with_retry(
        &mut self,
        position: &mut Position,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(String, String), String> {
        let _guard = crate::trader::CriticalOperationGuard::new(
            &format!("SELL {}", position.symbol)
        );

        let max_attempts = crate::arguments::get_max_exit_retries();
        for attempt in 1..=max_attempts {
            log(
                LogTag::Positions,
                "SELL_ATTEMPT",
                &format!(
                    "💰 Attempting to sell {} (attempt {}/{}) at {:.6} SOL",
                    position.symbol,
                    attempt,
                    max_attempts,
                    exit_price
                )
            );

            // Validate expected SOL output if provided
            if let Some(expected_sol) = Some(exit_price) {
                if expected_sol <= 0.0 || !expected_sol.is_finite() {
                    return Err(format!("Invalid expected SOL output: {:.10}", expected_sol));
                }
            }

            // Auto-retry with progressive slippage from config
            let slippages = &SELL_RETRY_SLIPPAGES;
            let shutdown = Some(self.shutdown.clone());
            let token_amount = position.token_amount.unwrap_or(0);

            let mut last_error = None;

            for (slippage_attempt, &slippage) in slippages.iter().enumerate() {
                // Abort before starting a new attempt if shutdown is in progress
                if let Some(ref s) = shutdown {
                    if check_shutdown_or_delay(s, tokio::time::Duration::from_millis(0)).await {
                        log(
                            LogTag::Swap,
                            "SHUTDOWN",
                            &format!(
                                "⏹️  Aborting further sell attempts for {} due to shutdown (before attempt {} with {:.1}% slippage)",
                                token.symbol,
                                slippage_attempt + 1,
                                slippage
                            )
                        );
                        return Err("Shutdown in progress - aborting sell".to_string());
                    }
                }

                log(
                    LogTag::Swap,
                    "SELL_ATTEMPT",
                    &format!(
                        "🔴 Sell attempt {} for {} with {:.1}% slippage",
                        slippage_attempt + 1,
                        token.symbol,
                        slippage
                    )
                );

                // Execute sell_token_with_slippage logic inline
                if is_debug_swaps_enabled() {
                    log(
                        LogTag::Swap,
                        "SELL_START",
                        &format!(
                            "🔴 Starting SELL operation for {} ({}) - Expected amount: {} tokens, Slippage: {:.1}%",
                            token.symbol,
                            token.mint,
                            token_amount,
                            slippage
                        )
                    );
                }

                let wallet_address = match get_wallet_address() {
                    Ok(addr) => addr,
                    Err(e) => {
                        last_error = Some(format!("Failed to get wallet address: {}", e));
                        continue;
                    }
                };

                let actual_wallet_balance = match
                    tokio::time::timeout(
                        Duration::from_secs(45),
                        get_token_balance(&wallet_address, &token.mint)
                    ).await
                {
                    Ok(Ok(balance)) => balance,
                    Ok(Err(e)) => {
                        last_error = Some(format!("Failed to get token balance: {}", e));
                        continue;
                    }
                    Err(_) => {
                        last_error = Some(
                            format!("Timeout getting token balance for {}", token.symbol)
                        );
                        continue;
                    }
                };

                if actual_wallet_balance == 0 {
                    log(
                        LogTag::Swap,
                        "WARNING",
                        &format!(
                            "⚠️ No {} tokens in wallet to sell (expected: {}, actual: 0)",
                            token.symbol,
                            token_amount
                        )
                    );
                    return Err(format!("No {} tokens in wallet", token.symbol));
                }

                let actual_sell_amount = actual_wallet_balance;

                log(
                    LogTag::Swap,
                    "SELL_AMOUNT",
                    &format!(
                        "💰 Selling {} {} tokens (position: {}, wallet: {})",
                        actual_sell_amount,
                        token.symbol,
                        token_amount,
                        actual_wallet_balance
                    )
                );

                // DUPLICATE SWAP PREVENTION: Check if similar sell was recently attempted
                let expected_sol_amount = exit_price; // Use expected SOL from exit calculation
                if is_duplicate_swap_attempt(&token.mint, expected_sol_amount, "SELL").await {
                    last_error = Some(
                        format!(
                            "Duplicate sell prevented for {} - similar sell attempted within last {}s",
                            token.symbol,
                            DUPLICATE_SWAP_PREVENTION_SECS
                        )
                    );
                    continue;
                }

                let best_quote = match
                    get_best_quote(
                        &token.mint,
                        SOL_MINT,
                        actual_sell_amount,
                        &wallet_address,
                        slippage
                    ).await
                {
                    Ok(quote) => quote,
                    Err(e) => {
                        last_error = Some(format!("Failed to get quote: {}", e));
                        continue;
                    }
                };

                let swap_result = match
                    execute_best_swap(
                        token,
                        &token.mint,
                        SOL_MINT,
                        actual_sell_amount,
                        best_quote
                    ).await
                {
                    Ok(result) => {
                        if let Some(ref signature) = result.transaction_signature {
                            log(
                                LogTag::Swap,
                                "TRANSACTION",
                                &format!(
                                    "Sell transaction {} will be monitored by positions manager",
                                    &signature[..8]
                                )
                            );
                        }

                        log(
                            LogTag::Swap,
                            "SELL_SUCCESS",
                            &format!(
                                "✅ Sell successful for {} on attempt {} with {:.1}% slippage",
                                token.symbol,
                                slippage_attempt + 1,
                                slippage
                            )
                        );

                        if is_debug_swaps_enabled() {
                            log(
                                LogTag::Swap,
                                "SELL_COMPLETE",
                                &format!(
                                    "🔴 SELL operation completed for {} - Success: {} | TX: {}",
                                    token.symbol,
                                    result.success,
                                    result.transaction_signature
                                        .as_ref()
                                        .unwrap_or(&"None".to_string())
                                )
                            );
                        }

                        result
                    }
                    Err(e) => {
                        let error_str = format!("{}", e);
                        log(
                            LogTag::Swap,
                            "SELL_RETRY",
                            &format!(
                                "⚠️ Sell attempt {} failed for {} with {:.1}% slippage: {}",
                                slippage_attempt + 1,
                                token.symbol,
                                slippage,
                                e
                            )
                        );

                        // Check for error types that should not be retried
                        if
                            error_str.contains("insufficient balance") ||
                            error_str.contains("InvalidAmount") ||
                            error_str.contains("ConfigError")
                        {
                            if error_str.contains("insufficient balance") {
                                log(
                                    LogTag::Swap,
                                    "SELL_FAILED_NO_RETRY",
                                    &format!(
                                        "❌ Stopping retries for {} - insufficient balance (tokens may have been sold in previous attempt)",
                                        token.symbol
                                    )
                                );
                            } else {
                                log(
                                    LogTag::Swap,
                                    "SELL_FAILED_NO_RETRY",
                                    &format!(
                                        "❌ Stopping retries for {} - unretryable error: {}",
                                        token.symbol,
                                        error_str
                                    )
                                );
                            }
                            return Err(error_str);
                        }

                        last_error = Some(error_str);

                        // If this isn't the last attempt, wait and continue
                        if slippage_attempt < slippages.len() - 1 {
                            // Before retry delay, check for shutdown and abort if requested
                            if let Some(ref s) = shutdown {
                                if
                                    check_shutdown_or_delay(
                                        s,
                                        tokio::time::Duration::from_millis(0)
                                    ).await
                                {
                                    log(
                                        LogTag::Swap,
                                        "SHUTDOWN",
                                        &format!(
                                            "⏹️  Skipping sell retry for {} due to shutdown (next slippage would be {:.1}%)",
                                            token.symbol,
                                            slippages[slippage_attempt + 1]
                                        )
                                    );
                                    return Err(
                                        "Shutdown in progress - aborting sell retries".to_string()
                                    );
                                }
                            }

                            // Wait before retry
                            tokio::time::sleep(
                                tokio::time::Duration::from_secs(
                                    ((slippage_attempt + 1) as u64) * 2
                                )
                            ).await;
                        }
                        continue;
                    }
                };

                // Success case - process the swap result
                let exit_signature = swap_result.transaction_signature.clone().unwrap_or_default();

                // Update position
                position.exit_transaction_signature = Some(exit_signature.clone());
                position.exit_price = Some(exit_price);
                position.exit_time = Some(exit_time);

                /// Save updated position (in-memory only)
                if let Some(pos) = self.positions.iter_mut().find(|p| p.mint == position.mint) {
                    *pos = position.clone();

                    // Save positions to disk after updating position
                    self.save_positions_to_disk().await;
                }

                // Log exit transaction with comprehensive verification
                log(
                    LogTag::Positions,
                    "POSITION_EXIT",
                    &format!(
                        "📝 Exit transaction {} added to comprehensive verification queue (RPC + transaction analysis)",
                        get_signature_prefix(&exit_signature)
                    )
                );

                // Track for comprehensive verification using RPC and transaction analysis
                self.pending_verifications.insert(exit_signature.clone(), Utc::now());

                // Set verification deadline to prevent premature reset
                self.set_exit_verification_deadline(&exit_signature, 5); // 5 minute deadline

                // Record close time for re-entry cooldown
                self.record_close_time_for_mint(&position.mint, exit_time);

                log(
                    LogTag::Positions,
                    "SUCCESS",
                    &format!(
                        "✅ POSITION CLOSED: {} | TX: {} | Exit Price: {:.12} SOL | Verification: Pending (Deadline: 5min)",
                        position.symbol,
                        get_signature_prefix(&exit_signature),
                        exit_price
                    )
                );

                return Ok((position.mint.clone(), exit_signature));
            }

            // All slippage attempts failed
            log(
                LogTag::Swap,
                "SELL_FAILED",
                &format!(
                    "❌ All sell attempts failed for {} after {} tries",
                    token.symbol,
                    slippages.len()
                )
            );

            let final_error = last_error.unwrap_or_else(|| "Unknown error".to_string());

            match attempt {
                _ if attempt < max_attempts => {
                    log(
                        LogTag::Positions,
                        "SELL_FAILED",
                        &format!(
                            "❌ Sell attempt {}/{} failed for {}: {}",
                            attempt,
                            max_attempts,
                            position.symbol,
                            final_error
                        )
                    );

                    // Check if it's a frozen account error
                    if is_frozen_account_error(&final_error) {
                        self.add_mint_to_frozen_cooldown(&position.mint);
                        return Err(format!("Token frozen, added to cooldown: {}", final_error));
                    }

                    // Wait before retry
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                _ => {
                    // Last attempt failed
                    // Add to retry queue for later
                    self.retry_queue.insert(position.mint.clone(), (
                        Utc::now() + chrono::Duration::minutes(5),
                        1,
                    ));
                    return Err(
                        format!("All sell attempts failed, added to retry queue: {}", final_error)
                    );
                }
            }
        }

        Err("Unexpected end of sell retry loop".to_string())
    }

    /// Update position tracking
    fn update_position_tracking(&mut self, mint: &str, current_price: f64) -> bool {
        if current_price == 0.0 {
            log(
                LogTag::Positions,
                "WARN",
                &format!(
                    "Skipping position tracking update for mint {}: current_price is zero",
                    get_mint_prefix(&mint)
                )
                    .yellow()
                    .dimmed()
                    .to_string()
            );
            return false;
        }

        if let Some(position) = self.positions.iter_mut().find(|p| p.mint == mint) {
            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "📊 Updating tracking for {} - price: {:.8} (prev high: {:.8}, low: {:.8})",
                        position.symbol,
                        current_price,
                        position.price_highest,
                        position.price_lowest
                    )
                );
            }

            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            if position.price_highest == 0.0 {
                position.price_highest = entry_price;
                position.price_lowest = entry_price;
            }

            let mut updated = false;
            if current_price > position.price_highest {
                position.price_highest = current_price;
                updated = true;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("📈 New high for {}: {:.8} SOL", position.symbol, current_price)
                    );
                }
            }
            if current_price < position.price_lowest {
                position.price_lowest = current_price;
                updated = true;
                if is_debug_positions_enabled() {
                    log(
                        LogTag::Positions,
                        "DEBUG",
                        &format!("📉 New low for {}: {:.8} SOL", position.symbol, current_price)
                    );
                }
            }

            // Update current price (always, regardless of high/low changes)
            position.current_price = Some(current_price);

            if is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "TRACK",
                    &format!(
                        "📊 Updated current price for {}: {:.8} SOL (high: {:.8}, low: {:.8})",
                        position.symbol,
                        current_price,
                        position.price_highest,
                        position.price_lowest
                    )
                );
            }

            // in-memory only; no persistence
            true
        } else {
            false
        }
    }

    /// Remove position by entry signature
    async fn remove_position_by_entry_signature(&mut self, signature: &str, reason: &str) -> bool {
        let before = self.positions.len();
        self.positions.retain(|p| {
            let target = p.entry_transaction_signature.as_deref() == Some(signature);
            let should_remove = target && !p.transaction_entry_verified;
            if should_remove {
                log(
                    LogTag::Positions,
                    "POSITION_REMOVED",
                    &format!(
                        "🗑️ Removing unverified position {} ({}): {}",
                        p.symbol,
                        get_mint_prefix(&p.mint),
                        reason
                    )
                );
            }
            !should_remove
        });
        let removed = before != self.positions.len();

        // Save positions to disk after removal
        if removed {
            self.save_positions_to_disk().await;
        }

        removed
    }

    /// Force reverification of all positions with unverified transactions
    fn force_reverify_all_positions(&mut self) -> usize {
        let mut reverified_count = 0;
        let now = Utc::now();

        log(
            LogTag::Positions,
            "FORCE_REVERIFY",
            "🔄 Starting forced reverification of all unverified transactions"
        );

        for position in &self.positions {
            // Check entry transaction
            if let Some(entry_sig) = &position.entry_transaction_signature {
                if !position.transaction_entry_verified {
                    log(
                        LogTag::Positions,
                        "FORCE_REVERIFY",
                        &format!(
                            "📝 Re-queuing unverified entry transaction {} for position {}",
                            get_signature_prefix(entry_sig),
                            position.symbol
                        )
                    );
                    self.pending_verifications.insert(entry_sig.clone(), now);
                    reverified_count += 1;
                }
            }

            // Check exit transaction
            if let Some(exit_sig) = &position.exit_transaction_signature {
                if !position.transaction_exit_verified {
                    log(
                        LogTag::Positions,
                        "FORCE_REVERIFY",
                        &format!(
                            "📝 Re-queuing unverified exit transaction {} for position {}",
                            get_signature_prefix(exit_sig),
                            position.symbol
                        )
                    );
                    self.pending_verifications.insert(exit_sig.clone(), now);
                    reverified_count += 1;
                }
            }
        }

        log(
            LogTag::Positions,
            "FORCE_REVERIFY",
            &format!("✅ Force reverification complete: {} transactions re-queued for verification", reverified_count)
        );

        reverified_count
    }

    /// Get active frozen cooldowns
    fn get_active_frozen_cooldowns(&mut self) -> Vec<(String, i64)> {
        let mut active_cooldowns = Vec::new();
        let now = Utc::now();
        let mut expired_mints = Vec::new();

        for (mint, cooldown_time) in self.frozen_cooldowns.iter() {
            let minutes_since_cooldown = (now - *cooldown_time).num_minutes();
            if minutes_since_cooldown >= FROZEN_ACCOUNT_COOLDOWN_MINUTES {
                expired_mints.push(mint.clone());
            } else {
                let remaining_minutes = FROZEN_ACCOUNT_COOLDOWN_MINUTES - minutes_since_cooldown;
                active_cooldowns.push((mint.clone(), remaining_minutes));
            }
        }

        for mint in expired_mints {
            self.frozen_cooldowns.remove(&mint);
        }

        active_cooldowns
    }

    /// Get remaining reentry cooldown for mint
    fn get_remaining_reentry_cooldown_minutes(&self, mint: &str) -> Option<i64> {
        if POSITION_CLOSE_COOLDOWN_MINUTES <= 0 {
            return None;
        }
        if let Some(last_close) = self.last_close_time_per_mint.get(mint) {
            let now = Utc::now();
            let minutes = (now - *last_close).num_minutes();
            if minutes < POSITION_CLOSE_COOLDOWN_MINUTES {
                return Some(POSITION_CLOSE_COOLDOWN_MINUTES - minutes);
            }
        }
        None
    }

    /// Record close time for mint
    fn record_close_time_for_mint(&mut self, mint: &str, when: DateTime<Utc>) {
        self.last_close_time_per_mint.insert(mint.to_string(), when);
    }

    /// Try to acquire open cooldown
    fn try_acquire_open_cooldown(&mut self) -> Result<(), i64> {
        let now = Utc::now();
        if let Some(prev) = self.last_open_position_at {
            let elapsed = (now - prev).num_seconds();
            if elapsed < POSITION_OPEN_COOLDOWN_SECS {
                return Err(POSITION_OPEN_COOLDOWN_SECS - elapsed);
            }
        }
        self.last_open_position_at = Some(now);
        Ok(())
    }

    /// Add mint to frozen cooldown
    fn add_mint_to_frozen_cooldown(&mut self, mint: &str) {
        self.frozen_cooldowns.insert(mint.to_string(), Utc::now());
        log(
            LogTag::Positions,
            "COOLDOWN",
            &format!(
                "Added {} to frozen account cooldown for {} minutes",
                mint,
                FROZEN_ACCOUNT_COOLDOWN_MINUTES
            )
        );
    }

    /// Add verification for transaction signature
    pub fn add_verification(&mut self, signature: String) {
        self.pending_verifications.insert(signature, Utc::now());
    }

    /// Add retry for failed sell
    pub fn add_retry_failed_sell(&mut self, mint: String) {
        self.retry_queue.insert(mint, (Utc::now() + chrono::Duration::minutes(5), 1));
    }

    /// Handle phantom position detection and resolution
    async fn handle_phantom_position(
        &mut self,
        position: &mut Position,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) {
        log(
            LogTag::Positions,
            "PHANTOM",
            &format!(
                "🔍 PHANTOM POSITION DETECTED: {} - wallet has 0 tokens but position exists",
                position.symbol
            )
        );

        // Try to resolve by checking transaction history
        if
            let Err(e) = self.verify_and_resolve_position_state(
                position,
                token,
                exit_price,
                exit_time
            ).await
        {
            log(
                LogTag::Positions,
                "ERROR",
                &format!("Failed to resolve phantom position for {}: {}", position.symbol, e)
            );
        }
    }

    /// Check pending verifications and update positions accordingly
    /// Enhanced with comprehensive transaction verification using RPC and transaction analysis
    async fn check_pending_verifications(&mut self) {
        let signatures_to_check: Vec<String> = self.pending_verifications.keys().cloned().collect();

        if is_debug_positions_enabled() && !signatures_to_check.is_empty() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("🔍 Checking {} pending verifications", signatures_to_check.len())
            );
        }

        for signature in signatures_to_check {
            match self.verify_transaction_comprehensively(&signature).await {
                Ok(verification_result) => {
                    match verification_result {
                        Some(transaction) => {
                            // Transaction is confirmed and successful - determine if entry or exit and use correct verification
                            let mut verification_success = false;
                            let signature_clone = signature.clone();

                            // Check for entry transaction
                            if
                                let Some(position_index) = self.positions
                                    .iter()
                                    .position(|p| {
                                        p.entry_transaction_signature.as_ref() ==
                                            Some(&signature.to_string())
                                    })
                            {
                                // Temporarily extract position to avoid borrowing conflicts
                                let mut position = self.positions.remove(position_index);
                                self.entry_verification(&mut position, &transaction).await;
                                self.positions.insert(position_index, position);
                                verification_success = true;
                                log(
                                    LogTag::Positions,
                                    "ENTRY_VERIFIED",
                                    &format!(
                                        "✅ Entry transaction {} verified using correct verification method",
                                        get_signature_prefix(&signature_clone)
                                    )
                                );
                            } else if
                                // Check for exit transaction
                                let Some(position_index) = self.positions
                                    .iter()
                                    .position(|p| {
                                        p.exit_transaction_signature.as_ref() ==
                                            Some(&signature.to_string())
                                    })
                            {
                                // Temporarily extract position to avoid borrowing conflicts
                                let mut position = self.positions.remove(position_index);
                                self.exit_verification(&mut position, &transaction).await;
                                self.positions.insert(position_index, position);
                                verification_success = true;
                                log(
                                    LogTag::Positions,
                                    "EXIT_VERIFIED",
                                    &format!(
                                        "✅ Exit transaction {} verified using correct verification method",
                                        get_signature_prefix(&signature_clone)
                                    )
                                );
                            } else {
                                log(
                                    LogTag::Positions,
                                    "WARN",
                                    &format!(
                                        "⚠️ No position found for verified transaction: {}",
                                        get_signature_prefix(&signature)
                                    )
                                );
                            }

                            if verification_success {
                                // Save positions to disk after verification update
                                self.save_positions_to_disk().await;
                            }

                            // Remove from pending
                            self.pending_verifications.remove(&signature);
                        }
                        None => {
                            // Transaction still not confirmed, check timeout
                            if let Some(added_at) = self.pending_verifications.get(&signature) {
                                let elapsed_minutes = Utc::now()
                                    .signed_duration_since(*added_at)
                                    .num_minutes();
                                let elapsed_seconds = Utc::now()
                                    .signed_duration_since(*added_at)
                                    .num_seconds();

                                // Add debug log to see what's happening
                                log(
                                    LogTag::Positions,
                                    "DEBUG",
                                    &format!(
                                        "🔍 Transaction {} still pending - elapsed: {}s ({}m)",
                                        get_signature_prefix(&signature),
                                        elapsed_seconds,
                                        elapsed_minutes
                                    )
                                );

                                if elapsed_seconds > 15 {
                                    // 15 seconds timeout - swaps should be fast!
                                    log(
                                        LogTag::Positions,
                                        "TIMEOUT",
                                        &format!(
                                            "⏰ Transaction verification timeout for {}: {}s elapsed ({}m)",
                                            get_signature_prefix(&signature),
                                            elapsed_seconds,
                                            elapsed_minutes
                                        )
                                    );

                                    // Handle timeout - treat as failed for safety
                                    if
                                        let Err(e) = self.handle_transaction_timeout(
                                            &signature
                                        ).await
                                    {
                                        log(
                                            LogTag::Positions,
                                            "ERROR",
                                            &format!(
                                                "Failed to handle transaction timeout {}: {}",
                                                get_signature_prefix(&signature),
                                                e
                                            )
                                        );
                                    }

                                    self.pending_verifications.remove(&signature);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Positions,
                        "FAILED_TX",
                        &format!(
                            "❌ Transaction verification failed for {}: {}",
                            get_signature_prefix(&signature),
                            e
                        )
                    );

                    // Handle failed transaction - remove phantom position if it exists
                    if let Err(cleanup_err) = self.handle_failed_transaction(&signature, &e).await {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!(
                                "Failed to handle failed transaction {}: {}",
                                get_signature_prefix(&signature),
                                cleanup_err
                            )
                        );
                    }

                    // Remove from pending verifications
                    self.pending_verifications.remove(&signature);
                }
            }
        }
    }

    /// Process retry queue for failed sells
    async fn process_retry_queue(&mut self) {
        let mints_to_retry: Vec<String> = self.retry_queue
            .iter()
            .filter(|(_, (retry_time, _))| &Utc::now() >= retry_time)
            .map(|(mint, _)| mint.clone())
            .collect();

        for mint in mints_to_retry {
            if let Some((_, attempt_count)) = self.retry_queue.remove(&mint) {
                log(
                    LogTag::Positions,
                    "RETRY",
                    &format!(
                        "🔄 Retrying failed sell for {} (attempt {})",
                        get_mint_prefix(&mint),
                        attempt_count + 1
                    )
                );

                // A future enhancement could push a command into the actor mailbox to trigger sell
                // We'll handle this in the next iteration
                // For now, just log and re-add with longer delay if needed
                if attempt_count < 5 {
                    self.retry_queue.insert(mint, (
                        Utc::now() + chrono::Duration::minutes(10),
                        attempt_count + 1,
                    ));
                }
            }
        }
    }

    /// Update position data from verified transaction with proper status handling
    async fn update_position_from_transaction(&mut self, signature: &str) -> Result<(), String> {
        // Get transaction from transactions manager
        let transaction = match get_transaction(signature).await? {
            Some(tx) => tx,
            None => {
                return Err("Transaction not found or still pending".to_string());
            }
        };

        // Check transaction status first
        match transaction.status {
            TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                // Transaction is confirmed - proceed with update
            }
            TransactionStatus::Pending => {
                return Err("Transaction still pending verification".to_string());
            }
            TransactionStatus::Failed(ref error) => {
                return Err(format!("Transaction failed: {}", error));
            }
        }

        // Find the position with this signature
        let mut position_updated = false;
        let mut position_symbol = String::new();

        if
            let Some(position) = self.positions
                .iter_mut()
                .find(|p| {
                    p.entry_transaction_signature.as_ref() == Some(&signature.to_string()) ||
                        p.exit_transaction_signature.as_ref() == Some(&signature.to_string())
                })
        {
            // Update position with transaction data
            if position.entry_transaction_signature.as_ref() == Some(&signature.to_string()) {
                position.transaction_entry_verified = transaction.success;
                if !transaction.success {
                    log(
                        LogTag::Positions,
                        "ENTRY_FAILED",
                        &format!(
                            "❌ Entry transaction {} failed for position {}: {}",
                            get_signature_prefix(signature),
                            position.symbol,
                            transaction.error_message.unwrap_or("Unknown error".to_string())
                        )
                    );
                    // Clear failed entry transaction
                    position.entry_transaction_signature = None;
                }
            } else if position.exit_transaction_signature.as_ref() == Some(&signature.to_string()) {
                position.transaction_exit_verified = transaction.success;
                if !transaction.success {
                    log(
                        LogTag::Positions,
                        "EXIT_FAILED",
                        &format!(
                            "❌ Exit transaction {} failed for position {}: {}",
                            get_signature_prefix(signature),
                            position.symbol,
                            transaction.error_message.unwrap_or("Unknown error".to_string())
                        )
                    );
                    // Clear failed exit transaction
                    position.exit_transaction_signature = None;
                }
            }

            position_symbol = position.symbol.clone();
            position_updated = true;
        }

        if position_updated {
            log(
                LogTag::Positions,
                "VERIFIED",
                &format!(
                    "✅ Position updated from verified transaction: {} | {} (success: {})",
                    position_symbol,
                    get_signature_prefix(signature),
                    transaction.success
                )
            );

            // Save positions to disk after verification update
            self.save_positions_to_disk().await;
        }

        Ok(())
    }

    /// Comprehensive transaction verification using RPC and transaction analysis
    /// This replaces the simple is_transaction_verified check with detailed verification
    /// Returns Transaction if confirmed, None if pending, Error if failed
    async fn verify_transaction_comprehensively(
        &self,
        signature: &str
    ) -> Result<Option<Transaction>, String> {
        log(
            LogTag::Positions,
            "VERIFY",
            &format!(
                "🔍 Performing comprehensive verification for transaction {}",
                get_signature_prefix(signature)
            )
        );

        // Use the centralized transactions system to get the full transaction
        match get_transaction(signature).await {
            Ok(Some(transaction)) => {
                // Check transaction status and success
                match transaction.status {
                    TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                        if transaction.success {
                            log(
                                LogTag::Positions,
                                "VERIFY_SUCCESS",
                                &format!(
                                    "✅ Transaction {} verified successfully: fee={:.6} SOL, sol_change={:.6} SOL",
                                    get_signature_prefix(signature),
                                    transaction.fee_sol,
                                    transaction.sol_balance_change
                                )
                            );
                            return Ok(Some(transaction));
                        } else {
                            return Err(
                                format!(
                                    "Transaction failed on-chain: {}",
                                    transaction.error_message.unwrap_or("Unknown error".to_string())
                                )
                            );
                        }
                    }
                    TransactionStatus::Pending => {
                        log(
                            LogTag::Positions,
                            "VERIFY_PENDING",
                            &format!(
                                "⏳ Transaction {} still pending verification",
                                get_signature_prefix(signature)
                            )
                        );
                        return Ok(None);
                    }
                    TransactionStatus::Failed(error) => {
                        return Err(format!("Transaction failed: {}", error));
                    }
                }
            }
            Ok(None) => {
                // Transaction not found in our system - check verification age
                let verification_age_minutes = if
                    let Some(added_at) = self.pending_verifications.get(signature)
                {
                    Utc::now().signed_duration_since(*added_at).num_minutes()
                } else {
                    0
                };

                let verification_age_seconds = if
                    let Some(added_at) = self.pending_verifications.get(signature)
                {
                    Utc::now().signed_duration_since(*added_at).num_seconds()
                } else {
                    0
                };

                // Add debug logging to understand what's happening
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🔍 Transaction {} not found in system - age: {}s ({}m)",
                        get_signature_prefix(signature),
                        verification_age_seconds,
                        verification_age_minutes
                    )
                );

                if verification_age_seconds > 15 {
                    // Extended propagation grace: first check RPC signature status before declaring failure
                    // Propagation grace aligned with RPC propagation policy (3 attempts * 5s = 15s)
                    let propagation_grace_secs: i64 = 15; // allow up to 15s for propagation
                    if (verification_age_seconds as i64) <= propagation_grace_secs {
                        log(
                            LogTag::Positions,
                            "VERIFY_PENDING",
                            &format!(
                                "⏳ Transaction {} still within propagation grace ({}s <= {}s)",
                                get_signature_prefix(signature),
                                verification_age_seconds,
                                propagation_grace_secs
                            )
                        );
                        return Ok(None);
                    }

                    // As a final safeguard, attempt a lightweight getSignatureStatuses call
                    match
                        crate::rpc::get_rpc_client().wait_for_signature_propagation(signature).await
                    {
                        Ok(true) => {
                            // Status appeared just now; treat as pending still
                            log(
                                LogTag::Positions,
                                "VERIFY_PENDING",
                                &format!(
                                    "⏳ Transaction {} appeared during final grace poll ({}s)",
                                    get_signature_prefix(signature),
                                    verification_age_seconds
                                )
                            );
                            return Ok(None);
                        }
                        Ok(false) | Err(_) => {
                            return Err(
                                format!(
                                    "Transaction not found in system after {}s ({}m) - likely failed",
                                    verification_age_seconds,
                                    verification_age_minutes
                                )
                            );
                        }
                    }
                } else {
                    // Still within reasonable time window, treat as pending
                    log(
                        LogTag::Positions,
                        "VERIFY_PENDING",
                        &format!(
                            "⏳ Transaction {} not yet processed by system ({}s elapsed, {}m)",
                            get_signature_prefix(signature),
                            verification_age_seconds,
                            verification_age_minutes
                        )
                    );
                    return Ok(None);
                }
            }
            Err(e) => {
                return Err(format!("Error getting transaction: {}", e));
            }
        }
    }

    /// Handle failed transaction by removing phantom positions or updating state
    async fn handle_failed_transaction(
        &mut self,
        signature: &str,
        error: &str
    ) -> Result<(), String> {
        log(
            LogTag::Positions,
            "HANDLE_FAILED",
            &format!(
                "🚨 Handling failed transaction {}: {}",
                get_signature_prefix(signature),
                error
            )
        );

        // Find the position with this signature
        if
            let Some(position) = self.positions
                .iter_mut()
                .find(|p| {
                    p.entry_transaction_signature.as_ref() == Some(&signature.to_string()) ||
                        p.exit_transaction_signature.as_ref() == Some(&signature.to_string())
                })
        {
            if position.entry_transaction_signature.as_ref() == Some(&signature.to_string()) {
                // Entry transaction failed - remove phantom position
                log(
                    LogTag::Positions,
                    "REMOVE_PHANTOM",
                    &format!(
                        "🗑️ Removing phantom position for {} due to failed entry transaction",
                        position.symbol
                    )
                );

                position.phantom_remove = true;
                position.transaction_entry_verified = false;
            } else if position.exit_transaction_signature.as_ref() == Some(&signature.to_string()) {
                // Exit transaction failed - reset exit data and add to retry queue
                log(
                    LogTag::Positions,
                    "RESET_EXIT",
                    &format!(
                        "🔄 Resetting exit data for {} due to failed exit transaction",
                        position.symbol
                    )
                );

                position.exit_transaction_signature = None;
                position.exit_price = None;
                position.exit_time = None;
                position.transaction_exit_verified = false;

                // Add to retry queue
                self.retry_queue.insert(position.mint.clone(), (
                    Utc::now() + chrono::Duration::minutes(5),
                    1,
                ));
            }

            // Save positions to disk after handling failure
            self.save_positions_to_disk().await;
        }

        Ok(())
    }

    /// Check if exit transaction should be reset based on verification deadline and transaction status
    fn should_reset_exit_transaction(&self, signature: &str) -> bool {
        let now = Utc::now();

        // Check if we have a verification deadline for this signature
        if let Some(deadline) = self.verification_deadlines.get(signature) {
            if now < *deadline {
                // Still within verification window - don't reset yet
                return false;
            }
        }

        // Beyond deadline or no deadline set - check if we've already applied this signature
        if self.applied_exit_signatures.contains_key(signature) {
            // Already applied successfully - don't reset
            return false;
        }

        // Safe to reset - either past deadline or no verification pending
        true
    }

    /// Record failed exit attempt for analytics
    fn record_failed_exit_attempt(&mut self, mint: &str, signature: &str) {
        log(
            LogTag::Positions,
            "EXIT_FAILURE",
            &format!(
                "❌ Recording failed exit attempt for {} with signature {}",
                get_mint_prefix(mint),
                get_signature_prefix(signature)
            )
        );

        // Remove from verification deadlines since it's confirmed failed
        self.verification_deadlines.remove(signature);
    }

    /// Set verification deadline for exit transaction
    fn set_exit_verification_deadline(&mut self, signature: &str, deadline_minutes: i64) {
        let deadline = Utc::now() + chrono::Duration::minutes(deadline_minutes);
        self.verification_deadlines.insert(signature.to_string(), deadline);

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "EXIT_DEADLINE_SET",
                &format!(
                    "⏰ Set verification deadline for {} at {}",
                    get_signature_prefix(signature),
                    deadline.format("%H:%M:%S")
                )
            );
        }
    }
    async fn handle_transaction_timeout(&mut self, signature: &str) -> Result<(), String> {
        log(
            LogTag::Positions,
            "HANDLE_TIMEOUT",
            &format!("⏰ Handling transaction timeout for {}", get_signature_prefix(signature))
        );

        // Treat timeout as failure for safety
        self.handle_failed_transaction(signature, "Transaction verification timeout").await
    }

    /// Clean up phantom positions
    async fn cleanup_phantom_positions(&mut self) {
        log(LogTag::Positions, "CLEANUP", "🧹 Checking for phantom positions to cleanup");
        // Criteria for phantom removal:
        // 1. Explicitly flagged with phantom_remove (set when entry tx failed)
        // 2. Entry tx unverified for > 10 minutes AND transaction no longer found in cache
        // 3. Entry tx unverified AND wallet holds zero tokens for mint
        // Positions that meet criteria are removed to prevent inflated exposure accounting

        let now = Utc::now();
        let mut to_remove: Vec<usize> = Vec::new();

        for (idx, position) in self.positions.iter().enumerate() {
            // Skip already closed positions
            if position.exit_transaction_signature.is_some() {
                continue;
            }

            let mut remove = false;

            // Condition 1: Explicit phantom flag
            if position.phantom_remove {
                remove = true;
            }

            // Condition 2: Aged unverified entry tx not found
            if
                !remove &&
                position.entry_transaction_signature.is_some() &&
                !position.transaction_entry_verified
            {
                let age_minutes = now.signed_duration_since(position.entry_time).num_minutes();
                if age_minutes > 10 {
                    // Try quick lookup of transaction; if still missing, mark remove
                    if let Some(sig) = &position.entry_transaction_signature {
                        match crate::transactions::get_transaction(sig).await {
                            Ok(Some(_)) => {/* exists - keep */}
                            _ => {
                                remove = true;
                            }
                        }
                    }
                }
            }

            // Condition 3: No tokens in wallet for this mint (best-effort, ignore errors)
            if !remove && position.token_amount.unwrap_or(0) == 0 {
                // Only check wallet balance if entry still unverified to avoid RPC load
                if !position.transaction_entry_verified {
                    if let Ok(wallet) = crate::utils::get_wallet_address() {
                        if
                            let Ok(balance) = crate::utils::get_token_balance(
                                &wallet,
                                &position.mint
                            ).await
                        {
                            if balance == 0 {
                                remove = true;
                            }
                        }
                    }
                }
            }

            if remove {
                to_remove.push(idx);
            }
        }

        // Remove in reverse order to keep indices valid
        if !to_remove.is_empty() {
            for idx in to_remove.iter().rev() {
                if let Some(removed) = self.positions.get(*idx) {
                    log(
                        LogTag::Positions,
                        "PHANTOM_REMOVE",
                        &format!(
                            "🗑️ Removing phantom position {} ({}) - unverified entry tx {}",
                            removed.symbol,
                            get_mint_prefix(&removed.mint),
                            removed.entry_transaction_signature
                                .as_ref()
                                .map(|s| get_signature_prefix(s))
                                .unwrap_or_else(|| "NONE".to_string())
                        )
                    );
                }
                self.positions.remove(*idx);
            }
            // Persist updated positions
            self.save_positions_to_disk().await;
        }
    }

    /// Verify and resolve position state using transaction history
    async fn verify_and_resolve_position_state(
        &mut self,
        position: &mut Position,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(), String> {
        log(
            LogTag::Positions,
            "VERIFY",
            &format!(
                "🔍 Verifying position state for {} using transaction history",
                position.symbol
            )
        );

        // This would use the transactions manager to check for any untracked sell transactions
        // For now, return an error to indicate phantom position
        Err("Phantom position detected - requires manual investigation".to_string())
    }

    /// Apply entry verification data to position using analyze-swaps-exact logic
    async fn entry_verification(
        &mut self,
        position: &mut crate::positions::Position,
        transaction: &Transaction
    ) {
        // Check if transaction was successful
        if !transaction.success {
            position.transaction_entry_verified = false;
            log(
                LogTag::Positions,
                "POSITION_ENTRY_FAILED",
                &format!(
                    "❌ Entry transaction {} failed for position {}: marking as failed verification - PENDING TRANSACTION SHOULD BE REMOVED",
                    &transaction.signature[..8],
                    position.symbol
                )
            );
            return;
        }

        log(
            LogTag::Positions,
            "POSITION_ENTRY_PROCESSING",
            &format!(
                "🔄 Processing successful entry transaction {} for position {} - converting to swap PnL info",
                &transaction.signature[..8],
                position.symbol
            )
        );

        // Use convert_to_swap_pnl_info for the exact same calculation as analyze swaps display
        let empty_cache = std::collections::HashMap::new();
        if
            let Some(swap_pnl_info) = self.convert_to_swap_pnl_info(
                transaction,
                &empty_cache,
                false
            ).await
        {
            log(
                LogTag::Positions,
                "POSITION_ENTRY_SWAP_INFO",
                &format!(
                    "📊 Entry swap info for {}: type={}, token_mint={}, sol_amount={}, token_amount={}, price={:.9}",
                    position.symbol,
                    swap_pnl_info.swap_type,
                    &swap_pnl_info.token_mint[..8],
                    swap_pnl_info.sol_amount,
                    swap_pnl_info.token_amount,
                    swap_pnl_info.calculated_price_sol
                )
            );

            if swap_pnl_info.swap_type == "Buy" && swap_pnl_info.token_mint == position.mint {
                // Update position with analyze-swaps-exact calculations using effective pricing
                position.transaction_entry_verified = true;

                // Calculate effective entry price using effective SOL spent (excludes ATA rent)
                let effective_price = if
                    swap_pnl_info.token_amount.abs() > 0.0 &&
                    swap_pnl_info.effective_sol_spent > 0.0
                {
                    swap_pnl_info.effective_sol_spent / swap_pnl_info.token_amount.abs()
                } else {
                    swap_pnl_info.calculated_price_sol // Fallback to regular price
                };

                position.effective_entry_price = Some(effective_price);
                position.total_size_sol = swap_pnl_info.sol_amount;

                // Convert token amount from float to units (with decimals)
                if
                    let Some(token_decimals) = crate::tokens::get_token_decimals(
                        &position.mint
                    ).await
                {
                    let token_amount_units = (swap_pnl_info.token_amount.abs() *
                        (10_f64).powi(token_decimals as i32)) as u64;
                    position.token_amount = Some(token_amount_units);

                    log(
                        LogTag::Positions,
                        "POSITION_ENTRY_TOKEN_AMOUNT",
                        &format!(
                            "🔢 Converted token amount for {}: {} tokens ({} units with {} decimals)",
                            position.symbol,
                            swap_pnl_info.token_amount,
                            token_amount_units,
                            token_decimals
                        )
                    );
                }

                // Convert fee from SOL to lamports
                position.entry_fee_lamports = Some(sol_to_lamports(swap_pnl_info.fee_sol));

                log(
                    LogTag::Positions,
                    "POSITION_ENTRY_VERIFIED",
                    &format!(
                        "✅ ENTRY TRANSACTION VERIFIED: Position {} marked as verified, price={:.9} SOL, PENDING TRANSACTION CLEARED",
                        position.symbol,
                        swap_pnl_info.calculated_price_sol
                    )
                );

                // Log entry verification completion (no longer using cleanup)
                if let Some(ref entry_sig) = position.entry_transaction_signature {
                    log(
                        LogTag::Positions,
                        "POSITION_ENTRY_VERIFIED",
                        &format!(
                            "✅ Entry transaction {} verified for position {}",
                            get_signature_prefix(entry_sig),
                            position.symbol
                        )
                    );
                }
            } else {
                position.transaction_entry_verified = false;
                log(
                    LogTag::Positions,
                    "POSITION_ENTRY_MISMATCH",
                    &format!(
                        "⚠️ Entry transaction {} type/token mismatch for position {}: expected Buy {}, got {} {} - PENDING TRANSACTION SHOULD BE REMOVED",
                        get_signature_prefix(&transaction.signature),
                        position.symbol,
                        get_mint_prefix(&position.mint),
                        swap_pnl_info.swap_type,
                        get_mint_prefix(&swap_pnl_info.token_mint)
                    )
                );
            }
        } else {
            // Transaction manager not available - this should not happen with proper initialization order
            log(
                LogTag::Positions,
                "POSITION_ENTRY_NO_SWAP",
                &format!(
                    "⚠️ Entry transaction {} has no valid swap analysis for position {} - TransactionsManager may not be ready",
                    &transaction.signature[..8],
                    position.symbol
                )
            );
            // Don't mark as failed - let it retry on next verification tick
        }
    }

    /// Apply exit verification data to position using analyze-swaps-exact logic
    async fn exit_verification(
        &mut self,
        position: &mut crate::positions::Position,
        transaction: &Transaction
    ) {
        // Check if transaction was successful
        if !transaction.success {
            position.transaction_exit_verified = false;
            log(
                LogTag::Positions,
                "POSITION_EXIT_FAILED",
                &format!(
                    "❌ Exit transaction {} failed for position {}: marking as failed verification",
                    &transaction.signature[..8],
                    position.symbol
                )
            );
            return;
        }

        // Use convert_to_swap_pnl_info for the exact same calculation as analyze swaps display
        let empty_cache = std::collections::HashMap::new();
        if
            let Some(swap_pnl_info) = self.convert_to_swap_pnl_info(
                transaction,
                &empty_cache,
                false
            ).await
        {
            if swap_pnl_info.swap_type == "Sell" && swap_pnl_info.token_mint == position.mint {
                // Update position with analyze-swaps-exact calculations using effective pricing
                position.transaction_exit_verified = true;

                // Calculate effective exit price using effective SOL received (excludes ATA rent)
                let effective_price = if
                    swap_pnl_info.token_amount.abs() > 0.0 &&
                    swap_pnl_info.effective_sol_received > 0.0
                {
                    swap_pnl_info.effective_sol_received / swap_pnl_info.token_amount.abs()
                } else {
                    swap_pnl_info.calculated_price_sol // Fallback to regular price
                };

                position.effective_exit_price = Some(effective_price);
                position.sol_received = Some(swap_pnl_info.sol_amount);

                // Update exit price if not set
                if position.exit_price.is_none() {
                    position.exit_price = Some(swap_pnl_info.calculated_price_sol);
                }

                // Convert fee from SOL to lamports
                position.exit_fee_lamports = Some(sol_to_lamports(swap_pnl_info.fee_sol));

                // Set exit time if not set
                if position.exit_time.is_none() {
                    position.exit_time = Some(swap_pnl_info.timestamp);
                }

                log(
                    LogTag::Positions,
                    "POSITION_EXIT_UPDATED",
                    &format!(
                        "📝 Updated exit data for position {}: verified=true, price={:.9} SOL (analyze-swaps-exact)",
                        position.symbol,
                        swap_pnl_info.calculated_price_sol
                    )
                );
            } else {
                position.transaction_exit_verified = false;
                log(
                    LogTag::Positions,
                    "POSITION_EXIT_MISMATCH",
                    &format!(
                        "⚠️ Exit transaction {} type/token mismatch for position {}: expected Sell {}, got {} {}",
                        &transaction.signature[..8],
                        position.symbol,
                        position.mint,
                        swap_pnl_info.swap_type,
                        swap_pnl_info.token_mint
                    )
                );
            }
        } else {
            position.transaction_exit_verified = false;
            log(
                LogTag::Positions,
                "POSITION_EXIT_NO_SWAP",
                &format!(
                    "⚠️ Exit transaction {} has no valid swap analysis for position {}",
                    &transaction.signature[..8],
                    position.symbol
                )
            );
        }
    }

    /// Get swap PnL info using the global TransactionsManager's convert_to_swap_pnl_info method
    pub async fn convert_to_swap_pnl_info(
        &self,
        transaction: &Transaction,
        token_symbol_cache: &std::collections::HashMap<String, String>,
        silent: bool
    ) -> Option<crate::transactions::SwapPnLInfo> {
        // Access the global transaction manager with timeout to prevent deadlocks
        use crate::transactions::GLOBAL_TRANSACTION_MANAGER;

        if !silent {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "🔍 Attempting to access global TransactionsManager for tx {}",
                    &transaction.signature[..8]
                )
            );
        }

        // Use timeout to prevent indefinite blocking
        let lock_result = tokio::time::timeout(
            Duration::from_secs(5),
            GLOBAL_TRANSACTION_MANAGER.lock()
        ).await;

        let manager_guard = match lock_result {
            Ok(guard) => guard,
            Err(_) => {
                if !silent {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        "🔒 Timeout acquiring GLOBAL_TRANSACTION_MANAGER lock"
                    );
                }
                return None;
            }
        };

        if let Some(ref manager) = *manager_guard {
            if !silent {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    "✅ Global TransactionsManager found, calling convert_to_swap_pnl_info"
                );
            }
            manager.convert_to_swap_pnl_info(transaction, token_symbol_cache, silent)
        } else {
            if !silent {
                log(
                    LogTag::Positions,
                    "ERROR",
                    "❌ Global TransactionsManager not initialized - verification cannot proceed"
                );
            }
            None
        }
    }

    /// Load positions from disk on startup
    async fn load_positions_from_disk(&mut self) {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("📂 Loading positions from disk: {}", POSITIONS_FILE)
            );
        }

        match tokio::fs::read_to_string(POSITIONS_FILE).await {
            Ok(content) => {
                match serde_json::from_str::<Vec<Position>>(&content) {
                    Ok(positions) => {
                        self.positions = positions;
                        log(
                            LogTag::Positions,
                            "INFO",
                            &format!(
                                "📁 Loaded {} positions from disk ({})",
                                self.positions.len(),
                                POSITIONS_FILE
                            )
                        );

                        if is_debug_positions_enabled() {
                            let open_count = self.get_open_positions_count();
                            let closed_count = self.positions.len() - open_count;
                            log(
                                LogTag::Positions,
                                "DEBUG",
                                &format!(
                                    "📊 Position breakdown - Open: {}, Closed: {}",
                                    open_count,
                                    closed_count
                                )
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!("Failed to parse positions file {}: {}", POSITIONS_FILE, e)
                        );
                        self.positions = Vec::new();
                    }
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    log(
                        LogTag::Positions,
                        "INFO",
                        &format!("📁 No existing positions file found ({}), starting with empty positions", POSITIONS_FILE)
                    );
                } else {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!("Failed to read positions file {}: {}", POSITIONS_FILE, e)
                    );
                }
                self.positions = Vec::new();
            }
        }
    }

    /// Re-queue unverified transactions for comprehensive verification on startup
    /// This ensures that positions with unverified transactions get re-verified
    fn requeue_unverified_transactions(&mut self) {
        let mut requeued_count = 0;

        for position in &self.positions {
            // Check entry transactions that need verification
            if let Some(entry_sig) = &position.entry_transaction_signature {
                if !position.transaction_entry_verified {
                    self.pending_verifications.insert(entry_sig.clone(), Utc::now());
                    requeued_count += 1;

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "🔄 Re-queued entry transaction {} for verification ({})",
                                get_signature_prefix(entry_sig),
                                position.symbol
                            )
                        );
                    }
                }
            }

            // Check exit transactions that need verification
            if let Some(exit_sig) = &position.exit_transaction_signature {
                if !position.transaction_exit_verified {
                    self.pending_verifications.insert(exit_sig.clone(), Utc::now());
                    requeued_count += 1;

                    if is_debug_positions_enabled() {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "🔄 Re-queued exit transaction {} for verification ({})",
                                get_signature_prefix(exit_sig),
                                position.symbol
                            )
                        );
                    }
                }
            }
        }

        if requeued_count > 0 {
            log(
                LogTag::Positions,
                "REQUEUE",
                &format!("🔄 Re-queued {} unverified transactions for comprehensive verification", requeued_count)
            );
        }
    }

    /// Save positions to disk after changes
    async fn save_positions_to_disk(&mut self) {
        // First, remove phantom positions marked for deletion
        let initial_count = self.positions.len();
        self.positions.retain(|p| !p.phantom_remove);
        let final_count = self.positions.len();

        if initial_count != final_count {
            log(
                LogTag::Positions,
                "CLEANUP",
                &format!("🗑️ Removed {} phantom positions during save", initial_count - final_count)
            );
        }

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("💾 Saving {} positions to disk: {}", final_count, POSITIONS_FILE)
            );
        }

        // Ensure data directory exists
        if let Err(e) = ensure_data_directories() {
            log(LogTag::Positions, "ERROR", &format!("Failed to create data directories: {}", e));
            return;
        }

        match serde_json::to_string_pretty(&self.positions) {
            Ok(json_content) => {
                match tokio::fs::write(POSITIONS_FILE, json_content).await {
                    Ok(_) => {
                        log(
                            LogTag::Positions,
                            "DEBUG",
                            &format!(
                                "💾 Saved {} positions to disk ({})",
                                self.positions.len(),
                                POSITIONS_FILE
                            )
                        );
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!("Failed to write positions file {}: {}", POSITIONS_FILE, e)
                        );
                    }
                }
            }
            Err(e) => {
                log(LogTag::Positions, "ERROR", &format!("Failed to serialize positions: {}", e));
            }
        }
    }
}

// =============================================================================
// ACTOR INTERFACE (Requests + Handle) and Service Startup
// =============================================================================

#[allow(clippy::large_enum_variant)]
pub enum PositionsRequest {
    OpenPosition {
        token: Token,
        price: f64,
        percent_change: f64,
        size_sol: f64,
        reply: oneshot::Sender<Result<(String, String), String>>,
    },
    ClosePosition {
        mint: String,
        token: Token,
        exit_price: f64,
        exit_time: DateTime<Utc>,
        reply: oneshot::Sender<Result<(String, String), String>>,
    },
    AddVerification {
        signature: String,
    },
    AddRetryFailedSell {
        mint: String,
    },
    UpdateTracking {
        mint: String,
        current_price: f64,
        reply: oneshot::Sender<bool>,
    },
    GetOpenPositionsCount {
        reply: oneshot::Sender<usize>,
    },
    GetOpenPositions {
        reply: oneshot::Sender<Vec<Position>>,
    },
    GetClosedPositions {
        reply: oneshot::Sender<Vec<Position>>,
    },
    GetOpenMints {
        reply: oneshot::Sender<Vec<String>>,
    },
    IsOpen {
        mint: String,
        reply: oneshot::Sender<bool>,
    },
    GetByState {
        state: PositionState,
        reply: oneshot::Sender<Vec<Position>>,
    },
    RemoveByEntrySignature {
        signature: String,
        reason: String,
        reply: oneshot::Sender<bool>,
    },
    GetActiveFrozenCooldowns {
        reply: oneshot::Sender<Vec<(String, i64)>>,
    },
    ForceReverifyAll {
        reply: oneshot::Sender<usize>,
    },
}

#[derive(Clone)]
pub struct PositionsHandle {
    tx: mpsc::Sender<PositionsRequest>,
}

impl PositionsHandle {
    pub fn new(tx: mpsc::Sender<PositionsRequest>) -> Self {
        Self { tx }
    }

    pub async fn open_position(
        &self,
        token: Token,
        price: f64,
        percent_change: f64,
        size_sol: f64
    ) -> Result<(String, String), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = PositionsRequest::OpenPosition {
            token,
            price,
            percent_change,
            size_sol,
            reply: reply_tx,
        };
        self.tx.send(msg).await.map_err(|_| "PositionsManager unavailable".to_string())?;
        reply_rx.await.map_err(|_| "PositionsManager dropped".to_string())?
    }

    pub async fn close_position(
        &self,
        mint: String,
        token: Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(String, String), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = PositionsRequest::ClosePosition {
            mint,
            token,
            exit_price,
            exit_time,
            reply: reply_tx,
        };
        self.tx.send(msg).await.map_err(|_| "PositionsManager unavailable".to_string())?;
        reply_rx.await.map_err(|_| "PositionsManager dropped".to_string())?
    }

    pub async fn add_verification(&self, signature: String) {
        let _ = self.tx.send(PositionsRequest::AddVerification { signature }).await;
    }

    pub async fn add_retry_failed_sell(&self, mint: String) {
        let _ = self.tx.send(PositionsRequest::AddRetryFailedSell { mint }).await;
    }

    pub async fn update_tracking(&self, mint: String, current_price: f64) -> bool {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::UpdateTracking {
            mint,
            current_price,
            reply: txr,
        }).await;
        rxr.await.unwrap_or(false)
    }

    pub async fn get_open_positions_count(&self) -> usize {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetOpenPositionsCount { reply: txr }).await;
        rxr.await.unwrap_or(0)
    }

    pub async fn get_open_positions(&self) -> Vec<Position> {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetOpenPositions { reply: txr }).await;
        rxr.await.unwrap_or_default()
    }

    pub async fn get_closed_positions(&self) -> Vec<Position> {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetClosedPositions { reply: txr }).await;
        rxr.await.unwrap_or_default()
    }

    pub async fn get_open_mints(&self) -> Vec<String> {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetOpenMints { reply: txr }).await;
        rxr.await.unwrap_or_default()
    }

    pub async fn is_open(&self, mint: String) -> bool {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::IsOpen { mint, reply: txr }).await;
        rxr.await.unwrap_or(false)
    }

    pub async fn remove_by_entry_signature(&self, signature: String, reason: String) -> bool {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::RemoveByEntrySignature {
            signature,
            reason,
            reply: txr,
        }).await;
        rxr.await.unwrap_or(false)
    }

    pub async fn get_active_frozen_cooldowns(&self) -> Vec<(String, i64)> {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetActiveFrozenCooldowns { reply: txr }).await;
        rxr.await.unwrap_or_default()
    }

    pub async fn get_positions_by_state(&self, state: PositionState) -> Vec<Position> {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::GetByState { state, reply: txr }).await;
        rxr.await.unwrap_or_default()
    }

    pub async fn force_reverify_all(&self) -> usize {
        let (txr, rxr) = oneshot::channel();
        let _ = self.tx.send(PositionsRequest::ForceReverifyAll { reply: txr }).await;
        rxr.await.unwrap_or(0)
    }
}

static GLOBAL_POSITIONS_HANDLE: Lazy<AsyncMutex<Option<PositionsHandle>>> = Lazy::new(||
    AsyncMutex::new(None)
);

pub async fn set_positions_handle(handle: PositionsHandle) {
    let mut guard = GLOBAL_POSITIONS_HANDLE.lock().await;
    *guard = Some(handle);
}

pub async fn get_positions_handle() -> Option<PositionsHandle> {
    let guard = GLOBAL_POSITIONS_HANDLE.lock().await;
    guard.clone()
}

/// Start the PositionsManager service (actor) and expose a global handle
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) {
    let (tx, rx) = mpsc::channel::<PositionsRequest>(256);
    let handle = PositionsHandle::new(tx.clone());
    set_positions_handle(handle).await;

    let mut manager = PositionsManager::new(shutdown.clone());
    manager.initialize().await;
    tokio::spawn(async move {
        manager.run_actor(rx).await;
    });

    log(LogTag::Positions, "INFO", "PositionsManager service initialized (actor)");
}

// =============================================================================
// Public async helpers for external modules (thin facade over the global handle)
// =============================================================================

pub async fn get_open_positions() -> Vec<Position> {
    if let Some(h) = get_positions_handle().await {
        h.get_open_positions().await
    } else {
        Vec::new()
    }
}

pub async fn get_closed_positions() -> Vec<Position> {
    if let Some(h) = get_positions_handle().await {
        h.get_closed_positions().await
    } else {
        Vec::new()
    }
}

pub async fn get_open_positions_count() -> usize {
    if let Some(h) = get_positions_handle().await { h.get_open_positions_count().await } else { 0 }
}

pub async fn get_positions_by_state(state: PositionState) -> Vec<Position> {
    if let Some(h) = get_positions_handle().await {
        h.get_positions_by_state(state).await
    } else {
        Vec::new()
    }
}

/// Check if a position is currently open for the given mint
pub async fn is_open_position(mint: &str) -> bool {
    if let Some(h) = get_positions_handle().await {
        h.is_open(mint.to_string()).await
    } else {
        false
    }
}

/// Compatibility function for old SAVED_POSITIONS usage - returns all positions (open + closed)
/// This replaces the old SAVED_POSITIONS.lock() pattern
pub async fn get_all_positions() -> Vec<Position> {
    if let Some(h) = get_positions_handle().await {
        let mut all_positions = h.get_open_positions().await;
        all_positions.extend(h.get_closed_positions().await);
        all_positions
    } else {
        Vec::new()
    }
}

pub async fn get_active_frozen_cooldowns() -> Vec<(String, i64)> {
    if let Some(h) = get_positions_handle().await {
        h.get_active_frozen_cooldowns().await
    } else {
        Vec::new()
    }
}

// Global helper functions for opening and closing positions
pub async fn open_position_global(
    token: Token,
    price: f64,
    percent_change: f64,
    size_sol: f64
) -> Result<(String, String), String> {
    if let Some(h) = get_positions_handle().await {
        h.open_position(token, price, percent_change, size_sol).await
    } else {
        Err("PositionsManager not available".to_string())
    }
}

pub async fn close_position_global(
    mint: String,
    token: Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), String> {
    if let Some(h) = get_positions_handle().await {
        h.close_position(mint, token, exit_price, exit_time).await
    } else {
        Err("PositionsManager not available".to_string())
    }
}

/// Background task execution for close position operations
/// This prevents blocking the PositionsManager actor while processing expensive RPC operations
pub async fn execute_close_position_background(
    mint: String,
    token: Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), String> {
    // Execute the close position logic without blocking the actor
    // We'll use the existing global position management API to update positions

    // Find the position to close by getting all open positions
    let all_positions = get_open_positions().await;
    let position_opt = all_positions
        .iter()
        .find(|pos| {
            let matches_mint = pos.mint == mint;
            let no_exit_sig = pos.exit_transaction_signature.is_none();
            let failed_exit =
                pos.exit_transaction_signature.is_some() && !pos.transaction_exit_verified;
            let can_close = no_exit_sig || failed_exit;

            if crate::arguments::is_debug_positions_enabled() {
                log(
                    LogTag::Positions,
                    "DEBUG",
                    &format!(
                        "🎯 Position check: mint_match={}, no_exit_sig={}, failed_exit={}, can_close={}",
                        matches_mint,
                        no_exit_sig,
                        failed_exit,
                        can_close
                    )
                );
            }

            matches_mint && can_close
        })
        .cloned();

    let mut position = match position_opt {
        Some(pos) => {
            // Handle retry case
            if pos.exit_transaction_signature.is_some() && !pos.transaction_exit_verified {
                log(
                    LogTag::Positions,
                    "RETRY_EXIT",
                    &format!(
                        "🔄 Previous exit transaction failed for {} - will clear failed exit data",
                        pos.symbol
                    )
                );
                // Note: We can't modify the position directly here since we're not in the actor
                // The retry logic will be handled in the swap execution
            }
            pos
        }
        None => {
            return Err("Position not found or already closed".to_string());
        }
    };

    // DRY-RUN MODE CHECK
    if crate::arguments::is_dry_run_enabled() {
        log(
            LogTag::Positions,
            "DRY-RUN",
            &format!(
                "🚫 DRY-RUN: Would close position for {} at {:.6} SOL",
                position.symbol,
                exit_price
            )
        );
        return Err("DRY-RUN: Position would be closed".to_string());
    }

    // Check wallet balance
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            return Err(format!("Failed to get wallet address: {}", e));
        }
    };

    let wallet_balance = match
        crate::utils::get_token_balance(&wallet_address, &position.mint).await
    {
        Ok(balance) => balance,
        Err(e) => {
            return Err(format!("Failed to get token balance: {}", e));
        }
    };

    if wallet_balance == 0 {
        // Handle phantom position - we'll need to signal this through the normal position update mechanism
        log(
            LogTag::Positions,
            "PHANTOM",
            &format!(
                "👻 Phantom position detected for {} - marking as closed with 0 SOL received",
                position.symbol
            )
        );

        // This will need to be handled by the verification system
        return Err("Phantom position detected - needs verification handling".to_string());
    }

    // Execute sell transaction with retry logic
    execute_sell_with_retry_background(&mut position, &token, exit_price, exit_time).await
}

/// Execute sell with retry logic in background task
async fn execute_sell_with_retry_background(
    position: &mut Position,
    token: &Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), String> {
    let _guard = crate::trader::CriticalOperationGuard::new(&format!("SELL {}", position.symbol));

    let max_attempts = crate::arguments::get_max_exit_retries();
    for attempt in 1..=max_attempts {
        log(
            LogTag::Positions,
            "SELL_ATTEMPT",
            &format!(
                "💰 Attempting to sell {} (attempt {}/{}) at {:.6} SOL",
                position.symbol,
                attempt,
                max_attempts,
                exit_price
            )
        );

        // Validate expected SOL output if provided
        if let Some(expected_sol) = Some(exit_price) {
            if expected_sol <= 0.0 || !expected_sol.is_finite() {
                return Err(format!("Invalid expected SOL output: {:.10}", expected_sol));
            }
        }

        // Auto-retry with progressive slippage from config
        let slippages = &SELL_RETRY_SLIPPAGES;
        let token_amount = position.token_amount.unwrap_or(0);

        let mut last_error = None;

        for (slippage_attempt, &slippage) in slippages.iter().enumerate() {
            log(
                LogTag::Swap,
                "SELL_ATTEMPT",
                &format!(
                    "🔴 Sell attempt {} for {} with {:.1}% slippage",
                    slippage_attempt + 1,
                    token.symbol,
                    slippage
                )
            );

            // Execute sell_token_with_slippage logic inline
            if crate::arguments::is_debug_swaps_enabled() {
                log(
                    LogTag::Swap,
                    "SELL_START",
                    &format!(
                        "🔴 Starting SELL operation for {} ({}) - Expected amount: {} tokens, Slippage: {:.1}%",
                        token.symbol,
                        token.mint,
                        token_amount,
                        slippage
                    )
                );
            }

            let wallet_address = match get_wallet_address() {
                Ok(addr) => addr,
                Err(e) => {
                    last_error = Some(format!("Failed to get wallet address: {}", e));
                    continue;
                }
            };

            let actual_wallet_balance = match
                tokio::time::timeout(
                    Duration::from_secs(45),
                    get_token_balance(&wallet_address, &token.mint)
                ).await
            {
                Ok(Ok(balance)) => balance,
                Ok(Err(e)) => {
                    last_error = Some(format!("Failed to get token balance: {}", e));
                    continue;
                }
                Err(_) => {
                    last_error = Some(
                        format!("Timeout getting token balance for {}", token.symbol)
                    );
                    continue;
                }
            };

            if actual_wallet_balance == 0 {
                log(
                    LogTag::Swap,
                    "WARNING",
                    &format!(
                        "⚠️ No {} tokens in wallet to sell (expected: {}, actual: 0)",
                        token.symbol,
                        token_amount
                    )
                );
                return Err(format!("No {} tokens in wallet", token.symbol));
            }

            let actual_sell_amount = actual_wallet_balance;

            log(
                LogTag::Swap,
                "SELL_AMOUNT",
                &format!(
                    "💰 Selling {} {} tokens (position: {}, wallet: {})",
                    actual_sell_amount,
                    token.symbol,
                    token_amount,
                    actual_wallet_balance
                )
            );

            // DUPLICATE SWAP PREVENTION: Check if similar sell was recently attempted
            let expected_sol_amount = exit_price; // Use expected SOL from exit calculation
            if is_duplicate_swap_attempt(&token.mint, expected_sol_amount, "SELL").await {
                last_error = Some(
                    format!(
                        "Duplicate sell prevented for {} - similar sell attempted within last {}s",
                        token.symbol,
                        DUPLICATE_SWAP_PREVENTION_SECS
                    )
                );
                continue;
            }

            let best_quote = match
                get_best_quote(
                    &token.mint,
                    SOL_MINT,
                    actual_sell_amount,
                    &wallet_address,
                    slippage
                ).await
            {
                Ok(quote) => quote,
                Err(e) => {
                    last_error = Some(format!("Failed to get quote: {}", e));
                    continue;
                }
            };

            let swap_result = match
                execute_best_swap(
                    token,
                    &token.mint,
                    SOL_MINT,
                    actual_sell_amount,
                    best_quote
                ).await
            {
                Ok(result) => {
                    if let Some(ref signature) = result.transaction_signature {
                        log(
                            LogTag::Swap,
                            "TRANSACTION",
                            &format!(
                                "Sell transaction {} will be monitored by positions manager",
                                &signature[..8]
                            )
                        );
                    }

                    log(
                        LogTag::Swap,
                        "SELL_SUCCESS",
                        &format!(
                            "✅ Sell successful for {} on attempt {} with {:.1}% slippage",
                            token.symbol,
                            slippage_attempt + 1,
                            slippage
                        )
                    );

                    if crate::arguments::is_debug_swaps_enabled() {
                        log(
                            LogTag::Swap,
                            "SELL_COMPLETE",
                            &format!(
                                "🔴 SELL operation completed for {} - Signature: {:?}",
                                token.symbol,
                                result.transaction_signature
                            )
                        );
                    }

                    result
                }
                Err(e) => {
                    last_error = Some(format!("Swap execution failed: {}", e));
                    continue;
                }
            };

            // Process successful swap result
            let transaction_signature = match swap_result.transaction_signature {
                Some(sig) => sig,
                None => {
                    last_error = Some("Swap result missing signature".to_string());
                    continue;
                }
            };

            // The router type is available from swap_result.router_used
            let quote_label = format!("{:?}", swap_result.router_used);

            log(
                LogTag::Positions,
                "CLOSE_SUCCESS",
                &format!(
                    "✅ Position closed for {} with signature {} ({})",
                    position.symbol,
                    get_signature_prefix(&transaction_signature),
                    quote_label
                )
            );

            return Ok((transaction_signature, quote_label));
        }

        if let Some(error) = last_error {
            log(
                LogTag::Positions,
                "ERROR",
                &format!("❌ Sell attempt {} failed: {}", attempt, error)
            );
        }

        // Small delay between main attempts
        if attempt < max_attempts {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    Err(format!("Failed to sell {} after {} attempts", position.symbol, max_attempts))
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Checks if an error is a frozen account error (error code 0x11)
fn is_frozen_account_error(error_msg: &str) -> bool {
    error_msg.contains("custom program error: 0x11") ||
        error_msg.contains("Account is frozen") ||
        error_msg.contains("Error: Account is frozen")
}

/// Safe 8-char prefix for signatures (avoids direct string indexing)
fn get_signature_prefix(s: &str) -> String {
    s.chars().take(8).collect()
}

/// Safe 8-char prefix for mints (avoids direct string indexing)
fn get_mint_prefix(s: &str) -> String {
    s.chars().take(8).collect()
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

/// Verify a transaction using the positions manager's comprehensive verification system
/// This should be used instead of direct RPC calls to ensure consistent verification logic
/// Returns Transaction if verified and successful, None if pending, Error if failed
pub async fn verify_transaction_with_positions_manager(
    signature: &str
) -> Result<Option<Transaction>, String> {
    log(
        LogTag::Positions,
        "EXTERNAL_VERIFY",
        &format!(
            "🔍 External verification request for transaction {} - using positions manager verification system",
            get_signature_prefix(signature)
        )
    );

    // Use the centralized transactions system directly instead of creating temporary manager
    // This is more efficient and uses the same verification logic
    match get_transaction(signature).await {
        Ok(Some(transaction)) => {
            match transaction.status {
                TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                    if transaction.success {
                        log(
                            LogTag::Positions,
                            "EXTERNAL_VERIFY_SUCCESS",
                            &format!(
                                "✅ External verification successful for transaction {}",
                                get_signature_prefix(signature)
                            )
                        );
                        Ok(Some(transaction))
                    } else {
                        let error = transaction.error_message.unwrap_or(
                            "Transaction failed on-chain".to_string()
                        );
                        log(
                            LogTag::Positions,
                            "EXTERNAL_VERIFY_FAILED",
                            &format!(
                                "❌ External verification failed for transaction {}: {}",
                                get_signature_prefix(signature),
                                error
                            )
                        );
                        Err(error)
                    }
                }
                TransactionStatus::Pending => {
                    log(
                        LogTag::Positions,
                        "EXTERNAL_VERIFY_PENDING",
                        &format!(
                            "⏳ External verification pending for transaction {}",
                            get_signature_prefix(signature)
                        )
                    );
                    Ok(None)
                }
                TransactionStatus::Failed(error) => {
                    log(
                        LogTag::Positions,
                        "EXTERNAL_VERIFY_FAILED",
                        &format!(
                            "❌ External verification failed for transaction {}: {}",
                            get_signature_prefix(signature),
                            error
                        )
                    );
                    Err(error)
                }
            }
        }
        Ok(None) => {
            log(
                LogTag::Positions,
                "EXTERNAL_VERIFY_PENDING",
                &format!(
                    "⏳ External verification pending for transaction {} (not found)",
                    get_signature_prefix(signature)
                )
            );
            Ok(None)
        }
        Err(e) => {
            log(
                LogTag::Positions,
                "EXTERNAL_VERIFY_ERROR",
                &format!(
                    "❌ External verification error for transaction {}: {}",
                    get_signature_prefix(signature),
                    e
                )
            );
            Err(e)
        }
    }
}

/// Check if a transaction is verified using positions manager verification
/// This provides a simple boolean check while using the comprehensive verification system
pub async fn is_transaction_verified_comprehensive(signature: &str) -> bool {
    match verify_transaction_with_positions_manager(signature).await {
        Ok(Some(_)) => true,
        _ => false,
    }
}
