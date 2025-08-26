use crate::global::*;
use crate::logger::{ log, LogTag, log_price_change };
use crate::rpc::{ lamports_to_sol, get_rpc_client, sol_to_lamports };
use crate::positions_db::{
    PositionsDatabase,
    with_positions_database_async,
    initialize_positions_database,
};
use crate::errors::{
    ScreenerBotError,
    blockchain::{ BlockchainError, parse_solana_error },
    PositionError,
    DataError,
    ConfigurationError,
    NetworkError,
};
use crate::swaps::{ get_best_quote, execute_best_swap, RouterType, SwapResult };
use crate::swaps::types::SwapData;
use crate::swaps::config::{ SOL_MINT, QUOTE_SLIPPAGE_PERCENT, SELL_RETRY_SLIPPAGES };
use crate::tokens::Token;
use crate::arguments::is_debug_positions_enabled;
use crate::trader::*;
use crate::transactions::{
    get_transaction,
    is_transaction_verified,
    get_global_swap_transactions,
    Transaction,
    SwapPnLInfo,
    TransactionStatus,
};
use crate::utils::*;

use chrono::{ DateTime, Utc };
use colored::Colorize;
use once_cell::sync::Lazy;
use serde::{ Deserialize, Serialize };
use std::collections::{ HashMap, HashSet };
use std::sync::Arc;
use tokio::sync::{ mpsc, oneshot, Notify, Mutex as AsyncMutex };
use tokio::fs;
use tokio::time::{ interval, Duration, Instant };

/// Unified profit/loss calculation for both open and closed positions
/// Uses effective prices and actual token amounts when available
/// For closed positions with sol_received, uses actual SOL invested vs SOL received
/// NOTE: sol_received should contain ONLY the SOL from token sale, excluding ATA rent reclaim
pub async fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub id: Option<i64>, // Database ID - None for new positions
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
    // Phantom detection tracking (persisted)
    pub phantom_confirmations: u32, // How many times we confirmed zero wallet balance while still open
    pub phantom_first_seen: Option<DateTime<Utc>>, // When first confirmed phantom
    pub synthetic_exit: bool, // True if we synthetically closed due to missing exit tx
    pub closed_reason: Option<String>, // Optional reason for closure (e.g., "synthetic_phantom_closure")
}

// =============================================================================
// SIMPLIFIED POSITION MANAGEMENT
// =============================================================================
//
// Direct async operations with per-position locking for true parallelism
// Each position has individual mutex preventing conflicts while allowing
// concurrent operations on different positions
//
// =============================================================================

// =============================================================================
// POSITION OPERATION CONSTANTS
// =============================================================================

// Phantom detection thresholds (for synthetic closure of sold-but-open positions)
const PHANTOM_CONFIRMATION_THRESHOLD: u32 = 3; // need N reconciliation confirmations
const PHANTOM_MIN_DURATION_SECS: i64 = 30; // minimum seconds since first phantom detection before synthetic close
// Verification safety windows
const ENTRY_VERIFICATION_MAX_SECS: i64 = 90; // hard cap for entry verification age before treating as timeout
const PENDING_VERIFICATION_PROTECT_SECS: i64 = 90; // window during which cleanup defers to pending verification
const AGED_UNVERIFIED_CLEANUP_MINUTES: i64 = 10; // after this, aged unverified entries can be cleaned up

// =============================================================================
// PRICE INFO STRUCTURE FOR COMPREHENSIVE LOGGING
// =============================================================================

/// Additional price information for comprehensive position price logging
#[derive(Debug, Clone)]
pub struct PositionPriceInfo {
    pub price_source: String, // "pool", "api", "cache"
    pub pool_type: Option<String>, // e.g., "RAYDIUM CPMM"
    pub pool_address: Option<String>,
    pub api_price: Option<f64>,
}

impl Default for PositionPriceInfo {
    fn default() -> Self {
        Self {
            price_source: "unknown".to_string(),
            pool_type: None,
            pool_address: None,
            api_price: None,
        }
    }
}

// =============================================================================
// POSITIONS MANAGER - CENTRALIZED POSITION HANDLING
// =============================================================================

/// Simplified position state enum with clear lifecycle tracking
#[derive(Debug, Clone, PartialEq)]
pub enum PositionState {
    Open, // No exit transaction, actively trading
    Closing, // Exit transaction submitted but not yet verified
    Closed, // Exit transaction verified and exit_price set
}

/// PositionsManager handles all position operations with per-position locking for true parallelism
pub struct PositionsManager {
    positions: Arc<AsyncMutex<HashMap<String, Arc<AsyncMutex<Position>>>>>, // mint -> position
    shutdown: Arc<Notify>,
}

/// Constants for cooldowns
const FROZEN_ACCOUNT_COOLDOWN_MINUTES: i64 = 15;
const POSITION_OPEN_COOLDOWN_SECS: i64 = 0;
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 15;

