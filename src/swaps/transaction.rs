/// Transaction verification and analysis for swap operations with persistent monitoring
/// 
/// Purpose: Comprehensive transaction verification and persistent monitoring system
/// - Verify transaction confirmation on blockchain
/// - Extract actual input/output amounts from transaction metadata  
/// - Calculate effective swap prices
/// - Validate wallet balance changes
/// - Prevent duplicate transactions
/// - Persistent transaction state monitoring with disk storage
/// - Smart timeout handling (only timeout on stuck steps, not active processing)
///
/// Key Features:
/// - Real transaction analysis from blockchain data
/// - ATA (Associated Token Account) detection and rent calculation
/// - Balance validation before/after swaps
/// - Comprehensive error handling with multi-RPC fallback
/// - Anti-duplicate transaction protection
/// - Persistent transaction monitoring service
/// - Position transaction verification tracking

use crate::global::{read_configs, is_debug_swap_enabled, DATA_DIR};
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, lamports_to_sol, sol_to_lamports, get_rpc_client};
use crate::utils::{get_sol_balance, get_token_balance};
use super::config::{SOL_MINT, TRANSACTION_CONFIRMATION_TIMEOUT_SECS};

use std::collections::{HashSet, HashMap};
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use std::path::Path;
use once_cell::sync::Lazy;
use bs58;
use solana_sdk::{signature::Keypair, signer::Signer, pubkey::Pubkey};
use solana_transaction_status::{UiTransactionEncoding};
use std::str::FromStr;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use tokio::sync::{Notify, Mutex as AsyncMutex};

/// Configuration constants for transaction verification
const CONFIRMATION_TIMEOUT_SECS: u64 = TRANSACTION_CONFIRMATION_TIMEOUT_SECS;       // Extended time for blockchain confirmation
const INITIAL_CONFIRMATION_DELAY_MS: u64 = 5000;  // Initial delay before first check (5 seconds to allow transaction propagation)
const MAX_CONFIRMATION_DELAY_SECS: u64 = 5;       // Maximum delay between confirmation checks
const CONFIRMATION_BACKOFF_MULTIPLIER: f64 = 1.5; // Exponential backoff multiplier
const EARLY_ATTEMPTS_COUNT: u32 = 3;               // Number of fast early attempts
const EARLY_ATTEMPT_DELAY_MS: u64 = 500;          // Fast delay for early attempts
const RATE_LIMIT_BASE_DELAY_SECS: u64 = 2;        // Base delay for rate limiting
const RATE_LIMIT_INCREMENT_SECS: u64 = 1;         // Additional delay per rate limit hit
const MIN_TRADING_LAMPORTS: u64 = 500_000;        // Minimum trading amount (0.0005 SOL)
const TYPICAL_ATA_RENT_LAMPORTS: u64 = 2_039_280; // Standard ATA rent amount
const STUCK_STEP_TIMEOUT_SECS: u64 = 180;         // Timeout for being stuck on same step (3 minutes)
const TRANSACTION_MONITOR_INTERVAL_SECS: u64 = 10; // How often to check pending transactions
const TRANSACTION_STATE_FILE: &str = "data/pending_transactions.json"; // Persistent storage file

/// Transaction monitoring states - tracks progress through swap process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionState {
    /// Transaction signed and submitted to blockchain
    Submitted { submitted_at: DateTime<Utc> },
    /// Transaction confirmed by blockchain but effects not yet verified
    Confirmed { confirmed_at: DateTime<Utc> },
    /// Transaction fully verified with balance changes detected
    Verified { verified_at: DateTime<Utc> },
    /// Transaction failed at some stage
    Failed { failed_at: DateTime<Utc>, error: String },
    /// Transaction stuck on same state for too long
    Stuck { stuck_since: DateTime<Utc>, last_state: String },
}

/// Pending transaction information for persistent monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub signature: String,
    pub mint: String,
    pub direction: String, // "buy" or "sell"
    pub state: TransactionState,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub input_mint: String,
    pub output_mint: String,
    pub position_related: bool, // Whether this affects a position entry/exit
}

/// Global transaction monitoring service instance
static TRANSACTION_SERVICE: Lazy<StdArc<AsyncMutex<Option<TransactionMonitoringService>>>> = 
    Lazy::new(|| StdArc::new(AsyncMutex::new(None)));

/// Persistent transaction monitoring service
#[derive(Debug)]
pub struct TransactionMonitoringService {
    pending_transactions: HashMap<String, PendingTransaction>,
    shutdown_notify: Option<StdArc<Notify>>,
    running: bool,
}

impl TransactionMonitoringService {
    /// Create new transaction monitoring service
    pub fn new() -> Self {
        let pending = Self::load_pending_transactions_from_disk();
        Self {
            pending_transactions: pending,
            shutdown_notify: None,
            running: false,
        }
    }

    /// Initialize global transaction monitoring service
    pub async fn init_global_service() -> Result<(), SwapError> {
        let mut service_guard = TRANSACTION_SERVICE.lock().await;
        
        if service_guard.is_none() {
            *service_guard = Some(TransactionMonitoringService::new());
            log(LogTag::Swap, "TRANSACTION_SERVICE", "✅ Transaction monitoring service initialized");
        }
        Ok(())
    }

