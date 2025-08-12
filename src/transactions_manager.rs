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
use tabled::{Table, Tabled, settings::{Style, Modify, object::Rows, Alignment}};

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
    AtaCreate {
        mint: String,
        owner: String,
        ata_address: String,
        cost: f64,
    },
    AtaClose {
        mint: String,
        owner: String,
        ata_address: String,
        rent_reclaimed: f64,
    },
    SpamBulk {
        transaction_count: usize,
        suspected_spam_type: String,
    },
    ProgramDeploy {
        program_id: String,
        deployer: String,
    },
    ProgramUpgrade {
        program_id: String,
        authority: String,
    },
    StakingDelegate {
        stake_account: String,
        validator: String,
        amount: f64,
    },
    StakingWithdraw {
        stake_account: String,
        amount: f64,
    },
    ComputeBudget {
        compute_units: u32,
        compute_unit_price: u64,
    },
    NftMint {
        collection_id: String,
        leaf_asset_id: String,
        nft_type: String, // "Compressed NFT", "Standard NFT", etc.
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
    pub rent_costs: f64,            // Account rent costs (in SOL) - infrastructure, not trading fees
    pub ata_creation_cost: f64,     // Associated Token Account creation costs (in SOL) - infrastructure
    pub total_fees: f64,            // Total of TRADING fees only (excludes infrastructure costs)
    pub fee_percentage: f64,        // Trading fee as percentage of transaction value
    
    // Enhanced ATA tracking
    pub ata_creations_count: u32,   // Number of ATAs created
    pub ata_closures_count: u32,    // Number of ATAs closed  
    pub net_ata_rent_flow: f64,     // Net ATA rent flow: positive = net recovery, negative = net cost
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
    pub timestamp: DateTime<Utc>,
    pub signature: String,
    pub router: String,
    pub fee_sol: f64,
    pub ata_rents: f64,     // ATA creation and rent costs (in SOL)
    pub slot: Option<u64>,  // Solana slot number for reliable chronological sorting
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionAnalysis {
    pub token_mint: String,
    pub token_symbol: String,
    pub status: PositionStatus,
    pub total_tokens_bought: f64,
    pub total_tokens_sold: f64,
    pub remaining_tokens: f64,
    pub total_sol_invested: f64,
    pub total_sol_received: f64,
    pub net_sol_flow: f64,
    pub average_buy_price: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub total_pnl: f64,
    pub total_fees: f64,
    pub total_ata_rents: f64,
    pub buy_count: u32,
    pub sell_count: u32,
    pub first_buy_timestamp: Option<DateTime<Utc>>,
    pub last_activity_timestamp: Option<DateTime<Utc>>,
    pub duration_hours: f64,
    pub transactions: Vec<PositionTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionStatus {
    Open,           // Has remaining tokens, no sells
    Closed,         // No remaining tokens, fully sold
    PartiallyReduced, // Has remaining tokens, some sells
    Oversold,       // Negative token balance (sold more than bought)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionState {
    pub token_mint: String,
    pub token_symbol: String,
    pub total_tokens: f64,
    pub total_sol_invested: f64,
    pub total_sol_received: f64,
    pub total_fees: f64,
    pub total_ata_rents: f64,
    pub buy_count: u32,
    pub sell_count: u32,
    pub first_buy_slot: Option<u64>,
    pub last_activity_slot: Option<u64>,
    pub first_buy_timestamp: Option<DateTime<Utc>>,
    pub last_activity_timestamp: Option<DateTime<Utc>>,
    pub average_buy_price: f64,
    pub transactions: Vec<PositionTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionTransaction {
    pub signature: String,
    pub swap_type: String,
    pub sol_amount: f64,
    pub token_amount: f64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub slot: Option<u64>,
    pub router: String,
    pub fee_sol: f64,
    pub ata_rents: f64,
}

// =============================================================================
// TABLED DISPLAY STRUCTURES
// =============================================================================

/// Tabled structure for swap analysis display
#[derive(Tabled)]
pub struct SwapDisplayRow {
    #[tabled(rename = "Sig")]
    pub signature: String,
    #[tabled(rename = "Slot")]
    pub slot: String,
    #[tabled(rename = "Type")]
    pub swap_type: String,
    #[tabled(rename = "Token")]
    pub token: String,
    #[tabled(rename = "SOL Amount")]
    pub sol_amount: String,
    #[tabled(rename = "Token Amount")]
    pub token_amount: String,
    #[tabled(rename = "Price (SOL)")]
    pub price: String,
    #[tabled(rename = "ATA Rents")]
    pub ata_rents: String,
    #[tabled(rename = "Router")]
    pub router: String,
    #[tabled(rename = "Fee")]
    pub fee: String,
}

/// Tabled structure for position analysis display
#[derive(Tabled)]
pub struct PositionDisplayRow {
    #[tabled(rename = "Token")]
    pub token: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Buys")]
    pub buys: String,
    #[tabled(rename = "Sold")]
    pub sold: String,
    #[tabled(rename = "Remaining")]
    pub remaining: String,
    #[tabled(rename = "SOL In")]
    pub sol_in: String,
    #[tabled(rename = "SOL Out")]
    pub sol_out: String,
    #[tabled(rename = "Net PnL")]
    pub net_pnl: String,
    #[tabled(rename = "Avg Price")]
    pub avg_price: String,
    #[tabled(rename = "Fees")]
    pub fees: String,
    #[tabled(rename = "Duration")]
    pub duration: String,
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

    /// Process a single transaction (cache-first approach)
    pub async fn process_transaction(&mut self, signature: &str) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "PROCESS", &format!("Processing transaction: {}", &signature[..8]));
        }

        // Check if priority transaction
        let is_priority = self.priority_transactions.contains_key(signature);

        // Check if we already have this transaction cached and can avoid RPC call
        let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
        let use_cache = Path::new(&cache_file).exists();

        let mut transaction = if use_cache {
            // Load from cache and recalculate
            if self.debug_enabled {
                log(LogTag::Transactions, "CACHE_LOAD", &format!("Loading cached transaction: {}", &signature[..8]));
            }
            self.recalculate_cached_transaction(Path::new(&cache_file)).await?
        } else {
            // Fetch fresh data from RPC
            if self.debug_enabled {
                log(LogTag::Transactions, "RPC_FETCH", &format!("Fetching new transaction: {}", &signature[..8]));
            }
            
            let rpc_client = get_rpc_client();
            let tx_data = rpc_client
                .get_transaction_details_premium_rpc(signature)
                .await
                .map_err(|e| format!("Failed to fetch transaction details: {}", e))?;

            // Create Transaction structure
            // Convert block_time to proper timestamp if available
            let timestamp = if let Some(block_time) = tx_data.block_time {
                DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            let mut transaction = Transaction {
                signature: signature.to_string(),
                slot: Some(tx_data.slot),
                block_time: tx_data.block_time,
                timestamp,
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
                cache_file_path: cache_file.clone(),
            };

            // Analyze transaction type and extract details
            self.analyze_transaction(&mut transaction).await?;
            
            transaction
        };

        // Always cache the result (updates existing cache with new analysis)
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

    /// Fetch and analyze ALL wallet transactions from blockchain
    /// This method fetches comprehensive transaction history directly from the blockchain
    /// and processes each transaction with full analysis, bypassing the cache
    pub async fn fetch_all_wallet_transactions(&mut self, max_count: usize) -> Result<Vec<Transaction>, String> {
        log(LogTag::Transactions, "INFO", &format!(
            "Starting comprehensive blockchain fetch for wallet {} (max {} transactions)", 
            self.wallet_pubkey, max_count
        ));

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = 100; // Fetch in batches to avoid rate limits
        let mut total_fetched = 0;

        log(LogTag::Transactions, "FETCH", "Fetching transaction signatures from blockchain...");

        // Fetch transaction signatures in batches
        while total_fetched < max_count {
            let remaining = max_count - total_fetched;
            let current_batch_size = batch_size.min(remaining);

            let signatures = match rpc_client
                .get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    current_batch_size,
                    before_signature.as_deref(),
                )
                .await
            {
                Ok(sigs) => sigs,
                Err(e) => {
                    log(LogTag::Transactions, "ERROR", &format!(
                        "Failed to fetch signatures batch: {}", e
                    ));
                    break;
                }
            };

            if signatures.is_empty() {
                log(LogTag::Transactions, "INFO", "No more signatures available");
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;
            
            log(LogTag::Transactions, "FETCH", &format!(
                "Fetched batch of {} signatures (total: {}/{})", 
                batch_count, total_fetched, max_count
            ));

            // Process each transaction in this batch
            for (index, sig_info) in signatures.iter().enumerate() {
                let signature = sig_info.signature.clone();
                
                if self.debug_enabled && index % 10 == 0 {
                    log(LogTag::Transactions, "PROGRESS", &format!(
                        "Processing signature {}/{} in batch: {}", 
                        index + 1, batch_count, &signature[..8]
                    ));
                }

                // Process transaction with full analysis (bypassing cache check)
                match self.process_transaction_direct(&signature).await {
                    Ok(transaction) => {
                        all_transactions.push(transaction);
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "WARN", &format!(
                            "Failed to process transaction {}: {}", &signature[..8], e
                        ));
                    }
                }

                // Small delay to avoid overwhelming RPC
                if index % 5 == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }

            // Set the before signature for the next batch
            before_signature = Some(signatures.last().unwrap().signature.clone());

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        log(LogTag::Transactions, "SUCCESS", &format!(
            "Completed comprehensive fetch: {} transactions processed", 
            all_transactions.len()
        ));

        Ok(all_transactions)
    }

    /// Process transaction directly from blockchain (bypassing cache)
    /// This is similar to process_transaction but forces fresh fetch from RPC
    async fn process_transaction_direct(&mut self, signature: &str) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "DIRECT", &format!(
                "Processing transaction directly from blockchain: {}", &signature[..8]
            ));
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            commitment_state: CommitmentState::Confirmed,
            confirmation_status: ConfirmationStatus::Confirmed,
            finalized: true,
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            success: false,
            error_message: None,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            raw_transaction_data: None,
            log_messages: Vec::new(),
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
            is_priority: false,
            priority_added_at: None,
            last_updated: Utc::now(),
            cache_file_path: format!("{}/{}.json", get_transactions_cache_dir().display(), signature),
        };

        // Fetch fresh transaction data from blockchain
        self.fetch_transaction_data(&mut transaction).await?;

        // Perform comprehensive analysis
        self.analyze_transaction_comprehensive(&mut transaction).await?;

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Recalculate analysis for existing transaction without re-fetching raw data
    /// This preserves all raw blockchain data and only updates calculated fields
    pub async fn recalculate_transaction_analysis(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "RECALC", &format!(
                "Recalculating analysis for transaction: {}", &transaction.signature[..8]
            ));
        }

        // Update timestamp
        transaction.last_updated = Utc::now();

        // Reset all calculated fields to default values (preserve raw data)
        transaction.transaction_type = TransactionType::Unknown;
        transaction.direction = TransactionDirection::Internal;
        transaction.sol_balance_change = 0.0;
        transaction.token_transfers = Vec::new();
        transaction.swap_analysis = None;
        transaction.position_impact = None;
        transaction.profit_calculation = None;
        transaction.fee_breakdown = None;
        transaction.token_info = None;
        transaction.calculated_token_price_sol = None;
        transaction.price_source = None;
        transaction.token_symbol = None;
        transaction.token_decimals = None;

        // Recalculate all analysis using existing raw data
        if transaction.raw_transaction_data.is_some() {
            // Re-run the comprehensive analysis using cached raw data
            self.analyze_transaction_comprehensive(transaction).await?;
            
            if self.debug_enabled {
                log(LogTag::Transactions, "RECALC", &format!(
                    "✅ Analysis recalculated: {} -> {:?}", 
                    &transaction.signature[..8], 
                    transaction.transaction_type
                ));
            }
        } else {
            log(LogTag::Transactions, "WARNING", &format!(
                "No raw transaction data available for {}, skipping recalculation", 
                &transaction.signature[..8]
            ));
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

    /// Get transaction data from cache first, fetch from blockchain only if needed
    async fn get_or_fetch_transaction_data(&self, signature: &str) -> Result<serde_json::Value, String> {
        // First, try to load from cache
        let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
        
        if Path::new(&cache_file).exists() {
            if self.debug_enabled {
                log(LogTag::Transactions, "CACHE_HIT", &format!("Using cached data for {}", &signature[..8]));
            }
            
            let content = fs::read_to_string(&cache_file)
                .map_err(|e| format!("Failed to read cache file: {}", e))?;
            
            let cached_transaction: Transaction = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse cached transaction: {}", e))?;
            
            if let Some(raw_data) = cached_transaction.raw_transaction_data {
                return Ok(raw_data);
            }
        }

        // Cache miss or no raw data - fetch from blockchain
        if self.debug_enabled {
            log(LogTag::Transactions, "CACHE_MISS", &format!("Fetching from blockchain for {}", &signature[..8]));
        }
        
        let rpc_client = get_rpc_client();
        let tx_details = rpc_client.get_transaction_details(signature).await
            .map_err(|e| format!("RPC error: {}", e))?;

        // Convert TransactionDetails to JSON for storage
        let raw_data = serde_json::to_value(tx_details)
            .map_err(|e| format!("Failed to serialize transaction data: {}", e))?;

        Ok(raw_data)
    }

    /// Fetch full transaction data from RPC (now uses cache-first strategy)
    async fn fetch_transaction_data(&self, transaction: &mut Transaction) -> Result<(), String> {
        transaction.raw_transaction_data = Some(self.get_or_fetch_transaction_data(&transaction.signature).await?);
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

                // Check if transaction succeeded (err field is None or null)
                transaction.success = meta.get("err").map_or(true, |v| v.is_null());
                
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
                                
                                // Handle direct programId format (already parsed format)
                                if let Some(program_id) = instruction.get("programId").and_then(|v| v.as_str()) {
                                    transaction.instructions.push(InstructionInfo {
                                        program_id: program_id.to_string(),
                                        instruction_type: format!("instruction_{}", index),
                                        accounts: vec![], // Would extract account details if needed
                                        data: instruction.get("data").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                    });
                                }
                                // Handle programIdIndex format (raw format that needs resolution)
                                else if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|v| v.as_u64()) {
                                    if let Some(account_keys) = message.get("accountKeys").and_then(|v| v.as_array()) {
                                        // Try to get program ID from account keys
                                        let program_id = if let Some(program_id_value) = account_keys.get(program_id_index as usize) {
                                            // Handle both string format and object format
                                            if let Some(pubkey_str) = program_id_value.as_str() {
                                                pubkey_str.to_string()
                                            } else if let Some(pubkey_obj) = program_id_value.get("pubkey").and_then(|v| v.as_str()) {
                                                pubkey_obj.to_string()
                                            } else {
                                                "unknown".to_string()
                                            }
                                        } else {
                                            "unknown".to_string()
                                        };
                                        
                                        transaction.instructions.push(InstructionInfo {
                                            program_id,
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

        // 10. Detect ATA (Associated Token Account) operations
        if let Ok(ata_data) = self.extract_ata_operations(transaction).await {
            transaction.transaction_type = ata_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - ATA operation detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 11. Detect staking operations
        if let Ok(staking_data) = self.extract_staking_operations(transaction).await {
            transaction.transaction_type = staking_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Staking operation detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 12. Detect program deployment/upgrade
        if let Ok(program_data) = self.extract_program_operations(transaction).await {
            transaction.transaction_type = program_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Program operation detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 13. Detect compute budget instructions
        if let Ok(compute_data) = self.extract_compute_budget_operations(transaction).await {
            transaction.transaction_type = compute_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Compute budget operation detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 14. Detect spam bulk transactions
        if let Ok(spam_data) = self.extract_spam_bulk_operations(transaction).await {
            transaction.transaction_type = spam_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Spam bulk operation detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 15. Enhanced: Instruction-based classification for remaining unknown transactions
        if let Ok(instruction_data) = self.extract_instruction_based_type(transaction).await {
            transaction.transaction_type = instruction_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "DETECTED", &format!("{} - Instruction-based type detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // Default to Unknown only after all detection methods have been tried
        transaction.transaction_type = TransactionType::Unknown;
        
        if self.debug_enabled {
            log(LogTag::Transactions, "UNKNOWN", &format!("{} - Could not classify transaction type after comprehensive analysis", 
                &transaction.signature[..8]));
            
            // Enhanced debugging for unknown transactions
            log(LogTag::Transactions, "DEBUG", &format!("Transaction details for {}:", &transaction.signature[..8]));
            log(LogTag::Transactions, "DEBUG", &format!("  Instructions: {}", transaction.instructions.len()));
            log(LogTag::Transactions, "DEBUG", &format!("  Token transfers: {}", transaction.token_transfers.len()));
            log(LogTag::Transactions, "DEBUG", &format!("  SOL balance change: {:.9}", transaction.sol_balance_change));
            log(LogTag::Transactions, "DEBUG", &format!("  Success: {}", transaction.success));
            
            if !transaction.instructions.is_empty() {
                log(LogTag::Transactions, "DEBUG", &format!("  First instruction program: {}", 
                    transaction.instructions[0].program_id));
            }
            
            if transaction.log_messages.len() > 0 {
                let log_preview = transaction.log_messages.join(" ");
                let preview = if log_preview.len() > 300 { 
                    format!("{}...", &log_preview[..300]) 
                } else { 
                    log_preview 
                };
                log(LogTag::Transactions, "DEBUG", &format!("  Log preview: {}", preview));
            }
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
            ata_creations_count: 0,
            ata_closures_count: 0,
            net_ata_rent_flow: 0.0,
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

                // Calculate priority fee from actual transaction data
                // Transaction fee = base fee (5000 lamports) + compute cost + priority fee
                let total_fee_lamports = (transaction.fee_sol * 1_000_000_000.0) as u64;
                let base_fee_lamports = 5000; // Standard Solana base fee
                let compute_cost_lamports = fee_breakdown.compute_units_consumed * 5; // 5 micro-lamports per CU converted to lamports
                
                // Priority fee is what's left after base fee and compute cost
                if total_fee_lamports > base_fee_lamports + compute_cost_lamports {
                    let priority_fee_lamports = total_fee_lamports - base_fee_lamports - compute_cost_lamports;
                    fee_breakdown.priority_fee = priority_fee_lamports as f64 / 1_000_000_000.0;
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Priority fee: {:.9} SOL (total: {} lamports, base: {}, compute: {}, priority: {})", 
                            &transaction.signature[..8], 
                            fee_breakdown.priority_fee,
                            total_fee_lamports,
                            base_fee_lamports,
                            compute_cost_lamports,
                            priority_fee_lamports
                        ));
                    }
                } else if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_DEBUG", &format!(
                        "{} - No priority fee detected (total fee covers base + compute only)", 
                        &transaction.signature[..8]
                    ));
                }

                // Analyze log messages for fee information
                self.analyze_fee_logs(&mut fee_breakdown, transaction).await?;

                // Analyze balance changes for rent costs
                self.analyze_rent_costs(&mut fee_breakdown, transaction).await?;

                // Calculate total fees
                // Note: ATA creation cost is a form of rent payment, so include it in rent_costs
                fee_breakdown.rent_costs += fee_breakdown.ata_creation_cost;
                
                // IMPORTANT: total_fees should ONLY include actual trading fees, NOT infrastructure costs
                // ATA creation and rent costs are one-time infrastructure costs, not trading fees
                fee_breakdown.total_fees = fee_breakdown.transaction_fee + 
                                         fee_breakdown.router_fee + 
                                         fee_breakdown.platform_fee + 
                                         fee_breakdown.priority_fee;
                                         // rent_costs and ata_creation_cost are tracked separately

                // Calculate fee percentage of transaction value
                // For swaps, calculate percentage against the actual swap amount (excluding ALL costs)
                if transaction.sol_balance_change.abs() > 0.0 {
                    // The actual swap amount is the SOL balance change minus ALL costs (fees + infrastructure)
                    let total_costs = fee_breakdown.total_fees + fee_breakdown.rent_costs;
                    let swap_amount = transaction.sol_balance_change.abs() - total_costs;
                    if swap_amount > 0.0 {
                        // Calculate fee percentage based on trading fees only (not infrastructure costs)
                        fee_breakdown.fee_percentage = (fee_breakdown.total_fees / swap_amount) * 100.0;
                    } else {
                        // If total costs >= balance change, calculate against balance change
                        fee_breakdown.fee_percentage = (fee_breakdown.total_fees / transaction.sol_balance_change.abs()) * 100.0;
                    }
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FEE_DEBUG", &format!(
                            "{} - Fee calculation: trading_fees={:.9}, infrastructure_costs={:.9}, balance_change={:.9}, swap_amount={:.9}", 
                            &transaction.signature[..8], 
                            fee_breakdown.total_fees,
                            fee_breakdown.rent_costs,
                            transaction.sol_balance_change.abs(),
                            swap_amount
                        ));
                    }
                }

                if self.debug_enabled {
                    log(LogTag::Transactions, "FEE_SUMMARY", &format!(
                        "{} - Trading fees: {:.9} SOL ({:.2}%), Infrastructure costs: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.total_fees,
                        fee_breakdown.fee_percentage,
                        fee_breakdown.rent_costs
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

    /// Comprehensive ATA analysis - detects creations, closures, and net rent impact
    /// ATA rent is recoverable: creating ATAs costs SOL, closing ATAs returns SOL
    async fn analyze_rent_costs(&self, fee_breakdown: &mut FeeBreakdown, transaction: &Transaction) -> Result<(), String> {
        let log_text = transaction.log_messages.join(" ");
        
        // CRITICAL: Count ATA operations accurately by analyzing both logs and balance changes
        let (ata_creations, ata_closures, net_ata_rent_flow) = self.calculate_precise_ata_operations(transaction).await?;
        
        if self.debug_enabled {
            log(LogTag::Transactions, "ATA_ANALYSIS", &format!(
                "Transaction {}: {} ATAs created, {} ATAs closed, net rent flow: {:.9} SOL", 
                &transaction.signature[..8], ata_creations, ata_closures, net_ata_rent_flow
            ));
        }
        
        // Store all ATA information in fee breakdown
        fee_breakdown.ata_creations_count = ata_creations;
        fee_breakdown.ata_closures_count = ata_closures;
        fee_breakdown.net_ata_rent_flow = net_ata_rent_flow;
        fee_breakdown.ata_creation_cost = ata_creations as f64 * 0.00203928;
        fee_breakdown.rent_costs = net_ata_rent_flow.abs(); // Absolute value for display
        
        if self.debug_enabled {
            log(LogTag::Transactions, "ATA_DETAILED", &format!(
                "ATA Details - Created: {} (cost: {:.9} SOL), Closed: {} (recovered: {:.9} SOL), Net: {:.9} SOL",
                ata_creations, ata_creations as f64 * 0.00203928,
                ata_closures, ata_closures as f64 * 0.00203928,
                net_ata_rent_flow
            ));
        }

        Ok(())
    }

    /// Calculate precise ATA operations and net rent flow
    /// Returns: (ata_creations, ata_closures, net_rent_flow)
    /// net_rent_flow: negative = net cost, positive = net recovery
    async fn calculate_precise_ata_operations(&self, transaction: &Transaction) -> Result<(u32, u32, f64), String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Method 1: Count from log messages (most accurate)
        let ata_creations = log_text.matches("Initialize the associated token account").count() as u32;
        let ata_closures = log_text.matches("Instruction: CloseAccount").count() as u32;
        
        // Method 2: Verify with balance changes (cross-validation)
        let balance_verification = self.verify_ata_operations_from_balances(transaction, ata_creations, ata_closures).await?;
        
        // Method 3: Calculate net rent flow
        // ATA creation costs ≈0.00203928 SOL each
        // ATA closure recovers ≈0.00203928 SOL each
        let estimated_net_flow = (ata_closures as f64 * 0.00203928) - (ata_creations as f64 * 0.00203928);
        
        // Use balance verification if there's a significant discrepancy
        let final_net_flow = if balance_verification.is_some() && 
            (estimated_net_flow - balance_verification.unwrap()).abs() > 0.001 {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "ATA_CORRECTION", &format!(
                    "Using balance-based ATA calculation: {:.9} vs estimated {:.9}",
                    balance_verification.unwrap(), estimated_net_flow
                ));
            }
            balance_verification.unwrap()
        } else {
            estimated_net_flow
        };
        
        if self.debug_enabled && (ata_creations > 0 || ata_closures > 0) {
            log(LogTag::Transactions, "ATA_SUMMARY", &format!(
                "Final ATA analysis: +{} created, -{} closed, net flow: {:.9} SOL",
                ata_creations, ata_closures, final_net_flow
            ));
        }
        
        Ok((ata_creations, ata_closures, final_net_flow))
    }

    /// Verify ATA operations by analyzing account balance changes
    /// This provides cross-validation for log-based counting
    async fn verify_ata_operations_from_balances(&self, transaction: &Transaction, expected_creations: u32, expected_closures: u32) -> Result<Option<f64>, String> {
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                // Look for accounts that went from 0 to ATA_RENT (creations) or ATA_RENT to 0 (closures)
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preBalances").and_then(|v| v.as_array()),
                    meta.get("postBalances").and_then(|v| v.as_array())
                ) {
                    let mut detected_creations = 0u32;
                    let mut detected_closures = 0u32;
                    let mut total_rent_recovered = 0.0;
                    let mut total_rent_spent = 0.0;
                    
                    // Standard ATA rent in lamports (≈2039280 lamports = 0.00203928 SOL)
                    let ata_rent_lamports = 2039280i64;
                    let tolerance = 10000i64; // Small tolerance for rent variations
                    
                    for (i, (pre, post)) in pre_balances.iter().zip(post_balances.iter()).enumerate() {
                        if let (Some(pre_val), Some(post_val)) = (pre.as_i64(), post.as_i64()) {
                            let change = post_val - pre_val;
                            
                            // Detect ATA creation: account went from 0 to ~ATA_RENT
                            if pre_val == 0 && (post_val - ata_rent_lamports).abs() < tolerance {
                                detected_creations += 1;
                                total_rent_spent += post_val as f64 / 1_000_000_000.0;
                                
                                if self.debug_enabled {
                                    log(LogTag::Transactions, "ATA_DETECT", &format!(
                                        "Detected ATA creation at account {}: {} lamports ({:.9} SOL)",
                                        i, post_val, post_val as f64 / 1_000_000_000.0
                                    ));
                                }
                            }
                            
                            // Detect ATA closure: account went from ~ATA_RENT to 0
                            if post_val == 0 && (pre_val - ata_rent_lamports).abs() < tolerance {
                                detected_closures += 1;
                                total_rent_recovered += pre_val as f64 / 1_000_000_000.0;
                                
                                if self.debug_enabled {
                                    log(LogTag::Transactions, "ATA_DETECT", &format!(
                                        "Detected ATA closure at account {}: {} lamports ({:.9} SOL) recovered",
                                        i, pre_val, pre_val as f64 / 1_000_000_000.0
                                    ));
                                }
                            }
                        }
                    }
                    
                    // Cross-validate with log-based counting
                    if detected_creations != expected_creations || detected_closures != expected_closures {
                        if self.debug_enabled {
                            log(LogTag::Transactions, "ATA_MISMATCH", &format!(
                                "Balance-based vs log-based mismatch: created {}/{}, closed {}/{}",
                                detected_creations, expected_creations, detected_closures, expected_closures
                            ));
                        }
                    }
                    
                    // Return net rent flow (positive = net recovery, negative = net cost)
                    let net_rent_flow = total_rent_recovered - total_rent_spent;
                    if detected_creations > 0 || detected_closures > 0 {
                        return Ok(Some(net_rent_flow));
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Determine the specific DEX router based on program IDs in the transaction
    fn determine_swap_router(&self, transaction: &Transaction) -> String {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for specific DEX program IDs in the logs
        if log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") {
            return "Pump.fun".to_string();
        }
        
        // Check instructions for program IDs
        for instruction in &transaction.instructions {
            match instruction.program_id.as_str() {
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
                    return "Pump.fun".to_string();
                }
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                    return "Raydium".to_string();
                }
                "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ" => {
                    return "Raydium CAMM".to_string();
                }
                "CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh" => {
                    return "Raydium CPMM".to_string();
                }
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
                    return "Raydium CPMM".to_string();
                }
                "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP" => {
                    return "Orca".to_string();
                }
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => {
                    return "Orca Whirlpool".to_string();
                }
                "srmqPiDkXBFmqxeQwEeozZGqw5VKc7QNNbE6Y5YNBqU" => {
                    return "Serum".to_string();
                }
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => {
                    // Jupiter aggregator - check for underlying DEX
                    if log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") {
                        return "Jupiter (via Pump.fun)".to_string();
                    }
                    if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
                        return "Jupiter (via Raydium)".to_string();
                    }
                    if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
                        return "Jupiter (via Raydium CPMM)".to_string();
                    }
                    if log_text.contains("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C") {
                        return "Jupiter (via Raydium CPMM)".to_string();
                    }
                    if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ") {
                        return "Jupiter (via Raydium CAMM)".to_string();
                    }
                    if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
                        return "Jupiter (via Orca)".to_string();
                    }
                    return "Jupiter".to_string();
                }
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => {
                    return "Jupiter v3".to_string();
                }
                "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo" => {
                    return "Jupiter v2".to_string();
                }
                "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1" => {
                    return "Orca v1".to_string();
                }
                "82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ" => {
                    return "Aldrin".to_string();
                }
                "SSwpkEEWHvVFuuiB1EePEIrkHTjLZZT3tMfnr5U3qL7n" => {
                    return "Step Finance".to_string();
                }
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                    // Token program alone doesn't indicate a specific DEX
                    continue;
                }
                _ => continue,
            }
        }
        
        // Fallback: check log messages for known DEX signatures
        if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            return "Jupiter".to_string();
        }
        if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            return "Raydium".to_string();
        }
        if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
            return "Raydium CLMM".to_string();
        }
        if log_text.contains("CAMMCzo5YL8w4VFF8KVHrK22GGUQpMDdHdVPZo2vadqQ") {
            return "Raydium CPMM".to_string();
        }
        if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            return "Orca".to_string();
        }
        
        // Default fallback
        "Unknown DEX".to_string()
    }

    async fn extract_jupiter_swap_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Parse Jupiter-specific swap data from logs and balance changes
        let log_text = transaction.log_messages.join(" ");
        
        // Determine the actual router being used
        let router = self.determine_swap_router(transaction);
        
        if self.debug_enabled {
            log(LogTag::Transactions, "JUPITER", &format!("Analyzing Jupiter transaction {} with router: {}", 
                &transaction.signature[..8], router));
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
                if transfer.mint == "So11111111111111111111111111111111111111112" {
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
                log(LogTag::Transactions, "JUPITER", &format!("Analysis: input_token={}, output_token={}, wsol_transfer={}, sol_change={:.9}", 
                    input_token.is_some(), output_token.is_some(), wsol_transfer.is_some(), transaction.sol_balance_change));
                if let Some(wsol) = wsol_transfer {
                    log(LogTag::Transactions, "JUPITER", &format!("WSOL transfer amount: {:.9}", wsol.amount));
                }
                if let Some(input) = input_token {
                    log(LogTag::Transactions, "JUPITER", &format!("Input token: {} amount: {:.9}", &input.mint[..8], input.amount));
                }
                if let Some(output) = output_token {
                    log(LogTag::Transactions, "JUPITER", &format!("Output token: {} amount: {:.9}", &output.mint[..8], output.amount));
                }
            }
            
            // Enhanced Pattern Matching: Use SOL balance change as primary indicator
            // since token transfer directions can be misleading in complex DEX operations
            
            // Pattern 1: Token-to-SOL swap - SOL balance increased (received SOL)
            // Enhanced: Don't require WSOL transfer if we have significant SOL change and input token
            if transaction.sol_balance_change > 0.00001 && input_token.is_some() {
                // Find the token being sold (prefer input_token, but use output_token if no input)
                let token_transfer = input_token.or(output_token);
                if let Some(token) = token_transfer {
                    if token.mint != "So11111111111111111111111111111111111111112" {
                        if self.debug_enabled {
                            log(LogTag::Transactions, "JUPITER", &format!("✅ Token-to-SOL swap detected: {} -> SOL (SOL balance increased)", 
                                &token.mint[..8]));
                        }
                        return Ok(TransactionType::SwapTokenToSol {
                            token_mint: token.mint.clone(),
                            token_amount: token.amount.abs(),
                            sol_amount: transaction.sol_balance_change, // Use actual SOL received
                            router: router.clone(),
                        });
                    }
                } else if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("❌ Pattern 1: no token transfer found"));
                }
            } else if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("❌ Pattern 1: sol_change={:.9} > 0.00001? {}, input_token.is_some()? {}", 
                    transaction.sol_balance_change, transaction.sol_balance_change > 0.00001, input_token.is_some()));
            }
            
            // Pattern 2: SOL-to-Token swap - SOL balance decreased (spent SOL)
            // Enhanced: Don't require WSOL transfer if we have significant SOL change and output token
            if transaction.sol_balance_change < -0.00001 && output_token.is_some() {
                // Find the token being bought (prefer output_token, but use input_token if no output)
                let token_transfer = output_token.or(input_token);
                if let Some(token) = token_transfer {
                    if token.mint != "So11111111111111111111111111111111111111112" {
                        if self.debug_enabled {
                            log(LogTag::Transactions, "JUPITER", &format!("✅ SOL-to-token swap detected: SOL -> {} (SOL balance decreased)", 
                                &token.mint[..8]));
                        }
                        return Ok(TransactionType::SwapSolToToken {
                            token_mint: token.mint.clone(),
                            sol_amount: transaction.sol_balance_change.abs(), // SOL spent
                            token_amount: token.amount.abs(),
                            router: router.clone(),
                        });
                    }
                } else if self.debug_enabled {
                    log(LogTag::Transactions, "JUPITER", &format!("❌ Pattern 2: no token transfer found"));
                }
            } else if self.debug_enabled {
                log(LogTag::Transactions, "JUPITER", &format!("❌ Pattern 2: sol_change={:.9} < -0.00001? {}, output_token.is_some()? {}", 
                    transaction.sol_balance_change, transaction.sol_balance_change < -0.00001, output_token.is_some()));
            }
            
            // Fallback Pattern 1: Traditional logic - Token-to-SOL swap (token sold, WSOL received)
            if let (Some(input), Some(wsol)) = (input_token, wsol_transfer) {
                if wsol.amount > 0.0 { // WSOL received
                    if self.debug_enabled {
                        log(LogTag::Transactions, "JUPITER", &format!("Token-to-SOL swap detected (fallback): {} -> SOL", 
                            &input.mint[..8]));
                    }
                    return Ok(TransactionType::SwapTokenToSol {
                        token_mint: input.mint.clone(),
                        token_amount: input.amount.abs(),
                        sol_amount: wsol.amount, // WSOL received represents SOL obtained
                        router: router.clone(),
                    });
                }
            }
            
            // Fallback Pattern 2: Traditional logic - SOL-to-Token swap (WSOL sent, token received)
            if let (Some(output), Some(wsol)) = (output_token, wsol_transfer) {
                if wsol.amount < 0.0 { // WSOL sent
                    if self.debug_enabled {
                        log(LogTag::Transactions, "JUPITER", &format!("SOL-to-token swap detected (fallback): SOL -> {}", 
                            &output.mint[..8]));
                    }
                    return Ok(TransactionType::SwapSolToToken {
                        token_mint: output.mint.clone(),
                        sol_amount: wsol.amount.abs(), // WSOL sent represents SOL spent
                        token_amount: output.amount,
                        router: router.clone(),
                    });
                }
            }

            // Pattern 3: Token-to-Token swap (only if neither token involved is wSOL)
            if let (Some(input), Some(output)) = (input_token, output_token) {
                // Only classify as token-to-token if neither side is wSOL
                if input.mint != "So11111111111111111111111111111111111111112" && 
                   output.mint != "So11111111111111111111111111111111111111112" {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "JUPITER", &format!("Token-to-token swap detected: {} -> {}", 
                            &input.mint[..8], &output.mint[..8]));
                    }
                    return Ok(TransactionType::SwapTokenToToken {
                        from_mint: input.mint.clone(),
                        to_mint: output.mint.clone(),
                        from_amount: input.amount.abs(),
                        to_amount: output.amount,
                        router: router.clone(),
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
                    router: router.clone(),
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
                    router: router.clone(),
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
                // Note: wSOL transfers are important for SOL-token swaps and should not be skipped
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
                
                // Determine router using comprehensive detection
                let router = self.determine_swap_router(transaction);
                
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
            
            // Look for significant token transfers 
            // Note: wSOL transfers are important for SOL-token swaps and should not be skipped
            for transfer in &transaction.token_transfers {
                
                // Filter out very small amounts that are likely dust/spam
                if transfer.amount.abs() > 0.001 {
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
                
                // Determine router using comprehensive detection
                let router = self.determine_swap_router(transaction);
                
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
            // Note: wSOL transfers are important for SOL-token swaps and should not be skipped
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
                    
                    // Determine router using comprehensive detection
                    let router = self.determine_swap_router(transaction);
                    
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

    /// Extract ATA (Associated Token Account) operations
    async fn extract_ata_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for ATA creation patterns
        if log_text.contains("Create associated token account") || 
           log_text.contains("AssociatedTokenAccountProgram") ||
           log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") {
            
            // Look for rent amounts and account addresses in logs
            let rent_cost = 0.00203928; // Standard ATA creation cost
            
            // Try to extract mint and owner from instruction data
            let mut mint = "Unknown".to_string();
            let mut owner = self.wallet_pubkey.to_string();
            let mut ata_address = "Unknown".to_string();
            
            // Look for token account creation in instructions
            for instruction in &transaction.instructions {
                if instruction.program_id == "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" {
                    if !instruction.accounts.is_empty() {
                        ata_address = instruction.accounts[0].clone();
                        if instruction.accounts.len() > 1 {
                            mint = instruction.accounts[1].clone();
                        }
                        if instruction.accounts.len() > 2 {
                            owner = instruction.accounts[2].clone();
                        }
                    }
                }
            }
            
            return Ok(TransactionType::AtaCreate {
                mint,
                owner,
                ata_address,
                cost: rent_cost,
            });
        }
        
        // Check for ATA closure patterns
        if log_text.contains("Close account") || 
           log_text.contains("CloseAccount") {
            
            let rent_reclaimed = 0.00203928; // Standard ATA rent reclaimed
            
            let mut mint = "Unknown".to_string();
            let mut owner = self.wallet_pubkey.to_string();
            let mut ata_address = "Unknown".to_string();
            
            // Try to extract account info from token transfers or instructions
            if !transaction.token_transfers.is_empty() {
                let transfer = &transaction.token_transfers[0];
                mint = transfer.mint.clone();
                ata_address = transfer.from.clone();
            }
            
            return Ok(TransactionType::AtaClose {
                mint,
                owner,
                ata_address,
                rent_reclaimed,
            });
        }
        
        Err("No ATA operation detected".to_string())
    }

    /// Extract staking operations
    async fn extract_staking_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for staking delegation
        if log_text.contains("Delegate") || log_text.contains("StakeProgram") ||
           log_text.contains("Stake11111111111111111111111111111111111112") {
            
            // Look for delegation patterns
            if log_text.contains("DelegateStake") || log_text.contains("delegate") {
                let stake_account = if !transaction.instructions.is_empty() {
                    transaction.instructions[0].accounts.get(0).cloned().unwrap_or_default()
                } else {
                    "Unknown".to_string()
                };
                
                let validator = if !transaction.instructions.is_empty() {
                    transaction.instructions[0].accounts.get(1).cloned().unwrap_or_default()
                } else {
                    "Unknown".to_string()
                };
                
                let amount = transaction.sol_balance_change.abs();
                
                return Ok(TransactionType::StakingDelegate {
                    stake_account,
                    validator,
                    amount,
                });
            }
            
            // Check for withdrawal patterns
            if log_text.contains("Withdraw") || log_text.contains("withdraw") {
                let stake_account = if !transaction.instructions.is_empty() {
                    transaction.instructions[0].accounts.get(0).cloned().unwrap_or_default()
                } else {
                    "Unknown".to_string()
                };
                
                let amount = transaction.sol_balance_change.abs();
                
                return Ok(TransactionType::StakingWithdraw {
                    stake_account,
                    amount,
                });
            }
        }
        
        Err("No staking operation detected".to_string())
    }

    /// Extract program deployment/upgrade operations
    async fn extract_program_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for program deployment
        if log_text.contains("Deploy") || log_text.contains("deploy") ||
           log_text.contains("BPFLoaderUpgradeab1e11111111111111111111111") {
            
            let program_id = if !transaction.instructions.is_empty() {
                transaction.instructions[0].program_id.clone()
            } else {
                "Unknown".to_string()
            };
            
            let deployer = self.wallet_pubkey.to_string();
            
            if log_text.contains("DeployWithMaxDataLen") || log_text.contains("Upgrade") {
                return Ok(TransactionType::ProgramUpgrade {
                    program_id,
                    authority: deployer,
                });
            } else {
                return Ok(TransactionType::ProgramDeploy {
                    program_id,
                    deployer,
                });
            }
        }
        
        Err("No program operation detected".to_string())
    }

    /// Extract compute budget operations
    async fn extract_compute_budget_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for compute budget instructions
        if log_text.contains("ComputeBudgetProgram") ||
           log_text.contains("ComputeBudget111111111111111111111111111111") {
            
            // Extract compute units and price from instructions or logs
            let mut compute_units = 0u32;
            let mut compute_unit_price = 0u64;
            
            // Look for compute budget patterns in logs
            if let Some(start) = log_text.find("compute units") {
                if let Some(number_start) = log_text[..start].rfind(char::is_numeric) {
                    if let Some(number_end) = log_text[number_start..start].find(char::is_whitespace) {
                        if let Ok(units) = log_text[number_start..number_start + number_end].parse::<u32>() {
                            compute_units = units;
                        }
                    }
                }
            }
            
            // Look for priority fee information
            if let Some(start) = log_text.find("priority fee") {
                // Extract priority fee amount
                compute_unit_price = 1000; // Default value
            }
            
            return Ok(TransactionType::ComputeBudget {
                compute_units,
                compute_unit_price,
            });
        }
        
        Err("No compute budget operation detected".to_string())
    }

    /// Extract spam bulk operations
    async fn extract_spam_bulk_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Detect spam based on patterns
        let log_text = transaction.log_messages.join(" ");
        
        // Check for bulk airdrop patterns
        if transaction.token_transfers.len() > 10 && transaction.sol_balance_change == 0.0 {
            return Ok(TransactionType::SpamBulk {
                transaction_count: transaction.token_transfers.len(),
                suspected_spam_type: "Bulk Airdrop".to_string(),
            });
        }
        
        // Check for spam token creation
        if log_text.contains("spam") || log_text.contains("Spam") {
            return Ok(TransactionType::Spam);
        }
        
        // Check for multiple failed instructions (common in spam)
        if transaction.instructions.len() > 20 && !transaction.success {
            return Ok(TransactionType::SpamBulk {
                transaction_count: transaction.instructions.len(),
                suspected_spam_type: "Failed Bulk Instructions".to_string(),
            });
        }
        
        // Enhanced: Detect system program spam - many identical system program instructions
        if transaction.instructions.len() >= 10 {
            let system_program_id = "11111111111111111111111111111111";
            let system_instructions: Vec<_> = transaction.instructions
                .iter()
                .filter(|inst| inst.program_id == system_program_id)
                .collect();
            
            // If 80% or more instructions are system program calls
            let system_ratio = system_instructions.len() as f64 / transaction.instructions.len() as f64;
            if system_ratio >= 0.8 && transaction.instructions.len() >= 15 {
                // Check if they have repeated data patterns (spam characteristic)
                if system_instructions.len() >= 3 {
                    let first_data = &system_instructions[0].data;
                    let repeated_data_count = system_instructions
                        .iter()
                        .filter(|inst| &inst.data == first_data)
                        .count();
                    
                    // If most instructions have the same data, it's likely spam
                    if repeated_data_count >= (system_instructions.len() * 2 / 3) {
                        return Ok(TransactionType::SpamBulk {
                            transaction_count: system_instructions.len(),
                            suspected_spam_type: "System Program Spam".to_string(),
                        });
                    }
                }
            }
        }
        
        // Check for excessive system program calls (another spam pattern)
        let system_program_count = transaction.instructions
            .iter()
            .filter(|inst| inst.program_id == "11111111111111111111111111111111")
            .count();
        
        if system_program_count >= 18 && transaction.token_transfers.is_empty() {
            // Very small SOL balance change with many system calls = likely spam
            if transaction.sol_balance_change.abs() < 0.001 {
                return Ok(TransactionType::SpamBulk {
                    transaction_count: system_program_count,
                    suspected_spam_type: "Excessive System Calls".to_string(),
                });
            }
        }
        
        Err("No spam bulk operation detected".to_string())
    }

    /// Extract transaction type based on instruction analysis
    async fn extract_instruction_based_type(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        if transaction.instructions.is_empty() {
            return Err("No instructions to analyze".to_string());
        }
        
        // Enhanced: Check for bulk system program operations first
        let system_program_id = "11111111111111111111111111111111";
        let system_instruction_count = transaction.instructions
            .iter()
            .filter(|inst| inst.program_id == system_program_id)
            .count();
        
        // If most instructions are system program calls with minimal balance change
        if system_instruction_count >= 10 {
            let total_instructions = transaction.instructions.len();
            let system_ratio = system_instruction_count as f64 / total_instructions as f64;
            
            // If 70% or more are system instructions with tiny balance change
            if system_ratio >= 0.7 && transaction.sol_balance_change.abs() < 0.01 {
                // Check for repeated data patterns
                let system_instructions: Vec<_> = transaction.instructions
                    .iter()
                    .filter(|inst| inst.program_id == system_program_id)
                    .collect();
                
                if system_instructions.len() >= 3 {
                    let first_data = &system_instructions[0].data;
                    let same_data_count = system_instructions
                        .iter()
                        .filter(|inst| &inst.data == first_data)
                        .count();
                    
                    // If most have the same data, classify as bulk system operation
                    if same_data_count >= (system_instructions.len() * 2 / 3) {
                        return Ok(TransactionType::SpamBulk {
                            transaction_count: system_instruction_count,
                            suspected_spam_type: "Bulk System Operations".to_string(),
                        });
                    }
                }
                
                // Even without repeated data, many system calls with tiny change = bulk operation
                return Ok(TransactionType::SpamBulk {
                    transaction_count: system_instruction_count,
                    suspected_spam_type: "Multiple System Operations".to_string(),
                });
            }
        }
        
        // Analyze the first instruction's program ID to classify transaction
        let program_id = &transaction.instructions[0].program_id;
        
        match program_id.as_str() {
            // System Program - usually transfers or account creation
            "11111111111111111111111111111111" => {
                if transaction.sol_balance_change.abs() > 0.001 {
                    return Ok(TransactionType::SolTransfer {
                        amount: transaction.sol_balance_change.abs(),
                        from: if transaction.sol_balance_change < 0.0 { 
                            self.wallet_pubkey.to_string() 
                        } else { 
                            "Unknown".to_string() 
                        },
                        to: if transaction.sol_balance_change > 0.0 { 
                            self.wallet_pubkey.to_string() 
                        } else { 
                            "Unknown".to_string() 
                        },
                    });
                }
                
                // Enhanced: For small system program transactions, classify as bulk operation
                if transaction.instructions.len() >= 5 {
                    return Ok(TransactionType::SpamBulk {
                        transaction_count: transaction.instructions.len(),
                        suspected_spam_type: "System Program Bulk".to_string(),
                    });
                }
            }
            
            // Token Program - token transfers
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                if !transaction.token_transfers.is_empty() {
                    let transfer = &transaction.token_transfers[0];
                    return Ok(TransactionType::TokenTransfer {
                        mint: transfer.mint.clone(),
                        amount: transfer.amount.abs(),
                        from: transfer.from.clone(),
                        to: transfer.to.clone(),
                    });
                }
            }
            
            // Compute Budget Program
            "ComputeBudget111111111111111111111111111111" => {
                return Ok(TransactionType::ComputeBudget {
                    compute_units: 200000, // Default
                    compute_unit_price: 1000, // Default
                });
            }
            
            // Associated Token Account Program
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => {
                return Ok(TransactionType::AtaCreate {
                    mint: "Unknown".to_string(),
                    owner: self.wallet_pubkey.to_string(),
                    ata_address: "Unknown".to_string(),
                    cost: 0.00203928,
                });
            }
            
            // Stake Program
            "Stake11111111111111111111111111111111111112" => {
                return Ok(TransactionType::StakingDelegate {
                    stake_account: "Unknown".to_string(),
                    validator: "Unknown".to_string(),
                    amount: transaction.sol_balance_change.abs(),
                });
            }
            
            // Bubblegum Program (Compressed NFTs)
            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY" => {
                // Check for NFT minting operations
                let mut collection_id = "Unknown".to_string();
                let mut leaf_asset_id = "Unknown".to_string();
                
                // Extract information from log messages
                for log in &transaction.log_messages {
                    if log.contains("MintToCollectionV1") || log.contains("MintV1") {
                        // This is an NFT minting operation
                    }
                    if log.contains("Leaf asset ID:") {
                        // Extract the leaf asset ID
                        if let Some(start) = log.find("Leaf asset ID:") {
                            let id_part = &log[start + 15..];
                            if let Some(end) = id_part.find(' ') {
                                leaf_asset_id = id_part[..end].trim().to_string();
                            } else {
                                leaf_asset_id = id_part.trim().to_string();
                            }
                        }
                    }
                }
                
                return Ok(TransactionType::NftMint {
                    collection_id,
                    leaf_asset_id,
                    nft_type: "Compressed NFT".to_string(),
                });
            }
            
            _ => {
                // For unknown programs, try to classify based on behavior
                if transaction.sol_balance_change.abs() > 0.001 && transaction.token_transfers.is_empty() {
                    return Ok(TransactionType::SolTransfer {
                        amount: transaction.sol_balance_change.abs(),
                        from: "Unknown".to_string(),
                        to: "Unknown".to_string(),
                    });
                }
                
                if !transaction.token_transfers.is_empty() && transaction.sol_balance_change.abs() < 0.001 {
                    let transfer = &transaction.token_transfers[0];
                    return Ok(TransactionType::TokenTransfer {
                        mint: transfer.mint.clone(),
                        amount: transfer.amount.abs(),
                        from: transfer.from.clone(),
                        to: transfer.to.clone(),
                    });
                }
            }
        }
        
        Err("Could not classify transaction from instructions".to_string())
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
                // Only consider token accounts owned by our wallet
                let wallet_address = self.wallet_pubkey.to_string();
                let mut token_balance_map: std::collections::HashMap<String, (f64, f64)> = std::collections::HashMap::new();
                
                // Collect pre token balances (only for our wallet)
                if let Some(pre_token_balances) = meta.get("preTokenBalances").and_then(|v| v.as_array()) {
                    for token_balance in pre_token_balances {
                        if let (Some(mint), Some(owner)) = (
                            token_balance.get("mint").and_then(|v| v.as_str()),
                            token_balance.get("owner").and_then(|v| v.as_str())
                        ) {
                            // Only process token accounts owned by our wallet
                            if owner == wallet_address {
                                let amount = token_balance
                                    .get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                token_balance_map.entry(mint.to_string()).or_insert((0.0, 0.0)).0 = amount;
                            }
                        }
                    }
                }
                
                // Collect post token balances (only for our wallet)
                if let Some(post_token_balances) = meta.get("postTokenBalances").and_then(|v| v.as_array()) {
                    for token_balance in post_token_balances {
                        if let (Some(mint), Some(owner)) = (
                            token_balance.get("mint").and_then(|v| v.as_str()),
                            token_balance.get("owner").and_then(|v| v.as_str())
                        ) {
                            // Only process token accounts owned by our wallet
                            if owner == wallet_address {
                                let amount = token_balance
                                    .get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                token_balance_map.entry(mint.to_string()).or_insert((0.0, 0.0)).1 = amount;
                            }
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
                        ata_creations_count: 0,
                        ata_closures_count: 0,
                        net_ata_rent_flow: 0.0,
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
            TransactionType::NftMint { leaf_asset_id, nft_type, .. } => {
                format!("NFT Mint: {} ({})", &leaf_asset_id[..8], nft_type)
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

        // Step 1: Ensure we have transaction data (cache-first approach)
        // For recalculations, we should already have cached data and avoid RPC calls
        if transaction.raw_transaction_data.is_none() {
            self.fetch_transaction_data(transaction).await?;
        } else if self.debug_enabled {
            log(LogTag::Transactions, "CACHE_USE", &format!("Using existing cached data for {}", &transaction.signature[..8]));
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
    pub fn extract_token_mint_from_transaction(&self, transaction: &Transaction) -> Option<String> {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToSol { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToToken { to_mint, .. } => Some(to_mint.clone()),
            _ => None,
        }
    }

    /// Extract router from transaction
    fn extract_router_from_transaction(&self, transaction: &Transaction) -> String {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, .. } => router.clone(),
            TransactionType::SwapTokenToSol { router, .. } => router.clone(),
            TransactionType::SwapTokenToToken { router, .. } => router.clone(),
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

    /// Bulk recalculate all cached transactions (no RPC calls)
    pub async fn recalculate_all_cached_transactions(&mut self, max_count: Option<usize>) -> Result<Vec<Transaction>, String> {
        let cache_dir = get_transactions_cache_dir();
        
        if !cache_dir.exists() {
            log(LogTag::Transactions, "WARN", "Transaction cache directory does not exist");
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?;

        let mut cache_files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                cache_files.push(path);
            }
        }

        // Sort by modification time (newest first) and limit if requested
        cache_files.sort_by(|a, b| {
            let a_meta = fs::metadata(a).ok();
            let b_meta = fs::metadata(b).ok();
            match (a_meta, b_meta) {
                (Some(a_meta), Some(b_meta)) => {
                    b_meta.modified().unwrap_or(std::time::UNIX_EPOCH)
                        .cmp(&a_meta.modified().unwrap_or(std::time::UNIX_EPOCH))
                }
                _ => std::cmp::Ordering::Equal,
            }
        });

        if let Some(max) = max_count {
            cache_files.truncate(max);
        }

        let mut recalculated_transactions = Vec::new();
        let total_files = cache_files.len();

        log(LogTag::Transactions, "INFO", &format!(
            "Recalculating {} cached transactions (no RPC calls)", total_files
        ));

        for (index, cache_file) in cache_files.iter().enumerate() {
            if self.debug_enabled {
                log(LogTag::Transactions, "PROGRESS", &format!(
                    "Processing transaction {}/{}: {}...", 
                    index + 1, total_files,
                    cache_file.file_stem().unwrap_or_default().to_string_lossy().chars().take(8).collect::<String>()
                ));
            }

            match self.recalculate_cached_transaction(cache_file).await {
                Ok(transaction) => {
                    recalculated_transactions.push(transaction);
                }
                Err(e) => {
                    log(LogTag::Transactions, "WARN", &format!(
                        "Failed to recalculate {}: {}", 
                        cache_file.file_stem().unwrap_or_default().to_string_lossy(),
                        e
                    ));
                }
            }
        }

        log(LogTag::Transactions, "INFO", &format!(
            "Recalculated {} transactions from cache", recalculated_transactions.len()
        ));

        Ok(recalculated_transactions)
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

        // Sort by slot (newest first for proper chronological order)
        // Handle Option<u64> slots properly - None slots go to end
        swap_transactions.sort_by(|a, b| {
            match (b.slot, a.slot) {
                (Some(b_slot), Some(a_slot)) => b_slot.cmp(&a_slot),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(swap_transactions)
    }

    /// Load transaction from cache file and recalculate with new analysis (no RPC calls)
    pub async fn recalculate_cached_transaction(&mut self, cache_file_path: &Path) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "RECALC", &format!("Recalculating cached transaction: {}", 
                cache_file_path.file_stem().unwrap_or_default().to_string_lossy()));
        }

        // Load existing cached transaction
        let mut transaction = self.load_transaction_from_cache(cache_file_path).await?;
        
        // Update last_updated timestamp
        transaction.last_updated = Utc::now();
        
        // Reset analysis fields that will be recalculated
        transaction.sol_balance_change = 0.0;
        transaction.token_transfers.clear();
        transaction.transaction_type = TransactionType::Unknown;
        transaction.swap_analysis = None;
        transaction.fee_breakdown = None;
        transaction.token_info = None;
        transaction.calculated_token_price_sol = None;
        transaction.price_source = None;
        transaction.token_symbol = None;
        transaction.token_decimals = None;
        
        // Recalculate using cached raw data (no RPC call)
        // raw_transaction_data should already be present from cache
        if transaction.raw_transaction_data.is_none() {
            return Err("Cached transaction missing raw data".to_string());
        }

        // Run comprehensive analysis on cached data
        self.analyze_transaction_comprehensive(&mut transaction).await?;

        // Update cache with new analysis
        self.cache_transaction(&transaction).await?;

        Ok(transaction)
    }

    /// Load transaction from cache file
    async fn load_transaction_from_cache(&self, path: &Path) -> Result<Transaction, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        
        let transaction: Transaction = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse transaction: {}", e))?;
        
        Ok(transaction)
    }

    /// Convert transaction to SwapPnLInfo using precise ATA rent detection
    fn convert_to_swap_pnl_info(&self, transaction: &Transaction) -> Option<SwapPnLInfo> {
        // Debug logging for our specific missing transaction
        if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
            log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                "Processing missing transaction: {} is_swap={} type={:?}",
                &transaction.signature[..8],
                self.is_swap_transaction(transaction),
                transaction.transaction_type
            ));
        }
        
        if !self.is_swap_transaction(transaction) {
            if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                log(LogTag::Transactions, "DEBUG_MISSING", "Missing transaction failed is_swap_transaction check - returning None");
            }
            return None;
        }

        let (swap_type, sol_amount_raw, token_amount, token_mint) = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, .. } => {
                ("Buy".to_string(), *sol_amount, *token_amount, token_mint.clone())
            }
            TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, .. } => {
                ("Sell".to_string(), *sol_amount, *token_amount, token_mint.clone())
            }
            TransactionType::SwapTokenToToken { from_mint, to_mint, from_amount, to_amount, .. } => {
                // Debug for our specific transaction
                if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                    log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                        "SwapTokenToToken processing: from_amount={}, to_amount={}, sol_balance_change={}, token_transfers={}",
                        from_amount, to_amount, transaction.sol_balance_change, transaction.token_transfers.len()
                    ));
                    
                    if !transaction.token_transfers.is_empty() {
                        let largest_transfer = transaction.token_transfers.iter()
                            .max_by(|a, b| a.amount.abs().partial_cmp(&b.amount.abs()).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap();
                        log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                            "Largest token transfer: amount={}, mint={}",
                            largest_transfer.amount, largest_transfer.mint
                        ));
                    }
                }
                
                // For SwapTokenToToken, we need to determine if this involves SOL by checking:
                // 1. SOL balance change direction (gained SOL = sell, spent SOL = buy)
                // 2. Token transfer amounts to get the actual token amounts traded
                
                // Get the token amount from token transfers for more accurate data
                let (token_transfer_amount, transfer_mint) = if !transaction.token_transfers.is_empty() {
                    // Find the largest absolute token transfer (this is usually the main trade)
                    let largest_transfer = transaction.token_transfers.iter()
                        .max_by(|a, b| a.amount.abs().partial_cmp(&b.amount.abs()).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap();
                    (largest_transfer.amount, largest_transfer.mint.clone())
                } else {
                    // Fallback to from_amount/to_amount if no token transfers
                    if *from_amount != 0.0 {
                        (*from_amount, from_mint.clone())
                    } else {
                        (*to_amount, to_mint.clone())
                    }
                };

                // If we gained SOL and have token outflow (negative), it's a sell
                if transaction.sol_balance_change > 0.0 && token_transfer_amount < 0.0 {
                    let result = ("Sell".to_string(), transaction.sol_balance_change, token_transfer_amount.abs(), transfer_mint);
                    if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                        log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                            "Detected as SELL: sol_gained={}, token_sold={}, mint={}",
                            result.1, result.2, result.3
                        ));
                    }
                    result
                }
                // If we spent SOL and have token inflow (positive), it's a buy
                else if transaction.sol_balance_change < 0.0 && token_transfer_amount > 0.0 {
                    let result = ("Buy".to_string(), transaction.sol_balance_change.abs(), token_transfer_amount, transfer_mint);
                    if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                        log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                            "Detected as BUY: sol_spent={}, token_bought={}, mint={}",
                            result.1, result.2, result.3
                        ));
                    }
                    result
                }
                // Fallback: use the original logic if token transfers don't help
                else if transaction.sol_balance_change > 0.0 && *from_amount != 0.0 {
                    let result = ("Sell".to_string(), transaction.sol_balance_change, *from_amount, from_mint.clone());
                    if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                        log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                            "Fallback SELL (from_amount): sol_gained={}, token_sold={}, mint={}",
                            result.1, result.2, result.3
                        ));
                    }
                    result
                }
                else if transaction.sol_balance_change < 0.0 && *to_amount != 0.0 {
                    let result = ("Buy".to_string(), transaction.sol_balance_change.abs(), *to_amount, to_mint.clone());
                    if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                        log(LogTag::Transactions, "DEBUG_MISSING", &format!(
                            "Fallback BUY (to_amount): sol_spent={}, token_bought={}, mint={}",
                            result.1, result.2, result.3
                        ));
                    }
                    result
                }
                else {
                    if transaction.signature == "5ffzbggfC1DaE5fz6FTZvpcpALRANRJcQbK147ffqHTf5X18ewp5QGYYr9YngvWwfnrc8GhrHM5buYhzrFGHf4ZK" {
                        log(LogTag::Transactions, "DEBUG_MISSING", "SwapTokenToToken: All conditions failed - returning None");
                    }
                    return None;
                }
            }
            _ => return None,
        };

        // Get precise ATA rent information from fee breakdown
        let (net_ata_rent_flow, ata_rents_display) = if let Some(fee_breakdown) = &transaction.fee_breakdown {
            (fee_breakdown.net_ata_rent_flow, fee_breakdown.ata_creation_cost + fee_breakdown.rent_costs)
        } else {
            (0.0, 0.0)
        };

        if self.debug_enabled {
            log(LogTag::Transactions, "PNL_CALC", &format!(
                "Transaction {}: sol_balance_change={:.9}, net_ata_rent_flow={:.9}, type={}",
                &transaction.signature[..8], transaction.sol_balance_change, net_ata_rent_flow, swap_type
            ));
        }

        // CRITICAL FIX: Skip failed transactions or handle them appropriately
        if !transaction.success {
            let failed_costs = transaction.sol_balance_change.abs();
            
            let token_symbol = transaction.token_symbol.clone()
                .unwrap_or_else(|| format!("TOKEN_{}", &token_mint[..8]));
            
            let router = self.extract_router_from_transaction(transaction);
            let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
                DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
            } else {
                transaction.timestamp
            };

            return Some(SwapPnLInfo {
                token_mint,
                token_symbol,
                swap_type: format!("Failed {}", swap_type),
                sol_amount: failed_costs,
                token_amount: 0.0,
                calculated_price_sol: 0.0,
                timestamp: blockchain_timestamp,
                signature: transaction.signature.clone(),
                router,
                fee_sol: transaction.fee_sol,
                ata_rents: ata_rents_display,
                slot: transaction.slot,
            });
        }

        // ADVANCED ALGORITHM: Calculate pure trade amount by separating ATA rent flows
        // 
        // Key insight: ATA rent is recoverable and NOT part of the actual trade
        // - When you create ATAs: you pay rent (negative flow)  
        // - When you close ATAs: you get rent back (positive flow)
        // - Pure trade amount = total SOL flow - ATA rent flows
        //
        let pure_trade_amount = match swap_type.as_str() {
            "Buy" => {
                // For BUY transactions: 
                // sol_balance_change is NEGATIVE (SOL spent)
                // net_ata_rent_flow can be positive (net rent recovered) or negative (net rent paid)
                // pure_trade_amount = |sol_balance_change| - |net_ata_rent_flow|
                let total_sol_spent = transaction.sol_balance_change.abs();
                let pure_trade = total_sol_spent - net_ata_rent_flow.abs();
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "BUY_CALC", &format!(
                        "Buy calculation: total_spent={:.9}, ata_flow={:.9}, pure_trade={:.9}",
                        total_sol_spent, net_ata_rent_flow, pure_trade
                    ));
                }
                
                pure_trade.max(0.0)
            }
            "Sell" => {
                // For SELL transactions:
                // sol_balance_change is POSITIVE (SOL received)  
                // net_ata_rent_flow can be positive (net rent recovered) or negative (net rent paid)
                // pure_trade_amount = sol_balance_change - net_ata_rent_flow
                let total_sol_received = transaction.sol_balance_change;
                let pure_trade = total_sol_received - net_ata_rent_flow;
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "SELL_CALC", &format!(
                        "Sell calculation: total_received={:.9}, ata_flow={:.9}, pure_trade={:.9}",
                        total_sol_received, net_ata_rent_flow, pure_trade
                    ));
                }
                
                pure_trade.max(0.0)
            }
            _ => {
                // Fallback for unknown swap types
                (transaction.sol_balance_change.abs() - net_ata_rent_flow.abs()).max(0.0)
            }
        };

        // Cross-validation: Check if our calculation makes sense
        let validation_threshold = 0.0001; // 0.1 mSOL tolerance
        if pure_trade_amount < validation_threshold {
            if self.debug_enabled {
                log(LogTag::Transactions, "VALIDATION_WARN", &format!(
                    "Pure trade amount very small ({:.9} SOL) - might be dust or calculation error",
                    pure_trade_amount
                ));
            }
            
            // For very small amounts, fall back to using balance change directly
            // This handles edge cases where ATA calculations might be imprecise
            let fallback_amount = transaction.sol_balance_change.abs();
            
            if fallback_amount > validation_threshold {
                if self.debug_enabled {
                    log(LogTag::Transactions, "FALLBACK", &format!(
                        "Using fallback calculation: {:.9} SOL", fallback_amount
                    ));
                }
            }
        }

        // Final amount calculation with multiple validation checks
        let final_sol_amount = if pure_trade_amount >= validation_threshold {
            pure_trade_amount
        } else {
            // Last resort: try to find meaningful SOL transfer in token_transfers
            let sol_transfer_amount = transaction.token_transfers
                .iter()
                .find(|transfer| transfer.mint == "So11111111111111111111111111111111111111112")
                .map(|transfer| transfer.amount.abs())
                .unwrap_or(0.0);
                
            if sol_transfer_amount >= validation_threshold {
                if self.debug_enabled {
                    log(LogTag::Transactions, "SOL_TRANSFER", &format!(
                        "Using SOL transfer amount: {:.9} SOL", sol_transfer_amount
                    ));
                }
                sol_transfer_amount
            } else {
                // Ultimate fallback
                transaction.sol_balance_change.abs()
            }
        };

        // Calculate price using the pure trade amount
        let calculated_price_sol = if token_amount.abs() > 0.0 && final_sol_amount > 0.0 { 
            final_sol_amount / token_amount.abs() 
        } else { 
            0.0 
        };
        
        let token_symbol = transaction.token_symbol.clone()
            .unwrap_or_else(|| format!("TOKEN_{}", &token_mint[..8]));
        
        let router = self.extract_router_from_transaction(transaction);
        let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
        } else {
            transaction.timestamp
        };

        if self.debug_enabled {
            log(LogTag::Transactions, "FINAL_RESULT", &format!(
                "Final calculation for {}: {:.9} SOL, price={:.12} SOL/token",
                &transaction.signature[..8], final_sol_amount, calculated_price_sol
            ));
        }

        Some(SwapPnLInfo {
            token_mint,
            token_symbol,
            swap_type,
            sol_amount: final_sol_amount,
            token_amount,
            calculated_price_sol,
            timestamp: blockchain_timestamp,
            signature: transaction.signature.clone(),
            router,
            fee_sol: transaction.fee_sol,
            ata_rents: ata_rents_display,
            slot: transaction.slot,
        })
    }

    /// Display comprehensive swap analysis table with proper sign conventions
    pub fn display_swap_analysis_table(&self, swaps: &[SwapPnLInfo]) {
        if swaps.is_empty() {
            log(LogTag::Transactions, "INFO", "No swap transactions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE SWAP ANALYSIS ===");

        // Convert swaps to display rows
        let mut display_rows: Vec<SwapDisplayRow> = Vec::new();
        let mut total_fees = 0.0;
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_sol_spent = 0.0;
        let mut total_sol_received = 0.0;

        for swap in swaps {
            let slot_str = match swap.slot {
                Some(slot) => format!("{}", slot),
                None => "Unknown".to_string(),
            };

            let sig_short = &swap.signature[..8.min(swap.signature.len())];

            // Apply intuitive sign conventions for final display:
            // SOL: negative for outflow (spent), positive for inflow (received)
            // Token: negative for outflow (sold), positive for inflow (bought)
            let (display_sol_amount, display_token_amount) = if swap.swap_type == "Buy" {
                // Buy: SOL spent (negative), tokens received (positive)
                (-swap.sol_amount, swap.token_amount.abs())
            } else {
                // Sell: SOL received (positive), tokens sold (negative)  
                (swap.sol_amount, -swap.token_amount.abs())
            };

            // Color coding for better readability
            let type_display = if swap.swap_type == "Buy" {
                "🟢Buy".to_string()  // Green for buy
            } else {
                "🔴Sell".to_string() // Red for sell
            };

            // Format SOL amount with colored sign
            let sol_formatted = if display_sol_amount >= 0.0 {
                format!("+{:.6}", display_sol_amount)
            } else {
                format!("{:.6}", display_sol_amount)
            };

            // Format token amount with colored sign
            let token_formatted = if display_token_amount >= 0.0 {
                format!("+{:.2}", display_token_amount)
            } else {
                format!("{:.2}", display_token_amount)
            };

            display_rows.push(SwapDisplayRow {
                signature: sig_short.to_string(),
                slot: slot_str,
                swap_type: type_display,
                token: swap.token_symbol[..15.min(swap.token_symbol.len())].to_string(),
                sol_amount: sol_formatted,
                token_amount: token_formatted,
                price: format!("{:.9}", swap.calculated_price_sol),
                ata_rents: format!("{:.6}", swap.ata_rents),
                router: swap.router[..12.min(swap.router.len())].to_string(),
                fee: format!("{:.6}", swap.fee_sol),
            });

            total_fees += swap.fee_sol;
            if swap.swap_type == "Buy" {
                buy_count += 1;
                total_sol_spent += swap.sol_amount;
            } else {
                sell_count += 1;
                total_sol_received += swap.sol_amount;
            }
        }

        // Create and display the table
        let table_string = Table::new(display_rows)
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        // Print the entire table directly to console
        println!("{}", table_string);

        // Print summary
        println!("📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
            buy_count, total_sol_spent, sell_count, total_sol_received, total_fees, 
            total_sol_received - total_sol_spent - total_fees
        );
        println!("=== END ANALYSIS ===");
        
        log(LogTag::Transactions, "TABLE", &format!(
            "📊 SUMMARY: {} Buys ({:.3} SOL), {} Sells ({:.3} SOL), Total Fees: {:.6} SOL, Net SOL: {:.3}",
            buy_count, total_sol_spent, sell_count, total_sol_received, total_fees, 
            total_sol_received - total_sol_spent - total_fees
        ));
        log(LogTag::Transactions, "TABLE", "=== END ANALYSIS ===");
    }

    /// Analyze and display position lifecycle with PnL calculations
    pub async fn analyze_positions(&mut self, max_count: Option<usize>) -> Result<(), String> {
        let swaps = self.get_all_swap_transactions().await?;
        let positions = self.calculate_position_analysis(&swaps);
        self.display_position_analysis_table(&positions);
        Ok(())
    }

    /// Calculate position analysis from swap transactions
    fn calculate_position_analysis(&self, swaps: &[SwapPnLInfo]) -> Vec<PositionAnalysis> {
        use std::collections::HashMap;
        
        let mut positions: HashMap<String, PositionState> = HashMap::new();
        let mut completed_positions = Vec::new();

        // Sort swaps by slot for proper chronological processing
        let mut sorted_swaps = swaps.to_vec();
        sorted_swaps.sort_by(|a, b| {
            match (a.slot, b.slot) {
                (Some(a_slot), Some(b_slot)) => a_slot.cmp(&b_slot),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.timestamp.cmp(&b.timestamp),
            }
        });

        log(LogTag::Transactions, "POSITION_CALC", &format!(
            "Processing {} swaps for position analysis", sorted_swaps.len()
        ));

        for swap in &sorted_swaps {
            // Skip failed transactions
            if swap.swap_type.starts_with("Failed") {
                continue;
            }

            let position_state = positions.entry(swap.token_mint.clone()).or_insert_with(|| {
                PositionState {
                    token_mint: swap.token_mint.clone(),
                    token_symbol: swap.token_symbol.clone(),
                    total_tokens: 0.0,
                    total_sol_invested: 0.0,
                    total_sol_received: 0.0,
                    total_fees: 0.0,
                    total_ata_rents: 0.0,
                    buy_count: 0,
                    sell_count: 0,
                    first_buy_slot: None,
                    last_activity_slot: None,
                    first_buy_timestamp: None,
                    last_activity_timestamp: None,
                    average_buy_price: 0.0,
                    transactions: Vec::new(),
                }
            });

            // Track transaction
            position_state.transactions.push(PositionTransaction {
                signature: swap.signature.clone(),
                swap_type: swap.swap_type.clone(),
                sol_amount: swap.sol_amount,
                token_amount: swap.token_amount,
                price: swap.calculated_price_sol,
                timestamp: swap.timestamp,
                slot: swap.slot,
                router: swap.router.clone(),
                fee_sol: swap.fee_sol,
                ata_rents: swap.ata_rents,
            });

            // Update position state
            match swap.swap_type.as_str() {
                "Buy" => {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "DEBUG_BUY", &format!(
                            "Processing BUY for {}: +{:.2} tokens, current total: {:.2} -> {:.2}",
                            swap.token_symbol, swap.token_amount, position_state.total_tokens, 
                            position_state.total_tokens + swap.token_amount
                        ));
                    }
                    
                    position_state.total_tokens += swap.token_amount;
                    position_state.total_sol_invested += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.buy_count += 1;

                    // Track first buy
                    if position_state.first_buy_slot.is_none() || 
                       (swap.slot.is_some() && position_state.first_buy_slot.unwrap_or(u64::MAX) > swap.slot.unwrap()) {
                        position_state.first_buy_slot = swap.slot;
                        position_state.first_buy_timestamp = Some(swap.timestamp);
                    }

                    // Calculate average buy price (weighted by amount)
                    if position_state.total_tokens > 0.0 {
                        position_state.average_buy_price = position_state.total_sol_invested / position_state.total_tokens;
                    }
                }
                "Sell" => {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "DEBUG_SELL", &format!(
                            "Processing SELL for {}: -{:.2} tokens, current total: {:.2} -> {:.2}",
                            swap.token_symbol, swap.token_amount.abs(), position_state.total_tokens, 
                            position_state.total_tokens - swap.token_amount.abs()
                        ));
                    }
                    
                    position_state.total_tokens -= swap.token_amount.abs(); // Always use absolute value for sells
                    position_state.total_sol_received += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.sell_count += 1;

                    // If position is fully closed (or oversold), move to completed
                    if position_state.total_tokens <= 0.0001 { // Small epsilon for rounding
                        let position_analysis = self.finalize_position_analysis(position_state.clone());
                        completed_positions.push(position_analysis);
                        
                        // Mark this position as processed by clearing the buy count
                        // This prevents it from being re-added in the final loop
                        *position_state = PositionState {
                            token_mint: swap.token_mint.clone(),
                            token_symbol: swap.token_symbol.clone(),
                            total_tokens: position_state.total_tokens.min(0.0), // Keep negative if oversold
                            total_sol_invested: 0.0,
                            total_sol_received: position_state.total_sol_received,
                            total_fees: position_state.total_fees,
                            total_ata_rents: position_state.total_ata_rents,
                            buy_count: 0, // Reset to 0 to prevent re-addition
                            sell_count: position_state.sell_count,
                            first_buy_slot: None,
                            last_activity_slot: swap.slot,
                            first_buy_timestamp: None,
                            last_activity_timestamp: Some(swap.timestamp),
                            average_buy_price: 0.0,
                            transactions: vec![position_state.transactions.last().unwrap().clone()],
                        };
                    }
                }
                _ => {} // Ignore other transaction types
            }

            // Update last activity
            position_state.last_activity_slot = swap.slot;
            position_state.last_activity_timestamp = Some(swap.timestamp);
        }

        // Add remaining open positions
        for (_, position_state) in positions {
            if position_state.total_tokens > 0.0001 || position_state.buy_count > 0 {
                let position_analysis = self.finalize_position_analysis(position_state);
                completed_positions.push(position_analysis);
            }
        }

        // Sort by first buy timestamp (newest first)
        completed_positions.sort_by(|a, b| {
            match (&b.first_buy_timestamp, &a.first_buy_timestamp) {
                (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        log(LogTag::Transactions, "POSITION_CALC", &format!(
            "Generated {} position analyses", completed_positions.len()
        ));

        completed_positions
    }

    /// Finalize position analysis with PnL calculations
    fn finalize_position_analysis(&self, state: PositionState) -> PositionAnalysis {
        let net_sol_flow = state.total_sol_received - state.total_sol_invested;
        // Only include trading fees in costs, not ATA rents (they're mostly recoverable infrastructure costs)
        let total_costs = state.total_fees;
        let realized_pnl = net_sol_flow - total_costs;
        
        // Calculate unrealized PnL for open positions
        let unrealized_pnl = if state.total_tokens > 0.0001 {
            // Would need current token price for accurate unrealized PnL
            // For now, estimate based on average buy price
            0.0 // TODO: Integrate with current price data
        } else {
            0.0
        };

        let total_pnl = realized_pnl + unrealized_pnl;
        
        // Determine position status
        let status = if state.total_tokens > 0.0001 {
            if state.sell_count > 0 {
                PositionStatus::PartiallyReduced
            } else {
                PositionStatus::Open
            }
        } else if state.total_tokens < -0.0001 {
            PositionStatus::Oversold
        } else {
            PositionStatus::Closed
        };

        // Calculate position duration
        let duration_hours = if let (Some(first), Some(last)) = (&state.first_buy_timestamp, &state.last_activity_timestamp) {
            let duration = last.signed_duration_since(*first);
            duration.num_hours() as f64 + (duration.num_minutes() % 60) as f64 / 60.0
        } else {
            0.0
        };

        PositionAnalysis {
            token_mint: state.token_mint,
            token_symbol: state.token_symbol,
            status,
            total_tokens_bought: state.transactions.iter()
                .filter(|t| t.swap_type == "Buy")
                .map(|t| t.token_amount)
                .sum(),
            total_tokens_sold: state.transactions.iter()
                .filter(|t| t.swap_type == "Sell")
                .map(|t| t.token_amount.abs())  // Use absolute value for sell amounts
                .sum(),
            remaining_tokens: state.total_tokens,
            total_sol_invested: state.total_sol_invested,
            total_sol_received: state.total_sol_received,
            net_sol_flow,
            average_buy_price: state.average_buy_price,
            realized_pnl,
            unrealized_pnl,
            total_pnl,
            total_fees: state.total_fees,
            total_ata_rents: state.total_ata_rents,
            buy_count: state.buy_count,
            sell_count: state.sell_count,
            first_buy_timestamp: state.first_buy_timestamp,
            last_activity_timestamp: state.last_activity_timestamp,
            duration_hours,
            transactions: state.transactions,
        }
    }

    /// Display comprehensive position analysis table
    pub fn display_position_analysis_table(&self, positions: &[PositionAnalysis]) {
        if positions.is_empty() {
            log(LogTag::Transactions, "INFO", "No positions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE POSITION ANALYSIS ===");
        
        // Print header
        println!("=== COMPREHENSIVE POSITION ANALYSIS ===");

        // Convert positions to display rows
        let mut display_rows: Vec<PositionDisplayRow> = Vec::new();
        let mut total_invested = 0.0;
        let mut total_received = 0.0;
        let mut total_fees = 0.0;
        let mut total_pnl = 0.0;
        let mut open_positions = 0;
        let mut closed_positions = 0;

        for position in positions {
            let status_display = match position.status {
                PositionStatus::Open => "🟢 Open".to_string(),
                PositionStatus::Closed => "🔴 Closed".to_string(), 
                PositionStatus::PartiallyReduced => "🟡 Partial".to_string(),
                PositionStatus::Oversold => "🟣 Oversold".to_string(),
            };

            // Format SOL amounts with proper signs for intuitive display
            // Invested: negative (outflow), Received: positive (inflow)
            let sol_in_display = if position.total_sol_invested > 0.0 {
                format!("-{:.3}", position.total_sol_invested)
            } else {
                format!("{:.3}", position.total_sol_invested)
            };

            let sol_out_display = if position.total_sol_received > 0.0 {
                format!("+{:.3}", position.total_sol_received)
            } else {
                format!("{:.3}", position.total_sol_received)
            };

            // Format PnL
            let pnl_display = if position.total_pnl > 0.0 {
                format!("+{:.3}", position.total_pnl)
            } else if position.total_pnl < 0.0 {
                format!("{:.3}", position.total_pnl)
            } else {
                format!("{:.3}", position.total_pnl)
            };

            // Format token amounts
            let bought_display = format!("{}", position.buy_count);
            let sold_display = if position.total_tokens_sold > 0.0 {
                format!("{:.2}", position.total_tokens_sold)
            } else {
                "0.00".to_string()
            };
            let remaining_display = if position.remaining_tokens > 0.0 {
                format!("{:.2}", position.remaining_tokens)
            } else {
                "0.00".to_string()
            };

            // Format duration - fix negative duration issue
            let duration_display = if position.duration_hours > 0.0 {
                if position.duration_hours > 24.0 {
                    format!("{:.1}d", position.duration_hours / 24.0)
                } else {
                    format!("{:.1}h", position.duration_hours)
                }
            } else {
                "0.0h".to_string()
            };

            display_rows.push(PositionDisplayRow {
                token: position.token_symbol[..15.min(position.token_symbol.len())].to_string(),
                status: status_display,
                buys: bought_display,
                sold: sold_display,
                remaining: remaining_display,
                sol_in: sol_in_display,
                sol_out: sol_out_display,
                net_pnl: pnl_display,
                avg_price: format!("{:.9}", position.average_buy_price),
                fees: format!("{:.6}", position.total_fees), // Only trading fees, not ATA rents
                duration: duration_display,
            });

            // Update totals
            total_invested += position.total_sol_invested;
            total_received += position.total_sol_received;
            total_fees += position.total_fees + position.total_ata_rents;
            total_pnl += position.total_pnl;

            match position.status {
                PositionStatus::Open | PositionStatus::PartiallyReduced => open_positions += 1,
                PositionStatus::Closed | PositionStatus::Oversold => closed_positions += 1,
            }
        }

        // Create and display the table
        let table_string = Table::new(display_rows)
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .to_string();

        // Print the entire table directly to console
        println!("{}", table_string);
        
        let net_pnl_display = if total_pnl > 0.0 {
            format!("+{:.3}", total_pnl)
        } else if total_pnl < 0.0 {
            format!("{:.3}", total_pnl)
        } else {
            format!("{:.3}", total_pnl)
        };

        // Print summary
        println!("📊 SUMMARY: {} Open, {} Closed | Invested: {:.3} SOL | Received: {:.3} SOL | Fees: {:.3} SOL | Net PnL: {}",
            open_positions, closed_positions, total_invested, total_received, total_fees, net_pnl_display
        );
        println!("=== END POSITION ANALYSIS ===");

        log(LogTag::Transactions, "TABLE", &format!(
            "📊 SUMMARY: {} Open, {} Closed | Invested: {:.3} SOL | Received: {:.3} SOL | Fees: {:.3} SOL | Net PnL: {}",
            open_positions, closed_positions, total_invested, total_received, total_fees, net_pnl_display
        ));
        log(LogTag::Transactions, "TABLE", "=== END POSITION ANALYSIS ===");
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
            ata_creations_count: 0,
            ata_closures_count: 0,
            net_ata_rent_flow: 0.0,
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

/// Get transaction by signature (for positions.rs integration) - cache-first approach
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
    
    if !Path::new(&cache_file).exists() {
        // Try to fetch and cache if not found
        let wallet_address = match load_wallet_address_from_config().await {
            Ok(addr) => addr,
            Err(_) => return Ok(None), // Can't fetch without wallet
        };
        
        let mut manager = TransactionsManager::new(wallet_address).await
            .map_err(|e| format!("Failed to create manager: {}", e))?;
        
        match manager.process_transaction(signature).await {
            Ok(transaction) => return Ok(Some(transaction)),
            Err(_) => return Ok(None), // Transaction not found or error
        }
    }

    // Load from cache
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
