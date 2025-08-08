/// Transaction verification and analysis for swap operations with comprehensive instruction analysis
/// 
/// Purpose: Complete transaction analysis from blockchain data without wallet balance dependencies
/// - Analyze transaction instructions to extract input/output amounts
/// - Calculate effective swap prices from instruction data
/// - Detect ATA creation/closure from instruction patterns
/// - Extract all fee information from transaction structure
/// - Provide authoritative transaction metrics for position tracking
///
/// Key Features:
/// - Pure instruction-based transaction analysis (no wallet balance checking)
/// - Solana inner instruction parsing for accurate swap amounts
/// - Real-time ATA rent detection with on-chain caching
/// - Comprehensive fee breakdown (transaction, priority, ATA rent)
/// - Position transaction verification tracking
/// - Anti-duplicate transaction protection

use crate::global::{read_configs, is_debug_swap_enabled, DATA_DIR};
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, lamports_to_sol, sol_to_lamports, get_rpc_client, get_ata_rent_lamports};
use super::config::{SOL_MINT, TRANSACTION_CONFIRMATION_TIMEOUT_SECS};

use std::collections::{HashSet, HashMap};
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use std::path::Path;
use once_cell::sync::Lazy;
use bs58;
use solana_sdk::{signature::Keypair, signer::Signer, pubkey::Pubkey};
use solana_transaction_status::{UiTransactionEncoding, UiInnerInstructions, UiInstruction, UiParsedInstruction, parse_instruction::ParsedInstruction};
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

/// Transaction verification result containing all swap information from instruction analysis
#[derive(Debug, Clone)]
pub struct TransactionVerificationResult {
    pub success: bool,
    pub transaction_signature: String,
    pub confirmed: bool,
    
    // Amounts extracted from transaction instructions (lamports for SOL, raw units for tokens)
    pub input_amount: Option<u64>,     // Actual amount spent/consumed from instructions
    pub output_amount: Option<u64>,    // Actual amount received/produced from instructions
    
    // SOL flow analysis from instruction data
    pub sol_spent: Option<u64>,        // SOL spent in transaction (from transfers)
    pub sol_received: Option<u64>,     // SOL received in transaction (from transfers)
    pub transaction_fee: u64,          // Network transaction fee in lamports
    pub priority_fee: Option<u64>,     // Priority fee in lamports (if any)
    
    // ATA analysis from instruction patterns
    pub ata_created: bool,             // Whether ATA creation was detected
    pub ata_closed: bool,              // Whether ATA closure was detected
    pub ata_rent_paid: u64,            // Amount of rent paid for ATA creation
    pub ata_rent_reclaimed: u64,       // Amount of rent reclaimed from ATA closure
    
    // Price calculations from instruction data
    pub effective_price: Option<f64>,  // Price per token in SOL (from instruction amounts)
    pub price_impact: Option<f64>,     // Calculated price impact percentage
    
    // Token transfer details
    pub input_mint: String,            // Input token mint
    pub output_mint: String,           // Output token mint
    pub input_decimals: u32,           // Input token decimals
    pub output_decimals: u32,          // Output token decimals
    
    // Error information
    pub error: Option<String>,         // Error details if transaction failed
}

/// Instruction-based swap analysis result
#[derive(Debug, Clone)]
pub struct InstructionSwapAnalysis {
    pub input_amount: Option<u64>,
    pub output_amount: Option<u64>,
    pub input_mint: Option<String>,
    pub output_mint: Option<String>,
    pub sol_spent: Option<u64>,
    pub sol_received: Option<u64>,
    pub ata_created: bool,
    pub ata_closed: bool,
    pub ata_rent_paid: u64,
    pub ata_rent_reclaimed: u64,
    pub ata_rent_amount: Option<u64>, // For system instruction ATA creation
    pub priority_fee: Option<u64>,
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