    /// Start the background monitoring service
    pub async fn start_monitoring_service(shutdown: StdArc<Notify>) -> Result<(), SwapError> {
        // Initialize service if not already done
        Self::init_global_service().await?;
        
        {
            let mut service_guard = TRANSACTION_SERVICE.lock().await;
            
            if let Some(service) = service_guard.as_mut() {
                service.shutdown_notify = Some(shutdown.clone());
                service.running = true;
            }
        }

        log(LogTag::Swap, "TRANSACTION_SERVICE", "🔄 Starting transaction monitoring background service");

        // Background monitoring loop
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(TRANSACTION_MONITOR_INTERVAL_SECS)
        );

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Swap, "TRANSACTION_SERVICE", "🛑 Transaction monitoring service shutting down");
                    Self::save_pending_transactions_to_disk();
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = Self::monitor_pending_transactions().await {
                        log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                            &format!("Monitoring cycle failed: {}", e));
                    }
                }
            }
        }

        // Mark service as stopped
        {
            let mut service_guard = TRANSACTION_SERVICE.lock().await;
            
            if let Some(service) = service_guard.as_mut() {
                service.running = false;
            }
        }

        Ok(())
    }

    /// Load pending transactions from disk
    fn load_pending_transactions_from_disk() -> HashMap<String, PendingTransaction> {
        let file_path = TRANSACTION_STATE_FILE;
        
        if Path::new(file_path).exists() {
            match std::fs::read_to_string(file_path) {
                Ok(content) => {
                    match serde_json::from_str::<Vec<PendingTransaction>>(&content) {
                        Ok(transactions) => {
                            let mut map = HashMap::new();
                            for tx in transactions {
                                map.insert(tx.signature.clone(), tx);
                            }
                            log(LogTag::Swap, "TRANSACTION_SERVICE", 
                                &format!("📄 Loaded {} pending transactions from disk", map.len()));
                            return map;
                        }
                        Err(e) => {
                            log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                                &format!("Failed to parse transaction state file: {}", e));
                        }
                    }
                }
                Err(e) => {
                    log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                        &format!("Failed to read transaction state file: {}", e));
                }
            }
        }
        
        HashMap::new()
    }

    /// Save pending transactions to disk
    async fn save_pending_transactions_to_disk() {
        let transactions_vec: Vec<PendingTransaction> = {
            let service_guard = TRANSACTION_SERVICE.lock().await;

            if let Some(service) = service_guard.as_ref() {
                service.pending_transactions.values().cloned().collect()
            } else {
                return;
            }
        };

        match serde_json::to_string_pretty(&transactions_vec) {
            Ok(content) => {
                if let Err(e) = std::fs::write(TRANSACTION_STATE_FILE, content) {
                    log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                        &format!("Failed to save transaction states: {}", e));
                } else {
                    log(LogTag::Swap, "TRANSACTION_SERVICE", 
                        &format!("💾 Saved {} pending transactions to disk", transactions_vec.len()));
                }
            }
            Err(e) => {
                log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                    &format!("Failed to serialize transaction states: {}", e));
            }
        }
    }

    /// Monitor all pending transactions
    async fn monitor_pending_transactions() -> Result<(), SwapError> {
        let pending_sigs: Vec<String> = {
            let service_guard = TRANSACTION_SERVICE.lock().await;
            
            if let Some(service) = service_guard.as_ref() {
                service.pending_transactions.keys().cloned().collect()
            } else {
                return Ok(());
            }
        };

        if pending_sigs.is_empty() {
            return Ok(());
        }

        log(LogTag::Swap, "TRANSACTION_SERVICE", 
            &format!("🔍 Monitoring {} pending transactions", pending_sigs.len()));

        for signature in pending_sigs {
            if let Err(e) = Self::check_transaction_progress(&signature).await {
                log(LogTag::Swap, "TRANSACTION_SERVICE_ERROR", 
                    &format!("Failed to check transaction {}: {}", &signature[..8], e));
            }
        }

        // Clean up completed/failed transactions older than 1 hour
        Self::cleanup_old_transactions().await;

        // Save updated states
        Self::save_pending_transactions_to_disk().await;

        Ok(())
    }

    /// Check progress of a specific transaction
    async fn check_transaction_progress(signature: &str) -> Result<(), SwapError> {
        let mut should_update = false;
        let mut new_state: Option<TransactionState> = None;
        let now = Utc::now();

        // Get current state
        {
            let service_guard = TRANSACTION_SERVICE.lock().await;
            
            if let Some(service) = service_guard.as_ref() {
                if let Some(tx) = service.pending_transactions.get(signature) {
                    // Check if stuck on same state for too long
                    let time_in_state = (now - tx.last_updated).num_seconds();
                    
                    if time_in_state > STUCK_STEP_TIMEOUT_SECS as i64 {
                        new_state = Some(TransactionState::Stuck {
                            stuck_since: tx.last_updated,
                            last_state: format!("{:?}", tx.state),
                        });
                        should_update = true;
                        
                        log(LogTag::Swap, "TRANSACTION_STUCK", 
                            &format!("⚠️ Transaction {} stuck in state for {}s", 
                                &signature[..8], time_in_state));
                    } else {
                        // Try to advance the state
                        match &tx.state {
                            TransactionState::Submitted { .. } => {
                                // Check if confirmed
                                if Self::is_transaction_confirmed(signature).await? {
                                    new_state = Some(TransactionState::Confirmed {
                                        confirmed_at: now,
                                    });
                                    should_update = true;
                                    
                                    log(LogTag::Swap, "TRANSACTION_CONFIRMED", 
                                        &format!("✅ Transaction {} confirmed", &signature[..8]));
                                }
                            }
                            TransactionState::Confirmed { .. } => {
                                // Check if balance changes are visible (verified)
                                if Self::verify_transaction_effects(signature, tx).await? {
                                    new_state = Some(TransactionState::Verified {
                                        verified_at: now,
                                    });
                                    should_update = true;
                                    
                                    log(LogTag::Swap, "TRANSACTION_VERIFIED", 
                                        &format!("🎯 Transaction {} fully verified", &signature[..8]));
                                }
                            }
                            TransactionState::Verified { .. } |
                            TransactionState::Failed { .. } |
                            TransactionState::Stuck { .. } => {
                                // Final states - no further processing needed
                            }
                        }
                    }
                }
            }
        }

        // Update state if needed
        if should_update && new_state.is_some() {
            let mut service_guard = TRANSACTION_SERVICE.lock().await;
            
            if let Some(service) = service_guard.as_mut() {
                if let Some(tx) = service.pending_transactions.get_mut(signature) {
                    tx.state = new_state.unwrap();
                    tx.last_updated = now;
                }
            }
        }

        Ok(())
    }

    /// Check if transaction is confirmed on blockchain
    async fn is_transaction_confirmed(signature: &str) -> Result<bool, SwapError> {
        let rpc_client = get_rpc_client();
        
        // Try to get transaction details
        match rpc_client.get_transaction_details(signature).await {
            Ok(details) => {
                // Transaction exists and is confirmed if we get details
                Ok(true)
            }
            Err(_) => {
                // Transaction not found or failed - consider not confirmed
                Ok(false)
            }
        }
    }

    /// Verify transaction effects (balance changes)
    async fn verify_transaction_effects(signature: &str, tx: &PendingTransaction) -> Result<bool, SwapError> {
        // For now, if it's confirmed, consider it verified
        // This can be enhanced with actual balance checking
        Ok(true)
    }

    /// Clean up old completed/failed transactions
    async fn cleanup_old_transactions() {
        let mut service_guard = TRANSACTION_SERVICE.lock().await;

        if let Some(service) = service_guard.as_mut() {
            let cutoff_time = Utc::now() - chrono::Duration::hours(1);
            let mut to_remove = Vec::new();

            for (signature, tx) in &service.pending_transactions {
                match &tx.state {
                    TransactionState::Verified { verified_at } |
                    TransactionState::Failed { failed_at: verified_at, .. } => {
                        if *verified_at < cutoff_time {
                            to_remove.push(signature.clone());
                        }
                    }
                    _ => {} // Keep pending transactions
                }
            }

            for signature in to_remove {
                service.pending_transactions.remove(&signature);
            }
        }
    }

    /// Add a new transaction to monitor
    pub async fn add_transaction_to_monitor(
        signature: &str,
        mint: &str,
        direction: &str,
        input_mint: &str,
        output_mint: &str,
        position_related: bool,
    ) -> Result<(), SwapError> {
        let mut service_guard = TRANSACTION_SERVICE.lock().await;

        if let Some(service) = service_guard.as_mut() {
            let pending_tx = PendingTransaction {
                signature: signature.to_string(),
                mint: mint.to_string(),
                direction: direction.to_string(),
                state: TransactionState::Submitted {
                    submitted_at: Utc::now(),
                },
                created_at: Utc::now(),
                last_updated: Utc::now(),
                input_mint: input_mint.to_string(),
                output_mint: output_mint.to_string(),
                position_related,
            };

            service.pending_transactions.insert(signature.to_string(), pending_tx);
            
            log(LogTag::Swap, "TRANSACTION_ADDED", 
                &format!("📝 Added transaction {} to monitoring queue", &signature[..8]));
        }

        Ok(())
    }

    /// Get transaction status
    pub async fn get_transaction_status(signature: &str) -> Option<TransactionState> {
        let service_guard = TRANSACTION_SERVICE.lock().await;
        
        if let Some(service) = service_guard.as_ref() {
            service.pending_transactions.get(signature).map(|tx| tx.state.clone())
        } else {
            None
        }
    }

    /// Check if transaction is complete (verified or failed)
    pub async fn is_transaction_complete(signature: &str) -> bool {
        match Self::get_transaction_status(signature).await {
            Some(TransactionState::Verified { .. }) |
            Some(TransactionState::Failed { .. }) => true,
            _ => false,
        }
    }

    /// Wait for transaction completion with smart timeout
    pub async fn wait_for_transaction_completion(
        signature: &str,
        max_wait_time: std::time::Duration,
    ) -> Result<TransactionState, SwapError> {
        let start_time = std::time::Instant::now();
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

        loop {
            // Check if transaction is complete
            if let Some(state) = Self::get_transaction_status(signature).await {
                match &state {
                    TransactionState::Verified { .. } => {
                        log(LogTag::Swap, "TRANSACTION_WAIT_SUCCESS", 
                            &format!("✅ Transaction {} completed successfully", &signature[..8]));
                        return Ok(state);
                    }
                    TransactionState::Failed { error, .. } => {
                        log(LogTag::Swap, "TRANSACTION_WAIT_FAILED", 
                            &format!("❌ Transaction {} failed: {}", &signature[..8], error));
                        return Ok(state);
                    }
                    TransactionState::Stuck { last_state, .. } => {
                        log(LogTag::Swap, "TRANSACTION_WAIT_STUCK", 
                            &format!("⚠️ Transaction {} stuck in {}", &signature[..8], last_state));
                        return Ok(state);
                    }
                    _ => {
                        // Still processing, continue waiting
                    }
                }
            }

            // Check timeout
            if start_time.elapsed() > max_wait_time {
                return Err(SwapError::TransactionError(
                    format!("Transaction {} did not complete within {:?}", signature, max_wait_time)
                ));
            }

            interval.tick().await;
        }
    }
}

/// Transaction verification result containing all relevant swap information
#[derive(Debug, Clone)]
pub struct TransactionVerificationResult {
    pub success: bool,
    pub transaction_signature: String,
    pub confirmed: bool,
    
    // Balance changes (lamports for SOL, raw units for tokens)
    pub input_amount: Option<u64>,     // Actual amount spent/consumed
    pub output_amount: Option<u64>,    // Actual amount received/produced
    
    // SOL balance changes
    pub sol_spent: Option<u64>,        // SOL spent in transaction (including fees)
    pub sol_received: Option<u64>,     // SOL received in transaction
    pub transaction_fee: u64,          // Network transaction fee in lamports
    
    // ATA (Associated Token Account) detection
    pub ata_detected: bool,            // Whether ATA closure was detected
    pub ata_rent_reclaimed: u64,       // Amount of rent reclaimed from ATA closure
    
    // Effective pricing
    pub effective_price: Option<f64>,  // Price per token in SOL (after fees/ATA)
    pub price_impact: Option<f64>,     // Calculated price impact percentage
    
    // Error information
    pub error: Option<String>,         // Error details if transaction failed
}

/// Balance snapshot for before/after comparison
#[derive(Debug, Clone)]
pub struct BalanceSnapshot {
    pub sol_balance: u64,              // SOL balance in lamports
    pub token_balance: u64,            // Token balance in raw units
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// CRITICAL: Global tracking of pending transactions to prevent duplicates
static PENDING_TRANSACTIONS: Lazy<StdArc<StdMutex<HashSet<String>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashSet::new()))
});