impl PositionsManager {
    /// Create new PositionsManager with simplified structure
    pub fn new(shutdown: Arc<Notify>) -> Self {
        if is_debug_positions_enabled() {
            log(LogTag::Positions, "DEBUG", "🏗️ Creating new PositionsManager instance");
        }

        let manager = Self {
            shutdown,
            positions: Arc::new(AsyncMutex::new(HashMap::new())),
        };

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                "📊 PositionsManager instance created with simplified per-position locking"
            );
        }

        manager
    }

    /// Async initialization after construction
    pub async fn initialize(&mut self) {
        // Initialize database first
        if let Err(e) = initialize_positions_database().await {
            log(
                LogTag::Positions,
                "ERROR",
                &format!("Failed to initialize positions database: {}", e)
            );
            return;
        }

        // Load positions from database on startup
        self.load_positions_from_database().await;

        if is_debug_positions_enabled() {
            let positions_count = self.positions.lock().await.len();
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("📊 PositionsManager initialized with {} positions loaded from disk", positions_count)
            );
        }
    }

    /// Acquire lock for specific position, creating if not exists
    async fn acquire_position_lock(
        &self,
        mint: &str
    ) -> Result<Arc<AsyncMutex<Position>>, ScreenerBotError> {
        let mut positions = self.positions.lock().await;

        if let Some(position_lock) = positions.get(mint) {
            Ok(position_lock.clone())
        } else {
            // Position doesn't exist, this is an error for most operations
            Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: format!("Position not found for mint {}", get_mint_prefix(mint)),
                })
            )
        }
    }

    /// Create new position with lock (for open operations)
    async fn create_position_with_lock(
        &self,
        mint: &str,
        position: Position
    ) -> Arc<AsyncMutex<Position>> {
        let mut positions = self.positions.lock().await;
        let position_lock = Arc::new(AsyncMutex::new(position));
        positions.insert(mint.to_string(), position_lock.clone());
        position_lock
    }

    /// Direct position opening with per-position locking
    pub async fn open_position(
        &self,
        token: &Token,
        price: f64,
        percent_change: f64,
        size_sol: f64
    ) -> Result<(String, String), ScreenerBotError> {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!(
                    "📈 Opening position for {} at price {} ({}% change) with size {} SOL",
                    token.symbol,
                    price,
                    percent_change,
                    size_sol
                )
            );
        }

        // Check if position already exists
        {
            let positions = self.positions.lock().await;
            if positions.contains_key(&token.mint) {
                return Err(
                    ScreenerBotError::Position(PositionError::Generic {
                        message: format!("Position already exists for {}", token.symbol),
                    })
                );
            }
        }

        // Create new position
        let position = Position {
            id: None,
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
            entry_transaction_signature: None,
            exit_transaction_signature: None,
            token_amount: None,
            effective_entry_price: None,
            effective_exit_price: None,
            sol_received: None,
            profit_target_min: None,
            profit_target_max: None,
            liquidity_tier: None,
            transaction_entry_verified: false,
            transaction_exit_verified: false,
            entry_fee_lamports: None,
            exit_fee_lamports: None,
            current_price: Some(price),
            current_price_updated: Some(Utc::now()),
            phantom_remove: false,
            phantom_confirmations: 0,
            phantom_first_seen: None,
            synthetic_exit: false,
            closed_reason: None,
        };

        let position_lock = self.create_position_with_lock(&token.mint, position).await;

        // Lock the position for the operation
        let mut position = position_lock.lock().await;

        // TODO: Implement actual swap execution here
        // For now, return success
        Ok((token.mint.clone(), token.symbol.clone()))
    }

    /// Direct position closing with per-position locking
    pub async fn close_position(
        &self,
        mint: &str,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(String, String), ScreenerBotError> {
        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "DEBUG",
                &format!("📉 Closing position for {} at price {}", token.symbol, exit_price)
            );
        }

        let position_lock = self.acquire_position_lock(mint).await?;
        let mut position = position_lock.lock().await;

        // Update position with exit information
        position.exit_price = Some(exit_price);
        position.exit_time = Some(exit_time);

        // TODO: Implement actual swap execution here
        // For now, return success
        Ok((mint.to_string(), token.symbol.clone()))
    }

    /// Direct position tracking update with per-position locking
    pub async fn update_tracking(&self, mint: &str, current_price: f64) -> bool {
        if let Ok(position_lock) = self.acquire_position_lock(mint).await {
            if let Ok(mut position) = position_lock.try_lock() {
                position.current_price = Some(current_price);
                position.current_price_updated = Some(Utc::now());

                // Update highest/lowest prices
                if current_price > position.price_highest {
                    position.price_highest = current_price;
                }
                if current_price < position.price_lowest {
                    position.price_lowest = current_price;
                }

                return true;
            }
        }
        false
    }

    /// Get open positions count (includes Open and Closing states)
    pub async fn get_open_positions_count(&self) -> usize {
        let positions = self.positions.lock().await;
        let mut count = 0;

        for position_lock in positions.values() {
            if let Ok(position) = position_lock.try_lock() {
                if position.position_type == "buy" {
                    let state = self.get_position_state(&position);
                    if state == PositionState::Open || state == PositionState::Closing {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Get open positions (includes Open and Closing states)
    pub async fn get_open_positions(&self) -> Vec<Position> {
        let positions = self.positions.lock().await;
        let mut open_positions = Vec::new();

        for position_lock in positions.values() {
            if let Ok(position) = position_lock.try_lock() {
                if position.position_type == "buy" {
                    let state = self.get_position_state(&position);
                    if matches!(state, PositionState::Open | PositionState::Closing) {
                        open_positions.push(position.clone());
                    }
                }
            }
        }
        open_positions
    }

    /// Get closed positions (only Closed state)
    pub async fn get_closed_positions(&self) -> Vec<Position> {
        let positions = self.positions.lock().await;
        let mut closed_positions = Vec::new();

        for position_lock in positions.values() {
            if let Ok(position) = position_lock.try_lock() {
                if
                    position.position_type == "buy" &&
                    self.get_position_state(&position) == PositionState::Closed
                {
                    closed_positions.push(position.clone());
                }
            }
        }
        closed_positions
    }

    /// Get open positions mints (includes Open and Closing states)
    pub async fn get_open_positions_mints(&self) -> Vec<String> {
        let positions = self.positions.lock().await;
        let mut open_mints = Vec::new();

        for (mint, position_lock) in positions.iter() {
            if let Ok(position) = position_lock.try_lock() {
                if position.position_type == "buy" {
                    let state = self.get_position_state(&position);
                    if matches!(state, PositionState::Open | PositionState::Closing) {
                        open_mints.push(mint.clone());
                    }
                }
            }
        }
        open_mints
    }

    /// Check if mint is an open position (includes Open and Closing states)
    pub async fn is_open_position(&self, mint: &str) -> bool {
        let positions = self.positions.lock().await;

        if let Some(position_lock) = positions.get(mint) {
            if let Ok(position) = position_lock.try_lock() {
                if position.position_type == "buy" {
                    let state = self.get_position_state(&position);
                    return matches!(state, PositionState::Open | PositionState::Closing);
                }
            }
        }
        false
    }

    /// Get positions by state
    pub async fn get_positions_by_state(&self, target_state: &PositionState) -> Vec<Position> {
        let positions = self.positions.lock().await;
        let mut filtered_positions = Vec::new();

        for position_lock in positions.values() {
            if let Ok(position) = position_lock.try_lock() {
                if
                    position.position_type == "buy" &&
                    self.get_position_state(&position) == *target_state
                {
                    filtered_positions.push(position.clone());
                }
            }
        }
        filtered_positions
    }

    /// Get position state with simplified detection
    pub fn get_position_state(&self, position: &Position) -> PositionState {
        // Fully closed: entry verified, exit verified, and has exit price
        if
            position.transaction_entry_verified &&
            position.transaction_exit_verified &&
            position.exit_price.is_some()
        {
            return PositionState::Closed;
        }

        // Exit transaction submitted but not verified yet
        if position.exit_transaction_signature.is_some() {
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

        let mut phantom_updates: Vec<usize> = Vec::new();
        let mut phantom_ready_to_resolve: Vec<usize> = Vec::new();
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
                        // Defer updates until after loop to avoid mutable borrow conflicts
                        phantom_updates.push(index);

                        log(
                            LogTag::Positions,
                            "RECONCILE",
                            &format!(
                                "👻 Confirmed phantom position {} (confirmations={}, first_seen={:?}) - searching for missing exit transaction",
                                symbol,
                                position.phantom_confirmations,
                                position.phantom_first_seen
                            )
                        );

                        // Search for missing exit transaction
                        if
                            let Some(exit_signature) =
                                self.find_missing_exit_transaction_targeted(position).await
                        {
                            if !self.applied_exit_signatures.contains_key(&exit_signature) {
                                positions_to_heal.push((index, exit_signature));
                            }
                        } else {
                            // If we didn't find an exit and thresholds reached, attempt synthetic closure via resolver
                            // Check if after increment it would meet thresholds (approximate since increment deferred)
                            let projected_confirmations = position.phantom_confirmations + 1; // since we'll increment later
                            let first_seen = position.phantom_first_seen.unwrap_or(Utc::now());
                            let duration_secs = Utc::now()
                                .signed_duration_since(first_seen)
                                .num_seconds();
                            if
                                projected_confirmations >= PHANTOM_CONFIRMATION_THRESHOLD &&
                                duration_secs >= PHANTOM_MIN_DURATION_SECS
                            {
                                phantom_ready_to_resolve.push(index);
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

        // Apply phantom updates
        let now_ts = Utc::now();
        let mut phantom_updates_made = false;
        for idx in phantom_updates {
            if let Some(p) = self.positions.get_mut(idx) {
                if p.phantom_first_seen.is_none() {
                    p.phantom_first_seen = Some(now_ts);
                }
                p.phantom_confirmations = p.phantom_confirmations.saturating_add(1);
                phantom_updates_made = true;
            }
        }

        // Resolve those already qualifying (avoid simultaneous mutable borrows by collecting first)
        let mut phantom_resolutions_made = false;
        for idx in phantom_ready_to_resolve {
            if let Some(p_mut) = self.positions.get_mut(idx) {
                // Double-check eligibility
                let duration_ok = p_mut.phantom_first_seen
                    .map(
                        |t|
                            Utc::now().signed_duration_since(t).num_seconds() >=
                            PHANTOM_MIN_DURATION_SECS
                    )
                    .unwrap_or(false);
                let confirmations_ok =
                    p_mut.phantom_confirmations >= PHANTOM_CONFIRMATION_THRESHOLD;
                if duration_ok && confirmations_ok && !p_mut.synthetic_exit {
                    let synthetic_price = p_mut.current_price.unwrap_or(p_mut.entry_price);
                    p_mut.exit_price = Some(synthetic_price);
                    p_mut.exit_time = Some(Utc::now());
                    p_mut.transaction_exit_verified = false;
                    p_mut.effective_exit_price = None;
                    p_mut.sol_received = None;
                    p_mut.synthetic_exit = true;
                    p_mut.closed_reason = Some("synthetic_phantom_closure".to_string());
                    phantom_resolutions_made = true;
                    log(
                        LogTag::Positions,
                        "PHANTOM_SYNTHETIC_CLOSED",
                        &format!(
                            "🧵 Synthetic closure applied to phantom {} at {:.9} (confirmations={}, duration_ok={}, confirmations_ok={})",
                            p_mut.symbol,
                            synthetic_price,
                            p_mut.phantom_confirmations,
                            duration_ok,
                            confirmations_ok
                        )
                    );
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

                    // Check if position is now fully closed and clean up watch list
                    if
                        position.transaction_entry_verified &&
                        position.transaction_exit_verified &&
                        position.exit_price.is_some()
                    {
                        log(
                            LogTag::Positions,
                            "RECONCILE_FULLY_CLOSED",
                            &format!(
                                "✅ Healed position {} is fully closed - removing from price watch list",
                                position.symbol
                            )
                        );
                    }

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
            self.cleanup_phantom_positions_database().await;
        } else if phantom_updates_made || phantom_resolutions_made {
            log(
                LogTag::Positions,
                "RECONCILE_COMPLETE",
                "🎯 Targeted reconciliation completed - phantom position updates saved"
            );
            self.cleanup_phantom_positions_database().await;
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
        let mut manager_guard = match
            tokio::time::timeout(Duration::from_secs(2), GLOBAL_TRANSACTION_MANAGER.lock()).await
        {
            Ok(guard) => guard,
            Err(_) => {
                log(
                    LogTag::Positions,
                    "WARN",
                    "⏰ Timeout acquiring GLOBAL_TRANSACTION_MANAGER lock for missing exit transaction search"
                );
                return None;
            }
        };

        if let Some(ref mut manager) = *manager_guard {
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

    /// Open a new position
    pub async fn open_position(
        &mut self,
        token: &Token,
        price: f64,
        percent_change: f64,
        size_sol: f64
    ) -> Result<(String, String), ScreenerBotError> {
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
            return Err(
                ScreenerBotError::Data(DataError::ValidationError {
                    field: "price".to_string(),
                    value: price.to_string(),
                    reason: "Price must be positive and finite".to_string(),
                })
            );
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
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: "DRY-RUN: Position would be opened".to_string(),
                })
            );
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
                ScreenerBotError::Position(PositionError::Generic {
                    message: format!(
                        "Re-entry cooldown active for {} ({}): wait {}m",
                        token.symbol,
                        get_mint_prefix(&token.mint),
                        remaining
                    ),
                })
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
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: format!("Opening positions cooldown active: wait {}s", remaining),
                })
            );
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
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: "Already have open position for this token".to_string(),
                })
            );
        }

        if open_positions_count >= MAX_OPEN_POSITIONS {
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: format!(
                        "Maximum open positions reached ({}/{})",
                        open_positions_count,
                        MAX_OPEN_POSITIONS
                    ),
                })
            );
        }

        // Execute the buy transaction
        let _guard = crate::trader::CriticalOperationGuard::new(&format!("BUY {}", token.symbol));

        // DUPLICATE SWAP PREVENTION: Check if similar swap was recently attempted
        if is_duplicate_swap_attempt(&token.mint, size_sol, "BUY").await {
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: format!(
                        "Duplicate swap prevented for {} - similar buy attempted within last {}s",
                        token.symbol,
                        DUPLICATE_SWAP_PREVENTION_SECS
                    ),
                })
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
            return Err(
                ScreenerBotError::Data(DataError::ValidationError {
                    field: "expected_price".to_string(),
                    value: format!("{:.10}", price),
                    reason: "Invalid expected price".to_string(),
                })
            );
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

        // ✅ CRITICAL: Add token to watch list before opening position
        // This ensures the token is monitored for price updates during and after the swap
        let price_service_result = match
            tokio::time::timeout(
                tokio::time::Duration::from_secs(10), // 10s timeout for price service
                crate::tokens::get_token_price_safe(&token.mint)
            ).await
        {
            Ok(result) => result,
            Err(_) => {
                log(
                    LogTag::Positions,
                    "TIMEOUT",
                    &format!(
                        "⏰ Price service timeout for {} after 10s - continuing without price check",
                        token.symbol
                    )
                );
                Some(0.0) // Default price, will continue with swap
            }
        };
        let _ = price_service_result;

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "WATCH_LIST",
                &format!("✅ Added {} to price monitoring watch list before swap", token.symbol)
            );
        }

        let wallet_address = match
            tokio::time::timeout(
                tokio::time::Duration::from_secs(5), // 5s timeout for wallet address
                async {
                    get_wallet_address()
                }
            ).await
        {
            Ok(Ok(addr)) => addr,
            Ok(Err(e)) => {
                log(LogTag::Positions, "ERROR", &format!("❌ Failed to get wallet address: {}", e));
                return Err(e);
            }
            Err(_) => {
                log(
                    LogTag::Positions,
                    "TIMEOUT",
                    &format!(
                        "⏰ Wallet address timeout for {} after 5s - critical operation will be released",
                        token.symbol
                    )
                );
                return Err(ScreenerBotError::api_error("Wallet address timeout".to_string()));
            }
        };

        // Add timeout wrapper to prevent hanging in quote requests
        let best_quote = match
            tokio::time::timeout(
                tokio::time::Duration::from_secs(20), // 20s total timeout for quote requests
                get_best_quote(
                    SOL_MINT,
                    &token.mint,
                    sol_to_lamports(size_sol),
                    &wallet_address,
                    QUOTE_SLIPPAGE_PERCENT
                )
            ).await
        {
            Ok(quote_result) => quote_result?,
            Err(_) => {
                log(
                    LogTag::Swap,
                    "QUOTE_TIMEOUT",
                    &format!(
                        "⏰ Quote request timeout for {} after 20s - critical operation will be released",
                        token.symbol
                    )
                );
                return Err(
                    ScreenerBotError::api_error(
                        format!("Quote request timeout for {}", token.symbol)
                    )
                );
            }
        };

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
        ).await?;

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
                    return Err(
                        ScreenerBotError::Data(DataError::ValidationError {
                            field: "transaction_signature".to_string(),
                            value: transaction_signature.clone(),
                            reason: "Transaction signature is invalid or empty".to_string(),
                        })
                    );
                }

                // Additional validation: Check if signature is valid base58
                if bs58::decode(&transaction_signature).into_vec().is_err() {
                    return Err(
                        ScreenerBotError::Data(DataError::ValidationError {
                            field: "transaction_signature".to_string(),
                            value: get_signature_prefix(&transaction_signature),
                            reason: "Invalid base58 format".to_string(),
                        })
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
                    id: None, // Will be set by database after insertion
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
                    phantom_confirmations: 0,
                    phantom_first_seen: None,
                    synthetic_exit: false,
                    closed_reason: None,
                };

                // Save position to database first
                match self.save_position_to_database(&new_position).await {
                    Ok(_) => {
                        // Add position to in-memory list
                        self.positions.push(new_position);

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
                    }
                    Err(e) => {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!("Failed to save position to database: {}", e)
                        );
                        return Err(ScreenerBotError::Position(PositionError::DatabaseError(e)));
                    }
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
    ) -> Result<(String, String), ScreenerBotError> {
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

        // 🔒 ATOMIC LOCK: Acquire exclusive lock for this position to prevent race conditions
        let _position_guard = match acquire_position_lock(mint).await {
            Ok(guard) => guard,
            Err(e) => {
                log(
                    LogTag::Positions,
                    "LOCK_ERROR",
                    &format!("❌ Failed to acquire position lock for {}: {}", token.symbol, e)
                );
                return Err(
                    ScreenerBotError::Position(PositionError::Generic {
                        message: format!("Position is busy: {}", e),
                    })
                );
            }
        };

        // Find the position to close with enhanced state validation
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

                // With atomic locks, we no longer need CLOSING_IN_PROGRESS placeholder
                // The lock itself prevents concurrent access
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
                pos.exit_price = None;
                pos.exit_time = None;
                pos.transaction_exit_verified = false;
                pos.sol_received = None;
                pos.effective_exit_price = None;
                pos.exit_fee_lamports = None;
                pos.exit_transaction_signature = None; // Clear failed signature
            }
            position_opt = Some(pos.clone());
        }

        let mut position = match position_opt {
            Some(pos) => pos,
            None => {
                return Err(
                    ScreenerBotError::Position(PositionError::PositionNotFound {
                        token_mint: mint.to_string(),
                        signature: "".to_string(),
                    })
                );
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
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: "DRY-RUN: Position would be closed".to_string(),
                })
            );
        }

        // Execute sell transaction with retry logic (balance check happens in retry function)
        self.execute_sell_with_retry(&mut position, token, exit_price, exit_time).await
    }

    async fn execute_sell_with_retry(
        &mut self,
        position: &mut Position,
        token: &Token,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(String, String), ScreenerBotError> {
        let _guard = crate::trader::CriticalOperationGuard::new(
            &format!("SELL {}", position.symbol)
        );

        // ✅ ENSURE token remains in watch list during sell process
        let price_service_result = match
            tokio::time::timeout(
                tokio::time::Duration::from_secs(10), // 10s timeout for price service
                crate::tokens::get_token_price_safe(&token.mint)
            ).await
        {
            Ok(result) => result,
            Err(_) => {
                log(
                    LogTag::Positions,
                    "TIMEOUT",
                    &format!(
                        "⏰ Price service timeout for {} during sell after 10s - continuing without price check",
                        token.symbol
                    )
                );
                Some(0.0) // Default price, will continue with swap
            }
        };
        let _ = price_service_result;

        if is_debug_positions_enabled() {
            log(
                LogTag::Positions,
                "WATCH_LIST",
                &format!("✅ Refreshed {} in watch list before sell execution", token.symbol)
            );
        }

        // Active sell registry guard
        if !register_active_sell(&position.mint).await {
            log(
                LogTag::Swap,
                "ACTIVE_SELL_SKIP",
                &format!(
                    "⏳ Skipping sell for {} - another sell already in progress for mint {}",
                    position.symbol,
                    get_mint_prefix(&position.mint)
                )
            );
            return Err(
                ScreenerBotError::Position(PositionError::Generic {
                    message: "Active sell already in progress".to_string(),
                })
            );
        }
        // Ensure cleanup at end of function scope
        struct ActiveSellCleanup {
            mint: String,
        }
        impl Drop for ActiveSellCleanup {
            fn drop(&mut self) {
                let mint = self.mint.clone();
                tokio::spawn(async move {
                    clear_active_sell(&mint).await;
                });
            }
        }
        let _active_cleanup = ActiveSellCleanup { mint: position.mint.clone() };

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
                    return Err(
                        ScreenerBotError::Data(DataError::ValidationError {
                            field: "expected_sol_output".to_string(),
                            value: format!("{:.10}", expected_sol),
                            reason: "Invalid expected SOL output".to_string(),
                        })
                    );
                }
            }

            // Auto-retry with progressive slippage from config
            let slippages = &SELL_RETRY_SLIPPAGES;
            let shutdown = Some(self.shutdown.clone());
            let token_amount = position.token_amount.unwrap_or(0);

            let mut last_error: Option<ScreenerBotError> = None;

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
                        return Err(
                            ScreenerBotError::Position(PositionError::Generic {
                                message: "Shutdown in progress - aborting sell".to_string(),
                            })
                        );
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

                // Get wallet balance (actual amount to sell)
                let wallet_address = match
                    tokio::time::timeout(
                        tokio::time::Duration::from_secs(5), // 5s timeout for wallet address
                        async {
                            get_wallet_address()
                        }
                    ).await
                {
                    Ok(Ok(addr)) => addr,
                    Ok(Err(e)) => {
                        last_error = Some(e);
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Positions,
                            "TIMEOUT",
                            &format!(
                                "⏰ Wallet address timeout during sell for {} after 5s",
                                token.symbol
                            )
                        );
                        last_error = Some(
                            ScreenerBotError::api_error(
                                "Wallet address timeout during sell".to_string()
                            )
                        );
                        continue;
                    }
                };

                let actual_sell_amount = match
                    tokio::time::timeout(
                        Duration::from_secs(45),
                        get_cached_token_balance(&wallet_address, &token.mint)
                    ).await
                {
                    Ok(Ok(balance)) => balance,
                    Ok(Err(e)) => {
                        last_error = Some(e);
                        continue;
                    }
                    Err(_) => {
                        last_error = Some(
                            ScreenerBotError::Network(NetworkError::Generic {
                                message: format!(
                                    "Timeout getting token balance for {}",
                                    token.symbol
                                ),
                            })
                        );
                        continue;
                    }
                };

                // Note: Zero balance check removed - phantom positions handled by verification system

                log(
                    LogTag::Swap,
                    "SELL_AMOUNT",
                    &format!(
                        "💰 Selling {} {} tokens (position: {}, wallet: {})",
                        actual_sell_amount,
                        token.symbol,
                        token_amount,
                        actual_sell_amount
                    )
                );

                // DUPLICATE SWAP PREVENTION (improved):
                // Previous logic blocked ALL attempts inside the slippage loop even when NO prior sell executed.
                // That resulted in perpetual duplicate prevention + repeated balance RPC calls while tokens were still present.
                // We now only apply duplicate prevention if wallet balance is LOWER than the recorded position amount
                // (indicating a prior partial/complete execution) OR if wallet balance is zero (already sold externally).
                let expected_sol_amount = exit_price; // Use expected SOL from exit calculation
                let full_position_intact = actual_sell_amount == token_amount;
                if !full_position_intact {
                    if is_duplicate_swap_attempt(&token.mint, expected_sol_amount, "SELL").await {
                        last_error = Some(
                            ScreenerBotError::Position(PositionError::Generic {
                                message: format!(
                                    "Duplicate sell prevented for {} - similar sell attempted within last {}s (wallet balance changed)",
                                    token.symbol,
                                    DUPLICATE_SWAP_PREVENTION_SECS
                                ),
                            })
                        );
                        continue;
                    }
                } else if crate::arguments::is_debug_swaps_enabled() {
                    log(
                        LogTag::Swap,
                        "DUPLICATE_SKIP",
                        &format!(
                            "🔄 Duplicate prevention skipped for {} (full balance intact: {} tokens)",
                            token.symbol,
                            actual_sell_amount
                        )
                    );
                }

                let best_quote = match
                    tokio::time::timeout(
                        tokio::time::Duration::from_secs(20), // 20s total timeout for quote requests
                        get_best_quote(
                            &token.mint,
                            SOL_MINT,
                            actual_sell_amount,
                            &wallet_address,
                            slippage
                        )
                    ).await
                {
                    Ok(Ok(quote)) => quote,
                    Ok(Err(e)) => {
                        last_error = Some(e);
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Swap,
                            "QUOTE_TIMEOUT",
                            &format!(
                                "⏰ Sell quote request timeout for {} after 20s (slippage: {:.1}%)",
                                token.symbol,
                                slippage
                            )
                        );
                        last_error = Some(
                            ScreenerBotError::api_error(
                                format!("Quote request timeout for {}", token.symbol)
                            )
                        );
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
                        let should_not_retry = match &e {
                            ScreenerBotError::Blockchain(
                                BlockchainError::InsufficientBalance { .. },
                            ) => {
                                log(
                                    LogTag::Swap,
                                    "SELL_FAILED_NO_RETRY",
                                    &format!(
                                        "❌ Stopping retries for {} - insufficient balance (tokens may have been sold in previous attempt)",
                                        token.symbol
                                    )
                                );
                                true
                            }
                            ScreenerBotError::Data(DataError::InvalidAmount { .. }) => {
                                log(
                                    LogTag::Swap,
                                    "SELL_FAILED_NO_RETRY",
                                    &format!(
                                        "❌ Stopping retries for {} - invalid amount error",
                                        token.symbol
                                    )
                                );
                                true
                            }
                            ScreenerBotError::Configuration(_) => {
                                log(
                                    LogTag::Swap,
                                    "SELL_FAILED_NO_RETRY",
                                    &format!(
                                        "❌ Stopping retries for {} - configuration error",
                                        token.symbol
                                    )
                                );
                                true
                            }
                            _ => {
                                // Check legacy string patterns for backward compatibility
                                let error_str = format!("{}", e);
                                error_str.contains("insufficient balance") ||
                                    error_str.contains("InvalidAmount") ||
                                    error_str.contains("ConfigError")
                            }
                        };

                        if should_not_retry {
                            return Err(e);
                        }

                        last_error = Some(e);

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
                                        ScreenerBotError::Position(PositionError::Generic {
                                            message: "Shutdown in progress - aborting sell retries".to_string(),
                                        })
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

                    // Save updated position to database
                    if let Err(e) = self.save_position_to_database(&position).await {
                        log(
                            LogTag::Positions,
                            "ERROR",
                            &format!("Failed to save position {} to database: {}", position.mint, e)
                        );
                    }
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

            let final_error = last_error.unwrap_or_else(||
                ScreenerBotError::Position(PositionError::Generic {
                    message: "Unknown error".to_string(),
                })
            );

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
                    if is_frozen_account_error(&format!("{}", final_error)) {
                        self.add_mint_to_frozen_cooldown(&position.mint);
                        return Err(
                            ScreenerBotError::Position(PositionError::Generic {
                                message: format!("Token frozen, added to cooldown: {}", final_error),
                            })
                        );
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
                        ScreenerBotError::Position(PositionError::Generic {
                            message: format!("All sell attempts failed, added to retry queue: {}", final_error),
                        })
                    );
                }
            }
        }

        Err(
            ScreenerBotError::Position(PositionError::Generic {
                message: "Unexpected end of sell retry loop".to_string(),
            })
        )
    }

    /// Get price information safely for comprehensive logging (non-blocking)
    async fn get_price_info_safe(&self, mint: &str) -> PositionPriceInfo {
        // Use timeout to prevent blocking
        let result = tokio::time::timeout(
            Duration::from_millis(200), // 200ms timeout for non-blocking
            async {
                // Try to get pool price information first
                let pool_service = crate::tokens::pool::get_pool_service();
                if let Some(pool_result) = pool_service.get_pool_price(mint, None).await {
                    return PositionPriceInfo {
                        price_source: "pool".to_string(),
                        pool_type: pool_result.pool_type,
                        pool_address: Some(pool_result.pool_address),
                        api_price: pool_result.price_sol,
                    };
                }

                // Try to get API price from price service
                if let Some(api_price) = crate::tokens::get_token_price_safe(mint).await {
                    return PositionPriceInfo {
                        price_source: "api".to_string(),
                        pool_type: None,
                        pool_address: None,
                        api_price: Some(api_price),
                    };
                }

                PositionPriceInfo::default()
            }
        ).await;

        result.unwrap_or_else(|_| {
            // Timeout occurred, return minimal info
            PositionPriceInfo {
                price_source: "timeout".to_string(),
                pool_type: None,
                pool_address: None,
                api_price: None,
            }
        })
    }

    /// Update position tracking
    async fn update_position_tracking(
        &mut self,
        mint: &str,
        current_price: f64,
        price_info: PositionPriceInfo
    ) -> bool {
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
            let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
            if position.price_highest == 0.0 {
                position.price_highest = entry_price;
                position.price_lowest = entry_price;
            }

            // Check for price change and log it BEFORE updating position
            let old_price = position.current_price.unwrap_or(entry_price);
            let price_change = current_price - old_price;
            let price_change_percent = if old_price != 0.0 {
                (price_change / old_price) * 100.0
            } else {
                0.0
            };

            // Log price change if significant (0.01% threshold for high sensitivity)
            let change_threshold = if old_price > 0.0 {
                (old_price * 0.0001).max(f64::EPSILON * 100.0) // 0.01% minimum
            } else {
                f64::EPSILON * 100.0
            };

            let price_diff = (old_price - current_price).abs();

            // Check if enough time has passed since last log (fallback logging every 30 seconds)
            let time_since_last_log = position.current_price_updated
                .map(|last| (Utc::now() - last).num_seconds())
                .unwrap_or(999); // Force log if no previous update
            let should_log_periodic = time_since_last_log >= 30; // Log every 30 seconds regardless

            if price_diff > change_threshold || should_log_periodic {
                // Calculate current P&L for logging
                let (pnl_sol, pnl_percent) = crate::positions::calculate_position_pnl(
                    position,
                    Some(current_price)
                ).await;

                crate::logger::log_price_change(
                    mint,
                    &position.symbol,
                    old_price,
                    current_price,
                    &price_info.price_source,
                    price_info.pool_type.as_deref(),
                    price_info.pool_address.as_deref(),
                    price_info.api_price,
                    Some((pnl_sol, pnl_percent))
                );
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
            position.current_price_updated = Some(Utc::now());

            // Return whether any tracking data was updated (for potential save to disk)
            true // Always return true since current_price was updated
        } else {
            false
        }
    }

    /// Update position with exit transaction signature (CRITICAL FIX for phantom positions)
    /// This ensures that when a close position operation succeeds, the signature is immediately
    /// saved to prevent phantom position scenarios where the transaction succeeds but isn't tracked
    async fn update_position_exit_signature(
        &mut self,
        mint: &str,
        signature: &str,
        router_used: &str
    ) {
        let now = Utc::now();
        let mut found_position = false;

        // Find and update the position
        {
            if let Some(position) = self.positions.iter_mut().find(|p| p.mint == mint) {
                log(
                    LogTag::Positions,
                    "EXIT_SIGNATURE_UPDATE",
                    &format!(
                        "💾 Updating position {} with exit signature {} ({})",
                        position.symbol,
                        get_signature_prefix(signature),
                        router_used
                    )
                );

                // Set the exit transaction signature
                position.exit_transaction_signature = Some(signature.to_string());
                found_position = true;

                log(
                    LogTag::Positions,
                    "EXIT_SIGNATURE_SUCCESS",
                    &format!(
                        "✅ Position {} exit signature saved and queued for verification",
                        position.symbol
                    )
                );
            }
        }

        if found_position {
            // Add to verification queue for proper completion
            self.pending_verifications.insert(signature.to_string(), now);

            // Clean up phantom positions from database
            self.cleanup_phantom_positions_database().await;
        } else {
            log(
                LogTag::Positions,
                "EXIT_SIGNATURE_ERROR",
                &format!(
                    "❌ Position not found for mint {} when updating exit signature {}",
                    get_mint_prefix(mint),
                    get_signature_prefix(signature)
                )
            );
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

        // Clean up phantom positions from database after removal
        if removed {
            self.cleanup_phantom_positions_database().await;
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
        // Prune stale pending verifications first to avoid forever-pending entries
        let now = Utc::now();
        let stale: Vec<String> = self.pending_verifications
            .iter()
            .filter_map(|(sig, added_at)| {
                let age_secs = now.signed_duration_since(*added_at).num_seconds();
                if (age_secs as i64) > ENTRY_VERIFICATION_MAX_SECS {
                    Some(sig.clone())
                } else {
                    None
                }
            })
            .collect();

        for sig in stale {
            let age_secs = if let Some(t) = self.pending_verifications.get(&sig) {
                now.signed_duration_since(*t).num_seconds()
            } else {
                0
            };
            log(
                LogTag::Positions,
                "PENDING_VERIFICATION_PRUNE",
                &format!(
                    "🧹 Pruning stale pending verification {} after {}s (> {}s)",
                    get_signature_prefix(&sig),
                    age_secs,
                    ENTRY_VERIFICATION_MAX_SECS
                )
            );
            let _ = self.handle_transaction_timeout(&sig).await;
            self.pending_verifications.remove(&sig);
        }

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
                                // Clean up phantom positions after verification update
                                self.cleanup_phantom_positions_database().await;
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

                                if elapsed_seconds > 60 {
                                    // 60 seconds timeout - account for Solana network propagation delays
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
    async fn update_position_from_transaction(
        &mut self,
        signature: &str
    ) -> Result<(), ScreenerBotError> {
        // Get transaction from transactions manager
        let transaction = match get_transaction(signature).await {
            Ok(Some(tx)) => tx,
            Ok(None) => {
                return Err(
                    ScreenerBotError::Position(PositionError::VerificationFailed {
                        signature: signature.to_string(),
                        reason: "Transaction not found or still pending".to_string(),
                    })
                );
            }
            Err(e) => {
                return Err(
                    ScreenerBotError::Data(DataError::Generic {
                        message: e,
                    })
                );
            }
        };

        // Check transaction status first
        match transaction.status {
            TransactionStatus::Finalized | TransactionStatus::Confirmed => {
                // Transaction is confirmed - proceed with update
            }
            TransactionStatus::Pending => {
                return Err(
                    ScreenerBotError::Position(PositionError::VerificationTimeout {
                        signature: signature.to_string(),
                        timeout_seconds: 300, // 5 minutes
                    })
                );
            }
            TransactionStatus::Failed(ref error) => {
                return Err(
                    ScreenerBotError::Position(PositionError::VerificationFailed {
                        signature: signature.to_string(),
                        reason: format!("Transaction failed: {}", error),
                    })
                );
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

            // Clean up phantom positions after verification update
            self.cleanup_phantom_positions_database().await;
        }

        Ok(())
    }

    /// Comprehensive transaction verification using RPC and transaction analysis
    /// This replaces the simple is_transaction_verified check with detailed verification
    /// Returns Transaction if confirmed, None if pending, Error if failed
    async fn verify_transaction_comprehensively(
        &self,
        signature: &str
    ) -> Result<Option<Transaction>, ScreenerBotError> {
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
                                ScreenerBotError::Blockchain(BlockchainError::TransactionDropped {
                                    signature: signature.to_string(),
                                    reason: format!(
                                        "Transaction failed on-chain: {}",
                                        transaction.error_message.unwrap_or(
                                            "Unknown error".to_string()
                                        )
                                    ),
                                    fee_paid: None,
                                    attempts: 1,
                                })
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
                        return Err(
                            ScreenerBotError::Blockchain(BlockchainError::TransactionDropped {
                                signature: signature.to_string(),
                                reason: format!("Transaction failed: {}", error),
                                fee_paid: None,
                                attempts: 1,
                            })
                        );
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
                                ScreenerBotError::Position(PositionError::VerificationTimeout {
                                    signature: signature.to_string(),
                                    timeout_seconds: verification_age_seconds as u64,
                                })
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
                return Err(
                    ScreenerBotError::Data(DataError::Generic {
                        message: format!("Error getting transaction: {}", e),
                    })
                );
            }
        }
    }

    /// Handle failed transaction by removing phantom positions or updating state
    async fn handle_failed_transaction(
        &mut self,
        signature: &str,
        error: &ScreenerBotError
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

        // Enhanced failure detection for immediate cleanup
        let error_str = error.to_string();
        let is_definitive_failure =
            error_str.to_lowercase().contains("propagation failed") ||
            error_str.to_lowercase().contains("likely failed") ||
            error_str.to_lowercase().contains("dropped by network") ||
            error_str.to_lowercase().contains("verification timeout") ||
            error_str.to_lowercase().contains("transaction failed");

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
                // Entry transaction verification failed - but be more conservative before removal
                log(
                    LogTag::Positions,
                    "ENTRY_VERIFICATION_FAILED",
                    &format!(
                        "⚠️ Entry verification failed for {} - checking for successful exit before removal",
                        position.symbol
                    )
                );

                // Before immediately flagging for phantom removal, check if there are successful exits
                let mut should_remove = true;
                if
                    let Ok(swap_transactions) =
                        crate::transactions::get_global_swap_transactions().await
                {
                    for swap in swap_transactions.iter() {
                        if
                            swap.token_mint == position.mint &&
                            swap.swap_type == "Sell" &&
                            swap.timestamp >= position.entry_time
                        {
                            log(
                                LogTag::Positions,
                                "ENTRY_FAILED_BUT_EXIT_EXISTS",
                                &format!(
                                    "✅ Entry failed but found successful exit {} for {} - will close position instead",
                                    get_signature_prefix(&swap.signature),
                                    position.symbol
                                )
                            );
                            should_remove = false;

                            // Set the exit transaction data directly
                            position.exit_transaction_signature = Some(swap.signature.clone());
                            position.exit_time = Some(swap.timestamp);
                            position.transaction_exit_verified = true;
                            position.sol_received = Some(swap.sol_amount);

                            // Calculate exit price
                            if let Some(token_amt) = position.token_amount {
                                if token_amt > 0 {
                                    let token_amt_f64 = token_amt as f64;
                                    position.exit_price = Some(swap.sol_amount / token_amt_f64);
                                    position.effective_exit_price = Some(
                                        swap.sol_amount / token_amt_f64
                                    );
                                }
                            }
                            break;
                        }
                    }
                }

                if should_remove {
                    // Only flag as phantom for definitively failed transactions or after safe windows
                    if is_definitive_failure {
                        log(
                            LogTag::Positions,
                            "DEFINITIVE_FAILURE_PHANTOM",
                            &format!(
                                "� Position {} flagged for phantom removal due to definitive failure: {}",
                                position.symbol,
                                error
                            )
                        );
                        position.phantom_remove = true;
                    } else {
                        // For timeout or network issues, be more conservative
                        let age_minutes = chrono::Utc
                            ::now()
                            .signed_duration_since(position.entry_time)
                            .num_minutes();

                        if age_minutes > AGED_UNVERIFIED_CLEANUP_MINUTES / 2 {
                            // After half of the cleanup window, allow phantom flag for non-definitive failures
                            log(
                                LogTag::Positions,
                                "CONSERVATIVE_PHANTOM",
                                &format!(
                                    "�️ Position {} flagged for phantom removal after {}m (conservative timeout): {}",
                                    position.symbol,
                                    age_minutes,
                                    error
                                )
                            );
                            position.phantom_remove = true;
                        } else {
                            log(
                                LogTag::Positions,
                                "PHANTOM_DELAYED",
                                &format!(
                                    "⏳ Delaying phantom flag for {} - only {}m old, waiting for network propagation",
                                    position.symbol,
                                    age_minutes
                                )
                            );
                            // Don't flag as phantom yet - wait for more time
                        }
                    }
                }

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

            // Clean up phantom positions after handling failure
            self.cleanup_phantom_positions_database().await;
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

        // Check if transaction exists now before marking as failed
        match crate::transactions::get_transaction(signature).await {
            Ok(Some(tx)) => {
                if
                    matches!(
                        tx.status,
                        crate::transactions::TransactionStatus::Confirmed |
                            crate::transactions::TransactionStatus::Finalized
                    ) &&
                    tx.success
                {
                    log(
                        LogTag::Positions,
                        "TIMEOUT_RECOVERY",
                        &format!(
                            "✅ Transaction {} appeared after timeout - will process normally",
                            get_signature_prefix(signature)
                        )
                    );
                    // Don't mark as failed - transaction is actually successful
                    return Ok(());
                }
            }
            _ => {
                log(
                    LogTag::Positions,
                    "TIMEOUT_CONFIRMED",
                    &format!(
                        "❌ Transaction {} still not found after timeout - marking as failed",
                        get_signature_prefix(signature)
                    )
                );
            }
        }

        // Only treat as failure if transaction is definitively not found or failed
        let timeout_error = ScreenerBotError::Position(PositionError::VerificationTimeout {
            signature: signature.to_string(),
            timeout_seconds: 300,
        });
        self.handle_failed_transaction(signature, &timeout_error).await
    }

    /// Clean up phantom positions
    async fn cleanup_phantom_positions(&mut self) {
        log(LogTag::Positions, "CLEANUP", "🧹 Checking for phantom positions to cleanup");
        // ENHANCED Criteria for phantom removal with exit transaction detection:
        // 1. Explicitly flagged with phantom_remove (set when entry tx failed) - PRIORITY: Remove immediately but check exits
        // 2. Entry tx unverified for > 10 minutes AND transaction no longer found in cache - BUT check for exits first
        // 3. Entry tx unverified AND wallet holds zero tokens for mint - BUT check for exits first
        // NEW: Before removing, always check if there are successful sell transactions for the token

        let now = Utc::now();
        let mut to_remove: Vec<usize> = Vec::new();
        let mut to_close: Vec<(usize, String, f64)> = Vec::new(); // (index, exit_sig, sol_received)

        let positions_count = self.positions.len();
        let phantom_flagged_count = self.positions
            .iter()
            .filter(|p| p.phantom_remove)
            .count();

        log(
            LogTag::Positions,
            "CLEANUP_STATS",
            &format!(
                "📊 Cleanup scan: {} positions total, {} flagged for phantom removal",
                positions_count,
                phantom_flagged_count
            )
        );

        for (idx, position) in self.positions.iter_mut().enumerate() {
            // Skip already closed positions
            if position.exit_transaction_signature.is_some() {
                continue;
            }

            // GLOBAL GRACE PERIOD: Don't cleanup positions created within last 2 minutes
            // This prevents race conditions between position creation and verification
            let age_seconds = now.signed_duration_since(position.entry_time).num_seconds();
            if age_seconds < 120 {
                // 2 minutes grace period
                log(
                    LogTag::Positions,
                    "GLOBAL_GRACE_PERIOD",
                    &format!(
                        "🕒 Skipping cleanup for {} - within 2-minute grace period ({}s old)",
                        position.symbol,
                        age_seconds
                    )
                );
                continue;
            }

            // VERIFICATION STATE PROTECTION: Don't cleanup positions still in verification queue
            if let Some(entry_sig) = &position.entry_transaction_signature {
                if self.pending_verifications.contains_key(entry_sig) {
                    let protect = if let Some(added) = self.pending_verifications.get(entry_sig) {
                        (now.signed_duration_since(*added).num_seconds() as i64) <=
                            PENDING_VERIFICATION_PROTECT_SECS
                    } else {
                        false
                    };
                    if protect {
                        log(
                            LogTag::Positions,
                            "PENDING_VERIFICATION_PROTECTION",
                            &format!(
                                "🔍 Skipping cleanup for {} - transaction {} still in verification queue",
                                position.symbol,
                                crate::transactions::get_signature_prefix(entry_sig)
                            )
                        );
                        continue;
                    } else {
                        log(
                            LogTag::Positions,
                            "PENDING_VERIFICATION_EXPIRED",
                            &format!(
                                "⏱️ Pending verification protection expired for {} - proceeding with cleanup checks",
                                position.symbol
                            )
                        );
                    }
                }
            }

            let mut remove = false;
            let mut removal_reason = String::new();

            // Condition 1: Explicit phantom flag - BUT apply universal grace period for recent transactions
            if position.phantom_remove {
                let age_minutes = now.signed_duration_since(position.entry_time).num_minutes();

                // UNIVERSAL GRACE PERIOD: For very recent positions (< 2 minutes), give more time
                // to handle normal network propagation delays regardless of router/DEX
                // INCREASED FROM 5 to 2 minutes to be more aggressive but still safe
                if age_minutes < 2 {
                    log(
                        LogTag::Positions,
                        "UNIVERSAL_GRACE_PERIOD",
                        &format!(
                            "🕒 Giving {} more time for verification - recent transaction ({}min old)",
                            position.symbol,
                            age_minutes
                        )
                    );
                    // Don't remove yet, give it more time for network propagation
                    continue;
                }

                remove = true;
                removal_reason = "explicitly_flagged_phantom".to_string();

                log(
                    LogTag::Positions,
                    "CLEANUP_PHANTOM_FLAGGED",
                    &format!(
                        "🚩 Position {} ({}) is flagged for phantom removal - checking for exits first",
                        position.symbol,
                        get_mint_prefix(&position.mint)
                    )
                );
            }

            // Condition 2: Aged unverified entry tx not found (increased grace period)
            if
                !remove &&
                position.entry_transaction_signature.is_some() &&
                !position.transaction_entry_verified
            {
                let age_minutes = now.signed_duration_since(position.entry_time).num_minutes();
                if age_minutes > AGED_UNVERIFIED_CLEANUP_MINUTES {
                    // Try quick lookup of transaction; if still missing, mark remove
                    if let Some(sig) = &position.entry_transaction_signature {
                        match crate::transactions::get_transaction(sig).await {
                            Ok(Some(_)) => {/* exists - keep */}
                            _ => {
                                remove = true;
                                removal_reason = "aged_unverified_transaction".to_string();

                                log(
                                    LogTag::Positions,
                                    "CLEANUP_AGED_UNVERIFIED",
                                    &format!(
                                        "⏰ Position {} ({}) has aged unverified transaction ({}min) - checking for exits",
                                        position.symbol,
                                        get_mint_prefix(&position.mint),
                                        age_minutes
                                    )
                                );
                            }
                        }
                    }
                }
            }

            // Condition 3: No tokens in wallet for this mint (best-effort, ignore errors)
            // ENHANCED: Add additional checks to prevent premature removal
            if !remove && position.token_amount.unwrap_or(0) == 0 {
                // Only check wallet balance if entry still unverified to avoid RPC load
                if !position.transaction_entry_verified {
                    // ADDITIONAL PROTECTION: Wait longer for recent positions before balance check
                    let age_minutes = now.signed_duration_since(position.entry_time).num_minutes();
                    if age_minutes < 5 {
                        log(
                            LogTag::Positions,
                            "BALANCE_CHECK_DELAY",
                            &format!(
                                "⏳ Delaying balance check for {} - position too recent ({}min old)",
                                position.symbol,
                                age_minutes
                            )
                        );
                        continue; // Skip balance check for recent positions
                    }

                    if let Ok(wallet) = crate::utils::get_wallet_address() {
                        if
                            let Ok(balance) = crate::utils::get_token_balance(
                                &wallet,
                                &position.mint
                            ).await
                        {
                            if balance == 0 {
                                // TRANSACTION VALIDATION: Check if entry transaction actually failed
                                let mut transaction_confirmed_failed = false;
                                if let Some(entry_sig) = &position.entry_transaction_signature {
                                    match crate::transactions::get_transaction(entry_sig).await {
                                        Ok(Some(tx)) => {
                                            // Transaction exists, check if it was successful
                                            if !tx.success {
                                                transaction_confirmed_failed = true;
                                                log(
                                                    LogTag::Positions,
                                                    "TRANSACTION_CONFIRMED_FAILED",
                                                    &format!(
                                                        "❌ Entry transaction {} confirmed failed for {}",
                                                        crate::transactions::get_signature_prefix(
                                                            entry_sig
                                                        ),
                                                        position.symbol
                                                    )
                                                );
                                            }
                                        }
                                        Ok(None) => {
                                            // Transaction not found - could be failed or still processing
                                            if age_minutes > 10 {
                                                transaction_confirmed_failed = true;
                                                log(
                                                    LogTag::Positions,
                                                    "TRANSACTION_NOT_FOUND_AGED",
                                                    &format!(
                                                        "❓ Entry transaction {} not found after {}min for {}",
                                                        crate::transactions::get_signature_prefix(
                                                            entry_sig
                                                        ),
                                                        age_minutes,
                                                        position.symbol
                                                    )
                                                );
                                            }
                                        }
                                        Err(_) => {
                                            // Error checking transaction - be conservative
                                            log(
                                                LogTag::Positions,
                                                "TRANSACTION_CHECK_ERROR",
                                                &format!(
                                                    "⚠️ Failed to check transaction {} for {} - skipping cleanup",
                                                    crate::transactions::get_signature_prefix(
                                                        entry_sig
                                                    ),
                                                    position.symbol
                                                )
                                            );
                                            continue; // Skip cleanup on error
                                        }
                                    }
                                }

                                // Only mark for removal if transaction is confirmed failed or very old
                                if transaction_confirmed_failed || age_minutes > 15 {
                                    remove = true;
                                    removal_reason = "zero_wallet_balance_validated".to_string();

                                    log(
                                        LogTag::Positions,
                                        "CLEANUP_ZERO_BALANCE_VALIDATED",
                                        &format!(
                                            "🔍 Position {} ({}) has zero wallet balance and {} - checking for exits",
                                            position.symbol,
                                            get_mint_prefix(&position.mint),
                                            if transaction_confirmed_failed {
                                                "confirmed failed transaction"
                                            } else {
                                                "aged transaction"
                                            }
                                        )
                                    );
                                } else {
                                    log(
                                        LogTag::Positions,
                                        "CLEANUP_ZERO_BALANCE_WAITING",
                                        &format!(
                                            "⏳ Position {} has zero balance but transaction not confirmed failed - waiting longer",
                                            position.symbol
                                        )
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // CRITICAL FIX: Before removing as phantom, check for successful sell transactions
            if remove {
                log(
                    LogTag::Positions,
                    "PHANTOM_CHECK_EXIT",
                    &format!(
                        "🔍 Checking for exit transactions before removing phantom position {} ({}) [reason: {}]",
                        position.symbol,
                        get_mint_prefix(&position.mint),
                        removal_reason
                    )
                );

                // Search for sell transactions for this token using optimized filtering
                if
                    let Ok(swap_transactions) =
                        crate::transactions::get_swap_transactions_for_token(
                            &position.mint,
                            Some("Sell"), // Only look for sell transactions
                            Some(50) // Limit to recent 50 transactions for performance
                        ).await
                {
                    let mut found_exit = false;
                    for swap in swap_transactions.iter() {
                        // Check if this sell happened after position entry
                        if swap.timestamp >= position.entry_time {
                            log(
                                LogTag::Positions,
                                "PHANTOM_FOUND_EXIT",
                                &format!(
                                    "✅ Found exit transaction {} for phantom position {} - closing instead of removing",
                                    get_signature_prefix(&swap.signature),
                                    position.symbol
                                )
                            );

                            // Mark for proper closure instead of removal
                            to_close.push((idx, swap.signature.clone(), swap.sol_amount));
                            found_exit = true;
                            break;
                        }
                    }

                    if !found_exit {
                        log(
                            LogTag::Positions,
                            "PHANTOM_NO_EXIT_CONFIRMED",
                            &format!(
                                "❌ No exit transactions found for phantom position {} - will remove",
                                position.symbol
                            )
                        );
                    } else {
                        remove = false; // Don't remove, we'll close it properly
                    }
                } else {
                    log(
                        LogTag::Positions,
                        "PHANTOM_EXIT_CHECK_ERROR",
                        &format!(
                            "⚠️ Failed to check swap transactions for {} - proceeding with removal",
                            position.symbol
                        )
                    );
                }
            }

            if remove {
                to_remove.push(idx);
            }
        }

        // First, handle positions that should be closed instead of removed
        if !to_close.is_empty() {
            for (idx, exit_sig, sol_received) in to_close.iter().rev() {
                if let Some(position) = self.positions.get_mut(*idx) {
                    // Get exit transaction details for proper closure
                    if let Ok(Some(exit_tx)) = crate::transactions::get_transaction(exit_sig).await {
                        log(
                            LogTag::Positions,
                            "PHANTOM_CLOSE",
                            &format!(
                                "🎯 Properly closing phantom position {} with exit transaction {} (SOL received: {:.6})",
                                position.symbol,
                                get_signature_prefix(exit_sig),
                                sol_received
                            )
                        );

                        // Set exit transaction data
                        position.exit_transaction_signature = Some(exit_sig.clone());
                        position.exit_time = Some(exit_tx.timestamp);
                        position.transaction_exit_verified = true;
                        position.sol_received = Some(*sol_received);

                        // Calculate exit price from transaction
                        if let Some(token_amt) = position.token_amount {
                            if token_amt > 0 {
                                let token_amt_f64 = token_amt as f64;
                                position.exit_price = Some(sol_received / token_amt_f64);
                                position.effective_exit_price = Some(sol_received / token_amt_f64);
                            }
                        }

                        // Clear phantom flag since we properly closed it
                        position.phantom_remove = false;
                    }
                }
            }
        }

        // Remove in reverse order to keep indices valid
        if !to_remove.is_empty() {
            log(
                LogTag::Positions,
                "CLEANUP_REMOVING",
                &format!("🗑️ Removing {} phantom positions", to_remove.len())
            );

            for idx in to_remove.iter().rev() {
                if let Some(removed) = self.positions.get(*idx) {
                    log(
                        LogTag::Positions,
                        "PHANTOM_REMOVE",
                        &format!(
                            "🗑️ Removing phantom position {} ({}) - unverified entry tx {} (no exit found)",
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
        } else {
            log(
                LogTag::Positions,
                "CLEANUP_NO_PHANTOMS",
                "✅ No phantom positions found for removal"
            );
        }

        // Persist updated positions if any changes were made
        if !to_remove.is_empty() || !to_close.is_empty() {
            log(
                LogTag::Positions,
                "CLEANUP_PERSISTING",
                &format!(
                    "💾 Persisting position changes: {} removed, {} closed",
                    to_remove.len(),
                    to_close.len()
                )
            );
            self.cleanup_phantom_positions_database().await;
        }
    }

    /// Verify and resolve position state using transaction history
    async fn verify_and_resolve_position_state(
        &mut self,
        position: &mut Position,
        exit_price: f64,
        exit_time: DateTime<Utc>
    ) -> Result<(), String> {
        log(
            LogTag::Positions,
            "VERIFY",
            &format!(
                "🔍 Verifying / resolving phantom state for {} (confirmations={}, first_seen={:?})",
                position.symbol,
                position.phantom_confirmations,
                position.phantom_first_seen
            )
        );

        if position.synthetic_exit {
            return Ok(());
        }

        // Attempt targeted search one more time
        if position.exit_transaction_signature.is_none() {
            if let Some(found_sig) = self.find_missing_exit_transaction_targeted(position).await {
                position.exit_transaction_signature = Some(found_sig.clone());
                log(
                    LogTag::Positions,
                    "PHANTOM_HEAL_FOUND_EXIT",
                    &format!(
                        "🎯 Found real exit transaction {} for phantom {} during verify step",
                        get_signature_prefix(&found_sig),
                        position.symbol
                    )
                );
                return Ok(()); // regular verification path will finalize
            }
        }

        let now = Utc::now();
        let duration_ok = position.phantom_first_seen
            .map(|t| now.signed_duration_since(t).num_seconds() >= PHANTOM_MIN_DURATION_SECS)
            .unwrap_or(false);
        let confirmations_ok = position.phantom_confirmations >= PHANTOM_CONFIRMATION_THRESHOLD;
        if !(duration_ok && confirmations_ok) {
            return Err("Phantom position not yet eligible for synthetic closure".to_string());
        }

        let synthetic_price = if exit_price > 0.0 {
            exit_price
        } else {
            position.current_price.unwrap_or(position.entry_price)
        };
        position.exit_price = Some(synthetic_price);
        position.exit_time = Some(exit_time);
        position.transaction_exit_verified = false;
        position.effective_exit_price = None;
        position.sol_received = None;
        position.synthetic_exit = true;
        position.closed_reason = Some("synthetic_phantom_closure".to_string());

        log(
            LogTag::Positions,
            "PHANTOM_SYNTHETIC_CLOSED",
            &format!(
                "🧵 Synthetic closure applied to phantom {} at {:.9} (confirmations={}, duration_ok={}, confirmations_ok={})",
                position.symbol,
                synthetic_price,
                position.phantom_confirmations,
                duration_ok,
                confirmations_ok
            )
        );

        self.cleanup_phantom_positions_database().await;
        Ok(())
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

                // Check if position is now fully closed and clean up watch list
                if
                    position.transaction_entry_verified &&
                    position.transaction_exit_verified &&
                    position.exit_price.is_some()
                {
                    log(
                        LogTag::Positions,
                        "POSITION_FULLY_CLOSED",
                        &format!(
                            "✅ Position {} is fully closed - removing from price watch list",
                            position.symbol
                        )
                    );
                }
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

    /// Get swap PnL info using priority transaction processing with retry logic
    pub async fn convert_to_swap_pnl_info(
        &self,
        transaction: &Transaction,
        token_symbol_cache: &std::collections::HashMap<String, String>,
        silent: bool
    ) -> Option<crate::transactions::SwapPnLInfo> {
        // First try with global transaction manager
        let global_result = self.try_convert_with_global_manager(
            transaction,
            token_symbol_cache,
            silent
        ).await;
        if global_result.is_some() {
            return global_result;
        }

        // If global manager fails, use priority transaction processing
        if !silent {
            log(
                LogTag::Positions,
                "CONVERT_PRIORITY_FALLBACK",
                &format!(
                    "🔄 Global manager failed, using priority transaction for {}",
                    get_signature_prefix(&transaction.signature)
                )
            );
        }

        // Get fresh transaction data with guaranteed full analysis
        match crate::transactions::get_priority_transaction(&transaction.signature).await {
            Ok(Some(fresh_transaction)) => {
                // Use the fresh transaction data for conversion
                if
                    let Some(result) = self.try_convert_with_global_manager(
                        &fresh_transaction,
                        token_symbol_cache,
                        true
                    ).await
                {
                    if !silent {
                        log(
                            LogTag::Positions,
                            "CONVERT_PRIORITY_SUCCESS",
                            &format!(
                                "✅ Priority conversion successful for {}",
                                get_signature_prefix(&transaction.signature)
                            )
                        );
                    }
                    return Some(result);
                }

                // If global manager still fails, create temporary manager
                self.convert_with_temporary_manager(
                    &fresh_transaction,
                    token_symbol_cache,
                    silent
                ).await
            }
            Ok(None) => {
                if !silent {
                    log(
                        LogTag::Positions,
                        "CONVERT_PRIORITY_NOT_FOUND",
                        &format!(
                            "❌ Priority transaction not found for {}",
                            get_signature_prefix(&transaction.signature)
                        )
                    );
                }
                None
            }
            Err(e) => {
                if !silent {
                    log(
                        LogTag::Positions,
                        "CONVERT_PRIORITY_ERROR",
                        &format!(
                            "❌ Priority transaction error for {}: {}",
                            get_signature_prefix(&transaction.signature),
                            e
                        )
                    );
                }
                None
            }
        }
    }

    /// Try conversion with global manager (helper method)
    async fn try_convert_with_global_manager(
        &self,
        transaction: &Transaction,
        token_symbol_cache: &std::collections::HashMap<String, String>,
        silent: bool
    ) -> Option<crate::transactions::SwapPnLInfo> {
        use crate::transactions::GLOBAL_TRANSACTION_MANAGER;

        // Use shorter timeout for global manager attempt
        let lock_result = tokio::time::timeout(
            Duration::from_secs(2),
            GLOBAL_TRANSACTION_MANAGER.lock()
        ).await;

        match lock_result {
            Ok(manager_guard) => {
                if let Some(ref manager) = *manager_guard {
                    let result = manager.convert_to_swap_pnl_info(
                        transaction,
                        token_symbol_cache,
                        silent
                    );
                    if result.is_some() && !silent {
                        log(
                            LogTag::Positions,
                            "CONVERT_GLOBAL_SUCCESS",
                            &format!(
                                "✅ Global manager conversion successful for {}",
                                get_signature_prefix(&transaction.signature)
                            )
                        );
                    }
                    result
                } else {
                    if !silent {
                        log(
                            LogTag::Positions,
                            "CONVERT_GLOBAL_NOT_INITIALIZED",
                            "❌ Global TransactionsManager not initialized"
                        );
                    }
                    None
                }
            }
            Err(_) => {
                if !silent {
                    log(
                        LogTag::Positions,
                        "CONVERT_GLOBAL_TIMEOUT",
                        &format!(
                            "⏱️ Timeout acquiring global manager for {}",
                            get_signature_prefix(&transaction.signature)
                        )
                    );
                }
                None
            }
        }
    }

    /// Convert using temporary manager (last resort)
    async fn convert_with_temporary_manager(
        &self,
        transaction: &Transaction,
        token_symbol_cache: &std::collections::HashMap<String, String>,
        silent: bool
    ) -> Option<crate::transactions::SwapPnLInfo> {
        if !silent {
            log(
                LogTag::Positions,
                "CONVERT_TEMPORARY",
                &format!(
                    "🔧 Creating temporary manager for {}",
                    get_signature_prefix(&transaction.signature)
                )
            );
        }

        // Use global transaction manager instead of creating new instance
        let result = crate::transactions::with_global_tx_manager(3, |manager| {
            manager.convert_to_swap_pnl_info(transaction, token_symbol_cache, silent)
        }).await;

        match result {
            Some(conversion_result) => {
                if conversion_result.is_some() && !silent {
                    log(
                        LogTag::Positions,
                        "CONVERT_SUCCESS",
                        &format!(
                            "✅ Global manager conversion successful for {}",
                            get_signature_prefix(&transaction.signature)
                        )
                    );
                }
                conversion_result
            }
            None => {
                if !silent {
                    log(
                        LogTag::Positions,
                        "CONVERT_UNAVAILABLE",
                        &format!(
                            "⚠️ Global transaction manager unavailable for conversion of {}",
                            get_signature_prefix(&transaction.signature)
                        )
                    );
                }
                None
            }
        }
    }

    /// Load positions from database into HashMap structure
    async fn load_positions_from_database(&self) {
        if is_debug_positions_enabled() {
            log(LogTag::Positions, "DEBUG", "📂 Loading positions from database");
        }

        match crate::positions_db::load_all_positions().await {
            Ok(positions_from_db) => {
                let mut positions = self.positions.lock().await;

                for position in positions_from_db {
                    let position_lock = Arc::new(AsyncMutex::new(position.clone()));
                    positions.insert(position.mint.clone(), position_lock);
                }

                log(
                    LogTag::Positions,
                    "INFO",
                    &format!("📁 Loaded {} positions from database", positions.len())
                );

                if is_debug_positions_enabled() {
                    let open_count = self.get_open_positions_count().await;
                    let closed_count = positions.len() - open_count;
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
                    &format!("Failed to load positions from database: {}", e)
                );
            }
        }
    }

    /// Save positions to disk after changes
    /// Save position to database (individual position)
    async fn save_position_to_database(&mut self, position: &Position) -> Result<(), String> {
        match crate::positions_db::save_position(position).await {
            Ok(new_id) => {
                // Update position in memory with new ID if it was newly inserted
                if position.id.is_none() {
                    for pos in &mut self.positions {
                        if
                            pos.mint == position.mint &&
                            pos.entry_time == position.entry_time &&
                            pos.id.is_none()
                        {
                            pos.id = Some(new_id);
                            break;
                        }
                    }
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Save all positions to database (batch operation)
    /// Remove phantom positions marked for deletion from database
    async fn cleanup_phantom_positions_database(&mut self) {
        // Find positions marked for removal
        let positions_to_remove: Vec<_> = self.positions
            .iter()
            .filter(|p| p.phantom_remove)
            .filter_map(|p| p.id)
            .collect();

        if !positions_to_remove.is_empty() {
            log(
                LogTag::Positions,
                "CLEANUP",
                &format!(
                    "�️ Removing {} phantom positions from database",
                    positions_to_remove.len()
                )
            );

            for position_id in positions_to_remove {
                if let Err(e) = crate::positions_db::delete_position_by_id(position_id).await {
                    log(
                        LogTag::Positions,
                        "ERROR",
                        &format!("Failed to delete phantom position {}: {}", position_id, e)
                    );
                }
            }
        }

        // Remove from memory
        let initial_count = self.positions.len();
        self.positions.retain(|p| !p.phantom_remove);
        let final_count = self.positions.len();

        if initial_count != final_count {
            log(
                LogTag::Positions,
                "CLEANUP",
                &format!("🗑️ Removed {} phantom positions from memory", initial_count - final_count)
            );
        }
    }
}

// =============================================================================
// ACTOR INTERFACE (Requests + Handle) and Service Startup
// =============================================================================

// =============================================================================
// GLOBAL POSITIONS MANAGER ACCESS
// =============================================================================

static GLOBAL_POSITIONS_MANAGER: Lazy<AsyncMutex<Option<Arc<PositionsManager>>>> = Lazy::new(||
    AsyncMutex::new(None)
);

pub async fn set_positions_manager(manager: Arc<PositionsManager>) {
    let mut guard = match
        tokio::time::timeout(Duration::from_secs(1), GLOBAL_POSITIONS_MANAGER.lock()).await
    {
        Ok(guard) => guard,
        Err(_) => {
            log(LogTag::Positions, "ERROR", "⏰ Timeout setting positions manager");
            return;
        }
    };
    *guard = Some(manager);
}

pub async fn get_positions_manager() -> Option<Arc<PositionsManager>> {
    let guard = match
        tokio::time::timeout(Duration::from_secs(1), GLOBAL_POSITIONS_MANAGER.lock()).await
    {
        Ok(guard) => guard,
        Err(_) => {
            log(LogTag::Positions, "WARN", "⏰ Timeout getting positions manager");
            return None;
        }
    };
    guard.clone()
}

/// Start the simplified PositionsManager service with direct access
pub async fn start_positions_manager_service(shutdown: Arc<Notify>) {
    let mut manager = PositionsManager::new(shutdown.clone());
    manager.initialize().await;

    let manager_arc = Arc::new(manager);
    set_positions_manager(manager_arc).await;

    log(LogTag::Positions, "INFO", "PositionsManager service initialized (direct access)");
}

// =============================================================================
// Public async helpers for external modules (direct manager access)
// =============================================================================

pub async fn get_open_positions() -> Vec<Position> {
    if let Some(manager) = get_positions_manager().await {
        manager.get_open_positions().await
    } else {
        Vec::new()
    }
}

pub async fn get_closed_positions() -> Vec<Position> {
    if let Some(manager) = get_positions_manager().await {
        manager.get_closed_positions().await
    } else {
        Vec::new()
    }
}

pub async fn get_open_positions_count() -> usize {
    if let Some(manager) = get_positions_manager().await {
        manager.get_open_positions_count().await
    } else {
        0
    }
}

pub async fn get_positions_by_state(state: PositionState) -> Vec<Position> {
    if let Some(manager) = get_positions_manager().await {
        manager.get_positions_by_state(&state).await
    } else {
        Vec::new()
    }
}

/// Check if a position is currently open for the given mint
pub async fn is_open_position(mint: &str) -> bool {
    if let Some(manager) = get_positions_manager().await {
        manager.is_open_position(mint).await
    } else {
        false
    }
}

// Global helper functions for opening and closing positions
pub async fn open_position_global(
    token: Token,
    price: f64,
    percent_change: f64,
    size_sol: f64
) -> Result<(String, String), ScreenerBotError> {
    if let Some(manager) = get_positions_manager().await {
        manager.open_position(&token, price, percent_change, size_sol).await
    } else {
        Err(
            ScreenerBotError::Position(PositionError::Generic {
                message: "PositionsManager not available".to_string(),
            })
        )
    }
}

pub async fn close_position_global(
    mint: String,
    token: Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), ScreenerBotError> {
    if let Some(manager) = get_positions_manager().await {
        manager.close_position(&mint, &token, exit_price, exit_time).await
    } else {
        Err(
            ScreenerBotError::Position(PositionError::Generic {
                message: "PositionsManager not available".to_string(),
            })
        )
    }
}

pub async fn update_position_tracking_global(mint: String, current_price: f64) -> bool {
    if let Some(manager) = get_positions_manager().await {
        manager.update_tracking(&mint, current_price).await
    } else {
        false
    }
}

/// Background task execution for close position operations
/// This prevents blocking the PositionsManager actor while processing expensive RPC operations
pub async fn execute_close_position_background(
    mint: String,
    token: Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), ScreenerBotError> {
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
            return Err(
                ScreenerBotError::Position(PositionError::PositionNotFound {
                    token_mint: mint,
                    signature: "".to_string(),
                })
            );
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
        return Err(
            ScreenerBotError::Position(PositionError::Generic {
                message: "DRY-RUN: Position would be closed".to_string(),
            })
        );
    }

    // Balance check and phantom detection happens in execute_sell_with_retry_background()

    // Execute sell transaction with retry logic
    execute_sell_with_retry_background(&mut position, &token, exit_price, exit_time).await
}

/// Execute sell with retry logic in background task
async fn execute_sell_with_retry_background(
    position: &mut Position,
    token: &Token,
    exit_price: f64,
    exit_time: DateTime<Utc>
) -> Result<(String, String), ScreenerBotError> {
    let _guard = crate::trader::CriticalOperationGuard::new(&format!("SELL {}", position.symbol));

    // Active sell registry guard (background context)
    if !register_active_sell(&position.mint).await {
        log(
            LogTag::Swap,
            "ACTIVE_SELL_SKIP",
            &format!(
                "⏳ Skipping sell (background) for {} - another sell already in progress for mint {}",
                position.symbol,
                get_mint_prefix(&position.mint)
            )
        );
        return Err(
            ScreenerBotError::Position(PositionError::Generic {
                message: "Active sell already in progress".to_string(),
            })
        );
    }
    struct ActiveSellCleanupBg {
        mint: String,
    }
    impl Drop for ActiveSellCleanupBg {
        fn drop(&mut self) {
            let mint = self.mint.clone();
            tokio::spawn(async move {
                clear_active_sell(&mint).await;
            });
        }
    }
    let _active_cleanup = ActiveSellCleanupBg { mint: position.mint.clone() };

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
                return Err(
                    ScreenerBotError::Data(DataError::ValidationError {
                        field: "expected_sol_output".to_string(),
                        value: expected_sol.to_string(),
                        reason: "Expected SOL output must be positive and finite".to_string(),
                    })
                );
            }
        }

        // Auto-retry with progressive slippage from config
        let slippages = &SELL_RETRY_SLIPPAGES;
        let token_amount = position.token_amount.unwrap_or(0);

        let mut last_error: Option<ScreenerBotError> = None;

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

            let wallet_address = match
                tokio::time::timeout(
                    tokio::time::Duration::from_secs(5), // 5s timeout for wallet address
                    async {
                        get_wallet_address()
                    }
                ).await
            {
                Ok(Ok(addr)) => addr,
                Ok(Err(e)) => {
                    last_error = Some(e);
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Positions,
                        "TIMEOUT",
                        &format!(
                            "⏰ Wallet address timeout during background sell for {} after 5s",
                            token.symbol
                        )
                    );
                    last_error = Some(
                        ScreenerBotError::api_error(
                            "Wallet address timeout during background sell".to_string()
                        )
                    );
                    continue;
                }
            };

            let actual_wallet_balance = match
                tokio::time::timeout(
                    Duration::from_secs(45),
                    get_cached_token_balance(&wallet_address, &token.mint)
                ).await
            {
                Ok(Ok(balance)) => balance,
                Ok(Err(e)) => {
                    last_error = Some(e);
                    continue;
                }
                Err(_) => {
                    last_error = Some(
                        ScreenerBotError::Network(NetworkError::Generic {
                            message: format!("Timeout getting token balance for {}", token.symbol),
                        })
                    );
                    continue;
                }
            };

            if actual_wallet_balance == 0 {
                log(
                    LogTag::Swap,
                    "PHANTOM",
                    &format!(
                        "👻 Phantom position detected for {} - expected {}, wallet 0. Marking as sold elsewhere.",
                        token.symbol,
                        token_amount
                    )
                );
                return Err(
                    ScreenerBotError::Position(PositionError::PhantomPositionDetected {
                        token_mint: token.mint.clone(),
                        signature: "unknown".to_string(),
                    })
                );
            }

            let actual_sell_amount = actual_wallet_balance; // may be partial

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

            // DUPLICATE SWAP PREVENTION (improved parity with method): Only block if wallet balance decreased vs recorded amount.
            let expected_sol_amount = exit_price;
            let full_position_intact = actual_wallet_balance == token_amount;
            if !full_position_intact {
                if is_duplicate_swap_attempt(&token.mint, expected_sol_amount, "SELL").await {
                    last_error = Some(
                        ScreenerBotError::Position(PositionError::Generic {
                            message: format!(
                                "Duplicate sell prevented for {} (background) - similar sell attempted within last {}s (wallet balance changed)",
                                token.symbol,
                                DUPLICATE_SWAP_PREVENTION_SECS
                            ),
                        })
                    );
                    continue;
                }
            } else if crate::arguments::is_debug_swaps_enabled() {
                log(
                    LogTag::Swap,
                    "DUPLICATE_SKIP",
                    &format!(
                        "🔄 Duplicate prevention skipped (background) for {} (full balance intact: {} tokens)",
                        token.symbol,
                        actual_wallet_balance
                    )
                );
            }

            let best_quote = match
                tokio::time::timeout(
                    tokio::time::Duration::from_secs(20), // 20s total timeout for quote requests
                    get_best_quote(
                        &token.mint,
                        SOL_MINT,
                        actual_sell_amount,
                        &wallet_address,
                        slippage
                    )
                ).await
            {
                Ok(Ok(quote)) => quote,
                Ok(Err(e)) => {
                    last_error = Some(e);
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Swap,
                        "QUOTE_TIMEOUT",
                        &format!(
                            "⏰ Sell quote request timeout for {} after 20s (slippage: {:.1}%)",
                            token.symbol,
                            slippage
                        )
                    );
                    last_error = Some(
                        ScreenerBotError::api_error(
                            format!("Quote request timeout for {}", token.symbol)
                        )
                    );
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
                    last_error = Some(e);
                    continue;
                }
            };

            // Process successful swap result
            let transaction_signature = match swap_result.transaction_signature {
                Some(sig) => sig,
                None => {
                    last_error = Some(
                        ScreenerBotError::Data(DataError::Generic {
                            message: "Swap result missing signature".to_string(),
                        })
                    );
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

            // CRITICAL FIX: Immediately update the position with the exit signature
            // This prevents phantom position scenarios where transaction succeeds but isn't tracked
            if let Some(positions_handle) = get_positions_handle().await {
                log(
                    LogTag::Positions,
                    "UPDATE_EXIT_SIGNATURE",
                    &format!(
                        "💾 Saving exit signature {} for {} to prevent phantom position",
                        get_signature_prefix(&transaction_signature),
                        position.symbol
                    )
                );

                // Update the position directly through the positions manager
                positions_handle.update_exit_signature_direct(
                    position.mint.clone(),
                    transaction_signature.clone(),
                    quote_label.clone()
                ).await;

                log(
                    LogTag::Positions,
                    "INFO",
                    &format!(
                        "✅ Exit signature saved for {}: {}",
                        position.symbol,
                        get_signature_prefix(&transaction_signature)
                    )
                );
            } else {
                log(
                    LogTag::Positions,
                    "ERROR",
                    &format!(
                        "❌ PositionsManager not available to save exit signature for {}",
                        position.symbol
                    )
                );
            }

            return Ok((transaction_signature, quote_label));
        } // Close the inner slippage loop

        if let Some(ref error) = last_error {
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

    Err(
        ScreenerBotError::Position(PositionError::Generic {
            message: format!("Failed to sell {} after {} attempts", position.symbol, max_attempts),
        })
    )
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
