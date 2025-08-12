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
use crate::tokens::{
    get_token_decimals,
    get_token_decimals_safe,
    initialize_price_service,
    TokenDatabase,
    types::PriceSourceType,
};
use crate::tokens::price::get_token_price_blocking_safe;

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
    pub fee_breakdown: Option<FeeBreakdown>,
    
    // Token information integration
    pub token_info: Option<TokenSwapInfo>,
    pub calculated_token_price_sol: Option<f64>,
    pub price_source: Option<PriceSourceType>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
    
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
    pub transaction_fee: f64,        // Base Solana transaction fee (in SOL)
    pub router_fee: f64,            // DEX router fee (in SOL)
    pub platform_fee: f64,          // Platform/referral fee (in SOL)
    pub compute_units_consumed: u64, // Compute units used
    pub compute_unit_price: u64,     // Price per compute unit (micro-lamports)
    pub priority_fee: f64,          // Priority fee paid (in SOL)
    pub rent_costs: f64,            // Account rent costs (in SOL)
    pub ata_creation_cost: f64,     // Associated Token Account creation costs (in SOL)
    pub total_fees: f64,            // Total of all fees (in SOL)
    pub fee_percentage: f64,        // Fee as percentage of transaction value
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSwapInfo {
    pub mint: String,
    pub symbol: String,
    pub decimals: u8,
    pub current_price_sol: Option<f64>,
    pub price_source: Option<PriceSourceType>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPnLInfo {
    pub token_mint: String,
    pub token_symbol: String,
    pub swap_type: String,  // "Buy" or "Sell"
    pub sol_amount: f64,
    pub token_amount: f64,
    pub calculated_price_sol: f64,
    pub market_price_sol: Option<f64>,
    pub price_difference_percent: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub router: String,
    pub fee_sol: f64,
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
    
    // Token database integration
    pub token_database: Option<TokenDatabase>,
}

impl TransactionsManager {
    /// Create new TransactionsManager instance with token database integration
    pub async fn new(wallet_pubkey: Pubkey) -> Result<Self, String> {
        // Initialize token database
        let token_database = match TokenDatabase::new() {
            Ok(db) => Some(db),
            Err(e) => {
                log(LogTag::Transactions, "WARN", &format!("Failed to initialize token database: {}", e));
                None
            }
        };

        // Initialize price service
        if let Err(e) = initialize_price_service().await {
            log(LogTag::Transactions, "WARN", &format!("Failed to initialize price service: {}", e));
        }

        Ok(Self {
            wallet_pubkey,
            debug_enabled: is_debug_transactions_enabled(),
            known_signatures: HashSet::new(),
            priority_transactions: HashMap::new(),
            last_signature_check: None,
            total_transactions: 0,
            new_transactions_count: 0,
            token_database,
        })
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
            fee_breakdown: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
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
        // Use comprehensive analysis instead of basic analysis
        self.analyze_transaction_comprehensive(transaction).await?;

        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYZE", &format!(
                "Transaction {} - Type: {:?}, SOL change: {:.6}", 
                &transaction.signature[..8], 
                transaction.transaction_type,
                transaction.sol_balance_change
            ));
        }

        Ok(())
    }    /// Cache transaction to disk
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
    let mut manager = match TransactionsManager::new(wallet_address).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to create TransactionsManager: {}", e));
            return;
        }
    };
    
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

            // Extract block time
            if let Some(block_time) = raw_data.get("blockTime").and_then(|v| v.as_i64()) {
                transaction.block_time = Some(block_time);
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

                // Extract log messages for analysis - THIS IS CRITICAL FOR SWAP DETECTION
                if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                    transaction.log_messages = logs.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    
                    if self.debug_enabled && !transaction.log_messages.is_empty() {
                        log(LogTag::Transactions, "LOGS", &format!("Found {} log messages for {}", 
                            transaction.log_messages.len(), &transaction.signature[..8]));
                    }
                }

                // Extract instruction information for program ID detection
                if let Some(transaction_data) = raw_data.get("transaction") {
                    if let Some(message) = transaction_data.get("message") {
                        if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
                            for (index, instruction) in instructions.iter().enumerate() {
                                if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|v| v.as_u64()) {
                                    if let Some(account_keys) = message.get("accountKeys").and_then(|v| v.as_array()) {
                                        if let Some(program_id_value) = account_keys.get(program_id_index as usize) {
                                            let program_id = program_id_value.as_str().unwrap_or("unknown").to_string();
                                            
                                            transaction.instructions.push(InstructionInfo {
                                                program_id: program_id.clone(),
                                                instruction_type: format!("instruction_{}", index),
                                                accounts: vec![], // Would extract account indices if needed
                                                data: instruction.get("data").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze transaction type based on instructions and log messages
    async fn analyze_transaction_type(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Analyze log messages to detect swap patterns
        let log_text = transaction.log_messages.join(" ");
        
        if self.debug_enabled {
            log(LogTag::Transactions, "DEBUG", &format!("Analyzing {} with {} log messages", 
                &transaction.signature[..8], transaction.log_messages.len()));
            if !log_text.is_empty() {
                log(LogTag::Transactions, "DEBUG", &format!("Log preview (first 200 chars): {}", 
                    &log_text.chars().take(200).collect::<String>()));
            }
        }
        
        // Enhanced swap detection - try multiple approaches
        
        // 1. Detect Jupiter swaps - multiple ways to identify
        if log_text.contains("Program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") || 
           log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") ||
           log_text.contains("Jupiter") {
            
            if let Ok(swap_data) = self.extract_jupiter_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Jupiter swap detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 2. Detect Raydium swaps
        if log_text.contains("Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") ||
           log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            
            if let Ok(swap_data) = self.extract_raydium_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Raydium swap detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 3. Detect Pump.fun transactions
        if log_text.contains("Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
           log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
            
            if let Ok(swap_data) = self.extract_pump_fun_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Pump.fun transaction detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 4. Detect GMGN swaps
        if log_text.to_lowercase().contains("gmgn") {
            if let Ok(swap_data) = self.extract_gmgn_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - GMGN swap detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 5. Detect Orca swaps
        if log_text.contains("Program 9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") ||
           log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            
            if let Ok(swap_data) = self.extract_orca_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Orca swap detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 6. Enhanced: Balance-based swap detection (catches swaps that failed overall but had meaningful transfers)
        if let Ok(swap_data) = self.extract_balance_based_swap_data(transaction).await {
            transaction.transaction_type = swap_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Balance-based swap detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 7. Enhanced: Token-to-token swap detection based on multiple token transfers
        if let Ok(swap_data) = self.extract_token_to_token_swap_data(transaction).await {
            transaction.transaction_type = swap_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Token-to-token swap detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 8. Check for generic swap patterns in logs
        if log_text.contains("swap") || log_text.contains("Swap") || 
           log_text.contains("buy") || log_text.contains("sell") {
            
            if let Ok(swap_data) = self.extract_generic_swap_data(transaction).await {
                transaction.transaction_type = swap_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Generic swap detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // 9. Detect simple SOL/token transfers
        if log_text.contains("Transfer") && transaction.sol_balance_change != 0.0 {
            if let Ok(transfer_data) = self.extract_transfer_data(transaction).await {
                transaction.transaction_type = transfer_data;
                if self.debug_enabled {
                    log(LogTag::Transactions, "DETECTED", &format!("{} - Transfer detected", 
                        &transaction.signature[..8]));
                }
                return Ok(());
            }
        }

        // Default to Unknown if we can't identify the type
        transaction.transaction_type = TransactionType::Unknown;
        
        if self.debug_enabled {
            log(LogTag::Transactions, "UNKNOWN", &format!("{} - Could not classify transaction type", 
                &transaction.signature[..8]));
        }
        
        Ok(())
    }

    /// Comprehensive fee analysis to extract all fee types
    async fn analyze_fees(&self, transaction: &Transaction) -> Result<FeeBreakdown, String> {
        let mut fee_breakdown = FeeBreakdown {
            transaction_fee: transaction.fee_sol,
            router_fee: 0.0,
            platform_fee: 0.0,
            compute_units_consumed: 0,
            compute_unit_price: 0,
            priority_fee: 0.0,
            rent_costs: 0.0,
            ata_creation_cost: 0.0,
            total_fees: transaction.fee_sol,
            fee_percentage: 0.0,
        };

        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                // Extract compute units information
                if let Some(compute_units) = meta.get("computeUnitsConsumed").and_then(|v| v.as_u64()) {
                    fee_breakdown.compute_units_consumed = compute_units;
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Compute units consumed: {}", 
                            &transaction.signature[..8], 
                            compute_units
                        ));
                    }
                }

                // Extract cost units (compute unit price)
                if let Some(cost_units) = meta.get("costUnits").and_then(|v| v.as_u64()) {
                    fee_breakdown.compute_unit_price = cost_units;
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Cost units: {}", 
                            &transaction.signature[..8], 
                            cost_units
                        ));
                    }
                }

                // Calculate priority fee (difference between cost units and base compute cost)
                // Cost units include both compute cost + priority fee
                let base_compute_cost = fee_breakdown.compute_units_consumed * 5; // 5 micro-lamports per compute unit
                if fee_breakdown.compute_unit_price > base_compute_cost {
                    let priority_fee_lamports = fee_breakdown.compute_unit_price - base_compute_cost;
                    fee_breakdown.priority_fee = priority_fee_lamports as f64 / 1_000_000_000.0;
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Priority fee: {:.9} SOL (base: {}, total: {})", 
                            &transaction.signature[..8], 
                            fee_breakdown.priority_fee,
                            base_compute_cost,
                            fee_breakdown.compute_unit_price
                        ));
                    }
                }

                // Analyze log messages for fee information
                self.analyze_fee_logs(&mut fee_breakdown, transaction).await?;

                // Analyze balance changes for rent costs
                self.analyze_rent_costs(&mut fee_breakdown, transaction).await?;

                // Calculate total fees
                fee_breakdown.total_fees = fee_breakdown.transaction_fee + 
                                         fee_breakdown.router_fee + 
                                         fee_breakdown.platform_fee + 
                                         fee_breakdown.rent_costs + 
                                         fee_breakdown.ata_creation_cost;

                // Calculate fee percentage of transaction value
                // For swaps, calculate percentage against the actual swap amount (excluding fees)
                if transaction.sol_balance_change.abs() > 0.0 {
                    // The actual swap amount is the SOL balance change minus the fees
                    let swap_amount = transaction.sol_balance_change.abs() - fee_breakdown.total_fees;
                    if swap_amount > 0.0 {
                        fee_breakdown.fee_percentage = (fee_breakdown.total_fees / swap_amount) * 100.0;
                    } else {
                        // If fees >= balance change, calculate against balance change
                        fee_breakdown.fee_percentage = (fee_breakdown.total_fees / transaction.sol_balance_change.abs()) * 100.0;
                    }
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Fee calculation: total_fees={:.9}, balance_change={:.9}, swap_amount={:.9}", 
                            &transaction.signature[..8], 
                            fee_breakdown.total_fees,
                            transaction.sol_balance_change.abs(),
                            swap_amount
                        ));
                    }
                }

                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_SUMMARY", &format!(
                        "{} - Total fees: {:.9} SOL ({:.2}%)", 
                        &transaction.signature[..8], 
                        fee_breakdown.total_fees,
                        fee_breakdown.fee_percentage
                    ));
                }
            }
        }

        Ok(fee_breakdown)
    }

    /// Analyze log messages for fee-related information
    async fn analyze_fee_logs(&self, fee_breakdown: &mut FeeBreakdown, transaction: &Transaction) -> Result<(), String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Look for Jupiter fee patterns (but only apply if Jupiter is detected)
        let is_jupiter = log_text.contains("Program JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
        let is_raydium = log_text.contains("Program 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
        
        if is_jupiter && !is_raydium {
            // Jupiter only - typically takes 0.1% fee
            if transaction.sol_balance_change.abs() > 0.0 {
                fee_breakdown.router_fee = transaction.sol_balance_change.abs() * 0.001; // 0.1%
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_DEBUG", &format!(
                        "{} - Estimated Jupiter router fee: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.router_fee
                    ));
                }
            }
        } else if is_raydium && !is_jupiter {
            // Raydium only - typically takes 0.25% fee
            if transaction.sol_balance_change.abs() > 0.0 {
                fee_breakdown.router_fee = transaction.sol_balance_change.abs() * 0.0025; // 0.25%
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_DEBUG", &format!(
                        "{} - Estimated Raydium router fee: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.router_fee
                    ));
                }
            }
        } else if is_jupiter && is_raydium {
            // Both Jupiter and Raydium detected - use Jupiter fee (usually the aggregator)
            if transaction.sol_balance_change.abs() > 0.0 {
                fee_breakdown.router_fee = transaction.sol_balance_change.abs() * 0.001; // 0.1%
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_DEBUG", &format!(
                        "{} - Jupiter + Raydium detected, using Jupiter fee: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.router_fee
                    ));
                }
            }
        }

        // Look for platform/referral fees in logs
        if log_text.contains("referral") || log_text.contains("platform") {
            // Platform fees are typically small, estimate 0.05%
            if transaction.sol_balance_change.abs() > 0.0 {
                fee_breakdown.platform_fee = transaction.sol_balance_change.abs() * 0.0005; // 0.05%
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_DEBUG", &format!(
                        "{} - Estimated platform fee: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.platform_fee
                    ));
                }
            }
        }

        Ok(())
    }

    /// Analyze balance changes for rent costs
    async fn analyze_rent_costs(&self, fee_breakdown: &mut FeeBreakdown, transaction: &Transaction) -> Result<(), String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Count ATA creations (each costs ~0.00203928 SOL)
        let ata_creations = log_text.matches("CreateIdempotent").count() + 
                           log_text.matches("Initialize the associated token account").count();
        
        if ata_creations > 0 {
            fee_breakdown.ata_creation_cost = ata_creations as f64 * 0.00203928; // Standard ATA rent
            
            if self.debug_enabled {
                log(LogTag::Transactions, "FEE_DEBUG", &format!(
                    "{} - ATA creation costs: {} accounts = {:.9} SOL", 
                    &transaction.signature[..8], 
                    ata_creations,
                    fee_breakdown.ata_creation_cost
                ));
            }
        }

        // Look for other rent costs in logs
        let rent_occurrences = log_text.matches("rent").count();
        if rent_occurrences > 0 {
            // Estimate additional rent costs
            fee_breakdown.rent_costs = rent_occurrences as f64 * 0.001; // Rough estimate
            
            if self.debug_enabled {
                log(LogTag::Transactions, "FEE_DEBUG", &format!(
                    "{} - Additional rent costs: {} occurrences = {:.9} SOL", 
                    &transaction.signature[..8], 
                    rent_occurrences,
                    fee_breakdown.rent_costs
                ));
            }
        }

        Ok(())
    }

    async fn extract_jupiter_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Parse Jupiter-specific swap data from logs and balance changes
        let log_text = transaction.log_messages.join(" ");
        
        if self.debug_enabled {
            log(LogTag::Transactions, "JUPITER", &format!("Analyzing Jupiter transaction {}", 
                &transaction.signature[..8]));
            log(LogTag::Transactions, "JUPITER", &format!("SOL change: {:.6}, Token transfers: {}", 
                transaction.sol_balance_change, transaction.token_transfers.len()));
        }
        
        // Look for Jupiter swap patterns in logs
        if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            // Calculate amounts from balance changes
            let sol_change = transaction.sol_balance_change.abs();
            
            if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("Jupiter program detected, SOL change: {:.6}", sol_change));
                for (i, transfer) in transaction.token_transfers.iter().enumerate() {
                    log(LogTag::Transactions, "JUPITER", &format!("Token transfer {}: {} of {}", 
                        i, transfer.amount, &transfer.mint[..8]));
                }
            }
            
            // Check token transfers for the other side of the swap
            let mut token_mint = "unknown".to_string();
            let mut token_amount = 0.0;
            
            // Enhanced: Look for the most significant token transfer
            let mut largest_transfer: Option<&TokenTransfer> = None;
            for transfer in &transaction.token_transfers {
                // Skip very small amounts (dust)
                if transfer.amount.abs() > 0.001 {
                    if largest_transfer.is_none() || transfer.amount.abs() > largest_transfer.unwrap().amount.abs() {
                        largest_transfer = Some(transfer);
                    }
                }
            }
            
            if let Some(transfer) = largest_transfer {
                token_mint = transfer.mint.clone();
                token_amount = transfer.amount.abs();
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!(
                        "{} - Jupiter swap: SOL {:.6}, Token {:.6} ({})", 
                        &transaction.signature[..8], sol_change, token_amount, &token_mint[..8]
                    ));
                }
            } else if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("No significant token transfers found"));
            }
            
            // Enhanced: Detect different swap patterns
            let mut input_token: Option<&TokenTransfer> = None;
            let mut output_token: Option<&TokenTransfer> = None;
            let mut wsol_transfer: Option<&TokenTransfer> = None;
            
            // Categorize token transfers
            for transfer in &transaction.token_transfers {
                if transfer.mint == "So11111111111111111111111111111111111111111112" {
                    // This is WSOL (wrapped SOL)
                    wsol_transfer = Some(transfer);
                } else if transfer.amount < 0.0 && transfer.amount.abs() > 0.001 {
                    // Token being sold (negative amount)
                    input_token = Some(transfer);
                } else if transfer.amount > 0.0 && transfer.amount > 0.001 {
                    // Token being bought (positive amount)
                    output_token = Some(transfer);
                }
            }
            
            if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("Analysis: input_token={}, output_token={}, wsol_transfer={}, sol_change={:.6}", 
                    input_token.is_some(), output_token.is_some(), wsol_transfer.is_some(), transaction.sol_balance_change));
            }
            
            // Pattern 1: Token-to-Token swap (2+ token transfers, minimal SOL change)
            if let (Some(input), Some(output)) = (input_token, output_token) {
                if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("Token-to-token swap detected: {} -> {}", 
                        &input.mint[..8], &output.mint[..8]));
                }
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: input.mint.clone(),
                    to_mint: output.mint.clone(),
                    from_amount: input.amount.abs(),
                    to_amount: output.amount,
                    router: "Jupiter".to_string(),
                });
            }
            
            // Pattern 2: Token-to-SOL swap (token sold, WSOL received, net SOL change)
            if let (Some(input), Some(wsol)) = (input_token, wsol_transfer) {
                if wsol.amount > 0.0 { // WSOL received
                    if self.debug_enabled {
                        log(LogTag::Transactions, "JUPITER", &format!("Token-to-SOL swap detected: {} -> SOL", 
                            &input.mint[..8]));
                    }
                    return Ok(TransactionType::SwapTokenToSol {
                        token_mint: input.mint.clone(),
                        token_amount: input.amount.abs(),
                        sol_amount: wsol.amount, // WSOL received represents SOL obtained
                        router: "Jupiter".to_string(),
                    });
                }
            }
            
            // Pattern 3: SOL-to-Token swap (WSOL sent, token received)
            if let (Some(output), Some(wsol)) = (output_token, wsol_transfer) {
                if wsol.amount < 0.0 { // WSOL sent
                    if self.debug_enabled {
                        log(LogTag::Transactions, "JUPITER", &format!("SOL-to-token swap detected: SOL -> {}", 
                            &output.mint[..8]));
                    }
                    return Ok(TransactionType::SwapSolToToken {
                        token_mint: output.mint.clone(),
                        sol_amount: wsol.amount.abs(), // WSOL sent represents SOL spent
                        token_amount: output.amount,
                        router: "Jupiter".to_string(),
                    });
                }
            }
            
            // Determine swap direction based on SOL balance change
            if transaction.sol_balance_change < -0.001 {
                // Negative SOL change = buying tokens with SOL
                if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("Detected SOL to token swap"));
                }
                return Ok(TransactionType::SwapSolToToken {
                    token_mint,
                    sol_amount: sol_change,
                    token_amount,
                    router: "Jupiter".to_string(),
                });
            } else if transaction.sol_balance_change > 0.001 {
                // Positive SOL change = selling tokens for SOL
                if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("Detected token to SOL swap"));
                }
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint,
                    token_amount,
                    sol_amount: sol_change,
                    router: "Jupiter".to_string(),
                });
            } else if !transaction.token_transfers.is_empty() && sol_change < 0.01 {
                // Minimal SOL change but token transfers exist - could be token-to-token
                if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("Detected potential token-to-token swap"));
                }
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: "unknown".to_string(),
                    to_mint: token_mint,
                    from_amount: 0.0,
                    to_amount: token_amount,
                    router: "Jupiter".to_string(),
                });
            }
            
            if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("Jupiter detected but no swap pattern matched"));
            }
        }

        Err("Not a Jupiter swap".to_string())
    }

    /// Extract Raydium swap data from transaction
    async fn extract_raydium_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            let sol_change = transaction.sol_balance_change.abs();
            let mut token_mint = "unknown".to_string();
            let mut token_amount = 0.0;
            
            if !transaction.token_transfers.is_empty() {
                token_mint = transaction.token_transfers[0].mint.clone();
                token_amount = transaction.token_transfers[0].amount;
            }
            
            if transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint,
                    sol_amount: sol_change,
                    token_amount,
                    router: "Raydium".to_string(),
                });
            } else if transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint,
                    token_amount,
                    sol_amount: sol_change,
                    router: "Raydium".to_string(),
                });
            }
        }

        Err("Not a Raydium swap".to_string())
    }

    /// Extract Pump.fun swap data from transaction
    async fn extract_pump_fun_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
            let sol_change = transaction.sol_balance_change.abs();
            let mut token_mint = "unknown".to_string();
            let mut token_amount = 0.0;
            
            if !transaction.token_transfers.is_empty() {
                token_mint = transaction.token_transfers[0].mint.clone();
                token_amount = transaction.token_transfers[0].amount;
            }
            
            if transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint,
                    sol_amount: sol_change,
                    token_amount,
                    router: "Pump.fun".to_string(),
                });
            } else if transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint,
                    token_amount,
                    sol_amount: sol_change,
                    router: "Pump.fun".to_string(),
                });
            }
        }

        Err("Not a Pump.fun transaction".to_string())
    }

    /// Extract GMGN swap data from transaction
    async fn extract_gmgn_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // GMGN-specific detection logic
        if log_text.to_lowercase().contains("gmgn") {
            let sol_change = transaction.sol_balance_change.abs();
            let mut token_mint = "unknown".to_string();
            let mut token_amount = 0.0;
            
            if !transaction.token_transfers.is_empty() {
                token_mint = transaction.token_transfers[0].mint.clone();
                token_amount = transaction.token_transfers[0].amount;
            }
            
            if transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint,
                    sol_amount: sol_change,
                    token_amount,
                    router: "GMGN".to_string(),
                });
            } else if transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint,
                    token_amount,
                    sol_amount: sol_change,
                    router: "GMGN".to_string(),
                });
            }
        }

        Err("Not a GMGN swap".to_string())
    }

    /// Extract Orca swap data from transaction
    async fn extract_orca_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            let sol_change = transaction.sol_balance_change.abs();
            let mut token_mint = "unknown".to_string();
            let mut token_amount = 0.0;
            
            if !transaction.token_transfers.is_empty() {
                token_mint = transaction.token_transfers[0].mint.clone();
                token_amount = transaction.token_transfers[0].amount;
            }
            
            if transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint,
                    sol_amount: sol_change,
                    token_amount,
                    router: "Orca".to_string(),
                });
            } else if transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint,
                    token_amount,
                    sol_amount: sol_change,
                    router: "Orca".to_string(),
                });
            }
        }

        Err("Not an Orca swap".to_string())
    }

    /// Extract generic swap data from transaction (for unidentified DEXes)
    async fn extract_generic_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Look for generic swap indicators
        if log_text.contains("swap") || log_text.contains("Swap") || 
           log_text.contains("buy") || log_text.contains("sell") ||
           log_text.contains("exchange") || log_text.contains("trade") {
            
            let sol_change = transaction.sol_balance_change.abs();
            
            // Enhanced: Lower threshold and also check for meaningful token transfers
            if sol_change > 0.0001 || !transaction.token_transfers.is_empty() {
                let mut token_mint = "unknown".to_string();
                let mut token_amount = 0.0;
                
                // Enhanced: Find the most significant token transfer
                let mut largest_transfer: Option<&TokenTransfer> = None;
                for transfer in &transaction.token_transfers {
                    if transfer.amount.abs() > 0.001 {
                        if largest_transfer.is_none() || transfer.amount.abs() > largest_transfer.unwrap().amount.abs() {
                            largest_transfer = Some(transfer);
                        }
                    }
                }
                
                if let Some(transfer) = largest_transfer {
                    token_mint = transfer.mint.clone();
                    token_amount = transfer.amount.abs();
                }
                
                // Determine router from any program mentions in logs
                let mut router = "Unknown DEX".to_string();
                if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
                    router = "Jupiter".to_string();
                } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
                    router = "Raydium".to_string();
                } else if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
                    router = "Orca".to_string();
                } else if log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
                    router = "Pump.fun".to_string();
                }
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "GENERIC_SWAP", &format!(
                        "{} - Generic swap detected via {}: SOL {:.6}, Token {:.6}", 
                        &transaction.signature[..8], router, sol_change, token_amount
                    ));
                }
                
                // Enhanced token-to-token detection for generic swaps
                if sol_change < 0.01 && transaction.token_transfers.len() >= 2 {
                    let mut input_token: Option<&TokenTransfer> = None;
                    let mut output_token: Option<&TokenTransfer> = None;
                    
                    for transfer in &transaction.token_transfers {
                        if transfer.amount < 0.0 && transfer.amount.abs() > 0.001 {
                            input_token = Some(transfer);
                        } else if transfer.amount > 0.0 && transfer.amount > 0.001 {
                            output_token = Some(transfer);
                        }
                    }
                    
                    if let (Some(input), Some(output)) = (input_token, output_token) {
                        return Ok(TransactionType::SwapTokenToToken {
                            from_mint: input.mint.clone(),
                            to_mint: output.mint.clone(),
                            from_amount: input.amount.abs(),
                            to_amount: output.amount,
                            router,
                        });
                    }
                }
                
                if transaction.sol_balance_change < -0.0001 {
                    return Ok(TransactionType::SwapSolToToken {
                        token_mint,
                        sol_amount: sol_change,
                        token_amount,
                        router,
                    });
                } else if transaction.sol_balance_change > 0.0001 {
                    return Ok(TransactionType::SwapTokenToSol {
                        token_mint,
                        token_amount,
                        sol_amount: sol_change,
                        router,
                    });
                }
            }
        }

        Err("Not a generic swap".to_string())
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

    /// Enhanced: Balance-based swap detection - detects swaps even if transaction failed
    /// but had meaningful SOL and token balance changes
    async fn extract_balance_based_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Check if we have meaningful balance changes
        let sol_change = transaction.sol_balance_change.abs();
        
        // Only consider as swap if SOL change is significant (more than dust + fees)
        if sol_change > 0.001 && !transaction.token_transfers.is_empty() {
            
            let mut significant_token_transfers = Vec::new();
            
            // Look for significant token transfers (not just tiny amounts)
            for transfer in &transaction.token_transfers {
                // Filter out very small amounts that are likely dust/spam
                if transfer.amount > 0.001 || transfer.mint.contains("So111111") { // Always include wrapped SOL
                    significant_token_transfers.push(transfer);
                }
            }
            
            if !significant_token_transfers.is_empty() {
                let token_mint = significant_token_transfers[0].mint.clone();
                let token_amount = significant_token_transfers[0].amount;
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "BALANCE_SWAP", &format!(
                        "{} - Balance-based detection: SOL change {:.6}, token transfers: {}", 
                        &transaction.signature[..8], sol_change, significant_token_transfers.len()
                    ));
                }
                
                // Determine router from any available program IDs in logs
                let mut router = "Unknown DEX".to_string();
                let log_text = transaction.log_messages.join(" ");
                
                if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
                    router = "Jupiter".to_string();
                } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
                    router = "Raydium".to_string();
                } else if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
                    router = "Orca".to_string();
                } else if log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
                    router = "Pump.fun".to_string();
                }
                
                // Determine swap direction
                if transaction.sol_balance_change < -0.001 {
                    // Bought tokens with SOL
                    return Ok(TransactionType::SwapSolToToken {
                        token_mint,
                        sol_amount: sol_change,
                        token_amount,
                        router,
                    });
                } else if transaction.sol_balance_change > 0.001 {
                    // Sold tokens for SOL
                    return Ok(TransactionType::SwapTokenToSol {
                        token_mint,
                        token_amount,
                        sol_amount: sol_change,
                        router,
                    });
                }
            }
        }

        Err("No significant balance-based swap detected".to_string())
    }

    /// Enhanced: Token-to-token swap detection based on multiple token transfers
    async fn extract_token_to_token_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Look for token-to-token swaps where SOL change is minimal (mostly fees)
        // but there are significant token movements in both directions
        
        if transaction.token_transfers.len() >= 2 {
            let mut input_tokens = Vec::new();
            let mut output_tokens = Vec::new();
            
            // Categorize token transfers by direction (negative = outgoing, positive = incoming)
            for transfer in &transaction.token_transfers {
                if transfer.amount < 0.0 {
                    input_tokens.push(transfer);
                } else if transfer.amount > 0.0 {
                    output_tokens.push(transfer);
                }
            }
            
            // Check if we have tokens going in both directions
            if !input_tokens.is_empty() && !output_tokens.is_empty() {
                let from_token = input_tokens[0];
                let to_token = output_tokens[0];
                
                // Filter out very small amounts (likely dust)
                if from_token.amount.abs() > 0.001 && to_token.amount > 0.001 {
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "TOKEN_SWAP", &format!(
                            "{} - Token-to-token detected: {} {} -> {} {}", 
                            &transaction.signature[..8], 
                            from_token.amount.abs(), &from_token.mint[..8],
                            to_token.amount, &to_token.mint[..8]
                        ));
                    }
                    
                    // Determine router
                    let mut router = "Unknown DEX".to_string();
                    let log_text = transaction.log_messages.join(" ");
                    
                    if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
                        router = "Jupiter".to_string();
                    } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
                        router = "Raydium".to_string();
                    } else if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
                        router = "Orca".to_string();
                    }
                    
                    return Ok(TransactionType::SwapTokenToToken {
                        from_mint: from_token.mint.clone(),
                        to_mint: to_token.mint.clone(),
                        from_amount: from_token.amount.abs(),
                        to_amount: to_token.amount,
                        router,
                    });
                }
            }
        }

        Err("No token-to-token swap detected".to_string())
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
                        
                        if self.debug_enabled {
                            log(LogTag::Transactions, "BALANCE", &format!(
                                "{} - SOL balance: {:.9} -> {:.9} (change: {:.9})", 
                                &transaction.signature[..8], pre_sol, post_sol, transaction.sol_balance_change
                            ));
                        }
                    }
                }

                // Enhanced: Calculate token balance changes by comparing pre and post token balances
                let mut token_balance_map: std::collections::HashMap<String, (f64, f64)> = std::collections::HashMap::new();
                
                // Collect pre token balances
                if let Some(pre_token_balances) = meta.get("preTokenBalances").and_then(|v| v.as_array()) {
                    for token_balance in pre_token_balances {
                        if let Some(mint) = token_balance.get("mint").and_then(|v| v.as_str()) {
                            let amount = token_balance
                                .get("uiTokenAmount")
                                .and_then(|ui| ui.get("uiAmount"))
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            token_balance_map.entry(mint.to_string()).or_insert((0.0, 0.0)).0 = amount;
                        }
                    }
                }
                
                // Collect post token balances
                if let Some(post_token_balances) = meta.get("postTokenBalances").and_then(|v| v.as_array()) {
                    for token_balance in post_token_balances {
                        if let Some(mint) = token_balance.get("mint").and_then(|v| v.as_str()) {
                            let amount = token_balance
                                .get("uiTokenAmount")
                                .and_then(|ui| ui.get("uiAmount"))
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                            token_balance_map.entry(mint.to_string()).or_insert((0.0, 0.0)).1 = amount;
                        }
                    }
                }
                
                // Calculate token balance changes and create token transfers
                for (mint, (pre_amount, post_amount)) in token_balance_map {
                    let change = post_amount - pre_amount;
                    
                    // Only record significant changes (not dust)
                    if change.abs() > 0.000001 {
                        transaction.token_transfers.push(TokenTransfer {
                            mint: mint.clone(),
                            amount: change, // Positive = received, negative = sent
                            from: "unknown".to_string(),
                            to: "unknown".to_string(),
                            program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                        });
                        
                        if self.debug_enabled {
                            log(LogTag::Transactions, "TOKEN", &format!(
                                "{} - Token {} balance: {:.6} -> {:.6} (change: {:.6})", 
                                &transaction.signature[..8], &mint[..8], pre_amount, post_amount, change
                            ));
                        }
                    }
                }
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "BALANCE", &format!(
                        "{} - Found {} token balance changes", 
                        &transaction.signature[..8], transaction.token_transfers.len()
                    ));
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
                    fee_breakdown: transaction.fee_breakdown.clone().unwrap_or(FeeBreakdown {
                        transaction_fee: transaction.fee_sol,
                        router_fee: 0.0,
                        platform_fee: 0.0,
                        compute_units_consumed: 0,
                        compute_unit_price: 0,
                        priority_fee: 0.0,
                        rent_costs: 0.0,
                        ata_creation_cost: 0.0,
                        total_fees: transaction.fee_sol,
                        fee_percentage: 0.0,
                    }),
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

    // =============================================================================
    // COMPREHENSIVE SWAP ANALYSIS AND PNL CALCULATION
    // =============================================================================

    /// Enhanced transaction analysis with token information integration
    async fn analyze_transaction_comprehensive(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYSIS", &format!(
                "Starting comprehensive analysis for {}", &transaction.signature[..8]
            ));
        }

        // Step 1: Fetch full transaction data from RPC if not already present
        if transaction.raw_transaction_data.is_none() {
            self.fetch_transaction_data(transaction).await?;
        }

        // Step 2: Extract basic transaction info
        self.extract_basic_transaction_info(transaction).await?;

        // Step 3: Calculate balance changes BEFORE analyzing transaction type (needed for swap detection)
        self.calculate_balance_changes(transaction).await?;

        // Step 4: Analyze transaction type and extract swap data (now has balance data)
        self.analyze_transaction_type(transaction).await?;

        // Step 5: For swap transactions, integrate token information
        if self.is_swap_transaction(transaction) {
            self.integrate_token_information(transaction).await?;
            self.calculate_swap_price(transaction).await?;
            self.enhance_swap_analysis(transaction).await?;
        }

        // Step 6: Analyze fees comprehensively
        let fee_breakdown = self.analyze_fees(transaction).await?;
        transaction.fee_breakdown = Some(fee_breakdown.clone());

        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYSIS", &format!(
                "Comprehensive analysis complete for {}", &transaction.signature[..8]
            ));
        }

        Ok(())
    }

    /// Integrate token information from tokens module
    async fn integrate_token_information(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        let token_mint = match self.extract_token_mint_from_transaction(transaction) {
            Some(mint) => mint,
            None => return Ok(()), // No token involved
        };

        if self.debug_enabled {
            log(LogTag::Transactions, "TOKEN_INFO", &format!(
                "Integrating token info for mint: {}", &token_mint[..8]
            ));
        }

        // Get token decimals
        let decimals = get_token_decimals(&token_mint).await.unwrap_or(9);
        transaction.token_decimals = Some(decimals);

        // Get token symbol from database
        let symbol = if let Some(ref db) = self.token_database {
            match db.get_token_by_mint(&token_mint) {
                Ok(Some(token_info)) => token_info.symbol,
                _ => format!("TOKEN_{}", &token_mint[..8]),
            }
        } else {
            format!("TOKEN_{}", &token_mint[..8])
        };
        transaction.token_symbol = Some(symbol.clone());

        // Get current market price from price service
        match get_token_price_blocking_safe(&token_mint).await {
            Some(price_sol) => {
                transaction.calculated_token_price_sol = Some(price_sol);
                transaction.price_source = Some(PriceSourceType::CachedPrice);
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "PRICE", &format!(
                        "Market price for {}: {:.12} SOL", 
                        symbol, price_sol
                    ));
                }
            }
            None => {
                log(LogTag::Transactions, "WARN", &format!(
                    "Failed to get market price for {}", symbol
                ));
            }
        }

        // Create TokenSwapInfo
        transaction.token_info = Some(TokenSwapInfo {
            mint: token_mint,
            symbol: symbol.clone(),
            decimals,
            current_price_sol: transaction.calculated_token_price_sol,
            price_source: transaction.price_source.clone(),
            is_verified: transaction.success,
        });

        Ok(())
    }

    /// Calculate effective price paid/received in the swap
    async fn calculate_swap_price(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        let (sol_amount, token_amount) = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { sol_amount, token_amount, .. } => (*sol_amount, *token_amount),
            TransactionType::SwapTokenToSol { sol_amount, token_amount, .. } => (*sol_amount, *token_amount),
            _ => return Ok(()), // Not a swap
        };

        if token_amount > 0.0 {
            let effective_price = sol_amount / token_amount;
            
            // Update swap analysis with calculated price
            if let Some(ref mut swap_analysis) = transaction.swap_analysis {
                swap_analysis.effective_price = effective_price;
            } else {
                // Create basic swap analysis
                let router = self.extract_router_from_transaction(transaction);
                transaction.swap_analysis = Some(SwapAnalysis {
                    router,
                    input_token: self.extract_input_token(transaction),
                    output_token: self.extract_output_token(transaction),
                    input_amount: if matches!(transaction.transaction_type, TransactionType::SwapSolToToken { .. }) { sol_amount } else { token_amount },
                    output_amount: if matches!(transaction.transaction_type, TransactionType::SwapSolToToken { .. }) { token_amount } else { sol_amount },
                    effective_price,
                    slippage: 0.0, // Calculate separately if needed
                    fee_breakdown: transaction.fee_breakdown.clone().unwrap_or_default(),
                });
            }

            if self.debug_enabled {
                log(LogTag::Transactions, "PRICE_CALC", &format!(
                    "Effective price for {}: {:.12} SOL per token", 
                    &transaction.signature[..8], effective_price
                ));
            }
        }

        Ok(())
    }

    /// Enhance swap analysis with additional calculations
    async fn enhance_swap_analysis(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if let Some(ref mut swap_analysis) = transaction.swap_analysis {
            // Calculate slippage if we have market price
            if let Some(market_price) = transaction.calculated_token_price_sol {
                if market_price > 0.0 {
                    let price_diff = (swap_analysis.effective_price - market_price).abs();
                    swap_analysis.slippage = (price_diff / market_price) * 100.0;
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "SLIPPAGE", &format!(
                            "Slippage for {}: {:.2}% (effective: {:.12}, market: {:.12})", 
                            &transaction.signature[..8], 
                            swap_analysis.slippage,
                            swap_analysis.effective_price,
                            market_price
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract token mint from transaction
    fn extract_token_mint_from_transaction(&self, transaction: &Transaction) -> Option<String> {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToSol { token_mint, .. } => Some(token_mint.clone()),
            _ => None,
        }
    }

    /// Extract router from transaction
    fn extract_router_from_transaction(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, .. } => router.clone(),
            TransactionType::SwapTokenToSol { router, .. } => router.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract input token
    fn extract_input_token(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { .. } => "SOL".to_string(),
            TransactionType::SwapTokenToSol { token_mint, .. } => token_mint.clone(),
            _ => "Unknown".to_string(),
        }
    }

    /// Extract output token
    fn extract_output_token(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => token_mint.clone(),
            TransactionType::SwapTokenToSol { .. } => "SOL".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Get all swap transactions for comprehensive analysis
    pub async fn get_all_swap_transactions(&mut self) -> Result<Vec<SwapPnLInfo>, String> {
        let mut swap_transactions = Vec::new();
        
        // Load all cached transactions
        let cache_dir = get_transactions_cache_dir();
        
        if !cache_dir.exists() {
            log(LogTag::Transactions, "WARN", "Transaction cache directory does not exist");
            return Ok(swap_transactions);
        }

        let entries = fs::read_dir(&cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?;

        let mut processed_count = 0;
        let mut swap_count = 0;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if !path.is_file() || !path.extension().map_or(false, |ext| ext == "json") {
                continue;
            }

            // Read and parse transaction
            match self.load_transaction_from_cache(&path).await {
                Ok(mut transaction) => {
                    processed_count += 1;
                    
                    // Re-analyze if needed to ensure we have complete information
                    if transaction.token_info.is_none() && self.is_swap_transaction(&transaction) {
                        if let Err(e) = self.analyze_transaction_comprehensive(&mut transaction).await {
                            log(LogTag::Transactions, "WARN", &format!(
                                "Failed to re-analyze transaction {}: {}", &transaction.signature[..8], e
                            ));
                            continue;
                        }
                        
                        // Re-cache with updated information
                        if let Err(e) = self.cache_transaction(&transaction).await {
                            log(LogTag::Transactions, "WARN", &format!(
                                "Failed to re-cache transaction {}: {}", &transaction.signature[..8], e
                            ));
                        }
                    }
                    
                    // Convert to SwapPnLInfo if it's a swap
                    if let Some(swap_info) = self.convert_to_swap_pnl_info(&transaction) {
                        swap_transactions.push(swap_info);
                        swap_count += 1;
                    }
                }
                Err(e) => {
                    log(LogTag::Transactions, "WARN", &format!(
                        "Failed to load transaction from {}: {}", path.display(), e
                    ));
                }
            }
        }

        log(LogTag::Transactions, "INFO", &format!(
            "Processed {} transactions, found {} swaps", processed_count, swap_count
        ));

        // Sort by timestamp (newest first)
        swap_transactions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(swap_transactions)
    }

    /// Load transaction from cache file
    async fn load_transaction_from_cache(&self, path: &Path) -> Result<Transaction, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let transaction: Transaction = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse transaction: {}", e))?;
        
        Ok(transaction)
    }

    /// Convert transaction to SwapPnLInfo
    fn convert_to_swap_pnl_info(&self, transaction: &Transaction) -> Option<SwapPnLInfo> {
        if !self.is_swap_transaction(transaction) {
            return None;
        }

        let (swap_type, sol_amount, token_amount, token_mint) = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, .. } => {
                ("Buy".to_string(), *sol_amount, *token_amount, token_mint.clone())
            }
            TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, .. } => {
                ("Sell".to_string(), *sol_amount, *token_amount, token_mint.clone())
            }
            _ => return None,
        };

        let calculated_price_sol = if token_amount > 0.0 { sol_amount / token_amount } else { 0.0 };
        
        let market_price_sol = transaction.calculated_token_price_sol;
        let price_difference_percent = if let Some(market_price) = market_price_sol {
            if market_price > 0.0 {
                Some(((calculated_price_sol - market_price) / market_price) * 100.0)
            } else {
                None
            }
        } else {
            None
        };

        let token_symbol = transaction.token_symbol.clone()
            .unwrap_or_else(|| format!("TOKEN_{}", &token_mint[..8]));
        
        let router = self.extract_router_from_transaction(transaction);

        Some(SwapPnLInfo {
            token_mint,
            token_symbol,
            swap_type,
            sol_amount,
            token_amount,
            calculated_price_sol,
            market_price_sol,
            price_difference_percent,
            timestamp: transaction.timestamp,
            signature: transaction.signature.clone(),
            router,
            fee_sol: transaction.fee_sol,
        })
    }

    /// Display comprehensive swap analysis table
    pub fn display_swap_analysis_table(&self, swaps: &[SwapPnLInfo]) {
        if swaps.is_empty() {
            log(LogTag::Transactions, "INFO", "No swap transactions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE SWAP ANALYSIS ===");
        log(LogTag::Transactions, "TABLE", &format!(
            "{:<12} {:<8} {:<15} {:<12} {:<15} {:<15} {:<15} {:<8} {:<12} {:<8}",
            "Timestamp", "Type", "Token", "SOL Amount", "Token Amount", "Calc Price", "Market Price", "Diff %", "Router", "Fee SOL"
        ));
        log(LogTag::Transactions, "TABLE", &"-".repeat(140));

        let mut total_fees = 0.0;
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_sol_spent = 0.0;
        let mut total_sol_received = 0.0;

        for swap in swaps {
            let timestamp_str = swap.timestamp.format("%m-%d %H:%M").to_string();
            let diff_str = match swap.price_difference_percent {
                Some(diff) => format!("{:+.1}%", diff),
                None => "N/A".to_string(),
            };
            let market_price_str = match swap.market_price_sol {
                Some(price) => format!("{:.2e}", price),
                None => "N/A".to_string(),
            };

            log(LogTag::Transactions, "TABLE", &format!(
                "{:<12} {:<8} {:<15} {:<12.6} {:<15.2} {:<15.2e} {:<15} {:<8} {:<12} {:<8.6}",
                timestamp_str,
                swap.swap_type,
                &swap.token_symbol[..15.min(swap.token_symbol.len())],
                swap.sol_amount,
                swap.token_amount,
                swap.calculated_price_sol,
                market_price_str,
                diff_str,
                &swap.router[..12.min(swap.router.len())],
                swap.fee_sol
            ));

            total_fees += swap.fee_sol;
            if swap.swap_type == "Buy" {
                buy_count += 1;
                total_sol_spent += swap.sol_amount;
            } else {
                sell_count += 1;
                total_sol_received += swap.sol_amount;
            }
        }

        log(LogTag::Transactions, "TABLE", &"-".repeat(140));
        log(LogTag::Transactions, "TABLE", &format!(
            "SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
            buy_count, total_sol_spent, sell_count, total_sol_received, total_fees, 
            total_sol_received - total_sol_spent - total_fees
        ));
        log(LogTag::Transactions, "TABLE", "=== END ANALYSIS ===");
    }
}

impl Default for FeeBreakdown {
    fn default() -> Self {
        Self {
            transaction_fee: 0.0,
            router_fee: 0.0,
            platform_fee: 0.0,
            compute_units_consumed: 0,
            compute_unit_price: 0,
            priority_fee: 0.0,
            rent_costs: 0.0,
            ata_creation_cost: 0.0,
            total_fees: 0.0,
            fee_percentage: 0.0,
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