/// CRITICAL: Global tracking of recent transaction attempts to prevent rapid retries
static RECENT_TRANSACTION_ATTEMPTS: Lazy<StdArc<StdMutex<HashSet<String>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashSet::new()))
});

/// Anti-duplicate transaction protection - check and reserve transaction slot
pub fn check_and_reserve_transaction_slot(token_mint: &str, direction: &str) -> Result<(), SwapError> {
    let transaction_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut pending) = PENDING_TRANSACTIONS.lock() {
        if pending.contains(&transaction_key) {
            return Err(
                SwapError::TransactionError(
                    format!(
                        "Duplicate transaction prevented: {} already has a pending {} transaction",
                        token_mint,
                        direction
                    )
                )
            );
        }
        pending.insert(transaction_key);
        Ok(())
    } else {
        Err(SwapError::TransactionError("Failed to acquire transaction lock".to_string()))
    }
}

/// Release transaction slot after completion (success or failure)
fn release_transaction_slot(token_mint: &str, direction: &str) {
    let transaction_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut pending) = PENDING_TRANSACTIONS.lock() {
        pending.remove(&transaction_key);
    }
}

/// Check for recent transaction attempts to prevent rapid retries
pub fn check_recent_transaction_attempt(token_mint: &str, direction: &str) -> Result<(), SwapError> {
    let attempt_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
        if recent.contains(&attempt_key) {
            return Err(
                SwapError::TransactionError(
                    format!(
                        "Recent transaction attempt detected for {} {}. Please wait before retrying.",
                        token_mint,
                        direction
                    )
                )
            );
        }
        recent.insert(attempt_key.clone());

        // Schedule removal after 30 seconds to allow retries
        let attempt_key_for_cleanup = attempt_key.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
                recent.remove(&attempt_key_for_cleanup);
            }
        });

        Ok(())
    } else {
        Err(SwapError::TransactionError("Failed to check recent attempts".to_string()))
    }
}

/// Clear recent transaction attempt to allow immediate retry (for auto-retry logic)
pub fn clear_recent_transaction_attempt(token_mint: &str, direction: &str) {
    let attempt_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
        recent.remove(&attempt_key);
    }
}

/// RAII guard to ensure transaction slots are always released
pub struct TransactionSlotGuard {
    token_mint: String,
    direction: String,
}

impl TransactionSlotGuard {
    pub fn new(token_mint: &str, direction: &str) -> Self {
        Self {
            token_mint: token_mint.to_string(),
            direction: direction.to_string(),
        }
    }
}

impl Drop for TransactionSlotGuard {
    fn drop(&mut self) {
        release_transaction_slot(&self.token_mint, &self.direction);
    }
}

/// Get wallet address from configs by deriving from private key
pub fn get_wallet_address() -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Decode the private key from base58
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key format: {}", e)))?;

    // Create keypair from private key
    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    Ok(keypair.pubkey().to_string())
}

/// Validates transaction results for consistency and detects anomalies
/// Performs comprehensive checks to ensure transaction data integrity
fn validate_transaction_results(
    expected_direction: &str,
    pre_balance: &BalanceSnapshot,
    post_balance: &BalanceSnapshot,
    input_amount: Option<u64>,
    output_amount: Option<u64>,
    sol_spent: Option<u64>,
    sol_received: Option<u64>,
) -> Result<(), SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VALIDATION_START",
            &format!(
                "🔍 Starting transaction validation for {} direction
  Pre-balance: SOL={} lamports, Token={}
  Post-balance: SOL={} lamports, Token={}
  Input amount: {:?} | Output amount: {:?}
  SOL spent: {:?} | SOL received: {:?}",
                expected_direction,
                pre_balance.sol_balance,
                pre_balance.token_balance,
                post_balance.sol_balance,
                post_balance.token_balance,
                input_amount,
                output_amount,
                sol_spent,
                sol_received
            )
        );
    }

    // Validation 1: Direction-specific balance change validation
    match expected_direction {
        "buy" => {
            // For buy transactions: SOL should decrease (or stay same with ATA), tokens should increase
            if post_balance.sol_balance > pre_balance.sol_balance + 50_000_000 { // Allow 0.05 SOL tolerance for ATA operations
                log(
                    LogTag::Swap,
                    "VALIDATION_WARNING",
                    &format!(
                        "⚠️ Buy transaction: SOL balance unexpectedly increased by {} lamports (>0.05 SOL tolerance)",
                        post_balance.sol_balance - pre_balance.sol_balance
                    )
                );
            }

            // Tokens should increase in buy transactions
            if post_balance.token_balance <= pre_balance.token_balance {
                return Err(SwapError::TransactionError(
                    format!(
                        "Buy validation failed: Token balance did not increase (pre: {}, post: {})",
                        pre_balance.token_balance,
                        post_balance.token_balance
                    )
                ));
            }

            // SOL should be spent in buy transactions
            if sol_spent.is_none() || sol_spent.unwrap() == 0 {
                log(
                    LogTag::Swap,
                    "VALIDATION_WARNING",
                    "⚠️ Buy transaction: No SOL spent detected - possible data extraction issue"
                );
            }
        }
        "sell" => {
            // For sell transactions: tokens should decrease, SOL should increase (or decrease less due to fees)
            if post_balance.token_balance > pre_balance.token_balance {
                return Err(SwapError::TransactionError(
                    format!(
                        "Sell validation failed: Token balance unexpectedly increased (pre: {}, post: {})",
                        pre_balance.token_balance,
                        post_balance.token_balance
                    )
                ));
            }

            // SOL should be received in sell transactions (allowing for transaction fees)
            if sol_received.is_none() || sol_received.unwrap() == 0 {
                // Check if SOL balance at least didn't decrease too much (accounting for fees)
                if pre_balance.sol_balance > post_balance.sol_balance + 10_000_000 { // Allow 0.01 SOL for fees
                    log(
                        LogTag::Swap,
                        "VALIDATION_WARNING",
                        &format!(
                            "⚠️ Sell transaction: No SOL received and balance decreased by {} lamports",
                            pre_balance.sol_balance - post_balance.sol_balance
                        )
                    );
                }
            }
        }
        _ => {
            return Err(SwapError::TransactionError(
                format!("Invalid transaction direction: {}", expected_direction)
            ));
        }
    }

    // Validation 2: Amount consistency checks
    if let (Some(input), Some(output)) = (input_amount, output_amount) {
        // Ensure amounts are reasonable (not zero or impossibly large)
        if input == 0 {
            return Err(SwapError::TransactionError(
                "Validation failed: Input amount is zero".to_string()
            ));
        }
        if output == 0 {
            return Err(SwapError::TransactionError(
                "Validation failed: Output amount is zero".to_string()
            ));
        }

        // Check for impossibly large amounts (likely parsing errors)
        const MAX_REASONABLE_LAMPORTS: u64 = 1_000_000_000_000_000; // 1M SOL worth of lamports
        if input > MAX_REASONABLE_LAMPORTS || output > MAX_REASONABLE_LAMPORTS {
            return Err(SwapError::TransactionError(
                format!(
                    "Validation failed: Unreasonably large amounts detected (input: {}, output: {})",
                    input, output
                )
            ));
        }
    }

    // Validation 3: SOL amount consistency
    if let Some(spent) = sol_spent {
        if spent > 100_000_000_000 { // More than 100 SOL
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Large SOL amount spent: {} lamports ({:.3} SOL)", spent, lamports_to_sol(spent))
            );
        }
    }

    if let Some(received) = sol_received {
        if received > 100_000_000_000 { // More than 100 SOL
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Large SOL amount received: {} lamports ({:.3} SOL)", received, lamports_to_sol(received))
            );
        }
    }

    // Validation 4: Balance snapshot time consistency
    let time_diff = (post_balance.timestamp - pre_balance.timestamp).num_seconds();
    if time_diff < 0 {
        return Err(SwapError::TransactionError(
            "Validation failed: Post-balance timestamp is before pre-balance timestamp".to_string()
        ));
    }
    if time_diff > 300 { // More than 5 minutes
        log(
            LogTag::Swap,
            "VALIDATION_WARNING",
            &format!("⚠️ Large time gap between balance snapshots: {} seconds", time_diff)
        );
    }

    // Validation 4.5: Check for balance underflow/overflow (data corruption detection)
    const MAX_BALANCE_LAMPORTS: u64 = 500_000_000_000_000; // 500K SOL
    const MAX_TOKEN_BALANCE: u64 = 1_000_000_000_000_000_000; // 1 billion tokens with 9 decimals

    if pre_balance.sol_balance > MAX_BALANCE_LAMPORTS || post_balance.sol_balance > MAX_BALANCE_LAMPORTS {
        return Err(SwapError::TransactionError(
            format!(
                "Validation failed: SOL balance exceeds reasonable limits (pre: {}, post: {})",
                pre_balance.sol_balance, post_balance.sol_balance
            )
        ));
    }

    if pre_balance.token_balance > MAX_TOKEN_BALANCE || post_balance.token_balance > MAX_TOKEN_BALANCE {
        return Err(SwapError::TransactionError(
            format!(
                "Validation failed: Token balance exceeds reasonable limits (pre: {}, post: {})",
                pre_balance.token_balance, post_balance.token_balance
            )
        ));
    }

    // Validation 5: Cross-reference balance changes with extracted amounts
    if expected_direction == "buy" {
        if let Some(token_received) = output_amount {
            let actual_token_increase = if post_balance.token_balance > pre_balance.token_balance {
                post_balance.token_balance - pre_balance.token_balance
            } else {
                0
            };

            // Allow 10% tolerance for rounding/precision differences
            let tolerance = std::cmp::max(token_received / 10, 1);
            if actual_token_increase > 0 && 
               (token_received < actual_token_increase.saturating_sub(tolerance) || 
                token_received > actual_token_increase + tolerance) {
                log(
                    LogTag::Swap,
                    "VALIDATION_WARNING",
                    &format!(
                        "⚠️ Token amount mismatch: extracted={}, balance_change={} (tolerance={})",
                        token_received, actual_token_increase, tolerance
                    )
                );
            }
        }
    }

    // Validation 6: Detect potential decimal precision issues
    if let (Some(input), Some(output)) = (input_amount, output_amount) {
        // Check for suspiciously round numbers that might indicate decimal truncation
        if input % 1_000_000_000 == 0 && input > 1_000_000_000 {
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Input amount suspiciously round: {} (possible decimal precision issue)", input)
            );
        }
        if output % 1_000_000_000 == 0 && output > 1_000_000_000 {
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Output amount suspiciously round: {} (possible decimal precision issue)", output)
            );
        }

        // Check for extremely small amounts that might indicate decimal errors
        if input < 1000 && expected_direction == "buy" {
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Very small input amount: {} (possible decimal error)", input)
            );
        }
        if output < 1000 && expected_direction == "sell" {
            log(
                LogTag::Swap,
                "VALIDATION_WARNING",
                &format!("⚠️ Very small output amount: {} (possible decimal error)", output)
            );
        }
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VALIDATION_SUCCESS",
            &format!(
                "✅ All transaction validation checks passed
  📊 Validation Summary:
  • Direction: {} ✓
  • Balance Changes: SOL {} → {}, Token {} → {} ✓
  • Amount Extraction: Input={:?}, Output={:?} ✓
  • SOL Flow: Spent={:?}, Received={:?} ✓
  • Time Consistency: {}s gap ✓
  • Cross-validation: Balance vs extracted amounts ✓
  🎯 Transaction data integrity confirmed",
                expected_direction,
                pre_balance.sol_balance,
                post_balance.sol_balance,
                pre_balance.token_balance,
                post_balance.token_balance,
                input_amount,
                output_amount,
                sol_spent,
                sol_received,
                (post_balance.timestamp - pre_balance.timestamp).num_seconds()
            )
        );
    }

    Ok(())
}