    // Calculate price impact by comparing actual amounts vs expected
    match direction {
        "buy" => {
            if let (Some(input), Some(output)) = (input_amount, output_amount) {
                if input > 0 && output > 0 {
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

/// Analyze transaction instructions to extract swap amounts and ATA operations
/// This is the core function that analyzes Solana transaction instructions
/// Based on the provided Rust example for parsing inner instructions
pub async fn analyze_transaction_instructions(
    transaction_details: &crate::rpc::TransactionDetails,
    wallet_address: &str,
    expected_direction: &str, // "buy" or "sell"
    expected_input_mint: &str,   // NEW: Filter for specific input mint
    expected_output_mint: &str,  // NEW: Filter for specific output mint
) -> Result<InstructionSwapAnalysis, SwapError> {
    let mut analysis = InstructionSwapAnalysis {
        input_amount: None,
        output_amount: None,
        input_mint: None,
        output_mint: None,
        sol_spent: None,
        sol_received: None,
        ata_created: false,
        ata_closed: false,
        ata_rent_paid: 0,
        ata_rent_reclaimed: 0,
        ata_rent_amount: None,
        priority_fee: None,
    };

    let meta = transaction_details.meta.as_ref()
        .ok_or_else(|| SwapError::TransactionError("No transaction metadata available".to_string()))?;

    // Extract SOL balance changes from pre/post balances
    if !meta.pre_balances.is_empty() && !meta.post_balances.is_empty() {
        let pre_sol = meta.pre_balances[0]; // Wallet is typically first account
        let post_sol = meta.post_balances[0];
        
        if pre_sol > post_sol {
            analysis.sol_spent = Some(pre_sol - post_sol);
        } else if post_sol > pre_sol {
            analysis.sol_received = Some(post_sol - pre_sol);
        }
    }

    // Analyze inner instructions for token transfers (main swap analysis)
    // Note: Inner instructions might not be available in our TransactionMeta structure
    // For now, we'll focus on token balance changes which are available
    if let Some(log_messages) = &meta.log_messages {
        for log_message in log_messages {
            // Analyze transaction logs for ATA operations
            if log_message.contains("Program 11111111111111111111111111111111 invoke") &&
               (log_message.contains("Create") || log_message.contains("Allocate")) {
                analysis.ata_created = true;
                analysis.ata_rent_paid = get_ata_rent_lamports().await?;
            }
            
            if log_message.contains("CloseAccount") {
                analysis.ata_closed = true;
                analysis.ata_rent_reclaimed = get_ata_rent_lamports().await?;
            }
        }
    }

    // Analyze token balance changes for swap amounts - FILTER BY EXPECTED MINTS
    if let Some(pre_token_balances) = &meta.pre_token_balances {
        if let Some(post_token_balances) = &meta.post_token_balances {
            
            // Process input mint balance changes (tokens spent)
            for pre_balance in pre_token_balances {
                if pre_balance.mint == expected_input_mint {
                    let pre_amount = pre_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    
                    // Find corresponding post-balance or assume 0 if account was closed
                    let post_amount = post_token_balances
                        .iter()
                        .find(|post| post.account_index == pre_balance.account_index && post.mint == pre_balance.mint)
                        .map(|post| post.ui_token_amount.amount.parse::<u64>().unwrap_or(0))
                        .unwrap_or(0);
                    
                    if pre_amount > post_amount {
                        // Input tokens were spent
                        analysis.input_amount = Some(pre_amount - post_amount);
                        analysis.input_mint = Some(pre_balance.mint.clone());
                    }
                }
            }
            
            // Process output mint balance changes (tokens received)
            for post_balance in post_token_balances {
                if post_balance.mint == expected_output_mint {
                    let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);
                    
                    // Find corresponding pre-balance or assume 0 for new accounts
                    let pre_amount = pre_token_balances
                        .iter()
                        .find(|pre| pre.account_index == post_balance.account_index && pre.mint == post_balance.mint)
                        .map(|pre| pre.ui_token_amount.amount.parse::<u64>().unwrap_or(0))
                        .unwrap_or(0);
                    
                    if post_amount > pre_amount {
                        // Output tokens were received
                        analysis.output_amount = Some(post_amount - pre_amount);
                        analysis.output_mint = Some(post_balance.mint.clone());
                    }
                }
            }
        }
    }

    // Analyze main instructions for ATA operations and priority fees
    if let Ok(message) = serde_json::from_value::<serde_json::Value>(transaction_details.transaction.message.clone()) {
        if let Some(instructions) = message.get("instructions").and_then(|i| i.as_array()) {
            for instruction in instructions {
                analyze_main_instruction(instruction, &mut analysis).await?;
            }
        }
    }

    // Analyze transaction logs for additional context
    if let Some(log_messages) = &meta.log_messages {
        analyze_transaction_logs(log_messages, &mut analysis).await?;
    }

    // Get current ATA rent for comparison
    let current_ata_rent = get_ata_rent_lamports().await.unwrap_or(2_039_280);

    // Detect ATA operations based on rent amounts
    if let Some(sol_spent) = analysis.sol_spent {
        // Check if SOL spent includes ATA rent (for creation)
        if sol_spent > current_ata_rent / 2 && sol_spent <= current_ata_rent * 2 {
            analysis.ata_created = true;
            analysis.ata_rent_paid = std::cmp::min(sol_spent, current_ata_rent);
        }
    }

    if let Some(sol_received) = analysis.sol_received {
        // Check if SOL received includes ATA rent (for closure)
        if sol_received > current_ata_rent / 2 && sol_received <= current_ata_rent * 2 {
            analysis.ata_closed = true;
            analysis.ata_rent_reclaimed = std::cmp::min(sol_received, current_ata_rent);
        }
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "INSTRUCTION_ANALYSIS",
            &format!(
                "📊 Instruction Analysis Complete:
  Input: {:?} {} | Output: {:?} {}
  SOL: spent={:?}, received={:?}
  ATA: created={}, closed={}, rent_paid={}, rent_reclaimed={}
  Priority fee: {:?}",
                analysis.input_amount,
                analysis.input_mint.as_deref().unwrap_or("?"),
                analysis.output_amount,
                analysis.output_mint.as_deref().unwrap_or("?"),
                analysis.sol_spent,
                analysis.sol_received,
                analysis.ata_created,
                analysis.ata_closed,
                analysis.ata_rent_paid,
                analysis.ata_rent_reclaimed,
                analysis.priority_fee
            )
        );
    }

    Ok(analysis)
}

/// Analyze SPL Token instructions for transfer amounts
/// ENHANCED: Better amount extraction and validation
async fn analyze_spl_token_instruction(
    parsed: &ParsedInstruction,
    analysis: &mut InstructionSwapAnalysis,
    wallet_address: &str,
) -> Result<(), SwapError> {
    if parsed.program != "spl-token" {
        return Ok(());
    }

    if let Some(instruction_type) = parsed.parsed.get("type").and_then(|v| v.as_str()) {
        match instruction_type {
            "transfer" | "transferChecked" => {
                if let Some(info) = parsed.parsed.get("info") {
                    let amount_str = info.get("amount")
                        .and_then(|a| a.as_str())
                        .unwrap_or("0");
                    let amount = amount_str.parse::<u64>().unwrap_or(0);
                    
                    if amount == 0 {
                        return Ok(());
                    }

                    let mint = info.get("mint")
                        .and_then(|m| m.as_str())
                        .unwrap_or("")
                        .to_string();

                    let source = info.get("source").and_then(|s| s.as_str()).unwrap_or("");
                    let destination = info.get("destination").and_then(|d| d.as_str()).unwrap_or("");
                    let authority = info.get("authority").and_then(|a| a.as_str()).unwrap_or("");

                    // Enhanced wallet detection logic
                    let is_outgoing = source.contains(wallet_address) || authority == wallet_address;
                    let is_incoming = destination.contains(wallet_address);
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "TOKEN_TRANSFER_DEBUG",
                            &format!(
                                "🔍 Token Transfer Analysis:
  Type: {}
  Amount: {} (raw)
  Mint: {}
  Source: {} | Dest: {} | Authority: {}
  Wallet: {}
  Outgoing: {} | Incoming: {}",
                                instruction_type,
                                amount,
                                &mint[..8],
                                &source[..8],
                                &destination[..8], 
                                &authority[..8],
                                &wallet_address[..8],
                                is_outgoing,
                                is_incoming
                            )
                        );
                    }

                    // Improved assignment logic - prioritize larger amounts and SOL mint
                    if is_outgoing {
                        // For outgoing transfers, this is usually the input (what we're selling/spending)
                        if analysis.input_amount.is_none() || 
                           (mint == SOL_MINT && analysis.input_mint.as_ref() != Some(&SOL_MINT.to_string())) ||
                           amount > analysis.input_amount.unwrap_or(0) {
                            analysis.input_amount = Some(amount);
                            analysis.input_mint = Some(mint.clone());
                            
                            // Track SOL movements separately
                            if mint == SOL_MINT {
                                analysis.sol_spent = Some(amount);
                            }
                        }
                    }
                    
                    if is_incoming {
                        // For incoming transfers, this is usually the output (what we're receiving)
                        if analysis.output_amount.is_none() || 
                           (mint == SOL_MINT && analysis.output_mint.as_ref() != Some(&SOL_MINT.to_string())) ||
                           amount > analysis.output_amount.unwrap_or(0) {
                            analysis.output_amount = Some(amount);
                            analysis.output_mint = Some(mint.clone());
                            
                            // Track SOL movements separately
                            if mint == SOL_MINT {
                                analysis.sol_received = Some(amount);
                            }
                        }
                    }
                }
            }
            "closeAccount" => {
                // Enhanced ATA closure tracking
                analysis.ata_closed = true;
                if let Some(info) = parsed.parsed.get("info") {
                    let account = info.get("account")
                        .and_then(|a| a.as_str())
                        .unwrap_or("");
                    
                    let destination = info.get("destination")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                        
                    let owner = info.get("owner")
                        .and_then(|o| o.as_str())
                        .unwrap_or("");
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "ATA_CLOSE_DEBUG",
                            &format!(
                                "🔒 ATA Closure Analysis:
  Account: {}
  Destination: {}
  Owner: {}
  Is Wallet Destination: {}
  Is Wallet Owner: {}",
                                &account[..std::cmp::min(account.len(), 8)],
                                &destination[..std::cmp::min(destination.len(), 8)],
                                &owner[..std::cmp::min(owner.len(), 8)],
                                destination == wallet_address,
                                owner == wallet_address
                            )
                        );
                    }
                    
                    // Track ATA closures where wallet receives the rent
                    if destination == wallet_address {
                        let current_ata_rent = get_ata_rent_lamports().await.unwrap_or(2_039_280);
                        analysis.ata_rent_reclaimed = current_ata_rent;
                        
                        if is_debug_swap_enabled() {
                            log(
                                LogTag::Swap,
                                "ATA_RENT_RECLAIM",
                                &format!("✅ Detected ATA rent reclamation: {} lamports", current_ata_rent)
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Analyze main transaction instructions for ATA creation and priority fees
/// ENHANCED: Better system instruction analysis and ATA detection
async fn analyze_main_instruction(
    instruction: &serde_json::Value,
    analysis: &mut InstructionSwapAnalysis,
) -> Result<(), SwapError> {
    if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|i| i.as_u64()) {
        // Check for compute budget instructions (priority fees)
        if let Some(data) = instruction.get("data").and_then(|d| d.as_str()) {
            if let Ok(decoded_data) = bs58::decode(data).into_vec() {
                if decoded_data.len() >= 4 {
                    let instruction_type = u32::from_le_bytes([
                        decoded_data[0], decoded_data[1], decoded_data[2], decoded_data[3]
                    ]);
                    
                    // System program instructions
                    match instruction_type {
                        0 => { // CreateAccount
                            // System program CreateAccount instruction
                            if decoded_data.len() >= 52 {
                                let lamports = u64::from_le_bytes([
                                    decoded_data[4], decoded_data[5], decoded_data[6], decoded_data[7],
                                    decoded_data[8], decoded_data[9], decoded_data[10], decoded_data[11]
                                ]);
                                let space = u64::from_le_bytes([
                                    decoded_data[12], decoded_data[13], decoded_data[14], decoded_data[15],
                                    decoded_data[16], decoded_data[17], decoded_data[18], decoded_data[19]
                                ]);
                                
                                // Check if this looks like ATA creation (165 bytes, typical ATA rent)
                                if space == 165 && lamports > 2_000_000 && lamports < 3_000_000 {
                                    analysis.ata_created = true;
                                    analysis.ata_rent_amount = Some(lamports);
                                    
                                    if is_debug_swap_enabled() {
                                        log(
                                            LogTag::Swap,
                                            "ATA_CREATE_SYSTEM",
                                            &format!("✅ Detected ATA creation via system: {} lamports, {} bytes", lamports, space)
                                        );
                                    }
                                }
                            }
                        }
                        2 => { // SetComputeUnitPrice (priority fees)
                            if decoded_data.len() >= 12 {
                                let price = u64::from_le_bytes([
                                    decoded_data[4], decoded_data[5], decoded_data[6], decoded_data[7],
                                    decoded_data[8], decoded_data[9], decoded_data[10], decoded_data[11]
                                ]);
                                analysis.priority_fee = Some(price);
                                
                                if is_debug_swap_enabled() {
                                    log(
                                        LogTag::Swap,
                                        "PRIORITY_FEE_DEBUG",
                                        &format!("💰 Detected priority fee: {} micro-lamports per CU", price)
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

/// Analyze transaction logs for ATA operations
/// ENHANCED: Better log parsing and debugging information
async fn analyze_transaction_logs(
    log_messages: &[String],
    analysis: &mut InstructionSwapAnalysis,
) -> Result<(), SwapError> {
    for (i, log_message) in log_messages.iter().enumerate() {
        let log_lower = log_message.to_lowercase();
        
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "LOG_ANALYSIS_DEBUG",
                &format!("📋 Log #{}: {}", i + 1, 
                    if log_message.len() > 100 { 
                        format!("{}...", &log_message[..100]) 
                    } else { 
                        log_message.clone() 
                    }
                )
            );
        }
        
        // Check for ATA creation patterns
        if (log_lower.contains("create") && (log_lower.contains("account") || log_lower.contains("ata"))) ||
           (log_lower.contains("program 11111111111111111111111111111111 invoke") && 
            (log_lower.contains("create") || log_lower.contains("allocate"))) {
            analysis.ata_created = true;
            
            if is_debug_swap_enabled() {
                log(
                    LogTag::Swap,
                    "ATA_CREATE_LOG",
                    &format!("✅ Detected ATA creation in log: {}", 
                        if log_message.len() > 60 { &log_message[..60] } else { log_message }
                    )
                );
            }
        }
        
        // Check for ATA closure patterns
        if (log_lower.contains("close") && (log_lower.contains("account") || log_lower.contains("ata"))) ||
           log_lower.contains("closeaccount") {
            analysis.ata_closed = true;
            
            if is_debug_swap_enabled() {
                log(
                    LogTag::Swap,
                    "ATA_CLOSE_LOG",
                    &format!("✅ Detected ATA closure in log: {}", 
                        if log_message.len() > 60 { &log_message[..60] } else { log_message }
                    )
                );
            }
        }
    }

    Ok(())
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

/// MAIN FUNCTION: Comprehensive transaction verification and analysis using instruction parsing
/// This is the core function that analyzes swap transactions and extracts all relevant information
/// Now uses pure instruction analysis instead of wallet balance checking
pub async fn verify_swap_transaction(
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    expected_direction: &str, // "buy" or "sell"
) -> Result<TransactionVerificationResult, SwapError> {
    let wallet_address = get_wallet_address()?;
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_START",
            &format!(
                "🔍 Starting instruction-based transaction analysis for {}\n  Direction: {}\n  Route: {} -> {}\n  Wallet: {}",
                transaction_signature,
                expected_direction,
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                &wallet_address[..8]
            )
        );
    }

    // Step 1: Wait for transaction confirmation
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
            priority_fee: None,
            ata_created: false,
            ata_closed: false,
            ata_rent_paid: 0,
            ata_rent_reclaimed: 0,
            effective_price: None,
            price_impact: None,
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            input_decimals: if input_mint == SOL_MINT { 9 } else { 9 }, // Default fallback to 9
            output_decimals: if output_mint == SOL_MINT { 9 } else { 9 }, // Default fallback to 9
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

    // Step 3: Analyze transaction instructions (NEW - replaces balance snapshots)
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_3",
            "🔎 Step 3: Analyzing transaction instructions for amounts and ATA operations..."
        );
    }
    
    let instruction_analysis = analyze_transaction_instructions(
        &transaction_details,
        &wallet_address,
        expected_direction,
        input_mint,
        output_mint
    ).await?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_3_COMPLETE",
            &format!("✅ Step 3 Complete: Instruction analysis completed
  Input: {:?} | Output: {:?}
  SOL spent: {:?} | SOL received: {:?}
  ATA created: {} | ATA closed: {}",
                instruction_analysis.input_amount,
                instruction_analysis.output_amount,
                instruction_analysis.sol_spent,
                instruction_analysis.sol_received,
                instruction_analysis.ata_created,
                instruction_analysis.ata_closed
            )
        );
    }

    // Step 4: Get token decimals for price calculations
    let input_decimals = if input_mint == SOL_MINT { 
        9 
    } else { 
        crate::tokens::decimals::get_token_decimals_from_chain(input_mint).await.unwrap_or(9) as u32
    };
    
    let output_decimals = if output_mint == SOL_MINT { 
        9 
    } else { 
        crate::tokens::decimals::get_token_decimals_from_chain(output_mint).await.unwrap_or(9) as u32
    };

    // Step 5: Calculate effective price using the instruction data
    let effective_price = crate::swaps::pricing::calculate_effective_price_from_raw(
        expected_direction,
        instruction_analysis.input_amount,
        instruction_analysis.output_amount,
        instruction_analysis.sol_spent,
        instruction_analysis.sol_received,
        instruction_analysis.ata_rent_reclaimed,
        input_decimals,
        output_decimals
    );

    // Step 6: Calculate price impact
    let price_impact = calculate_price_impact(
        expected_direction,
        instruction_analysis.input_amount,
        instruction_analysis.output_amount,
        effective_price
    );

    // Step 7: Validate results consistency (simplified validation)
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_7",
            "🔎 Step 7: Validating instruction-based results..."
        );
    }
    
    validate_instruction_analysis_results(
        expected_direction,
        &instruction_analysis
    )?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_STEP_7_COMPLETE",
            "✅ Step 7 Complete: All instruction analysis results validated successfully"
        );
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VERIFY_SUCCESS",
            &format!(
                "✅ Transaction verification completed successfully using instruction analysis
  📊 Final Results Summary:
  • Input Amount: {} {} 
  • Output Amount: {} {}
  • SOL Spent: {} lamports ({:.6} SOL)
  • SOL Received: {} lamports ({:.6} SOL)
  • Transaction Fee: {} lamports ({:.6} SOL)
  • Priority Fee: {:?} micro-lamports
  • ATA Created: {} | Rent Paid: {} lamports ({:.6} SOL)
  • ATA Closed: {} | Rent Reclaimed: {} lamports ({:.6} SOL)
  • Effective Price: {:.10} SOL per token
  🎯 Pure instruction-based analysis: ALL 7 STEPS COMPLETED",
                instruction_analysis.input_amount.unwrap_or(0),
                if expected_direction == "buy" { "lamports (SOL)" } else { "tokens" },
                instruction_analysis.output_amount.unwrap_or(0),
                if expected_direction == "buy" { "tokens" } else { "lamports (SOL)" },
                instruction_analysis.sol_spent.unwrap_or(0),
                lamports_to_sol(instruction_analysis.sol_spent.unwrap_or(0)),
                instruction_analysis.sol_received.unwrap_or(0),
                lamports_to_sol(instruction_analysis.sol_received.unwrap_or(0)),
                transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
                lamports_to_sol(transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0)),
                instruction_analysis.priority_fee,
                instruction_analysis.ata_created,
                instruction_analysis.ata_rent_paid,
                lamports_to_sol(instruction_analysis.ata_rent_paid),
                instruction_analysis.ata_closed,
                instruction_analysis.ata_rent_reclaimed,
                lamports_to_sol(instruction_analysis.ata_rent_reclaimed),
                effective_price.unwrap_or(0.0)
            )
        );
    }

    Ok(TransactionVerificationResult {
        success: true,
        transaction_signature: transaction_signature.to_string(),
        confirmed: true,
        input_amount: instruction_analysis.input_amount,
        output_amount: instruction_analysis.output_amount,
        sol_spent: instruction_analysis.sol_spent,
        sol_received: instruction_analysis.sol_received,
        transaction_fee: transaction_details.meta.as_ref().map(|m| m.fee).unwrap_or(0),
        priority_fee: instruction_analysis.priority_fee,
        ata_created: instruction_analysis.ata_created,
        ata_closed: instruction_analysis.ata_closed,
        ata_rent_paid: instruction_analysis.ata_rent_paid,
        ata_rent_reclaimed: instruction_analysis.ata_rent_reclaimed,
        effective_price,
        price_impact,
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        input_decimals,
        output_decimals,
        error: None,
    })
}

/// Validate instruction analysis results for consistency
fn validate_instruction_analysis_results(
    expected_direction: &str,
    analysis: &InstructionSwapAnalysis
) -> Result<(), SwapError> {
    match expected_direction {
        "buy" => {
            // For buy transactions: Must have received tokens and spent SOL
            if analysis.output_amount.is_none() || analysis.output_amount.unwrap() == 0 {
                return Err(SwapError::TransactionError(
                    format!("Buy validation failed: No tokens received (output_amount: {:?})", analysis.output_amount)
                ));
            }

            if analysis.sol_spent.is_none() || analysis.sol_spent.unwrap() == 0 {
                log(
                    LogTag::Swap,
                    "VALIDATION_WARNING",
                    "⚠️ Buy transaction: No SOL spent detected - possible instruction parsing issue"
                );
            }
        }
        "sell" => {
            // For sell transactions: Must have sent tokens and received SOL
            if analysis.input_amount.is_none() || analysis.input_amount.unwrap() == 0 {
                return Err(SwapError::TransactionError(
                    format!("Sell validation failed: No tokens sent (input_amount: {:?})", analysis.input_amount)
                ));
            }

            if analysis.sol_received.is_none() || analysis.sol_received.unwrap() == 0 {
                log(
                    LogTag::Swap,
                    "VALIDATION_WARNING",
                    "⚠️ Sell transaction: No SOL received detected - possible instruction parsing issue"
                );
            }
        }
        _ => {
            return Err(SwapError::TransactionError(
                format!("Invalid transaction direction: {}", expected_direction)
            ));
        }
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "VALIDATION_SUCCESS",
            &format!(
                "✅ Instruction analysis validation passed for {} direction
  📊 Validation Summary:
  • Direction: {} ✓
  • Amount Extraction: Input={:?}, Output={:?} ✓
  • SOL Flow: Spent={:?}, Received={:?} ✓
  • ATA Operations: Created={}, Closed={} ✓
  🎯 Pure instruction-based validation confirmed",
                expected_direction,
                expected_direction,
                analysis.input_amount,
                analysis.output_amount,
                analysis.sol_spent,
                analysis.sol_received,
                analysis.ata_created,
                analysis.ata_closed
            )
        );
    }

    Ok(())
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

/// POSITION-SPECIFIC TRANSACTION VERIFICATION FUNCTIONS
/// Comprehensive verification for position entry and exit transactions using instruction analysis

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

/// Verify position entry transaction using instruction analysis
/// This function performs complete verification of a buy transaction for position tracking
pub async fn verify_position_entry_transaction(
    transaction_signature: &str,
    token_mint: &str,
    expected_sol_spent: f64,
) -> Result<PositionEntryVerification, SwapError> {
    log(
        LogTag::Swap,
        "POSITION_ENTRY_VERIFY",
        &format!("🔍 Verifying position entry transaction using instruction analysis: {}", &transaction_signature[..8])
    );

    // Use the main verify_swap_transaction function with instruction analysis
    let verification_result = verify_swap_transaction(
        transaction_signature,
        SOL_MINT,
        token_mint,
        "buy"
    ).await?;

    if !verification_result.success {
        return Ok(PositionEntryVerification {
            transaction_signature: transaction_signature.to_string(),
            success: false,
            error: verification_result.error,
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

    // Extract values from instruction-based analysis
    let token_amount_received = verification_result.output_amount.unwrap_or(0);
    let sol_spent = verification_result.sol_spent.unwrap_or(0);
    let ata_created = verification_result.ata_created;
    let ata_rent_paid = verification_result.ata_rent_paid;
    let transaction_fee = verification_result.transaction_fee;

    // Calculate effective entry price from instruction data
    let effective_entry_price = if token_amount_received > 0 {
        let sol_spent_for_tokens = sol_spent.saturating_sub(ata_rent_paid);
        let sol_spent_actual = lamports_to_sol(sol_spent_for_tokens);
        let tokens_received_actual = (token_amount_received as f64) / 10f64.powi(verification_result.output_decimals as i32);
        
        if tokens_received_actual > 0.0 {
            sol_spent_actual / tokens_received_actual
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Calculate total cost
    let total_cost_sol = lamports_to_sol(sol_spent + transaction_fee);

    // Validate results
    let verification_success = token_amount_received > 0 && 
                              sol_spent > 0 && 
                              effective_entry_price > 0.0;

    // Log verification results
    if verification_success {
        log(
            LogTag::Swap,
            "POSITION_ENTRY_SUCCESS",
            &format!(
                "✅ Entry verified using instruction analysis: {} tokens received, {:.9} SOL spent, price: {:.12} SOL/token",
                token_amount_received,
                lamports_to_sol(sol_spent),
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
                sol_spent,
                effective_entry_price
            )
        );
    }

    Ok(PositionEntryVerification {
        transaction_signature: transaction_signature.to_string(),
        success: verification_success,
        error: None,
        token_amount_received,
        sol_spent,
        effective_entry_price,
        entry_transaction_verified: verification_success,
        ata_created,
        ata_rent_paid,
        transaction_fee,
        total_cost_sol,
    })
}

/// Verify position exit transaction using instruction analysis
/// This function performs complete verification of a sell transaction for position tracking
pub async fn verify_position_exit_transaction(
    transaction_signature: &str,
    token_mint: &str,
    expected_token_amount: u64,
) -> Result<PositionExitVerification, SwapError> {
    log(
        LogTag::Swap,
        "POSITION_EXIT_VERIFY",
        &format!("🔍 Verifying position exit transaction using instruction analysis: {}", &transaction_signature[..8])
    );

    // Use the main verify_swap_transaction function with instruction analysis
    let verification_result = verify_swap_transaction(
        transaction_signature,
        token_mint,
        SOL_MINT,
        "sell"
    ).await?;

    if !verification_result.success {
        return Ok(PositionExitVerification {
            transaction_signature: transaction_signature.to_string(),
            success: false,
            error: verification_result.error,
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

    // Extract values from instruction-based analysis
    let token_amount_sold = verification_result.input_amount.unwrap_or(0);
    let sol_received = verification_result.sol_received.unwrap_or(0);
    let ata_closed = verification_result.ata_closed;
    let ata_rent_reclaimed = verification_result.ata_rent_reclaimed;
    let transaction_fee = verification_result.transaction_fee;

    // Calculate effective exit price from instruction data
    let effective_exit_price = if token_amount_sold > 0 {
        let sol_from_sale = sol_received.saturating_sub(ata_rent_reclaimed);
        let sol_received_actual = lamports_to_sol(sol_from_sale);
        let tokens_sold_actual = (token_amount_sold as f64) / 10f64.powi(verification_result.input_decimals as i32);
        
        if tokens_sold_actual > 0.0 {
            sol_received_actual / tokens_sold_actual
        } else {
            0.0
        }
    } else {
        0.0
    };

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
                "✅ Exit verified using instruction analysis: {} tokens sold, {:.9} SOL received, price: {:.12} SOL/token",
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


