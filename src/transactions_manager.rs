/// Transactions Manager - Real-time background transaction monitoring and analysis
/// Tracks wallet transactions, caches data, detects transaction types, and integrates with positions
///
/// **All transaction analysis functionality is integrated directly into this module.**
/// This includes DEX detection, swap analysis, balance calculations, and type classification.
/// 
/// Debug Tool: Use `cargo run --bin main_transactions_debug` for comprehensive debugging,
/// monitoring, analysis, and performance testing of the transaction management system.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{Duration, interval};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    commitment_config::CommitmentConfig,
};
use std::str::FromStr;

use crate::logger::{log, LogTag};
use crate::global::{
    is_debug_transactions_enabled, 
    get_transactions_cache_dir,
    read_configs,
    load_wallet_from_config
};
use crate::rpc::get_rpc_client;
use crate::utils::get_wallet_address;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Main Transaction structure used throughout the bot
/// Contains all Solana data + our calculations and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    // Core identification
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub timestamp: DateTime<Utc>,
    
    // Transaction state and commitment
    pub commitment_state: CommitmentState,
    pub confirmation_status: ConfirmationStatus,
    pub finalized: bool,
    
    // Transaction type and analysis
    pub transaction_type: TransactionType,
    pub direction: TransactionDirection,
    pub success: bool,
    pub error_message: Option<String>,
    
    // Financial data
    pub fee_sol: f64,
    pub sol_balance_change: f64,
    pub token_transfers: Vec<TokenTransfer>,
    
    // Raw Solana data (cached)
    pub raw_transaction_data: Option<serde_json::Value>,
    pub log_messages: Vec<String>,
    pub instructions: Vec<InstructionInfo>,
    
    // Balance changes
    pub sol_balance_changes: Vec<SolBalanceChange>,
    pub token_balance_changes: Vec<TokenBalanceChange>,
    
    // Our analysis and calculations
    pub swap_analysis: Option<SwapAnalysis>,
    pub position_impact: Option<PositionImpact>,
    pub profit_calculation: Option<ProfitCalculation>,
    
    // Priority and tracking
    pub is_priority: bool,
    pub priority_added_at: Option<DateTime<Utc>>,
    pub last_updated: DateTime<Utc>,
    pub cache_file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitmentState {
    Sent,
    Confirmed,
    Finalized,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfirmationStatus {
    Pending,
    Confirmed,
    Finalized,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    SwapSolToToken {
        token_mint: String,
        sol_amount: f64,
        token_amount: f64,
        router: String,
    },
    SwapTokenToSol {
        token_mint: String,
        token_amount: f64,
        sol_amount: f64,
        router: String,
    },
    SwapTokenToToken {
        from_mint: String,
        to_mint: String,
        from_amount: f64,
        to_amount: f64,
        router: String,
    },
    SolTransfer {
        amount: f64,
        from: String,
        to: String,
    },
    TokenTransfer {
        mint: String,
        amount: f64,
        from: String,
        to: String,
    },
    Spam,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
    pub mint: String,
    pub amount: f64,
    pub from: String,
    pub to: String,
    pub program_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolBalanceChange {
    pub account: String,
    pub pre_balance: f64,
    pub post_balance: f64,
    pub change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalanceChange {
    pub mint: String,
    pub decimals: u8,
    pub pre_balance: Option<f64>,
    pub post_balance: Option<f64>,
    pub change: f64,
    pub usd_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    pub program_id: String,
    pub instruction_type: String,
    pub accounts: Vec<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapAnalysis {
    pub router: String,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: f64,
    pub output_amount: f64,
    pub effective_price: f64,
    pub slippage: f64,
    pub fee_breakdown: FeeBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub transaction_fee: f64,
    pub router_fee: f64,
    pub platform_fee: f64,
    pub total_fees: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionImpact {
    pub token_mint: String,
    pub position_change: f64,
    pub new_position_size: f64,
    pub entry_exit: PositionChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionChange {
    Entry,
    Exit,
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitCalculation {
    pub realized_profit_sol: f64,
    pub unrealized_profit_sol: f64,
    pub profit_percentage: f64,
    pub hold_duration: Option<Duration>,
}

// =============================================================================
// TRANSACTIONS MANAGER
// =============================================================================

/// TransactionsManager - Main service for real-time transaction monitoring
pub struct TransactionsManager {
    pub wallet_pubkey: Pubkey,
    pub debug_enabled: bool,
    pub known_signatures: HashSet<String>,
    pub priority_transactions: HashMap<String, DateTime<Utc>>,
    pub last_signature_check: Option<String>,
    pub total_transactions: u64,
    pub new_transactions_count: u64,
}

impl TransactionsManager {
    /// Create new TransactionsManager instance
    pub fn new(wallet_pubkey: Pubkey) -> Self {
        Self {
            wallet_pubkey,
            debug_enabled: is_debug_transactions_enabled(),
            known_signatures: HashSet::new(),
            priority_transactions: HashMap::new(),
            last_signature_check: None,
            total_transactions: 0,
            new_transactions_count: 0,
        }
    }

    /// Load existing cached signatures to avoid re-processing
    pub async fn initialize_known_signatures(&mut self) -> Result<(), String> {
        let cache_dir = get_transactions_cache_dir();
        
        if !Path::new(&cache_dir).exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to create transactions cache dir: {}", e))?;
            return Ok(());
        }

        let entries = fs::read_dir(&cache_dir)
            .map_err(|e| format!("Failed to read cache dir: {}", e))?;

        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".json") {
                        let signature = file_name.replace(".json", "");
                        self.known_signatures.insert(signature);
                        self.total_transactions += 1;
                    }
                }
            }
        }

        if self.debug_enabled {
            log(LogTag::Transactions, "INIT", &format!(
                "Loaded {} existing cached transactions", 
                self.known_signatures.len()
            ));
        }

        Ok(())
    }

    /// Check for new transactions from wallet
    pub async fn check_new_transactions(&mut self) -> Result<Vec<String>, String> {
        let rpc_client = get_rpc_client();
        
        // Get recent signatures from wallet
        let signatures = rpc_client
            .get_wallet_signatures_main_rpc(&self.wallet_pubkey, 50, self.last_signature_check.as_deref())
            .await
            .map_err(|e| format!("Failed to fetch wallet signatures: {}", e))?;

        let mut new_signatures = Vec::new();

        for sig_info in signatures {
            let signature = sig_info.signature;
            
            // Skip if we already know this signature
            if self.known_signatures.contains(&signature) {
                continue;
            }

            // Add to known signatures
            self.known_signatures.insert(signature.clone());
            new_signatures.push(signature.clone());
            
            // Update last signature for pagination
            if self.last_signature_check.is_none() {
                self.last_signature_check = Some(signature);
            }
        }

        if !new_signatures.is_empty() {
            self.new_transactions_count += new_signatures.len() as u64;
            
            if self.debug_enabled {
                log(LogTag::Transactions, "NEW", &format!(
                    "Found {} new transactions to process", 
                    new_signatures.len()
                ));
            }
        }

        Ok(new_signatures)
    }

    /// Process a single transaction (fetch, analyze, cache)
    pub async fn process_transaction(&mut self, signature: &str) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "PROCESS", &format!("Processing transaction: {}", &signature[..8]));
        }

        // Check if priority transaction
        let is_priority = self.priority_transactions.contains_key(signature);

        // Fetch transaction data from RPC
        let rpc_client = get_rpc_client();
        let tx_data = rpc_client
            .get_transaction_details_premium_rpc(signature)
            .await
            .map_err(|e| format!("Failed to fetch transaction details: {}", e))?;

        // Create Transaction structure
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: Some(tx_data.slot),
            block_time: tx_data.block_time,
            timestamp: Utc::now(),
            commitment_state: CommitmentState::Finalized, // Since we fetched it
            confirmation_status: ConfirmationStatus::Finalized,
            finalized: true,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: tx_data.transaction.meta.as_ref().map_or(false, |meta| meta.err.is_none()),
            error_message: tx_data.transaction.meta.as_ref()
                .and_then(|meta| meta.err.as_ref())
                .map(|err| format!("{:?}", err)),
            fee_sol: tx_data.transaction.meta.as_ref().map_or(0.0, |meta| meta.fee as f64 / 1_000_000_000.0),
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: Some(serde_json::to_value(&tx_data).unwrap_or_default()),
            log_messages: tx_data.transaction.meta.as_ref()
                .map(|meta| match &meta.log_messages {
                    solana_transaction_status::option_serializer::OptionSerializer::Some(logs) => logs.clone(),
                    _ => Vec::new(),
                })
                .unwrap_or_default(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            swap_analysis: None,
            position_impact: None,
            profit_calculation: None,
            is_priority,
            priority_added_at: if is_priority { self.priority_transactions.get(signature).copied() } else { None },
            last_updated: Utc::now(),
            cache_file_path: format!("{}/{}.json", get_transactions_cache_dir().display(), signature),
        };

        // Analyze transaction type and extract details
        self.analyze_transaction(&mut transaction).await?;

        // Cache transaction to disk
        self.cache_transaction(&transaction).await?;

        // Remove from priority tracking if it was priority
        if is_priority {
            self.priority_transactions.remove(signature);
            if self.debug_enabled {
                log(LogTag::Transactions, "PRIORITY", &format!(
                    "Priority transaction {} processed and removed from tracking", 
                    &signature[..8]
                ));
            }
        }

        Ok(transaction)
    }

    /// Analyze transaction to determine type and extract data
    async fn analyze_transaction(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        // Basic analysis - this would be expanded with transactions_analyzer.rs integration
        
        // Check if it's a swap by looking for known program IDs in instructions
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(tx_obj) = raw_data.as_object() {
                if let Some(tx_data) = tx_obj.get("transaction") {
                    if let Some(message) = tx_data.get("message") {
                        if let Some(instructions) = message.get("instructions") {
                            if let Some(instructions_array) = instructions.as_array() {
                                // Look for Jupiter, Raydium, or other DEX program IDs
                                for instruction in instructions_array {
                                    if let Some(program_id_index) = instruction.get("programIdIndex") {
                                        // This is a simplified check - would be expanded with proper program ID detection
                                        transaction.transaction_type = TransactionType::Unknown;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate SOL balance change
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("transaction").and_then(|tx| tx.get("meta")) {
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preBalances").and_then(|b| b.as_array()),
                    meta.get("postBalances").and_then(|b| b.as_array())
                ) {
                    // Calculate balance changes (simplified - would be expanded)
                    if let (Some(pre_first), Some(post_first)) = (
                        pre_balances.get(0).and_then(|b| b.as_u64()),
                        post_balances.get(0).and_then(|b| b.as_u64())
                    ) {
                        transaction.sol_balance_change = (post_first as i64 - pre_first as i64) as f64 / 1_000_000_000.0;
                    }
                }
            }
        }

        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYZE", &format!(
                "Transaction {} - Type: {:?}, SOL change: {:.6}", 
                &transaction.signature[..8],
                transaction.transaction_type,
                transaction.sol_balance_change
            ));
        }

        Ok(())
    }

    /// Cache transaction to disk
    async fn cache_transaction(&self, transaction: &Transaction) -> Result<(), String> {
        let cache_dir = get_transactions_cache_dir();
        
        // Ensure cache directory exists
        if !Path::new(&cache_dir).exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let cache_file_path = format!("{}/{}.json", cache_dir.display(), transaction.signature);
        let json_data = serde_json::to_string_pretty(transaction)
            .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

        fs::write(&cache_file_path, json_data)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        if self.debug_enabled {
            log(LogTag::Transactions, "CACHE", &format!(
                "Cached transaction {} to disk", 
                &transaction.signature[..8]
            ));
        }

        Ok(())
    }

    /// Add priority transaction for monitoring
    pub fn add_priority_transaction(&mut self, signature: String) {
        self.priority_transactions.insert(signature.clone(), Utc::now());
        
        if self.debug_enabled {
            log(LogTag::Transactions, "PRIORITY", &format!(
                "Added priority transaction: {}", 
                &signature[..8]
            ));
        }
    }

    /// Check priority transactions for completion
    pub async fn check_priority_transactions(&mut self) -> Result<(), String> {
        let signatures_to_check: Vec<String> = self.priority_transactions.keys().cloned().collect();
        
        for signature in signatures_to_check {
            // If we already processed this signature, remove from priority
            if self.known_signatures.contains(&signature) {
                self.priority_transactions.remove(&signature);
                continue;
            }

            // Check if transaction is now available/finalized
            match self.process_transaction(&signature).await {
                Ok(_) => {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "PRIORITY", &format!(
                            "Priority transaction {} now available and processed", 
                            &signature[..8]
                        ));
                    }
                }
                Err(e) => {
                    // Still not available or failed - keep monitoring
                    if self.debug_enabled {
                        log(LogTag::Transactions, "PRIORITY", &format!(
                            "Priority transaction {} still pending: {}", 
                            &signature[..8], e
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get transaction statistics
    pub fn get_stats(&self) -> TransactionStats {
        TransactionStats {
            total_transactions: self.total_transactions,
            new_transactions_count: self.new_transactions_count,
            priority_transactions_count: self.priority_transactions.len() as u64,
            known_signatures_count: self.known_signatures.len() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub total_transactions: u64,
    pub new_transactions_count: u64,
    pub priority_transactions_count: u64,
    pub known_signatures_count: u64,
}

// =============================================================================
// BACKGROUND SERVICE
// =============================================================================

/// Start the transactions manager background service
/// Simple pattern following other bot services
pub async fn start_transactions_manager_service(shutdown: Arc<Notify>) {
    log(LogTag::Transactions, "INFO", "TransactionsManager service starting...");
    
    // Load wallet address fresh each time
    let wallet_address = match load_wallet_address_from_config().await {
        Ok(address) => address,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load wallet address: {}", e));
            return;
        }
    };

    // Create TransactionsManager instance
    let mut manager = TransactionsManager::new(wallet_address);
    
    // Initialize known signatures
    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize: {}", e));
        return;
    }

    log(LogTag::Transactions, "INFO", &format!(
        "TransactionsManager initialized for wallet: {} (known transactions: {})", 
        wallet_address,
        manager.known_signatures.len()
    ));

    // Simple monitoring loop
    let mut interval = interval(Duration::from_secs(10)); // Check every 10 seconds
    
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Transactions, "INFO", "TransactionsManager service shutting down");
                break;
            }
            _ = interval.tick() => {
                // Monitor new transactions
                if let Err(e) = do_monitoring_cycle(&mut manager).await {
                    log(LogTag::Transactions, "ERROR", &format!("Monitoring cycle failed: {}", e));
                }
            }
        }
    }
    
    log(LogTag::Transactions, "INFO", "TransactionsManager service stopped");
}

/// Perform one monitoring cycle
async fn do_monitoring_cycle(manager: &mut TransactionsManager) -> Result<(), String> {
    // Check for new transactions
    let new_signatures = manager.check_new_transactions().await?;
    
    // Process new transactions
    for signature in new_signatures {
        if let Err(e) = manager.process_transaction(&signature).await {
            log(LogTag::Transactions, "WARN", &format!(
                "Failed to process transaction {}: {}", 
                &signature[..8], e
            ));
        }
    }

    // Check priority transactions
    if let Err(e) = manager.check_priority_transactions().await {
        log(LogTag::Transactions, "WARN", &format!(
            "Priority transaction check failed: {}", e
        ));
    }

    // Log stats periodically
    if manager.debug_enabled {
        let stats = manager.get_stats();
        log(LogTag::Transactions, "STATS", &format!(
            "Total: {}, New: {}, Priority: {}, Cached: {}", 
            stats.total_transactions,
            stats.new_transactions_count,
            stats.priority_transactions_count,
            stats.known_signatures_count
        ));
    }

    Ok(())
}

/// Load wallet address from config
async fn load_wallet_address_from_config() -> Result<Pubkey, String> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    
    Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address format: {}", e))
}

// =============================================================================
// TRANSACTION ANALYSIS METHODS
// =============================================================================

impl TransactionsManager {
    /// Comprehensive transaction analysis - detects type, extracts swap data, calculates impact
    pub async fn analyze_transaction_comprehensive(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYZE", &format!("Starting comprehensive analysis for {}", &transaction.signature[..8]));
        }

        // Step 1: Fetch full transaction data from RPC if not already present
        if transaction.raw_transaction_data.is_none() {
            if let Err(e) = self.fetch_transaction_data(transaction).await {
                return Err(format!("Failed to fetch transaction data: {}", e));
            }
        }

        // Step 2: Extract basic transaction info (slot, block_time, fee, success)
        self.extract_basic_transaction_info(transaction).await?;

        // Step 3: Analyze transaction type and extract swap data
        self.analyze_transaction_type(transaction).await?;

        // Step 4: Calculate balance changes and position impact
        self.calculate_balance_changes(transaction).await?;

        // Step 5: Detect DEX router and extract router-specific data
        self.detect_dex_router(transaction).await?;

        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYZED", &format!(
                "Analysis complete for {} - Type: {:?}", 
                &transaction.signature[..8], 
                transaction.transaction_type
            ));
        }

        Ok(())
    }

    /// Fetch full transaction data from RPC
    async fn fetch_transaction_data(&self, transaction: &mut Transaction) -> Result<(), String> {
        let rpc_client = get_rpc_client();
        
        let tx_details = rpc_client.get_transaction_details(&transaction.signature).await
            .map_err(|e| format!("RPC error: {}", e))?;

        // Convert TransactionDetails to JSON for storage
        transaction.raw_transaction_data = Some(serde_json::to_value(tx_details)
            .map_err(|e| format!("Failed to serialize transaction data: {}", e))?);

        Ok(())
    }

    /// Extract basic transaction information (slot, time, fee, success)
    async fn extract_basic_transaction_info(&self, transaction: &mut Transaction) -> Result<(), String> {
        if let Some(raw_data) = &transaction.raw_transaction_data {
            // Extract slot directly from the transaction details
            if let Some(slot) = raw_data.get("slot").and_then(|v| v.as_u64()) {
                transaction.slot = Some(slot);
            }

            // Extract meta information
            if let Some(meta) = raw_data.get("meta") {
                // Extract fee
                if let Some(fee) = meta.get("fee").and_then(|v| v.as_u64()) {
                    transaction.fee_sol = fee as f64 / 1_000_000_000.0; // Convert lamports to SOL
                }

                // Check if transaction succeeded (err field is None)
                transaction.success = meta.get("err").is_none();
                
                if let Some(err) = meta.get("err") {
                    transaction.error_message = Some(err.to_string());
                }

                // Extract log messages for analysis
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    transaction.log_messages = logs.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }

        Ok(())
    }

    /// Analyze transaction type based on instructions and log messages
    async fn analyze_transaction_type(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Analyze log messages to detect swap patterns
        let log_text = transaction.log_messages.join(" ");
        
        // Detect Jupiter swaps
        if log_text.contains("Program JUP") || log_text.contains("Jupiter") {
            if let Ok(swap_data) = self.extract_jupiter_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                return Ok(());
            }
        }

        // Detect Raydium swaps
        if log_text.contains("Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            if let Ok(swap_data) = self.extract_raydium_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                return Ok(());
            }
        }

        // Detect Pump.fun transactions
        if log_text.contains("Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
            if let Ok(swap_data) = self.extract_pump_fun_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                return Ok(());
            }
        }

        // Detect GMGN swaps (look for GMGN-specific patterns)
        if log_text.contains("gmgn") || log_text.contains("GMGN") {
            if let Ok(swap_data) = self.extract_gmgn_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                return Ok(());
            }
        }

        // Detect simple SOL/token transfers
        if log_text.contains("Transfer") {
            if let Ok(transfer_data) = self.extract_transfer_data(transaction).await {
                transaction.transaction_type = transfer_data;
                return Ok(());
            }
        }

        // Default to Unknown if we can't identify the type
        transaction.transaction_type = TransactionType::Unknown;
        Ok(())
    }

    /// Extract Jupiter swap data from transaction
    async fn extract_jupiter_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Parse Jupiter-specific swap data from logs and balance changes
        let log_text = transaction.log_messages.join(" ");
        
        // Look for Jupiter swap patterns in logs
        if log_text.contains("SwapEvent") {
            // Try to extract swap amounts from logs
            // This is a simplified version - real implementation would parse instruction data
            return Ok(TransactionType::SwapSolToToken {
                token_mint: "unknown".to_string(),
                sol_amount: 0.0,
                token_amount: 0.0,
                router: "Jupiter".to_string(),
            });
        }

        Err("Not a Jupiter swap".to_string())
    }

    /// Extract Raydium swap data from transaction
    async fn extract_raydium_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("SwapBaseIn") || log_text.contains("SwapBaseOut") {
            return Ok(TransactionType::SwapSolToToken {
                token_mint: "unknown".to_string(),
                sol_amount: 0.0,
                token_amount: 0.0,
                router: "Raydium".to_string(),
            });
        }

        Err("Not a Raydium swap".to_string())
    }

    /// Extract Pump.fun swap data from transaction
    async fn extract_pump_fun_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("buy") || log_text.contains("sell") {
            return Ok(TransactionType::SwapSolToToken {
                token_mint: "unknown".to_string(),
                sol_amount: 0.0,
                token_amount: 0.0,
                router: "Pump.fun".to_string(),
            });
        }

        Err("Not a Pump.fun transaction".to_string())
    }

    /// Extract GMGN swap data from transaction
    async fn extract_gmgn_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // GMGN-specific detection logic
        if log_text.contains("swap") {
            return Ok(TransactionType::SwapSolToToken {
                token_mint: "unknown".to_string(),
                sol_amount: 0.0,
                token_amount: 0.0,
                router: "GMGN".to_string(),
            });
        }

        Err("Not a GMGN swap".to_string())
    }

    /// Extract transfer data from transaction
    async fn extract_transfer_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("Transfer") && transaction.sol_balance_change != 0.0 {
            return Ok(TransactionType::SolTransfer {
                amount: transaction.sol_balance_change.abs(),
                from: "unknown".to_string(),
                to: "unknown".to_string(),
            });
        }

        Err("Not a simple transfer".to_string())
    }

    /// Calculate balance changes from transaction data
    async fn calculate_balance_changes(&self, transaction: &mut Transaction) -> Result<(), String> {
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                // Calculate SOL balance change from pre/post balances
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preBalances").and_then(|v| v.as_array()),
                    meta.get("postBalances").and_then(|v| v.as_array())
                ) {
                    if pre_balances.len() == post_balances.len() && !pre_balances.is_empty() {
                        let pre_sol = pre_balances[0].as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                        let post_sol = post_balances[0].as_u64().unwrap_or(0) as f64 / 1_000_000_000.0;
                        transaction.sol_balance_change = post_sol - pre_sol;
                    }
                }

                // Extract token transfers from post token balances
                if let Some(token_balances) = meta.get("postTokenBalances").and_then(|v| v.as_array()) {
                    for token_balance in token_balances {
                        if let Some(mint) = token_balance.get("mint").and_then(|v| v.as_str()) {
                            let amount = if let Some(ui_amount) = token_balance
                                .get("uiTokenAmount")
                                .and_then(|ui| ui.get("uiAmount"))
                                .and_then(|v| v.as_f64()) {
                                ui_amount
                            } else {
                                0.0
                            };

                            transaction.token_transfers.push(TokenTransfer {
                                mint: mint.to_string(),
                                amount,
                                from: "unknown".to_string(), // Would need to analyze instructions for actual from/to
                                to: "unknown".to_string(),
                                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Detect DEX router and extract router-specific information
    async fn detect_dex_router(&self, transaction: &mut Transaction) -> Result<(), String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Extract swap analysis data based on detected router
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, .. } |
            TransactionType::SwapTokenToSol { router, .. } |
            TransactionType::SwapTokenToToken { router, .. } => {
                transaction.swap_analysis = Some(SwapAnalysis {
                    router: router.clone(),
                    input_token: "SOL".to_string(), // Simplified
                    output_token: "unknown".to_string(),
                    input_amount: transaction.sol_balance_change.abs(),
                    output_amount: 0.0, // Would calculate from token transfers
                    effective_price: 0.0, // Would calculate
                    slippage: 0.0, // Would calculate
                    fee_breakdown: FeeBreakdown {
                        transaction_fee: transaction.fee_sol,
                        router_fee: 0.0,
                        platform_fee: 0.0,
                        total_fees: transaction.fee_sol,
                    },
                });
            }
            _ => {}
        }

        Ok(())
    }

    /// Quick transaction type detection for filtering
    pub fn is_swap_transaction(&self, transaction: &Transaction) -> bool {
        matches!(transaction.transaction_type,
            TransactionType::SwapSolToToken { .. } |
            TransactionType::SwapTokenToSol { .. } |
            TransactionType::SwapTokenToToken { .. }
        )
    }

    /// Check if transaction involves specific token
    pub fn involves_token(&self, transaction: &Transaction, token_mint: &str) -> bool {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint: mint, .. } |
            TransactionType::SwapTokenToSol { token_mint: mint, .. } => {
                mint == token_mint
            }
            TransactionType::SwapTokenToToken { from_mint, to_mint, .. } => {
                from_mint == token_mint || to_mint == token_mint
            }
            TransactionType::TokenTransfer { mint, .. } => {
                mint == token_mint
            }
            _ => false
        }
    }

    /// Get effective price from swap transaction
    pub fn get_effective_price(&self, transaction: &Transaction) -> Option<f64> {
        if let Some(swap_analysis) = &transaction.swap_analysis {
            if swap_analysis.output_amount > 0.0 {
                return Some(swap_analysis.input_amount / swap_analysis.output_amount);
            }
        }
        None
    }

    /// Get transaction summary for logging
    pub fn get_transaction_summary(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, router } => {
                format!("BUY {} SOL → {} tokens via {}", sol_amount, token_amount, router)
            }
            TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, router } => {
                format!("SELL {} tokens → {} SOL via {}", token_amount, sol_amount, router)
            }
            TransactionType::SolTransfer { amount, .. } => {
                format!("SOL Transfer: {} SOL", amount)
            }
            TransactionType::TokenTransfer { mint, amount, .. } => {
                format!("Token Transfer: {} of {}", amount, &mint[..8])
            }
            TransactionType::Spam => "SPAM Transaction".to_string(),
            TransactionType::Unknown => "Unknown Transaction".to_string(),
            _ => "Other Transaction".to_string(),
        }
    }
}

// =============================================================================
// PUBLIC API FOR INTEGRATION
// =============================================================================

/// Add priority transaction from swaps module
pub async fn add_priority_transaction(signature: String) -> Result<(), String> {
    // For now, just log - would integrate with global manager instance
    log(LogTag::Transactions, "PRIORITY", &format!(
        "Priority transaction added: {}", 
        &signature[..8]
    ));
    Ok(())
}

/// Get transaction by signature (for positions.rs integration)
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
    
    if !Path::new(&cache_file).exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&cache_file)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;
    
    let transaction: Transaction = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cached transaction: {}", e))?;
    
    Ok(Some(transaction))
}

/// Check if transaction is verified/finalized
pub async fn is_transaction_verified(signature: &str) -> bool {
    match get_transaction(signature).await {
        Ok(Some(tx)) => tx.finalized && tx.success,
        _ => false,
    }
}

/// Get transaction statistics
pub async fn get_transaction_stats() -> TransactionStats {
    // Default stats - would integrate with global manager
    TransactionStats {
        total_transactions: 0,
        new_transactions_count: 0,
        priority_transactions_count: 0,
        known_signatures_count: 0,
    }
}