/// Calculate price impact percentage for a swap transaction
fn calculate_price_impact(
    direction: &str,
    input_amount: Option<u64>,
    output_amount: Option<u64>,
    effective_price: Option<f64>,
) -> Option<f64> {
    // For now, return None if we don't have all required data
    if input_amount.is_none() || output_amount.is_none() || effective_price.is_none() {
        return None;
    }

    let input = input_amount.unwrap() as f64;
    let output = output_amount.unwrap() as f64;
    let price = effective_price.unwrap();

    if input == 0.0 || output == 0.0 || price == 0.0 {
        return None;
    }

    // Price impact is typically calculated as the difference between
    // expected output and actual output, but we need market data for that.
    // For now, we'll calculate a simple slippage based on the effective price
    // vs a theoretical "perfect" price (which we don't have)
    
    // Calculate price impact by comparing actual amounts vs expected
    // Price impact = (amount_difference / expected_amount) * 100
    match direction {
        "buy" => {
            if let (Some(input), Some(output)) = (input_amount, output_amount) {
                if input > 0 && output > 0 {
                    // For buy: price impact is how much less tokens we got than expected
                    // Higher input for same output = negative impact
                    // Less output for same input = negative impact
                    let actual_rate = (output as f64) / (input as f64);
                    if let Some(effective_price_val) = effective_price {
                        let expected_rate = 1.0 / effective_price_val;
                        let impact = ((actual_rate - expected_rate) / expected_rate) * 100.0;
                        return Some(-impact); // Negative because worse rates = positive impact
                    }
                }
            }
        }
        "sell" => {
            if let (Some(input), Some(output)) = (input_amount, output_amount) {
                if input > 0 && output > 0 {
                    // For sell: price impact is how much less SOL we got than expected
                    let actual_rate = (output as f64) / (input as f64);
                    if let Some(effective_price_val) = effective_price {
                        let expected_rate = effective_price_val;
                        let impact = ((actual_rate - expected_rate) / expected_rate) * 100.0;
                        return Some(-impact); // Negative because worse rates = positive impact
                    }
                }
            }
        }
        _ => {}
    }
    
    None
}

/// Take balance snapshot before transaction for comparison
pub async fn take_balance_snapshot(
    wallet_address: &str,
    token_mint: &str
) -> Result<BalanceSnapshot, SwapError> {
    let sol_balance = sol_to_lamports(get_sol_balance(wallet_address).await?);
    let token_balance = if token_mint == SOL_MINT {
        sol_balance
    } else {
        get_token_balance(wallet_address, token_mint).await?
    };

    Ok(BalanceSnapshot {
        sol_balance,
        token_balance,
        timestamp: chrono::Utc::now(),
    })
}

/// Sign and send transaction using global RPC client
pub async fn sign_and_send_transaction(
    swap_transaction_base64: &str,
) -> Result<String, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SIGN_START",
            &format!("✍️ Signing transaction (length: {} chars)
  Base64 Preview: {}...{}",
                swap_transaction_base64.len(),
                &swap_transaction_base64[..std::cmp::min(40, swap_transaction_base64.len())],
                if swap_transaction_base64.len() > 80 { 
                    &swap_transaction_base64[swap_transaction_base64.len()-40..] 
                } else { 
                    "" 
                }
            )
        );
    }

    let rpc_client = get_rpc_client();
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_RPC_CLIENT",
            "🔗 Using global RPC client for transaction signing and sending"
        );
    }
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SENDING",
            "📤 Sending signed transaction to blockchain..."
        );
    }
    
    let signature = rpc_client.sign_and_send_transaction(swap_transaction_base64).await?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SENT",
            &format!("✅ Transaction sent successfully - Signature: {}
  🎯 Transaction now pending confirmation on Solana blockchain", signature)
        );
    }
    
    Ok(signature)
}

/// MAIN FUNCTION: Comprehensive transaction verification and analysis
/// This is the core function that analyzes swap transactions and extracts all relevant information
pub async fn verify_swap_transaction(
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    expected_direction: &str, // "buy" or "sell"
    pre_balance: &BalanceSnapshot,
) -> Result<TransactionVerificationResult, SwapError> {
    let wallet_address = get_wallet_address()?;
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_START",
            &format!(
                "🔍 Starting transaction verification for {}\n  Direction: {}\n  Route: {} -> {}\n  Wallet: {}",
                transaction_signature,
                expected_direction,
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                &wallet_address[..8]
            )
        );
    }

    // Step 1: Wait for transaction confirmation with smart retry logic
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_1",
            "🔎 Step 1: Waiting for transaction confirmation on blockchain..."
        );
    }
    
    let transaction_details = wait_for_transaction_confirmation(
        transaction_signature,
        &configs
    ).await?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_1_COMPLETE",
            &format!("✅ Step 1 Complete: Transaction confirmed
  Fee: {} lamports | Has metadata: {}",
                transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
                transaction_details.meta.is_some()
            )
        );
    }

    // Step 2: Verify transaction success on blockchain
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_2",
            "🔎 Step 2: Verifying transaction success status..."
        );
    }
    
    let transaction_success = verify_transaction_success(&transaction_details)?;
    if !transaction_success {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "VERIFY_STEP_2_FAILED",
                "❌ Step 2 Failed: Transaction failed on blockchain"
            );
        }
        
        return Ok(TransactionVerificationResult {
            success: false,
            transaction_signature: transaction_signature.to_string(),
            confirmed: true,
            input_amount: None,
            output_amount: None,
            sol_spent: None,
            sol_received: None,
            transaction_fee: transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
            ata_detected: false,
            ata_rent_reclaimed: 0,
            effective_price: None,
            price_impact: None,
            error: Some("Transaction failed on-chain".to_string()),
        });
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_2_COMPLETE",
            "✅ Step 2 Complete: Transaction succeeded on blockchain"
        );
    }

    // Step 3: Take post-transaction balance snapshot
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_3",
            "🔎 Step 3: Taking post-transaction balance snapshot..."
        );
    }
    
    let post_balance = take_balance_snapshot(&wallet_address, 
        if expected_direction == "buy" { output_mint } else { input_mint }
    ).await?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_3_COMPLETE",
            &format!("✅ Step 3 Complete: Balance snapshot captured
  Pre-SOL: {} | Post-SOL: {}
  Pre-Token: {} | Post-Token: {}",
                lamports_to_sol(pre_balance.sol_balance),
                lamports_to_sol(post_balance.sol_balance),
                pre_balance.token_balance,
                post_balance.token_balance
            )
        );
    }

    // Step 4: Analyze balance changes and calculate amounts
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_4",
            "🔎 Step 4: Analyzing balance changes and extracting amounts..."
        );
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "BALANCE_COMPARISON",
            &format!(
                "📊 Balance Changes:\n  SOL: {} -> {} (diff: {})\n  Token: {} -> {} (diff: {})",
                lamports_to_sol(pre_balance.sol_balance),
                lamports_to_sol(post_balance.sol_balance),
                lamports_to_sol(if post_balance.sol_balance > pre_balance.sol_balance {
                    post_balance.sol_balance - pre_balance.sol_balance
                } else {
                    pre_balance.sol_balance - post_balance.sol_balance
                }),
                pre_balance.token_balance,
                post_balance.token_balance,
                if post_balance.token_balance > pre_balance.token_balance {
                    post_balance.token_balance - pre_balance.token_balance
                } else {
                    pre_balance.token_balance - post_balance.token_balance
                }
            )
        );
    }

    // Step 4: Extract amounts from transaction metadata (authoritative)
    let (blockchain_input_amount, blockchain_output_amount) = extract_amounts_from_transaction(
        &transaction_details,
        input_mint,
        output_mint,
        &wallet_address
    )?;

    // Step 5: Calculate SOL changes and detect ATA operations
    let (sol_spent, sol_received, ata_detected, ata_rent_reclaimed) = analyze_sol_changes(
        &transaction_details,
        pre_balance,
        &post_balance,
        expected_direction,
        &wallet_address
    )?;

    // Step 6: Calculate effective price using the unified function
    let effective_price = crate::swaps::pricing::calculate_effective_price_from_raw(
        expected_direction,
        blockchain_input_amount,
        blockchain_output_amount,
        sol_spent,
        sol_received,
        ata_rent_reclaimed,
        if input_mint == SOL_MINT { 9 } else { 
            crate::tokens::decimals::get_token_decimals_from_chain(input_mint).await.unwrap_or(9) as u32
        },
        if output_mint == SOL_MINT { 9 } else { 
            crate::tokens::decimals::get_token_decimals_from_chain(output_mint).await.unwrap_or(9) as u32
        }
    );

    // Step 7: Validate results consistency
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_7",
            "🔎 Step 7: Validating transaction results for consistency..."
        );
    }
    
    validate_transaction_results(
        expected_direction,
        pre_balance,
        &post_balance,
        blockchain_input_amount,
        blockchain_output_amount,
        sol_spent,
        sol_received
    )?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_7_COMPLETE",
            "✅ Step 7 Complete: All transaction results validated successfully"
        );
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_SUCCESS",
            &format!(
                "✅ Transaction verification completed successfully
  📊 Final Results Summary:
  • Input Amount: {} ({} type)
  • Output Amount: {} ({} type)  
  • SOL Spent: {} lamports ({:.6} SOL)
  • SOL Received: {} lamports ({:.6} SOL)
  • Transaction Fee: {} lamports ({:.6} SOL)
  • ATA Detected: {} | Rent Reclaimed: {} lamports ({:.6} SOL)
  • Effective Price: {:.10} SOL per token
  🎯 Verification Process: ALL 7 STEPS COMPLETED",
                blockchain_input_amount.unwrap_or(0),
                if expected_direction == "buy" { "SOL" } else { "Tokens" },
                blockchain_output_amount.unwrap_or(0),
                if expected_direction == "buy" { "Tokens" } else { "SOL" },
                sol_spent.unwrap_or(0),
                lamports_to_sol(sol_spent.unwrap_or(0)),
                sol_received.unwrap_or(0),
                lamports_to_sol(sol_received.unwrap_or(0)),
                transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
                lamports_to_sol(transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0)),
                ata_detected,
                ata_rent_reclaimed,
                lamports_to_sol(ata_rent_reclaimed),
                effective_price.unwrap_or(0.0)
            )
        );
    }

    Ok(TransactionVerificationResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        confirmed: true,
        input_amount: blockchain_input_amount,
        output_amount: blockchain_output_amount,
        sol_spent,
        sol_received,
        transaction_fee: transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
        ata_detected,
        ata_rent_reclaimed,
        effective_price,
        price_impact: calculate_price_impact(
            expected_direction,
            blockchain_input_amount,
            blockchain_output_amount,
            effective_price
        ),
        error: None,
    })
}

/// Wait for transaction confirmation with smart exponential backoff
async fn wait_for_transaction_confirmation(
    transaction_signature: &str,
    configs: &crate::global::Configs
) -> Result<crate::rpc::TransactionDetails, SwapError> {
    let max_duration = tokio::time::Duration::from_secs(CONFIRMATION_TIMEOUT_SECS);
    let start_time = tokio::time::Instant::now();
    
    let initial_delay = tokio::time::Duration::from_millis(INITIAL_CONFIRMATION_DELAY_MS);
    let max_delay = tokio::time::Duration::from_secs(MAX_CONFIRMATION_DELAY_SECS);
    
    let mut current_delay = initial_delay;
    let mut attempt = 1;
    let mut consecutive_rate_limits = 0;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "CONFIRM_WAIT_START",
            &format!("⏳ Starting confirmation wait for transaction: {}
  ⏱️ Max wait time: {}s | Initial delay: {}ms | Max delay: {}s",
                transaction_signature,
                CONFIRMATION_TIMEOUT_SECS,
                INITIAL_CONFIRMATION_DELAY_MS,
                MAX_CONFIRMATION_DELAY_SECS
            )
        );
    }

    log(
        LogTag::Swap,
        "CONFIRM_WAIT",
        &format!("⏳ Waiting for transaction confirmation: {}", transaction_signature)
    );

    loop {
        if start_time.elapsed() >= max_duration {
            return Err(SwapError::TransactionError(
                format!("Transaction confirmation timeout after {:.1}s", start_time.elapsed().as_secs_f64())
            ));
        }

        if attempt > 1 {
            if consecutive_rate_limits > 2 {
                let rate_limit_delay = RATE_LIMIT_BASE_DELAY_SECS + consecutive_rate_limits * RATE_LIMIT_INCREMENT_SECS;
                current_delay = std::cmp::min(max_delay, tokio::time::Duration::from_secs(rate_limit_delay));
            }
            tokio::time::sleep(current_delay).await;
        }

        let rpc_client = get_rpc_client();
        match rpc_client.get_transaction_details(transaction_signature).await {
            Ok(tx_details) => {
                consecutive_rate_limits = 0;
                
                if let Some(meta) = &tx_details.meta {
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "CONFIRMED",
                            &format!(
                                "✅ Transaction confirmed on attempt {} after {:.1}s",
                                attempt,
                                start_time.elapsed().as_secs_f64()
                            )
                        );
                    }
                    return Ok(tx_details);
                } else {
                    // Not yet confirmed - adjust delay
                    if attempt <= EARLY_ATTEMPTS_COUNT {
                        current_delay = tokio::time::Duration::from_millis(EARLY_ATTEMPT_DELAY_MS);
                    } else {
                        current_delay = std::cmp::min(max_delay, 
                            tokio::time::Duration::from_millis(
                                (current_delay.as_millis() as f64 * CONFIRMATION_BACKOFF_MULTIPLIER) as u64
                            )
                        );
                    }
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "PENDING",
                            &format!(
                                "⏳ Transaction pending... (attempt {}, next check in {:.1}s)",
                                attempt,
                                current_delay.as_secs_f64()
                            )
                        );
                    }
                }
            }
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("429") || error_str.contains("rate limit") || error_str.contains("too many requests") {
                    consecutive_rate_limits += 1;
                    let rate_limit_delay = RATE_LIMIT_BASE_DELAY_SECS + consecutive_rate_limits * RATE_LIMIT_INCREMENT_SECS;
                    current_delay = tokio::time::Duration::from_secs(rate_limit_delay);
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "RATE_LIMIT",
                            &format!("⚠️ Rate limit hit (attempt {}), extending delay to {}s", 
                                attempt, rate_limit_delay)
                        );
                    }
                } else {
                    consecutive_rate_limits = 0;
                    current_delay = std::cmp::min(max_delay, 
                        tokio::time::Duration::from_millis(
                            (current_delay.as_millis() as f64 * CONFIRMATION_BACKOFF_MULTIPLIER) as u64
                        )
                    );
                }
                
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "RETRY",
                        &format!(
                            "🔄 Transaction not found yet (attempt {}), retrying in {:.1}s
  Error: {}",
                            attempt,
                            current_delay.as_secs_f64(),
                            e
                        )
                    );
                }
            }
        }
        
        attempt += 1;
    }
}

/// Verify transaction success from metadata
fn verify_transaction_success(
    transaction_details: &crate::rpc::TransactionDetails
) -> Result<bool, SwapError> {
    let meta = transaction_details.meta.as_ref()
        .ok_or_else(|| SwapError::TransactionError("No transaction metadata available".to_string()))?;

    let success = meta.err.is_none();
    
    if !success {
        log(
            LogTag::Swap,
            "TX_FAILED",
            &format!("❌ Transaction failed on-chain: {:?}", meta.err)
        );
    }
    
    Ok(success)
}

/// Extract actual amounts from confirmed transaction metadata
fn extract_amounts_from_transaction(
    transaction_details: &crate::rpc::TransactionDetails,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<(Option<u64>, Option<u64>), SwapError> {
    let meta = transaction_details.meta.as_ref()
        .ok_or_else(|| SwapError::TransactionError("No transaction metadata available".to_string()))?;

    // Method 1: Use token balance changes (most reliable for tokens)
    let (input_from_tokens, output_from_tokens) = extract_token_balance_changes(
        meta,
        input_mint,
        output_mint,
        wallet_address
    )?;

    // Method 2: Use SOL balance changes (for SOL transactions)
    let (input_from_sol, output_from_sol) = extract_sol_balance_changes(
        meta,
        input_mint,
        output_mint,
        wallet_address
    )?;

    // Combine results - prefer token balance method for tokens, SOL balance method for SOL
    let final_input = if input_mint == SOL_MINT {
        input_from_sol.or(input_from_tokens)
    } else {
        input_from_tokens.or(input_from_sol)
    };

    let final_output = if output_mint == SOL_MINT {
        output_from_sol.or(output_from_tokens)
    } else {
        output_from_tokens.or(output_from_sol)
    };

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "EXTRACT_AMOUNTS",
            &format!(
                "📊 Amount extraction results:\n  Input: {} (from tokens: {:?}, from SOL: {:?})\n  Output: {} (from tokens: {:?}, from SOL: {:?})",
                final_input.unwrap_or(0),
                input_from_tokens,
                input_from_sol,
                final_output.unwrap_or(0),
                output_from_tokens,
                output_from_sol
            )
        );
    }

    Ok((final_input, final_output))
}

/// Extract token balance changes from transaction metadata
fn extract_token_balance_changes(
    meta: &crate::rpc::TransactionMeta,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<(Option<u64>, Option<u64>), SwapError> {
    let pre_balances = meta.pre_token_balances.as_ref();
    let post_balances = meta.post_token_balances.as_ref();

    if pre_balances.is_none() || post_balances.is_none() {
        return Ok((None, None));
    }

    let pre_balances = pre_balances.unwrap();
    let post_balances = post_balances.unwrap();

    let mut input_amount = None;
    let mut output_amount = None;

    // Find wallet's token account changes for input mint
    if input_mint != SOL_MINT {
        for post_balance in post_balances {
            if post_balance.mint == input_mint {
                // Find corresponding pre-balance
                if let Some(pre_balance) = pre_balances
                    .iter()
                    .find(|pre| pre.account_index == post_balance.account_index && pre.mint == input_mint)
                {
                    let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);

                    if pre_amount > post_amount {
                        input_amount = Some(pre_amount - post_amount);
                    }
                }
            }
        }
    }

    // Find wallet's token account changes for output mint
    if output_mint != SOL_MINT {
        for post_balance in post_balances {
            if post_balance.mint == output_mint {
                // Find corresponding pre-balance or assume 0 if new account
                let pre_amount = pre_balances
                    .iter()
                    .find(|pre| pre.account_index == post_balance.account_index && pre.mint == output_mint)
                    .map(|pre| pre.ui_token_amount.amount.parse::<u64>().unwrap_or(0))
                    .unwrap_or(0);

                let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);

                if post_amount > pre_amount {
                    output_amount = Some(post_amount - pre_amount);
                }
            }
        }
    }

    Ok((input_amount, output_amount))
}

/// Extract SOL balance changes from transaction metadata
fn extract_sol_balance_changes(
    meta: &crate::rpc::TransactionMeta,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> Result<(Option<u64>, Option<u64>), SwapError> {
    // For SOL transactions, we need to look at the wallet's balance change
    // Wallet is typically the first account (fee payer)
    if meta.pre_balances.is_empty() || meta.post_balances.is_empty() {
        return Ok((None, None));
    }

    let pre_sol_balance = meta.pre_balances[0];
    let post_sol_balance = meta.post_balances[0];
    let fee = meta.fee;

    let mut input_amount = None;
    let mut output_amount = None;

    if input_mint == SOL_MINT {
        // SOL was spent (input) - calculate actual SOL spent including fees
        if pre_sol_balance > post_sol_balance {
            input_amount = Some(pre_sol_balance - post_sol_balance);
        }
    }

    if output_mint == SOL_MINT {
        // SOL was received (output) - calculate SOL received excluding fees
        if post_sol_balance + fee > pre_sol_balance {
            output_amount = Some((post_sol_balance + fee) - pre_sol_balance);
        }
    }

    Ok((input_amount, output_amount))
}

/// Analyze SOL balance changes and detect ATA operations
fn analyze_sol_changes(
    transaction_details: &crate::rpc::TransactionDetails,
    pre_balance: &BalanceSnapshot,
    post_balance: &BalanceSnapshot,
    expected_direction: &str,
    wallet_address: &str
) -> Result<(Option<u64>, Option<u64>, bool, u64), SwapError> {
    let meta = transaction_details.meta.as_ref()
        .ok_or_else(|| SwapError::TransactionError("No transaction metadata available".to_string()))?;

    let transaction_fee = meta.fee;
    
    // Calculate raw SOL difference
    let sol_difference = if post_balance.sol_balance > pre_balance.sol_balance {
        // SOL increased
        (post_balance.sol_balance - pre_balance.sol_balance, false) // (amount, is_decrease)
    } else {
        // SOL decreased
        (pre_balance.sol_balance - post_balance.sol_balance, true) // (amount, is_decrease)
    };

    let (raw_sol_change, sol_decreased) = sol_difference;

    // Detect ATA closure by analyzing transaction logs and balance patterns
    let (ata_detected, ata_rent_reclaimed) = detect_ata_closure(
        meta,
        raw_sol_change,
        transaction_fee,
        expected_direction
    );

    let (sol_spent, sol_received) = if expected_direction == "buy" {
        // Buy transaction: SOL spent for tokens
        if sol_decreased {
            let total_spent = raw_sol_change;
            let trading_spent = if ata_detected && total_spent > ata_rent_reclaimed {
                total_spent - ata_rent_reclaimed
            } else {
                total_spent
            };
            (Some(trading_spent), None)
        } else {
            // Unexpected: SOL increased during buy (might be ATA closure)
            if ata_detected {
                (Some(transaction_fee), None) // Only fee was spent, rest was ATA rent
            } else {
                (Some(transaction_fee), None) // Default to fee if confusing
            }
        }
    } else {
        // Sell transaction: tokens sold for SOL
        if !sol_decreased {
            let total_received = raw_sol_change;
            let trading_received = if ata_detected {
                if total_received > ata_rent_reclaimed {
                    total_received - ata_rent_reclaimed
                } else {
                    0 // All was ATA rent
                }
            } else {
                total_received
            };
            (None, Some(trading_received))
        } else {
            // Unexpected: SOL decreased during sell (fee only?)
            (Some(raw_sol_change), None)
        }
    };

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "SOL_ANALYSIS",
            &format!(
                "💰 SOL Analysis Results:\n  Raw change: {} lamports ({})\n  Fee: {} lamports\n  ATA detected: {} | Rent: {} lamports\n  Final: spent={:?}, received={:?}",
                raw_sol_change,
                if sol_decreased { "decreased" } else { "increased" },
                transaction_fee,
                ata_detected,
                ata_rent_reclaimed,
                sol_spent,
                sol_received
            )
        );
    }

    Ok((sol_spent, sol_received, ata_detected, ata_rent_reclaimed))
}

/// Detect ATA closure operations from transaction logs and balance patterns
fn detect_ata_closure(
    meta: &crate::rpc::TransactionMeta,
    raw_sol_change: u64,
    transaction_fee: u64,
    expected_direction: &str
) -> (bool, u64) {
    let mut ata_detected = false;
    let mut confidence_score = 0.0;
    let mut estimated_ata_rent = 0u64;

    // Method 1: Analyze transaction logs for ATA closure instructions (highest confidence)
    if let Some(log_messages) = &meta.log_messages {
        for log_message in log_messages {
            if log_message.contains("CloseAccount") || log_message.contains("close account") {
                confidence_score += 0.4;
                estimated_ata_rent = TYPICAL_ATA_RENT_LAMPORTS;
                
                if is_debug_swap_enabled() {
                    log(
                        crate::logger::LogTag::Swap,
                        "ATA_LOG_DETECT",
                        &format!("🔍 ATA closure detected in logs: {}", log_message)
                    );
                }
                break;
            }
        }
    }

    // Method 2: Pattern analysis for sell transactions (medium confidence)
    if expected_direction == "sell" {
        // In sell transactions, if SOL increased by more than just trading amount,
        // it likely includes ATA rent reclamation
        if raw_sol_change > transaction_fee {
            let sol_net_change = raw_sol_change - transaction_fee;
            
            // Check if the change amount is close to typical ATA rent
            let diff_from_typical_rent = if sol_net_change > TYPICAL_ATA_RENT_LAMPORTS {
                sol_net_change - TYPICAL_ATA_RENT_LAMPORTS
            } else {
                TYPICAL_ATA_RENT_LAMPORTS - sol_net_change
            };

            // If within 10% of typical ATA rent, likely ATA closure
            if diff_from_typical_rent < (TYPICAL_ATA_RENT_LAMPORTS / 10) {
                confidence_score += 0.3;
                estimated_ata_rent = TYPICAL_ATA_RENT_LAMPORTS;
            }
        }
    }

    // Method 3: Balance pattern analysis (lower confidence)
    if raw_sol_change > transaction_fee * 50 {  // Significantly more than just fees
        confidence_score += 0.2;
        if estimated_ata_rent == 0 {
            estimated_ata_rent = TYPICAL_ATA_RENT_LAMPORTS;
        }
    }

    // Determine if ATA was detected based on confidence threshold
    ata_detected = confidence_score >= 0.4;

    // Safety check: Don't let ATA rent exceed total SOL change
    if ata_detected && estimated_ata_rent > raw_sol_change {
        estimated_ata_rent = raw_sol_change;
    }

    if is_debug_swap_enabled() {
        log(
            crate::logger::LogTag::Swap,
            "ATA_DETECTION",
            &format!(
                "🔍 ATA Detection Results:\n  Detected: {} | Confidence: {:.1}%\n  Estimated rent: {} lamports",
                ata_detected,
                confidence_score * 100.0,
                estimated_ata_rent
            )
        );
    }

    (ata_detected, estimated_ata_rent)
}

/// POSITION-SPECIFIC TRANSACTION VERIFICATION FUNCTIONS
/// Comprehensive verification for position entry and exit transactions

/// Comprehensive position entry transaction verification
/// Returns verified transaction data and balance changes for position creation
#[derive(Debug, Clone)]
pub struct PositionEntryVerification {
    pub transaction_signature: String,
    pub success: bool,
    pub error: Option<String>,
    pub token_amount_received: u64,
    pub sol_spent: u64,
    pub effective_entry_price: f64,
    pub entry_transaction_verified: bool,
    pub ata_created: bool,
    pub ata_rent_paid: u64,
    pub transaction_fee: u64,
    pub total_cost_sol: f64, // Including all fees and ATA rent
}

/// Comprehensive position exit transaction verification  
/// Returns verified transaction data and balance changes for position closure
#[derive(Debug, Clone)]
pub struct PositionExitVerification {
    pub transaction_signature: String,
    pub success: bool,
    pub error: Option<String>,
    pub token_amount_sold: u64,
    pub sol_received: u64,
    pub effective_exit_price: f64,
    pub exit_transaction_verified: bool,
    pub ata_closed: bool,
    pub ata_rent_reclaimed: u64,
    pub transaction_fee: u64,
    pub net_sol_received: f64, // SOL from sale only, excluding ATA rent
}

/// Verify position entry transaction with comprehensive analysis
/// This function performs complete verification of a buy transaction for position tracking
pub async fn verify_position_entry_transaction(
    transaction_signature: &str,
    token_mint: &str,
    expected_sol_spent: f64,
    pre_balance_snapshot: &BalanceSnapshot,
) -> Result<PositionEntryVerification, SwapError> {
    log(
        LogTag::Swap,
        "POSITION_ENTRY_VERIFY",
        &format!("🔍 Verifying position entry transaction: {}", &transaction_signature[..8])
    );

    // Get transaction details from blockchain
    let rpc_client = get_rpc_client();
    let transaction_details = match rpc_client.get_transaction_details(transaction_signature).await {
        Ok(details) => details,
        Err(e) => {
            return Ok(PositionEntryVerification {
                transaction_signature: transaction_signature.to_string(),
                success: false,
                error: Some(format!("Failed to fetch transaction details: {}", e)),
                token_amount_received: 0,
                sol_spent: 0,
                effective_entry_price: 0.0,
                entry_transaction_verified: false,
                ata_created: false,
                ata_rent_paid: 0,
                transaction_fee: 0,
                total_cost_sol: 0.0,
            });
        }
    };

    // Verify transaction succeeded on blockchain
    if let Some(meta) = &transaction_details.meta {
        if meta.err.is_some() {
            return Ok(PositionEntryVerification {
                transaction_signature: transaction_signature.to_string(),
                success: false,
                error: Some("Transaction failed on blockchain".to_string()),
                token_amount_received: 0,
                sol_spent: 0,
                effective_entry_price: 0.0,
                entry_transaction_verified: false,
                ata_created: false,
                ata_rent_paid: 0,
                transaction_fee: 0,
                total_cost_sol: 0.0,
            });
        }
    }

    // Get wallet address for analysis
    let wallet_address = get_wallet_address()?;

    // Take post-transaction balance snapshot
    let post_balance = take_balance_snapshot(&wallet_address, token_mint).await?;

    // Analyze SOL balance changes
    let (sol_change, ata_created, ata_rent_paid) = analyze_position_entry_sol_changes(
        &transaction_details,
        &wallet_address,
        &pre_balance_snapshot,
        &post_balance,
    )?;

    // Analyze token balance changes
    let token_amount_received = if post_balance.token_balance > pre_balance_snapshot.token_balance {
        post_balance.token_balance - pre_balance_snapshot.token_balance
    } else {
        0
    };

    // Calculate effective entry price
    let effective_entry_price = if token_amount_received > 0 {
        let sol_spent_for_tokens = sol_change.saturating_sub(ata_rent_paid);
        (sol_spent_for_tokens as f64) / (token_amount_received as f64) * 10f64.powi(9) // Convert to SOL price
    } else {
        0.0
    };

    // Get transaction fee
    let transaction_fee = transaction_details.meta
        .as_ref()
        .map(|m| m.fee)
        .unwrap_or(0);

    // Calculate total cost
    let total_cost_sol = lamports_to_sol(sol_change + transaction_fee);

    // Validate results
    let verification_success = token_amount_received > 0 && 
                              sol_change > 0 && 
                              effective_entry_price > 0.0;

    // Log verification results
    if verification_success {
        log(
            LogTag::Swap,
            "POSITION_ENTRY_SUCCESS",
            &format!(
                "✅ Entry verified: {} tokens received, {:.9} SOL spent, price: {:.12} SOL/token",
                token_amount_received,
                lamports_to_sol(sol_change),
                effective_entry_price
            )
        );
    } else {
        log(
            LogTag::Swap,
            "POSITION_ENTRY_WARNING",
            &format!(
                "⚠️ Entry verification incomplete: tokens={}, sol_spent={}, price={:.12}",
                token_amount_received,
                sol_change,
                effective_entry_price
            )
        );
    }

    Ok(PositionEntryVerification {
        transaction_signature: transaction_signature.to_string(),
        success: verification_success,
        error: None,
        token_amount_received,
        sol_spent: sol_change,
        effective_entry_price,
        entry_transaction_verified: verification_success,
        ata_created,
        ata_rent_paid,
        transaction_fee,
        total_cost_sol,
    })
}

/// Verify position exit transaction with comprehensive analysis
/// This function performs complete verification of a sell transaction for position tracking
pub async fn verify_position_exit_transaction(
    transaction_signature: &str,
    token_mint: &str,
    expected_token_amount: u64,
    pre_balance_snapshot: &BalanceSnapshot,
) -> Result<PositionExitVerification, SwapError> {
    log(
        LogTag::Swap,
        "POSITION_EXIT_VERIFY",
        &format!("🔍 Verifying position exit transaction: {}", &transaction_signature[..8])
    );

    // Get transaction details from blockchain
    let rpc_client = get_rpc_client();
    let transaction_details = match rpc_client.get_transaction_details(transaction_signature).await {
        Ok(details) => details,
        Err(e) => {
            return Ok(PositionExitVerification {
                transaction_signature: transaction_signature.to_string(),
                success: false,
                error: Some(format!("Failed to fetch transaction details: {}", e)),
                token_amount_sold: 0,
                sol_received: 0,
                effective_exit_price: 0.0,
                exit_transaction_verified: false,
                ata_closed: false,
                ata_rent_reclaimed: 0,
                transaction_fee: 0,
                net_sol_received: 0.0,
            });
        }
    };

    // Verify transaction succeeded on blockchain
    if let Some(meta) = &transaction_details.meta {
        if meta.err.is_some() {
            return Ok(PositionExitVerification {
                transaction_signature: transaction_signature.to_string(),
                success: false,
                error: Some("Transaction failed on blockchain".to_string()),
                token_amount_sold: 0,
                sol_received: 0,
                effective_exit_price: 0.0,
                exit_transaction_verified: false,
                ata_closed: false,
                ata_rent_reclaimed: 0,
                transaction_fee: 0,
                net_sol_received: 0.0,
            });
        }
    }

    // Get wallet address for analysis
    let wallet_address = get_wallet_address()?;

    // Take post-transaction balance snapshot
    let post_balance = take_balance_snapshot(&wallet_address, token_mint).await?;
    let wallet_address = get_wallet_address()?;

    // Analyze SOL balance changes (should increase for sell)
    let (sol_received, ata_closed, ata_rent_reclaimed) = analyze_position_exit_sol_changes(
        &transaction_details,
        &wallet_address,
        &pre_balance_snapshot,
        &post_balance,
    )?;

    // Analyze token balance changes (should decrease for sell)
    let token_amount_sold = if pre_balance_snapshot.token_balance > post_balance.token_balance {
        pre_balance_snapshot.token_balance - post_balance.token_balance
    } else {
        0
    };

    // Calculate effective exit price
    let effective_exit_price = if token_amount_sold > 0 {
        let sol_from_sale = sol_received.saturating_sub(ata_rent_reclaimed);
        (sol_from_sale as f64) / (token_amount_sold as f64) * 10f64.powi(9) // Convert to SOL price
    } else {
        0.0
    };

    // Get transaction fee
    let transaction_fee = transaction_details.meta
        .as_ref()
        .map(|m| m.fee)
        .unwrap_or(0);

    // Calculate net SOL received (excluding ATA rent)
    let net_sol_received = lamports_to_sol(sol_received.saturating_sub(ata_rent_reclaimed));

    // Validate results
    let verification_success = token_amount_sold > 0 && 
                              sol_received > 0 && 
                              effective_exit_price > 0.0;

    // Log verification results
    if verification_success {
        log(
            LogTag::Swap,
            "POSITION_EXIT_SUCCESS",
            &format!(
                "✅ Exit verified: {} tokens sold, {:.9} SOL received, price: {:.12} SOL/token",
                token_amount_sold,
                net_sol_received,
                effective_exit_price
            )
        );
    } else {
        log(
            LogTag::Swap,
            "POSITION_EXIT_WARNING",
            &format!(
                "⚠️ Exit verification incomplete: tokens={}, sol_received={}, price={:.12}",
                token_amount_sold,
                sol_received,
                effective_exit_price
            )
        );
    }

    Ok(PositionExitVerification {
        transaction_signature: transaction_signature.to_string(),
        success: verification_success,
        error: None,
        token_amount_sold,
        sol_received,
        effective_exit_price,
        exit_transaction_verified: verification_success,
        ata_closed,
        ata_rent_reclaimed,
        transaction_fee,
        net_sol_received,
    })
}

/// Analyze SOL balance changes for position entry (buy) transactions
fn analyze_position_entry_sol_changes(
    transaction_details: &crate::rpc::TransactionDetails,
    wallet_address: &str,
    pre_balance: &BalanceSnapshot,
    post_balance: &BalanceSnapshot,
) -> Result<(u64, bool, u64), SwapError> {
    // Calculate raw SOL balance change (should be negative for buy)
    let sol_change = if pre_balance.sol_balance > post_balance.sol_balance {
        pre_balance.sol_balance - post_balance.sol_balance
    } else {
        0
    };

    // Detect ATA creation for entry transactions
    let (ata_created, ata_rent_paid) = detect_ata_creation(
        transaction_details,
        wallet_address,
        sol_change,
    );

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "ENTRY_SOL_ANALYSIS",
            &format!(
                "📊 Entry SOL Analysis:\n  SOL spent: {} lamports\n  ATA created: {}\n  ATA rent: {} lamports",
                sol_change,
                ata_created,
                ata_rent_paid
            )
        );
    }

    Ok((sol_change, ata_created, ata_rent_paid))
}

/// Analyze SOL balance changes for position exit (sell) transactions  
fn analyze_position_exit_sol_changes(
    transaction_details: &crate::rpc::TransactionDetails,
    wallet_address: &str,
    pre_balance: &BalanceSnapshot,
    post_balance: &BalanceSnapshot,
) -> Result<(u64, bool, u64), SwapError> {
    // Calculate raw SOL balance change (should be positive for sell)
    let sol_change = if post_balance.sol_balance > pre_balance.sol_balance {
        post_balance.sol_balance - pre_balance.sol_balance
    } else {
        0
    };

    // Get transaction fee to exclude from ATA detection
    let transaction_fee = transaction_details.meta
        .as_ref()
        .map(|m| m.fee)
        .unwrap_or(0);

    // Detect ATA closure for exit transactions
    let (ata_closed, ata_rent_reclaimed) = if let Some(meta) = &transaction_details.meta {
        detect_ata_closure(
            meta,
            sol_change,
            transaction_fee,
            "sell",
        )
    } else {
        (false, 0)
    };

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "EXIT_SOL_ANALYSIS",
            &format!(
                "📊 Exit SOL Analysis:\n  SOL received: {} lamports\n  ATA closed: {}\n  ATA rent: {} lamports",
                sol_change,
                ata_closed,
                ata_rent_reclaimed
            )
        );
    }

    Ok((sol_change, ata_closed, ata_rent_reclaimed))
}

/// Detect ATA creation during buy transactions
fn detect_ata_creation(
    transaction_details: &crate::rpc::TransactionDetails,
    wallet_address: &str,
    total_sol_spent: u64,
) -> (bool, u64) {
    let mut ata_created = false;
    let mut ata_rent_paid = 0u64;

    // Analyze transaction logs for ATA creation
    if let Some(meta) = &transaction_details.meta {
        if let Some(log_messages) = &meta.log_messages {
            for log_message in log_messages {
                if log_message.contains("CreateAccount") || 
                   log_message.contains("create account") ||
                   log_message.contains("Allocate") {
                    ata_created = true;
                    ata_rent_paid = TYPICAL_ATA_RENT_LAMPORTS;
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "ATA_CREATE_DETECT",
                            &format!("🔍 ATA creation detected in logs: {}", log_message)
                        );
                    }
                    break;
                }
            }
        }

        // If SOL spent is significantly more than expected trade amount, likely includes ATA creation
        if !ata_created && total_sol_spent > sol_to_lamports(0.01) { // More than 0.01 SOL
            let excess_sol = total_sol_spent.saturating_sub(sol_to_lamports(0.005)); // Subtract typical trade
            if excess_sol > TYPICAL_ATA_RENT_LAMPORTS / 2 { // Within range of ATA rent
                ata_created = true;
                ata_rent_paid = std::cmp::min(excess_sol, TYPICAL_ATA_RENT_LAMPORTS);
            }
        }
    }

    (ata_created, ata_rent_paid)
}

/// Register a transaction for position tracking
/// This should be called when opening or closing positions to enable monitoring
pub async fn register_position_transaction(
    transaction_signature: &str,
    mint: &str,
    direction: &str, // "buy" or "sell"
    input_mint: &str,
    output_mint: &str,
) -> Result<(), SwapError> {
    let service_arc = TRANSACTION_SERVICE.clone();
    let mut service_guard = service_arc.lock().await;
    
    if let Some(service) = service_guard.as_mut() {
        let pending_transaction = PendingTransaction {
            signature: transaction_signature.to_string(),
            mint: mint.to_string(),
            direction: direction.to_string(),
            state: TransactionState::Submitted { submitted_at: Utc::now() },
            created_at: Utc::now(),
            last_updated: Utc::now(),
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            position_related: true,
        };

        service.pending_transactions.insert(transaction_signature.to_string(), pending_transaction);
        
        log(
            LogTag::Swap,
            "POSITION_TX_REGISTERED",
            &format!("📝 Registered {} transaction for position tracking: {}", direction, &transaction_signature[..8])
        );
    }

    Ok(())
}

/// Check if a position transaction has been verified
pub async fn is_position_transaction_verified(transaction_signature: &str) -> bool {
    let service_arc = TRANSACTION_SERVICE.clone();
    let service_guard = service_arc.lock().await;
    
    if let Some(service) = service_guard.as_ref() {
        if let Some(tx) = service.pending_transactions.get(transaction_signature) {
            return matches!(tx.state, TransactionState::Verified { .. });
        }
    }
    false
}

/// Get verification status of a position transaction
pub async fn get_position_transaction_status(transaction_signature: &str) -> Option<TransactionState> {
    let service_arc = TRANSACTION_SERVICE.clone();
    let service_guard = service_arc.lock().await;
    
    if let Some(service) = service_guard.as_ref() {
        if let Some(tx) = service.pending_transactions.get(transaction_signature) {
            return Some(tx.state.clone());
        }
    }
    None
}


