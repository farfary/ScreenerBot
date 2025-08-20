/// Transactions Manager - Real-time background transaction monitoring and analysis
/// Tracks wallet transactions, caches data, detects transaction types, and integrates with positions
///
/// **All transaction analysis functionality is integrated directly into this module.**
/// This includes DEX detection, swap analysis, balance calculations, and type classification.
/// 
/// Debug Tool: Use `cargo run --bin main_debug` for comprehensive debugging,
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
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::str::FromStr;
use tabled::{Table, Tabled, settings::{Style, Modify, object::Rows, Alignment}};
use once_cell::sync::Lazy;
use rand;

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
use crate::tokens::decimals::{raw_to_ui_amount, lamports_to_sol, sol_to_lamports};
use crate::tokens::price::get_token_price_blocking_safe;

// =============================================================================
// SERDE HELPER FUNCTIONS
// =============================================================================

/// Helper function for serde to skip serializing zero f64 values
fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

/// Helper function for serde to skip serializing default TransactionType (Unknown)
fn is_transaction_type_unknown(transaction_type: &TransactionType) -> bool {
    matches!(transaction_type, TransactionType::Unknown)
}

/// Helper function for serde to skip serializing default TransactionDirection (Internal)
fn is_direction_internal(direction: &TransactionDirection) -> bool {
    matches!(direction, TransactionDirection::Internal)
}

/// Helper function to safely get signature prefix for logging
fn get_signature_prefix(signature: &str) -> &str {
    if signature.len() >= 8 {
        &signature[..8]
    } else {
        signature
    }
}

/// Helper function to safely get mint address prefix for logging
fn get_mint_prefix(mint: &str) -> &str {
    if mint.len() >= 8 {
        &mint[..8]
    } else {
        mint
    }
}

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Timing configuration for transaction manager - SIMPLIFIED
/// This replaces the complex adaptive timing system with predictable intervals

// Main monitoring intervals
const NORMAL_CHECK_INTERVAL_SECS: u64 = 15;      // Normal transaction checking every 15 seconds

// RPC and batch processing limits
const RPC_BATCH_SIZE: usize = 1000;                      // Transaction signatures fetch batch size (increased for fewer pages)
const TRANSACTION_DATA_BATCH_SIZE: usize = 50;           // Transaction data fetch batch size (optimized for speed)

// Solana network constants  
const ATA_RENT_COST_SOL: f64 = 0.00203928;              // Standard ATA creation/closure cost
const ATA_RENT_TOLERANCE_LAMPORTS: i64 = 10000;         // Tolerance for ATA rent variations (lamports)
const DEFAULT_COMPUTE_UNIT_PRICE: u64 = 1000;           // Default compute unit price (micro-lamports)
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112"; // Wrapped SOL mint address

// Analysis cache versioning (bump when snapshot schema changes)
const ANALYSIS_CACHE_VERSION: u32 = 1;


// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// Deferred retry record for signatures that timed out/dropped
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredRetry {
    pub signature: String,
    pub next_retry_at: DateTime<Utc>,
    pub remaining_attempts: i32,
    pub current_delay_secs: i64,
    pub last_error: Option<String>,
}

/// Main Transaction structure used throughout the bot
/// Contains all Solana data + our calculations and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    // Core identification
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub timestamp: DateTime<Utc>,
    
    // Transaction status (consolidated from commitment_state, confirmation_status, finalized)
    pub status: TransactionStatus,
    
    // Transaction type and analysis - NEVER CACHED - always calculated fresh  
    #[serde(skip_serializing, default)]
    pub transaction_type: TransactionType,
    #[serde(skip_serializing, default)]
    pub direction: TransactionDirection,
    pub success: bool,
    pub error_message: Option<String>,
    
    // Financial data - NEVER CACHED - always calculated fresh
    #[serde(skip_serializing, default)]
    pub fee_sol: f64,
    #[serde(skip_serializing, default)]
    pub sol_balance_change: f64,
    #[serde(skip_serializing, default)]
    pub token_transfers: Vec<TokenTransfer>,
    
    // Raw Solana data (cached - only raw blockchain data)
    pub raw_transaction_data: Option<serde_json::Value>,
    #[serde(skip_serializing, default)]
    pub log_messages: Vec<String>,
    #[serde(skip_serializing, default)]
    pub instructions: Vec<InstructionInfo>,
    
    // Balance changes - NEVER CACHED - always calculated fresh
    #[serde(skip_serializing, default)]
    pub sol_balance_changes: Vec<SolBalanceChange>,
    #[serde(skip_serializing, default)]
    pub token_balance_changes: Vec<TokenBalanceChange>,
    
    // Our analysis and calculations - NEVER CACHED - always calculated fresh
    #[serde(skip_serializing, default)]
    pub swap_analysis: Option<SwapAnalysis>,
    #[serde(skip_serializing, default)]
    pub position_impact: Option<PositionImpact>,
    #[serde(skip_serializing, default)]
    pub profit_calculation: Option<ProfitCalculation>,
    #[serde(skip_serializing, default)]
    pub fee_breakdown: Option<FeeBreakdown>,
    #[serde(skip_serializing, default)]
    pub ata_analysis: Option<AtaAnalysis>,        // SINGLE source of truth for ATA operations
    
    // Token information integration - NEVER CACHED - always calculated fresh
    #[serde(skip_serializing, default)]
    pub token_info: Option<TokenSwapInfo>,
    #[serde(skip_serializing, default)]
    pub calculated_token_price_sol: Option<f64>,
    #[serde(skip_serializing, default)]
    pub price_source: Option<PriceSourceType>,
    #[serde(skip_serializing, default)]
    pub token_symbol: Option<String>,
    #[serde(skip_serializing, default)]
    pub token_decimals: Option<u8>,
    
    // Cache file path and metadata
    pub last_updated: DateTime<Utc>,
    pub cache_file_path: String,

    // Optional persisted analysis snapshot for finalized txs to avoid re-analysis on every load
    #[serde(default)]
    pub cached_analysis: Option<CachedAnalysis>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Finalized,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    AtaClose {
        recovered_sol: f64,
        token_mint: String,
    },
    Other {
        description: String,
        details: String,
    },
    Unknown,
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionType::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
    Internal,
}

impl Default for TransactionDirection {
    fn default() -> Self {
        TransactionDirection::Internal
    }
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
    pub total_fees: f64,            // Total fees in SOL
    pub fee_percentage: f64,        // Trading fee as percentage of transaction value
}

/// Comprehensive ATA (Associated Token Account) analysis for a transaction
/// This is the SINGLE source of truth for all ATA operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaAnalysis {
    // Raw counts from transaction
    pub total_ata_creations: u32,           // Total ATA creations in transaction
    pub total_ata_closures: u32,            // Total ATA closures in transaction
    
    // Token-specific counts (for swap analysis)
    pub token_ata_creations: u32,           // ATA creations for specific token
    pub token_ata_closures: u32,            // ATA closures for specific token
    
    // WSOL-specific counts (for SOL wrapping/unwrapping)
    pub wsol_ata_creations: u32,            // WSOL ATA creations
    pub wsol_ata_closures: u32,             // WSOL ATA closures
    
    // Financial impact (in SOL)
    pub total_rent_spent: f64,              // Total SOL spent on ATA creation
    pub total_rent_recovered: f64,          // Total SOL recovered from ATA closure
    pub net_rent_impact: f64,               // Net impact: recovered - spent (positive = gained SOL, negative = spent SOL)
    
    // Token-specific financial impact (for accurate swap amounts)
    pub token_rent_spent: f64,              // SOL spent on token ATA creation
    pub token_rent_recovered: f64,          // SOL recovered from token ATA closure
    pub token_net_rent_impact: f64,         // Net token ATA impact
    
    // WSOL-specific financial impact
    pub wsol_rent_spent: f64,               // SOL spent on WSOL ATA creation
    pub wsol_rent_recovered: f64,           // SOL recovered from WSOL ATA closure  
    pub wsol_net_rent_impact: f64,          // Net WSOL ATA impact
    
    // Detected operations (for debugging)
    pub detected_operations: Vec<AtaOperation>,
}

/// Individual ATA operation detected in transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaOperation {
    pub operation_type: AtaOperationType,
    pub account_address: String,
    pub token_mint: String,             // The mint this ATA is associated with
    pub rent_amount: f64,               // SOL amount involved (spent or recovered)
    pub is_wsol: bool,                  // Whether this is a WSOL ATA
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AtaOperationType {
    Creation,
    Closure,
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

/// SwapPnLInfo - Swap analysis data structure
/// CRITICAL: This struct should NEVER be cached to disk
/// All SwapPnLInfo instances must be calculated fresh on every request
/// This ensures calculations are always current and accurate
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
    
    // New fields for effective price calculation (excluding ATA rent but including fees)
    pub effective_sol_spent: f64,    // For BUY: SOL spent for tokens (includes fees, excludes ATA rent)
    pub effective_sol_received: f64, // For SELL: SOL received for tokens (includes fees, excludes ATA rent)
    
    // Token-specific ATA operations for this swap (counts)
    pub ata_created_count: u32,
    pub ata_closed_count: u32,
    
    pub slot: Option<u64>,  // Solana slot number for reliable chronological sorting
    pub status: String,     // Transaction status: "✅ Success", "❌ Failed", "⚠️ Partial", etc.
}

// =============================================================================
// LIGHTWEIGHT CACHED ANALYSIS SNAPSHOT
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    pub version: u32,
    pub hydrated: bool,
    pub transaction_type: TransactionType,
    pub direction: TransactionDirection,
    pub success: bool,
    pub fee_sol: f64,
    pub sol_balance_change: f64,
    pub token_transfers: Vec<TokenTransfer>,
    pub sol_balance_changes: Vec<SolBalanceChange>,
    pub token_balance_changes: Vec<TokenBalanceChange>,
    pub ata_analysis: Option<AtaAnalysis>,
    pub token_info: Option<TokenSwapInfo>,
    pub calculated_token_price_sol: Option<f64>,
    pub price_source: Option<PriceSourceType>,
    pub token_symbol: Option<String>,
    pub token_decimals: Option<u8>,
}

impl CachedAnalysis {
    fn from_transaction(tx: &Transaction) -> Self {
        CachedAnalysis {
            version: ANALYSIS_CACHE_VERSION,
            hydrated: true,
            transaction_type: tx.transaction_type.clone(),
            direction: tx.direction.clone(),
            success: tx.success,
            fee_sol: tx.fee_sol,
            sol_balance_change: tx.sol_balance_change,
            token_transfers: tx.token_transfers.clone(),
            sol_balance_changes: tx.sol_balance_changes.clone(),
            token_balance_changes: tx.token_balance_changes.clone(),
            ata_analysis: tx.ata_analysis.clone(),
            token_info: tx.token_info.clone(),
            calculated_token_price_sol: tx.calculated_token_price_sol,
            price_source: tx.price_source.clone(),
            token_symbol: tx.token_symbol.clone(),
            token_decimals: tx.token_decimals,
        }
    }
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
    #[tabled(rename = "Date")]
    pub date: String,
    #[tabled(rename = "Time")]
    pub time: String,
    #[tabled(rename = "Signature")]
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
    #[tabled(rename = "Effective SOL")]
    pub effective_sol: String,  // Shows effective_sol_spent for buys, effective_sol_received for sells
    #[tabled(rename = "Effective Price")]
    pub effective_price: String,  // Price calculated using effective SOL amounts
    #[tabled(rename = "ATA Rents")]
    pub ata_rents: String,
    #[tabled(rename = "Router")]
    pub router: String,
    #[tabled(rename = "Fee")]
    pub fee: String,
    #[tabled(rename = "Status")]
    pub status: String,
}

/// Tabled structure for position analysis display
#[derive(Tabled)]
pub struct PositionDisplayRow {
    #[tabled(rename = "Token")]
    pub token: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Opened")]
    pub opened: String,
    #[tabled(rename = "Closed")]
    pub closed: String,
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



/// Helper function to shorten transaction signatures for display
/// Shows first 8 characters + "..." + last 4 characters
/// Example: "2iPhXfdKg4VsyPoLpsstHTmXb7VuoetfGSu9s1Ajrk7Xqmt8qEScRFjpynqUUPSKZ4ySrGUajEQnudL3AWPFoGiM"
/// becomes: "2iPhXfdK...oGiM"
fn shorten_signature(signature: &str) -> String {
    if signature.len() <= 16 {
        signature.to_string()
    } else {
        format!("{}...{}", &signature[..8], &signature[signature.len()-4..])
    }
}

// =============================================================================
// TRANSACTIONS MANAGER
// =============================================================================

/// TransactionsManager - Main service for real-time transaction monitoring
pub struct TransactionsManager {
    pub wallet_pubkey: Pubkey,
    pub debug_enabled: bool,
    pub known_signatures: HashSet<String>,
    pub last_signature_check: Option<String>,
    pub total_transactions: u64,
    pub new_transactions_count: u64,
    
    // Token database integration
    pub token_database: Option<TokenDatabase>,

    // Deferred retries for transactions that failed to process
    // This helps avoid losing verifications due to temporary RPC lag or network issues
    pub deferred_retries: HashMap<String, DeferredRetry>,
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
            last_signature_check: None,
            total_transactions: 0,
            new_transactions_count: 0,
            token_database,
            deferred_retries: HashMap::new(),
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
        
        if self.debug_enabled {
            log(LogTag::Transactions, "RPC_CALL", &format!(
                "Checking for new transactions (known: {}, using latest 50)", 
                self.known_signatures.len()
            ));
        }
        
        // Get recent signatures from wallet
        // IMPORTANT: Always fetch most recent page (no 'before' cursor) to avoid missing new txs
        let signatures = rpc_client
            .get_wallet_signatures_main_rpc(&self.wallet_pubkey, 50, None)
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
            
            // Do not advance pagination cursor here; we always fetch the latest page
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

        // Check if we already have this transaction cached and can avoid RPC call
        let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
        let use_cache = Path::new(&cache_file).exists();

        let mut transaction = if use_cache {
            // Load from cache and always recalculate
            if self.debug_enabled {
                log(LogTag::Transactions, "CACHE_LOAD", &format!("Loading cached transaction: {}", &signature[..8]));
            }
            let mut transaction = self.load_transaction_from_cache(Path::new(&cache_file)).await?;

            // Always recalculate transaction analysis
            if self.debug_enabled {
                log(LogTag::Transactions, "RECALC", &format!(
                    "Recalculating transaction: {}", &signature[..8]
                ));
            }
            self.recalculate_transaction_analysis(&mut transaction).await?;
            // Persist a snapshot for finalized transactions
            if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
                transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
            }

            transaction
        } else {
            // Fetch fresh data from RPC
            if self.debug_enabled {
                log(LogTag::Transactions, "RPC_FETCH", &format!("Fetching new transaction: {}", &signature[..8]));
            }
            
            let rpc_client = get_rpc_client();
            let tx_data = match rpc_client
                .get_transaction_details_premium_rpc(signature)
                .await
            {
                Ok(data) => {
                    log(LogTag::Rpc, "SUCCESS", &format!("Retrieved transaction details for {} from premium RPC", &signature[..8]));
                    data
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("not found") || error_msg.contains("no longer available") {
                        log(LogTag::Rpc, "NOT_FOUND", &format!("Transaction {} not found on-chain (likely failed swap)", &signature[..8]));
                        return Err(format!("Transaction not found: {}", signature));
                    } else {
                        log(LogTag::Rpc, "ERROR", &format!("RPC error fetching {}: {}", &signature[..8], error_msg));
                        return Err(format!("Failed to fetch transaction details: {}", e));
                    }
                }
            };

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
                status: TransactionStatus::Finalized, // Since we fetched it successfully
                transaction_type: TransactionType::Unknown,
                direction: TransactionDirection::Internal,
                success: tx_data.transaction.meta.as_ref().map_or(false, |meta| meta.err.is_none()),
                error_message: tx_data.transaction.meta.as_ref()
                    .and_then(|meta| meta.err.as_ref())
                    .map(|err| format!("{:?}", err)),
                fee_sol: tx_data.transaction.meta.as_ref().map_or(0.0, |meta| lamports_to_sol(meta.fee)),
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
                ata_analysis: None,
                token_info: None,
                calculated_token_price_sol: None,
                price_source: None,
                token_symbol: None,
                token_decimals: None,
                last_updated: Utc::now(),
                cache_file_path: cache_file.clone(),
                cached_analysis: None,
            };

            // Analyze transaction type and extract details
            self.analyze_transaction(&mut transaction).await?;

            // Persist a snapshot for finalized transactions to avoid future re-analysis
            if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
                transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
            }
            
            transaction
        };

        // Always cache the result (updates existing cache with new analysis)
        self.cache_transaction(&transaction).await?;

        Ok(transaction)
    }

    /// Analyze transaction to determine type and extract data
    async fn analyze_transaction(&mut self, transaction: &mut Transaction) -> Result<(), String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "ANALYZE", &format!(
                "Transaction {} - Type: {:?}, SOL change: {:.6}", 
                &transaction.signature[..8], 
                transaction.transaction_type,
                transaction.sol_balance_change
            ));
        }

        // CRITICAL: Extract basic transaction info from raw data FIRST
        // This populates slot, block_time, log_messages, success, fee, etc.
        self.extract_basic_transaction_info(transaction).await?;

        // Analyze transaction type from raw data (now has log messages)
        self.analyze_transaction_type(transaction).await?;
        
        // Additional analysis based on transaction type
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { .. } | TransactionType::SwapTokenToSol { .. } => {
                // For swaps, build precise ATA analysis so PnL can exclude ATA rent correctly
                if let Err(e) = self.compute_and_set_ata_analysis(transaction).await {
                    if self.debug_enabled {
                        log(LogTag::Transactions, "WARN", &format!(
                            "ATA analysis failed for swap {}: {}", &transaction.signature[..8], e
                        ));
                    }
                }
            }
            _ => {
                // For other transaction types (transfers, unknown, etc.), no additional analysis needed
            }
        }

        Ok(())
    }

    /// If transaction has a valid cached analysis snapshot, hydrate derived fields from it
    pub fn try_hydrate_from_cached_analysis(&self, transaction: &mut Transaction) -> bool {
        if let Some(snapshot) = &transaction.cached_analysis {
            if snapshot.version == ANALYSIS_CACHE_VERSION && snapshot.hydrated {
                transaction.transaction_type = snapshot.transaction_type.clone();
                transaction.direction = snapshot.direction.clone();
                transaction.success = snapshot.success;
                transaction.fee_sol = snapshot.fee_sol;
                transaction.sol_balance_change = snapshot.sol_balance_change;
                transaction.token_transfers = snapshot.token_transfers.clone();
                transaction.sol_balance_changes = snapshot.sol_balance_changes.clone();
                transaction.token_balance_changes = snapshot.token_balance_changes.clone();
                transaction.ata_analysis = snapshot.ata_analysis.clone();
                transaction.token_info = snapshot.token_info.clone();
                transaction.calculated_token_price_sol = snapshot.calculated_token_price_sol;
                transaction.price_source = snapshot.price_source.clone();
                transaction.token_symbol = snapshot.token_symbol.clone();
                transaction.token_decimals = snapshot.token_decimals;
                return true;
            }
        }
        false
    }
    
    /// Clean transaction for cache storage by removing all calculated fields
    /// Clean transaction for caching - keeps ONLY raw blockchain data
    /// CRITICAL: All calculated fields are removed and set to defaults
    /// This ensures no calculated values are ever cached to disk
    fn clean_transaction_for_cache(&self, transaction: &Transaction) -> Transaction {
        Transaction {
            // Keep ONLY essential metadata and raw blockchain data
            signature: transaction.signature.clone(),
            slot: transaction.slot,
            block_time: transaction.block_time,
            timestamp: transaction.timestamp,
            status: transaction.status.clone(),
            success: transaction.success,
            error_message: transaction.error_message.clone(),
            raw_transaction_data: transaction.raw_transaction_data.clone(),
            last_updated: transaction.last_updated,
            cache_file_path: transaction.cache_file_path.clone(),
            // Keep snapshot only if transaction is finalized to avoid redundant recalcs
            cached_analysis: match transaction.status {
                TransactionStatus::Finalized => transaction.cached_analysis.clone(),
                _ => None,
            },
            
            // ALL calculated/derived fields are set to defaults - NEVER CACHED
            transaction_type: TransactionType::Unknown,
            direction: TransactionDirection::Internal,
            fee_sol: 0.0,
            sol_balance_change: 0.0,
            token_transfers: Vec::new(),
            log_messages: Vec::new(),
            instructions: Vec::new(),
            sol_balance_changes: Vec::new(),
            token_balance_changes: Vec::new(),
            swap_analysis: None,
            position_impact: None,
            profit_calculation: None,
            fee_breakdown: None,
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
        }
    }
    
    /// Cache transaction to disk
    async fn cache_transaction(&self, transaction: &Transaction) -> Result<(), String> {
        let cache_dir = get_transactions_cache_dir();
        
        // Ensure cache directory exists
        if !Path::new(&cache_dir).exists() {
            fs::create_dir_all(&cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        // Clean transaction before caching to remove calculated fields
        let cleaned_transaction = self.clean_transaction_for_cache(transaction);

        let cache_file_path = format!("{}/{}.json", cache_dir.display(), transaction.signature);
        let json_data = serde_json::to_string_pretty(&cleaned_transaction)
            .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

        fs::write(&cache_file_path, json_data)
            .map_err(|e| format!("Failed to write cache file: {}", e))?;

        if self.debug_enabled {
            log(LogTag::Transactions, "CACHE", &format!(
                "Cached cleaned transaction {} to disk (calculated fields removed)", 
                &transaction.signature[..8]
            ));
        }

        Ok(())
    }

    /// Fetch and analyze ALL wallet transactions from blockchain (unlimited)
    /// This method fetches comprehensive transaction history directly from the blockchain
    /// and processes each transaction with full analysis, bypassing the cache
    pub async fn fetch_all_wallet_transactions(&mut self) -> Result<Vec<Transaction>, String> {
        log(LogTag::Transactions, "INFO", &format!(
            "Starting comprehensive blockchain fetch for wallet {} (no limit)", 
            self.wallet_pubkey
        ));

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(LogTag::Transactions, "ERROR", &format!(
                "Failed to initialize known signatures: {}", e
            ));
        } else if self.debug_enabled {
            log(LogTag::Transactions, "INIT", &format!(
                "Cache has {} transactions; will skip these during fetch", self.known_signatures.len()
            ));
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE; // Fetch in batches to avoid rate limits
        let mut total_fetched = 0;
        let mut total_skipped_cached = 0usize;

        log(LogTag::Transactions, "FETCH", "Fetching ALL transaction signatures from blockchain...");

        // Fetch transaction signatures in batches until exhausted
        loop {
            let signatures = match rpc_client
                .get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
                    batch_size,
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
                log(LogTag::Transactions, "INFO", "No more signatures available - completed full fetch");
                break;
            }

            let batch_count = signatures.len();
            total_fetched += batch_count;
            
            // Build list of signatures we don't already have cached
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            log(LogTag::Transactions, "FETCH", &format!(
                "Fetched batch of {} signatures (total seen: {}), to process (not cached): {} | skipped cached: {}", 
                batch_count, total_fetched, signatures_to_process.len(), total_skipped_cached
            ));
            
            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(LogTag::Transactions, "BATCH", &format!(
                    "Processing batch of {} transactions using batch RPC call", 
                    chunk_size
                ));

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(LogTag::Transactions, "BATCH", &format!(
                            "✅ Batch fetched {}/{} transactions successfully", 
                            batch_results.len(), chunk_size
                        ));

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(LogTag::Transactions, "BATCH", &format!(
                                    "Processing transaction from batch: {}", &signature[..8]
                                ));
                            }

                            match self.process_transaction_from_encoded_data(&signature, encoded_tx).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(LogTag::Transactions, "BATCH", &format!(
                                            "✅ Processed transaction: {}", &signature[..8]
                                        ));
                                    }
                                }
                                Err(e) => {
                                    log(LogTag::Transactions, "WARN", &format!(
                                        "Failed to process transaction {}: {}", &signature[..8], e
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!(
                            "Failed to batch fetch {} transactions: {}", chunk_size, e
                        ));
                        
                        // Fallback to individual processing if batch fails
                        log(LogTag::Transactions, "FALLBACK", "Falling back to individual transaction processing");
                        for signature in chunk {
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
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            before_signature = Some(signatures.last().unwrap().signature.clone());

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await; // Batch processing delay
        }

        log(LogTag::Transactions, "SUCCESS", &format!(
            "Completed comprehensive fetch: {} new transactions processed | {} cached skipped", 
            all_transactions.len(), total_skipped_cached
        ));

        Ok(all_transactions)
    }

    /// Fetch and analyze limited number of wallet transactions from blockchain (for testing)
    /// This method fetches a specific number of transactions for testing purposes
    pub async fn fetch_limited_wallet_transactions(&mut self, max_count: usize) -> Result<Vec<Transaction>, String> {
        log(LogTag::Transactions, "INFO", &format!(
            "Starting limited blockchain fetch for wallet {} (max {} transactions)", 
            self.wallet_pubkey, max_count
        ));

        // Initialize known signatures from cache so we can skip existing ones
        if let Err(e) = self.initialize_known_signatures().await {
            log(LogTag::Transactions, "ERROR", &format!(
                "Failed to initialize known signatures: {}", e
            ));
        } else if self.debug_enabled {
            log(LogTag::Transactions, "INIT", &format!(
                "Cache has {} transactions; will skip these during limited fetch", self.known_signatures.len()
            ));
        }

        let rpc_client = get_rpc_client();
        let mut all_transactions = Vec::new();
        let mut before_signature = None;
        let batch_size = RPC_BATCH_SIZE;
        let mut total_fetched = 0; // total signatures seen
        let mut total_skipped_cached = 0usize;
        let mut total_to_process = 0usize; // count of new (not cached) we attempted to process

        log(LogTag::Transactions, "FETCH", "Fetching transaction signatures from blockchain...");

        // Fetch transaction signatures in batches
    while total_to_process < max_count {
            let signatures = match rpc_client
                .get_wallet_signatures_main_rpc(
                    &self.wallet_pubkey,
            batch_size,
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
            
            // Filter out cached signatures; only process unknown ones, but cap by remaining_needed
            let mut signatures_to_process: Vec<String> = Vec::new();
            for s in &signatures {
                if self.known_signatures.contains(&s.signature) {
                    total_skipped_cached += 1;
                } else if signatures_to_process.len() + total_to_process < max_count {
                    signatures_to_process.push(s.signature.clone());
                }
            }

            total_to_process += signatures_to_process.len();

            log(LogTag::Transactions, "FETCH", &format!(
                "Fetched batch of {} signatures (seen total: {}), to process (not cached): {} (goal {}), skipped cached so far: {}", 
                batch_count, total_fetched, signatures_to_process.len(), max_count, total_skipped_cached
            ));
            
            for chunk in signatures_to_process.chunks(TRANSACTION_DATA_BATCH_SIZE) {
                let chunk_size = chunk.len();
                log(LogTag::Transactions, "BATCH", &format!(
                    "Processing batch of {} transactions using batch RPC call", 
                    chunk_size
                ));

                // Use batch RPC call to fetch all transactions in this chunk at once
                match rpc_client.batch_get_transaction_details_premium_rpc(chunk).await {
                    Ok(batch_results) => {
                        log(LogTag::Transactions, "BATCH", &format!(
                            "✅ Batch fetched {}/{} transactions successfully", 
                            batch_results.len(), chunk_size
                        ));

                        // Process each transaction from the batch results
                        for (signature, encoded_tx) in batch_results {
                            if self.debug_enabled {
                                log(LogTag::Transactions, "BATCH", &format!(
                                    "Processing transaction from batch: {}", &signature[..8]
                                ));
                            }

                            match self.process_transaction_from_encoded_data(&signature, encoded_tx).await {
                                Ok(transaction) => {
                                    all_transactions.push(transaction);
                                    if self.debug_enabled {
                                        log(LogTag::Transactions, "BATCH", &format!(
                                            "✅ Processed transaction: {}", &signature[..8]
                                        ));
                                    }
                                }
                                Err(e) => {
                                    log(LogTag::Transactions, "WARN", &format!(
                                        "Failed to process transaction {}: {}", &signature[..8], e
                                    ));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!(
                            "Failed to batch fetch {} transactions: {}", chunk_size, e
                        ));
                        
                        // Fallback to individual processing if batch fails
                        log(LogTag::Transactions, "FALLBACK", "Falling back to individual transaction processing");
                        for signature in chunk {
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
                        }
                    }
                }

                // Shorter delay between transaction batches
                if chunk_size == TRANSACTION_DATA_BATCH_SIZE {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }

            // Set the before signature for the next batch
            before_signature = Some(signatures.last().unwrap().signature.clone());

            // Batch processing delay
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        log(LogTag::Transactions, "SUCCESS", &format!(
            "Completed limited fetch: {} new transactions processed | {} cached skipped", 
            all_transactions.len(), total_skipped_cached
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
            status: TransactionStatus::Confirmed,
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
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cache_file_path: format!("{}/{}.json", get_transactions_cache_dir().display(), signature),
            cached_analysis: None,
        };

        // Fetch fresh transaction data from blockchain
        self.fetch_transaction_data(&mut transaction).await?;

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

        // Cache the processed transaction
        self.cache_transaction(&transaction).await?;

        // Update known signatures
        self.known_signatures.insert(signature.to_string());

        Ok(transaction)
    }

    /// Process transaction from encoded data (used for batch processing)
    /// This is optimized for batch processing where we already have the transaction data
    async fn process_transaction_from_encoded_data(
        &mut self, 
        signature: &str, 
        encoded_tx: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta
    ) -> Result<Transaction, String> {
        if self.debug_enabled {
            log(LogTag::Transactions, "BATCH_PROCESS", &format!(
                "Processing transaction from batch data: {}", &signature[..8]
            ));
        }

        // Create new transaction struct
        let mut transaction = Transaction {
            signature: signature.to_string(),
            slot: None,
            block_time: None,
            timestamp: Utc::now(),
            status: TransactionStatus::Confirmed,
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
            ata_analysis: None,
            token_info: None,
            calculated_token_price_sol: None,
            price_source: None,
            token_symbol: None,
            token_decimals: None,
            last_updated: Utc::now(),
            cache_file_path: format!("{}/{}.json", get_transactions_cache_dir().display(), signature),
            cached_analysis: None,
        };

        // Convert encoded transaction to raw data format
        let raw_data = serde_json::to_value(&encoded_tx)
            .map_err(|e| format!("Failed to serialize encoded transaction data: {}", e))?;
        
        transaction.raw_transaction_data = Some(raw_data);

        // Perform comprehensive analysis
        self.analyze_transaction(&mut transaction).await?;
        // Defensive: if raw data has block_time and no error, treat as finalized
        if transaction.block_time.is_some() && transaction.success {
            transaction.status = TransactionStatus::Finalized;
        }

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

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
        // Reset basic extracted fields
        transaction.slot = None;
        transaction.block_time = None;
        transaction.success = false;
        transaction.error_message = None;
        transaction.fee_sol = 0.0;
        transaction.log_messages = Vec::new();
        transaction.instructions = Vec::new();
        
        // Reset analysis fields
        transaction.transaction_type = TransactionType::Unknown;
        transaction.direction = TransactionDirection::Internal;
        transaction.sol_balance_change = 0.0;
        transaction.token_transfers = Vec::new();
        transaction.swap_analysis = None;
        transaction.position_impact = None;
        transaction.profit_calculation = None;
        transaction.fee_breakdown = None;
        transaction.ata_analysis = None;  // CRITICAL: Reset ATA analysis for recalculation
        transaction.token_info = None;
        transaction.calculated_token_price_sol = None;
        transaction.price_source = None;
        transaction.token_symbol = None;
        transaction.token_decimals = None;

        // Recalculate all analysis using existing raw data
        if transaction.raw_transaction_data.is_some() {
            // Re-run the comprehensive analysis using cached raw data
            self.analyze_transaction(&mut *transaction).await?;
            
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
            known_signatures_count: self.known_signatures.len() as u64,
        }
    }

    /// Get known signatures count (for testing)  
    pub fn known_signatures(&self) -> &HashSet<String> {
        &self.known_signatures
    }

    /// Get recent transactions from cache (for orphaned position recovery)
    pub async fn get_recent_transactions(&self, limit: usize) -> Result<Vec<Transaction>, String> {
        let cache_dir = get_transactions_cache_dir();
        
        // Get all cached transaction files
        let mut transaction_files = Vec::new();
        let entries = std::fs::read_dir(&cache_dir)
            .map_err(|e| format!("Failed to read transactions cache directory: {}", e))?;
            
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                    if let Some(metadata) = std::fs::metadata(&path).ok() {
                        if let Ok(modified) = metadata.modified() {
                            transaction_files.push((path, modified));
                        }
                    }
                }
            }
        }
        
        // Sort by modification time (newest first)
        transaction_files.sort_by(|a, b| b.1.cmp(&a.1));
        
        // Load up to 'limit' transactions
        let mut transactions = Vec::new();
        for (file_path, _) in transaction_files.into_iter().take(limit) {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                if let Ok(mut transaction) = serde_json::from_str::<Transaction>(&content) {
                    // Best-effort hydration from snapshot
                    let _ = self.try_hydrate_from_cached_analysis(&mut transaction);
                    transactions.push(transaction);
                }
            }
        }
        
        Ok(transactions)
    }


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
                // Update timestamp to use blockchain time instead of processing time
                transaction.timestamp = DateTime::<Utc>::from_timestamp(block_time, 0)
                    .unwrap_or(transaction.timestamp);
            }

            // Extract meta information
            if let Some(meta) = raw_data.get("meta") {
                // Extract fee
                if let Some(fee) = meta.get("fee").and_then(|v| v.as_u64()) {
                    transaction.fee_sol = lamports_to_sol(fee); // Convert lamports to SOL
                }

                // Calculate SOL balance change from pre/post balances (signed!)
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preBalances").and_then(|v| v.as_array()),
                    meta.get("postBalances").and_then(|v| v.as_array())
                ) {
                    if !pre_balances.is_empty() && !post_balances.is_empty() {
                        // First account is always the main wallet account
                        let pre_balance_lamports = pre_balances[0].as_i64().unwrap_or(0);
                        let post_balance_lamports = post_balances[0].as_i64().unwrap_or(0);

                        // Signed change in lamports and convert to SOL
                        let balance_change_lamports: i64 = post_balance_lamports - pre_balance_lamports;
                        transaction.sol_balance_change = (balance_change_lamports as f64) / 1_000_000_000.0;

                        if self.debug_enabled {
                            log(LogTag::Transactions, "BALANCE", &format!(
                                "SOL balance change for {}: {} lamports ({:.9} SOL)",
                                &transaction.signature[..8],
                                balance_change_lamports,
                                transaction.sol_balance_change
                            ));
                        }
                    }
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
                                
                                // Handle both parsed and raw instruction formats
                                let (program_id_str, instruction_type, accounts) = if let Some(program_id) = instruction.get("programId").and_then(|v| v.as_str()) {
                                    // Parsed instruction format
                                    let instruction_type = if let Some(parsed) = instruction.get("parsed") {
                                        if let Some(type_name) = parsed.get("type").and_then(|v| v.as_str()) {
                                            type_name.to_string()
                                        } else {
                                            "parsed".to_string()
                                        }
                                    } else {
                                        format!("instruction_{}", index)
                                    };
                                    
                                    // Extract account information from parsed instruction
                                    let accounts = if let Some(parsed) = instruction.get("parsed") {
                                        if let Some(info) = parsed.get("info") {
                                            let mut acc_list = Vec::new();
                                            // Extract common account fields
                                            if let Some(source) = info.get("source").and_then(|v| v.as_str()) {
                                                acc_list.push(source.to_string());
                                            }
                                            if let Some(destination) = info.get("destination").and_then(|v| v.as_str()) {
                                                acc_list.push(destination.to_string());
                                            }
                                            if let Some(owner) = info.get("owner").and_then(|v| v.as_str()) {
                                                acc_list.push(owner.to_string());
                                            }
                                            if let Some(mint) = info.get("mint").and_then(|v| v.as_str()) {
                                                acc_list.push(mint.to_string());
                                            }
                                            if let Some(wallet) = info.get("wallet").and_then(|v| v.as_str()) {
                                                acc_list.push(wallet.to_string());
                                            }
                                            if let Some(account) = info.get("account").and_then(|v| v.as_str()) {
                                                acc_list.push(account.to_string());
                                            }
                                            if let Some(authority) = info.get("authority").and_then(|v| v.as_str()) {
                                                acc_list.push(authority.to_string());
                                            }
                                            acc_list
                                        } else {
                                            Vec::new()
                                        }
                                    } else {
                                        Vec::new()
                                    };
                                    
                                    (program_id.to_string(), instruction_type, accounts)
                                    
                                } else if let Some(program_id_index) = instruction.get("programIdIndex").and_then(|v| v.as_u64()) {
                                    // Raw instruction format - need to resolve program_id from account keys
                                    let program_id_str = if let Some(account_keys) = message.get("accountKeys").and_then(|v| v.as_array()) {
                                        if let Some(account_obj) = account_keys.get(program_id_index as usize) {
                                            if let Some(pubkey) = account_obj.get("pubkey").and_then(|v| v.as_str()) {
                                                pubkey.to_string()
                                            } else {
                                                "unknown".to_string()
                                            }
                                        } else {
                                            "unknown".to_string()
                                        }
                                    } else {
                                        "unknown".to_string()
                                    };
                                    
                                    // Extract accounts from instruction
                                    let accounts = if let Some(accounts_array) = instruction.get("accounts").and_then(|v| v.as_array()) {
                                        accounts_array.iter()
                                            .filter_map(|v| v.as_u64())
                                            .filter_map(|idx| {
                                                if let Some(account_keys) = message.get("accountKeys").and_then(|v| v.as_array()) {
                                                    if let Some(account_obj) = account_keys.get(idx as usize) {
                                                        account_obj.get("pubkey").and_then(|v| v.as_str()).map(|s| s.to_string())
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                    
                                    (program_id_str, format!("instruction_{}", index), accounts)
                                } else {
                                    ("unknown".to_string(), format!("instruction_{}", index), Vec::new())
                                };
                                
                                transaction.instructions.push(InstructionInfo {
                                    program_id: program_id_str,
                                    instruction_type,
                                    accounts,
                                    data: instruction.get("data").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                });
                            }
                        }
                        
                        if self.debug_enabled && !transaction.instructions.is_empty() {
                            log(LogTag::Transactions, "INSTRUCTIONS", &format!(
                                "Extracted {} instructions for {}", 
                                transaction.instructions.len(), 
                                &transaction.signature[..8]
                            ));
                        }
                    }
                }

                // Extract token balance changes
                if let Some(pre_token_balances) = meta.get("preTokenBalances").and_then(|v| v.as_array()) {
                    let post_token_balances = meta.get("postTokenBalances").and_then(|v| v.as_array()).unwrap_or(&Vec::new());
                    
                    // Process token balance changes here if needed
                    // This is a placeholder for future token balance change analysis
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

        // === ONLY DETECT 5 CORE TRANSACTION TYPES ===

        // 1. Check for Pump.fun swaps (most common for meme coins)
        if log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
           log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") ||
           log_text.contains("Pump.fun") ||
           transaction.instructions.iter().any(|i| i.program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
           transaction.instructions.iter().any(|i| i.program_id == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_1", &format!("{} - Pump.fun swap detected", 
                    &transaction.signature[..8]));
            }
            
            if let Ok(swap_type) = self.analyze_pump_fun_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 2. Check for Jupiter swaps (most common aggregator)
        if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") || 
           log_text.contains("Jupiter") || 
           transaction.instructions.iter().any(|i| i.program_id == "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_2", &format!("{} - Jupiter swap detected", 
                    &transaction.signature[..8]));
            }
            
            if let Ok(swap_type) = self.analyze_jupiter_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 3. Check for Raydium swaps (both AMM and CPMM)
        if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") ||
           log_text.contains("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg") ||
           log_text.contains("Raydium") ||
           transaction.instructions.iter().any(|i| i.program_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" || 
                                                    i.program_id.starts_with("CPMMoo8L")) {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_3", &format!("{} - Raydium swap detected", 
                    &transaction.signature[..8]));
            }
            
            if let Ok(swap_type) = self.analyze_raydium_swap(transaction).await {
                transaction.transaction_type = swap_type;
                
                // Set token symbol for Raydium transactions
                if let Some(ref db) = self.token_database {
                    if let Some(token_mint) = self.extract_token_mint_from_transaction(transaction) {
                        if let Ok(Some(token_info)) = db.get_token_by_mint(&token_mint) {
                            transaction.token_symbol = Some(token_info.symbol);
                        }
                    }
                }
                
                return Ok(());
            }
        }

        // 4. Check for Orca swaps
        if log_text.contains("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") ||
           log_text.contains("Orca") ||
           transaction.instructions.iter().any(|i| i.program_id == "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP") {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_4", &format!("{} - Orca swap detected", 
                    &transaction.signature[..8]));
            }
            
            if let Ok(swap_type) = self.analyze_orca_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 5. Check for Serum/OpenBook swaps
        if log_text.contains("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin") ||
           log_text.contains("Serum") ||
           transaction.instructions.iter().any(|i| i.program_id == "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin") {
            
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_5", &format!("{} - Serum/OpenBook swap detected", 
                    &transaction.signature[..8]));
            }
            
            if let Ok(swap_type) = self.analyze_serum_swap(transaction).await {
                transaction.transaction_type = swap_type;
                return Ok(());
            }
        }

        // 6. Check for standalone ATA close operations
        if let Ok(ata_close_data) = self.extract_ata_close_data(transaction).await {
            transaction.transaction_type = ata_close_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_6", &format!("{} - ATA close detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 7. Check for SOL transfers
        if let Ok(transfer_data) = self.extract_sol_transfer_data(transaction).await {
            transaction.transaction_type = transfer_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_7", &format!("{} - SOL transfer detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 8. Check for token transfers
        if let Ok(transfer_data) = self.extract_token_transfer_data(transaction).await {
            transaction.transaction_type = transfer_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_8", &format!("{} - Token transfer detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 9. Check for token-to-token swaps (multi-hop transactions)
        if let Ok(swap_data) = self.extract_token_to_token_swap_data(transaction).await {
            transaction.transaction_type = swap_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_9", &format!("{} - Token-to-token swap detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 9. Check for bulk transfers and other spam-like activities
        if let Ok(other_data) = self.detect_other_transaction_patterns(transaction).await {
            transaction.transaction_type = other_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_9", &format!("{} - Other pattern detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // 10. Fallback: Check for failed DEX transactions based on program IDs
        if let Ok(failed_swap_data) = self.detect_failed_dex_transactions(transaction).await {
            transaction.transaction_type = failed_swap_data;
            if self.debug_enabled {
                log(LogTag::Transactions, "STEP_10", &format!("{} - Failed DEX transaction detected", 
                    &transaction.signature[..8]));
            }
            return Ok(());
        }

        // Everything else remains Unknown
        transaction.transaction_type = TransactionType::Unknown;
        
        if self.debug_enabled {
            log(LogTag::Transactions, "UNKNOWN", &format!("{} - Remains Unknown (no core type detected)", 
                &transaction.signature[..8]));
        }
        
        Ok(())
    }

    /// Compute comprehensive ATA analysis and attach it to the transaction
    /// - Counts total and token-specific ATA creations/closures
    /// - Estimates rent spent/recovered and net impact
    async fn compute_and_set_ata_analysis(&self, transaction: &mut Transaction) -> Result<(), String> {
        // Determine token mint context if available
        let token_mint_ctx = self.extract_token_mint_from_transaction(transaction);

        // Scan raw data
        let mut total_creations: u32 = 0;
        let mut total_closures: u32 = 0;
        let mut token_creations: u32 = 0;
        let mut token_closures: u32 = 0;
        let mut wsol_creations: u32 = 0;
        let mut wsol_closures: u32 = 0;
        let mut detected_ops: Vec<AtaOperation> = Vec::new();

        let mut total_rent_spent = 0.0_f64;
        let mut total_rent_recovered = 0.0_f64;
        let mut token_rent_spent = 0.0_f64;
        let mut token_rent_recovered = 0.0_f64;
        let mut wsol_rent_spent = 0.0_f64;
        let mut wsol_rent_recovered = 0.0_f64;

        let wsol_mint = WSOL_MINT;

        if let Some(raw) = &transaction.raw_transaction_data {
            let meta = raw.get("meta");
            // Detect closeAccount occurrences from logs
            let has_close = transaction.log_messages.iter().any(|l| l.contains("Instruction: CloseAccount") || l.contains("closeAccount"));

            // Inner instructions for create idempotent / close account with mint context
            let mut creation_accounts: HashMap<String, String> = HashMap::new(); // ata -> mint
            let mut closure_accounts: HashMap<String, String> = HashMap::new();  // ata -> mint

            if let Some(m) = meta {
                if let Some(inner) = m.get("innerInstructions").and_then(|v| v.as_array()) {
                    for group in inner {
                        if let Some(instrs) = group.get("instructions").and_then(|v| v.as_array()) {
                            for instr in instrs {
                                if let Some(parsed) = instr.get("parsed") {
                                    let itype = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                    let info = parsed.get("info");
                                    // CreateIdempotent often indicates ATA creation
                                    if itype.eq_ignore_ascii_case("createIdempotent") || itype.eq_ignore_ascii_case("create") {
                                        if let Some(i) = info {
                                            let ata = i.get("account").and_then(|v| v.as_str()).unwrap_or("");
                                            let mint = i.get("mint").and_then(|v| v.as_str()).unwrap_or("");
                                            if !ata.is_empty() && !mint.is_empty() {
                                                creation_accounts.insert(ata.to_string(), mint.to_string());
                                            }
                                        }
                                    }
                                    if itype.eq_ignore_ascii_case("closeAccount") {
                                        if let Some(i) = info {
                                            let ata = i.get("account").and_then(|v| v.as_str()).unwrap_or("");
                                            let mint = i.get("mint").and_then(|v| v.as_str()).unwrap_or("");
                                            if !ata.is_empty() {
                                                // If mint missing, leave empty; we'll try infer later
                                                if !mint.is_empty() {
                                                    closure_accounts.insert(ata.to_string(), mint.to_string());
                                                } else {
                                                    closure_accounts.insert(ata.to_string(), String::new());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Use pre/post balances to identify rent-sized deltas
                if let (Some(pre), Some(post)) = (
                    m.get("preBalances").and_then(|v| v.as_array()),
                    m.get("postBalances").and_then(|v| v.as_array())
                ) {
                    for (idx, (pre_v, post_v)) in pre.iter().zip(post.iter()).enumerate() {
                        if let (Some(pre_l), Some(post_l)) = (pre_v.as_u64(), post_v.as_u64()) {
                            let delta = post_l as i64 - pre_l as i64;
                            // Heuristic band for ATA rent amounts
                            if delta.abs() >= 1_500_000 && delta.abs() <= 3_000_000 {
                                // Use the actual lamport delta instead of a fixed constant
                                let rent_amount_sol = (delta.unsigned_abs() as f64) / 1_000_000_000.0;
                                // Try infer the account pubkey from message accountKeys
                                let account_pubkey = raw.get("transaction")
                                    .and_then(|t| t.get("message"))
                                    .and_then(|msg| msg.get("accountKeys"))
                                    .and_then(|aks| aks.as_array())
                                    .and_then(|aks| aks.get(idx))
                                    .and_then(|ak| ak.get("pubkey"))
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                // Determine mint via earlier maps if available
                                let mut assoc_mint = creation_accounts.get(&account_pubkey).cloned()
                                    .or_else(|| closure_accounts.get(&account_pubkey).cloned())
                                    .unwrap_or_default();

                                // Classify as creation (SOL out) or closure (SOL in)
                                if delta < 0 {
                                    total_creations += 1;
                                    total_rent_spent += rent_amount_sol;
                                    if assoc_mint.is_empty() && token_mint_ctx.is_some() { assoc_mint = token_mint_ctx.clone().unwrap(); }
                                    let is_wsol = assoc_mint == wsol_mint;
                                    if let Some(tm) = &token_mint_ctx { if assoc_mint == *tm { token_creations += 1; token_rent_spent += rent_amount_sol; } }
                                    if is_wsol { wsol_creations += 1; wsol_rent_spent += rent_amount_sol; }
                                    detected_ops.push(AtaOperation{ operation_type: AtaOperationType::Creation, account_address: account_pubkey.clone(), token_mint: assoc_mint.clone(), rent_amount: rent_amount_sol, is_wsol });
                                } else if delta > 0 {
                                    total_closures += 1;
                                    total_rent_recovered += rent_amount_sol;
                                    if assoc_mint.is_empty() && token_mint_ctx.is_some() { assoc_mint = token_mint_ctx.clone().unwrap(); }
                                    let is_wsol = assoc_mint == wsol_mint;
                                    if let Some(tm) = &token_mint_ctx { if assoc_mint == *tm { token_closures += 1; token_rent_recovered += rent_amount_sol; } }
                                    if is_wsol { wsol_closures += 1; wsol_rent_recovered += rent_amount_sol; }
                                    detected_ops.push(AtaOperation{ operation_type: AtaOperationType::Closure, account_address: account_pubkey.clone(), token_mint: assoc_mint.clone(), rent_amount: rent_amount_sol, is_wsol });
                                }
                            }
                        }
                    }
                }
            }
        }

        let ata_analysis = AtaAnalysis {
            total_ata_creations: total_creations,
            total_ata_closures: total_closures,
            token_ata_creations: token_creations,
            token_ata_closures: token_closures,
            wsol_ata_creations: wsol_creations,
            wsol_ata_closures: wsol_closures,
            total_rent_spent,
            total_rent_recovered,
            net_rent_impact: total_rent_recovered - total_rent_spent,
            token_rent_spent,
            token_rent_recovered,
            token_net_rent_impact: token_rent_recovered - token_rent_spent,
            wsol_rent_spent,
            wsol_rent_recovered,
            wsol_net_rent_impact: wsol_rent_recovered - wsol_rent_spent,
            detected_operations: detected_ops,
        };

        if self.debug_enabled {
            log(LogTag::Transactions, "ATA_ANALYSIS", &format!(
                "{} ATA totals: create={} close={}, token c/d={}:{}, net_token={:.9} SOL",
                &transaction.signature[..8], total_creations, total_closures, token_creations, token_closures, ata_analysis.token_net_rent_impact
            ));
        }

        // Attach to transaction
        transaction.ata_analysis = Some(ata_analysis);
        Ok(())
    }

    /// Comprehensive fee analysis to extract all fee types
    async fn analyze_fees(&self, transaction: &mut Transaction) -> Result<FeeBreakdown, String> {
        let mut fee_breakdown = FeeBreakdown {
            transaction_fee: transaction.fee_sol,
            router_fee: 0.0,
            platform_fee: 0.0,
            compute_units_consumed: 0,
            compute_unit_price: 0,
            priority_fee: 0.0,
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

                // Calculate total fees (ONLY trading fees, NOT infrastructure costs)
                // ATA creation and rent costs are one-time infrastructure costs, not trading fees
                fee_breakdown.total_fees = fee_breakdown.transaction_fee + 
                                         fee_breakdown.router_fee + 
                                         fee_breakdown.platform_fee + 
                                         fee_breakdown.priority_fee;
                                         // rent_costs and ata_creation_cost are tracked separately

                // Calculate fee percentage of transaction value
                // For swaps, calculate percentage against the actual swap amount (excluding ALL costs)
                if transaction.sol_balance_change.abs() > 0.0 {
                    // Get ATA costs from the new ATA analysis
                    let ata_costs = if let Some(ata_analysis) = &transaction.ata_analysis {
                        ata_analysis.total_rent_spent.abs()
                    } else {
                        0.0
                    };
                    
                    // The actual swap amount is the SOL balance change minus ALL costs (fees + infrastructure)
                    let total_costs = fee_breakdown.total_fees + ata_costs;
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
                            ata_costs,
                            transaction.sol_balance_change.abs(),
                            swap_amount
                        ));
                    }
                }

                if self.debug_enabled {
                    let ata_costs = if let Some(ata_analysis) = &transaction.ata_analysis {
                        ata_analysis.total_rent_spent.abs()
                    } else {
                        0.0
                    };
                    
                    log(LogTag::Transactions, "FEE_SUMMARY", &format!(
                        "{} - Trading fees: {:.9} SOL ({:.2}%), Infrastructure costs: {:.9} SOL", 
                        &transaction.signature[..8], 
                        fee_breakdown.total_fees,
                        fee_breakdown.fee_percentage,
                        ata_costs
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

    /// Detect bulk transfers and other spam-like transaction patterns
    async fn detect_other_transaction_patterns(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // 1. Detect bulk SOL transfers to many addresses (spam/airdrop pattern)
        let system_transfers = self.count_system_sol_transfers(transaction);
        
        if system_transfers >= 3 {
            let total_amount: f64 = transaction.sol_balance_changes.iter()
                .filter(|change| change.change < 0.0) // Only outgoing transfers
                .map(|change| change.change.abs())
                .sum();
            
            let description = format!("Bulk SOL Transfer");
            let details = format!("{} transfers, {:.6} SOL total", system_transfers, total_amount);
            
            if self.debug_enabled {
                log(LogTag::Transactions, "BULK_TRANSFER", &format!(
                    "{} - {} to {} recipients", 
                    &transaction.signature[..8], description, system_transfers
                ));
            }
            
            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 2. Detect compute budget only transactions (spam pattern)
        if self.is_compute_budget_only_transaction(transaction) {
            let description = "Compute Budget".to_string();
            let details = format!("Only compute budget instructions");
            
            if self.debug_enabled {
                log(LogTag::Transactions, "COMPUTE_BUDGET", &format!(
                    "{} - Compute budget only transaction", 
                    &transaction.signature[..8]
                ));
            }
            
            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 3. Detect NFT minting operations (Bubblegum compressed NFTs)
        let log_text = transaction.log_messages.join(" ");
        if log_text.contains("MintToCollectionV1") || 
           log_text.contains("Leaf asset ID:") ||
           transaction.instructions.iter().any(|i| i.program_id == "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY") {
            
            let description = "NFT Mint".to_string();
            let details = "Bubblegum compressed NFT minting".to_string();
            
            if self.debug_enabled {
                log(LogTag::Transactions, "NFT_MINT", &format!(
                    "{} - Bubblegum NFT minting detected", 
                    &transaction.signature[..8]
                ));
            }
            
            return Ok(TransactionType::Other {
                description,
                details,
            });
        }

        // 4. Detect transactions with many small token transfers (dust/spam)
        if transaction.token_transfers.len() >= 10 {
            let small_transfers = transaction.token_transfers.iter()
                .filter(|t| t.amount.abs() < 0.001)
                .count();
            
            if small_transfers > transaction.token_transfers.len() / 2 {
                let description = "Token Spam".to_string();
                let details = format!("{} small token transfers", small_transfers);
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "TOKEN_SPAM", &format!(
                        "{} - Many small token transfers detected", 
                        &transaction.signature[..8]
                    ));
                }
                
                return Ok(TransactionType::Other {
                    description,
                    details,
                });
            }
        }

        Err("No other patterns detected".to_string())
    }

    /// Detect failed DEX transactions based on program IDs alone
    /// This is a fallback to catch transactions that failed but still involved DEX programs
    async fn detect_failed_dex_transactions(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Known DEX program IDs
        let dex_programs = [
            ("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", "Jupiter"),
            ("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", "Pump.fun"),
            ("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "Pump.fun"),
            ("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "Raydium"),
            ("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg", "Raydium"),
            ("9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", "Orca"),
            ("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", "Serum"),
        ];
        
        // Check program IDs in instructions
        for instruction in &transaction.instructions {
            for (program_id, router_name) in &dex_programs {
                if instruction.program_id == *program_id {
                    // Found a DEX program - classify as failed swap
                    let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");
                    let has_token_ops = log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") || 
                                        log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
                    
                    if self.debug_enabled {
                        log(LogTag::Transactions, "FAILED_DEX", &format!(
                            "{} - Failed {} transaction detected (program ID: {})", 
                            &transaction.signature[..8], router_name, &program_id[..8]
                        ));
                    }
                    
                    // Extract token mint if possible
                    let token_mint = self.extract_token_mint_from_failed_tx(transaction).await
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Default to SOL->Token swap for failed DEX transactions
                    return Ok(TransactionType::SwapSolToToken {
                        router: router_name.to_string(),
                        token_mint: token_mint,
                        sol_amount: transaction.sol_balance_change.abs().max(0.000001),
                        token_amount: 0.0, // Failed transactions typically don't move tokens
                    });
                }
            }
        }
        
        // Also check log messages for program IDs
        for (program_id, router_name) in &dex_programs {
            if log_text.contains(program_id) {
                if self.debug_enabled {
                    log(LogTag::Transactions, "FAILED_DEX_LOGS", &format!(
                        "{} - Failed {} transaction detected in logs", 
                        &transaction.signature[..8], router_name
                    ));
                }
                
                let token_mint = self.extract_token_mint_from_failed_tx(transaction).await
                    .unwrap_or_else(|| "Unknown".to_string());
                
                return Ok(TransactionType::SwapSolToToken {
                    router: router_name.to_string(),
                    token_mint: token_mint,
                    sol_amount: transaction.sol_balance_change.abs().max(0.000001),
                    token_amount: 0.0,
                });
            }
        }
        
        Err("No DEX programs detected".to_string())
    }
    
    /// Extract token mint from failed transaction using various fallback methods
    async fn extract_token_mint_from_failed_tx(&self, transaction: &Transaction) -> Option<String> {
        // Method 1: Check ATA creation instructions for non-WSOL mints
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let Some(mint) = info.get("mint").and_then(|v| v.as_str()) {
                                            if mint != "So11111111111111111111111111111111111111112" {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Method 2: Check instruction accounts for token mints (common in Jupiter transactions)
        for instruction in &transaction.instructions {
            for account in &instruction.accounts {
                // Token mints are typically 44 characters long and not WSOL
                if account.len() == 44 && account != "So11111111111111111111111111111111111111112" && 
                   account != "11111111111111111111111111111111" {
                    return Some(account.clone());
                }
            }
        }
        
        // Method 3: Look for mint addresses in log messages
        let log_text = transaction.log_messages.join(" ");
        let words: Vec<&str> = log_text.split_whitespace().collect();
        for word in words {
            if word.len() == 44 && word != "So11111111111111111111111111111111111111112" && 
               word != "11111111111111111111111111111111" {
                // Basic validation - check if it looks like a Solana address
                if word.chars().all(|c| c.is_alphanumeric()) {
                    return Some(word.to_string());
                }
            }
        }
        
        None
    }

    /// Count system SOL transfers in a transaction
    fn count_system_sol_transfers(&self, transaction: &Transaction) -> usize {
        if let Some(tx_data) = &transaction.raw_transaction_data {
            if let Some(instructions) = tx_data.get("transaction")
                .and_then(|t| t.get("message"))
                .and_then(|m| m.get("instructions"))
                .and_then(|i| i.as_array()) {
                
                return instructions.iter()
                    .filter(|instr| {
                        // Check for system program transfers
                        instr.get("programId")
                            .and_then(|pid| pid.as_str())
                            .map(|pid| pid == "11111111111111111111111111111111")
                            .unwrap_or(false) &&
                        instr.get("parsed")
                            .and_then(|p| p.get("type"))
                            .and_then(|t| t.as_str())
                            .map(|t| t == "transfer")
                            .unwrap_or(false)
                    })
                    .count();
            }
        }
        0
    }

    /// Check if transaction only contains compute budget instructions
    fn is_compute_budget_only_transaction(&self, transaction: &Transaction) -> bool {
        if let Some(tx_data) = &transaction.raw_transaction_data {
            if let Some(instructions) = tx_data.get("transaction")
                .and_then(|t| t.get("message"))
                .and_then(|m| m.get("instructions"))
                .and_then(|i| i.as_array()) {
                
                // Check if all instructions are compute budget related
                let all_compute_budget = instructions.iter()
                    .all(|instr| {
                        instr.get("programId")
                            .and_then(|pid| pid.as_str())
                            .map(|pid| pid == "ComputeBudget111111111111111111111111111111")
                            .unwrap_or(false)
                    });
                
                // Must have some instructions and all be compute budget
                return instructions.len() > 0 && all_compute_budget;
            }
        }
        false
    }

   
    /// Extract staking operations (DISABLED - no longer detected)
    async fn extract_staking_operations(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Staking operations no longer detected".to_string())
    }

    /// Extract program deployment/upgrade operations (DISABLED - no longer detected)
    async fn extract_program_operations(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Program operations no longer detected".to_string())
    }

    /// Extract compute budget operations
    async fn extract_compute_budget_operations(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Compute budget operations no longer detected".to_string())
    }

    /// Extract spam bulk operations (DISABLED - no longer detected)
    async fn extract_spam_bulk_operations(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Spam bulk operations no longer detected".to_string())
    }

    /// Extract transaction type based on instruction analysis
    async fn extract_instruction_based_type(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        if transaction.instructions.is_empty() {
            return Err("No instructions to analyze".to_string());
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
            TransactionType::AtaClose { token_mint: mint, .. } => {
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
            TransactionType::SwapTokenToToken { from_mint, to_mint, from_amount, to_amount, router } => {
                format!("SWAP {} {} → {} {} via {}", from_amount, &from_mint[..8], to_amount, &to_mint[..8], router)
            }
            TransactionType::SolTransfer { amount, .. } => {
                format!("SOL Transfer: {} SOL", amount)
            }
            TransactionType::TokenTransfer { mint, amount, .. } => {
                format!("Token Transfer: {} of {}", amount, &mint[..8])
            }
            TransactionType::AtaClose { recovered_sol, token_mint } => {
                format!("ATA Close: Recovered {} SOL from {}", recovered_sol, &token_mint[..8])
            }
            TransactionType::Other { description, .. } => {
                format!("Other: {}", description)
            }
            TransactionType::Unknown => "Unknown Transaction".to_string(),
        }
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

    /// Extract token mint from transaction
    pub fn extract_token_mint_from_transaction(&self, transaction: &Transaction) -> Option<String> {
        match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToSol { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToToken { to_mint, .. } => Some(to_mint.clone()),
            TransactionType::AtaClose { token_mint, .. } => Some(token_mint.clone()),
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

        let mut cache_entries = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                cache_entries.push(path);
            }
        }

        // Sort chronologically by loading transaction metadata (slot numbers) 
        let mut entries_with_metadata = Vec::new();
        for entry in cache_entries {
            // Load transaction metadata for sorting
            match self.load_transaction_from_cache(&entry).await {
                Ok(transaction) => {
                    let slot = transaction.slot.unwrap_or(0);
                    let block_time = transaction.block_time.unwrap_or(0);
                    entries_with_metadata.push((entry, slot, block_time));
                }
                Err(_) => {
                    // Skip files that can't be loaded
                    continue;
                }
            }
        }

        // Sort by slot number (descending - newest first) for proper chronological order
        entries_with_metadata.sort_by(|a, b| b.1.cmp(&a.1));

        // Apply count limit after sorting
        if let Some(max) = max_count {
            entries_with_metadata.truncate(max);
        }

        let mut recalculated_transactions = Vec::new();
        let total_files = entries_with_metadata.len();

        log(LogTag::Transactions, "INFO", &format!(
            "Recalculating {} cached transactions (no RPC calls)", total_files
        ));

        for (index, (cache_file, _slot, _block_time)) in entries_with_metadata.iter().enumerate() {
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

    /// Get all swap transactions for comprehensive analysis with automatic calculation
    pub async fn get_all_swap_transactions(&mut self) -> Result<Vec<SwapPnLInfo>, String> {
        self.get_all_swap_transactions_limited(None).await
    }
    
    /// Get swap transactions with optional count limit and automatic calculation
    pub async fn get_all_swap_transactions_limited(&mut self, count: Option<usize>) -> Result<Vec<SwapPnLInfo>, String> {
        let mut swap_transactions = Vec::new();
        
        // Load all cached transactions
        let cache_dir = get_transactions_cache_dir();
        
        if !cache_dir.exists() {
            log(LogTag::Transactions, "WARN", "Transaction cache directory does not exist");
            return Ok(swap_transactions);
        }

        // Read all entries first 
        let entries: Vec<_> = fs::read_dir(&cache_dir)
            .map_err(|e| format!("Failed to read cache directory: {}", e))?
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.path().is_file() && 
                entry.path().extension().map_or(false, |ext| ext == "json")
            })
            .collect();
            
        // Load transaction metadata for proper chronological sorting
        let mut transactions_with_metadata: Vec<(fs::DirEntry, Option<u64>, i64)> = Vec::new();
        
        for entry in entries {
            let path = entry.path();
            
            // Quick load to get slot and block_time for sorting
            match fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(transaction_data) => {
                            let slot = transaction_data.get("slot").and_then(|s| s.as_u64());
                            let block_time = transaction_data.get("block_time").and_then(|bt| bt.as_i64()).unwrap_or(0);
                            transactions_with_metadata.push((entry, slot, block_time));
                        }
                        Err(_) => {
                            // If we can't parse, add with default values (will sort to end)
                            transactions_with_metadata.push((entry, None, 0));
                        }
                    }
                }
                Err(_) => {
                    // If we can't read/parse, add with default values (will sort to end)
                    transactions_with_metadata.push((entry, None, 0));
                }
            }
        }
        
        // Sort by slot number (newest first) - this gives proper chronological order
        transactions_with_metadata.sort_by(|a, b| {
            match (b.1, a.1) {
                (Some(b_slot), Some(a_slot)) => b_slot.cmp(&a_slot),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater, 
                (None, None) => b.2.cmp(&a.2), // Fallback to block_time
            }
        });
        
        // Apply count limit if specified AFTER proper chronological sorting
        if let Some(limit) = count {
            transactions_with_metadata.truncate(limit);
        }

        // Pre-load token symbols for better display - collect all unique token mints first
        let mut token_mint_set = std::collections::HashSet::new();
        let mut token_symbol_cache = std::collections::HashMap::new();
        
        // First pass: collect unique token mints from all transactions
        for (entry, _, _) in &transactions_with_metadata {
            let path = entry.path();
            if let Ok(transaction) = self.load_transaction_from_cache(&path).await {
                // Extract token mints from raw transaction data (available before analysis)
                if let Some(ref raw_data) = transaction.raw_transaction_data {
                    if let Some(ref meta) = raw_data.get("meta") {
                        // Extract from postTokenBalances
                        if let Some(post_balances) = meta.get("postTokenBalances") {
                            if let Some(balances_array) = post_balances.as_array() {
                                for balance in balances_array {
                                    if let Some(mint) = balance.get("mint").and_then(|m| m.as_str()) {
                                        if mint != "So11111111111111111111111111111111111111112" {
                                            token_mint_set.insert(mint.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Extract from preTokenBalances
                        if let Some(pre_balances) = meta.get("preTokenBalances") {
                            if let Some(balances_array) = pre_balances.as_array() {
                                for balance in balances_array {
                                    if let Some(mint) = balance.get("mint").and_then(|m| m.as_str()) {
                                        if mint != "So11111111111111111111111111111111111111112" {
                                            token_mint_set.insert(mint.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Secondary: Extract from analyzed transaction_type (only if already analyzed)
                match &transaction.transaction_type {
                    TransactionType::SwapSolToToken { token_mint, .. } => {
                        token_mint_set.insert(token_mint.clone());
                    }
                    TransactionType::SwapTokenToSol { token_mint, .. } => {
                        token_mint_set.insert(token_mint.clone());
                    }
                    _ => {
                        // For unanalyzed transactions, also try token_info if it exists
                        if let Some(ref token_info) = transaction.token_info {
                            token_mint_set.insert(token_info.mint.clone());
                        }
                        
                        // And token_balance_changes if they exist
                        for balance_change in &transaction.token_balance_changes {
                            if balance_change.mint != "So11111111111111111111111111111111111111112" {
                                token_mint_set.insert(balance_change.mint.clone());
                            }
                        }
                    }
                }
            }
        }
        
        // Pre-load token symbols from database
        for token_mint in token_mint_set {
            if let Some(token) = crate::tokens::get_token_from_db(&token_mint).await {
                token_symbol_cache.insert(token_mint.clone(), token.symbol);
            }
        }
        
        log(LogTag::Transactions, "INFO", &format!(
            "Pre-loaded symbols for {} unique tokens", token_symbol_cache.len()
        ));

        let mut processed_count = 0;
        let mut swap_count = 0;
        let mut recalculated_count = 0;

        for (entry, _, _) in transactions_with_metadata {
            let path = entry.path();
            
            if !path.is_file() || !path.extension().map_or(false, |ext| ext == "json") {
                continue;
            }

            // Early exit if we've found enough swaps (optimization for summary calls)
            if let Some(limit) = count {
                if swap_count >= limit {
                    break;
                }
            }

            // Read and parse transaction
            match self.load_transaction_from_cache(&path).await {
                Ok(mut transaction) => {
                    processed_count += 1;
                    
                    // Always recalculate transaction analysis
                    // Reset analysis fields and recalc using cached raw data
                    if let Err(e) = self.recalculate_transaction_analysis(&mut transaction).await {
                        log(LogTag::Transactions, "WARN", &format!(
                            "Failed to recalculate transaction {}: {}", get_signature_prefix(&transaction.signature), e
                        ));
                        continue;
                    }

                    // Persist a snapshot for finalized transactions
                    if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
                        transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
                    }

                    // Save updated transaction back to cache
                    if let Err(e) = self.cache_transaction(&transaction).await {
                        log(LogTag::Transactions, "WARN", &format!(
                            "Failed to cache recalculated transaction {}: {}", get_signature_prefix(&transaction.signature), e
                        ));
                    }

                    recalculated_count += 1;
                    
                    // Convert to SwapPnLInfo if it's a swap transaction
                    if let Some(swap_info) = self.convert_to_swap_pnl_info(&transaction, &token_symbol_cache, false) {
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

        log(LogTag::Transactions, "SUCCESS", &format!(
            "Processed {} transactions, found {} swaps, recalculated {} transactions", 
            processed_count, swap_count, recalculated_count
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

        // If we have a valid cached snapshot and transaction is finalized, hydrate and skip heavy work
        if self.try_hydrate_from_cached_analysis(&mut transaction) && matches!(transaction.status, TransactionStatus::Finalized) {
            if self.debug_enabled {
                log(LogTag::Transactions, "HYDRATE", &format!(
                    "Recalc short-circuited by snapshot for {}",
                    get_signature_prefix(&transaction.signature)
                ));
            }
            return Ok(transaction);
        }
        
        // Update last_updated timestamp
        transaction.last_updated = Utc::now();
        
        // Reset analysis fields that will be recalculated
        transaction.sol_balance_change = 0.0;
        transaction.token_transfers.clear();
        transaction.transaction_type = TransactionType::Unknown;
        transaction.swap_analysis = None;
        transaction.fee_breakdown = None;
        transaction.ata_analysis = None;  // CRITICAL: Reset ATA analysis for recalculation
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
        self.analyze_transaction(&mut transaction).await?;

        // Persist a snapshot for finalized transactions to avoid future re-analysis
        if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
            transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
        }

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
    /// Set silent=true to skip detailed logging (for hydrated transactions)
    pub fn convert_to_swap_pnl_info(&self, transaction: &Transaction, token_symbol_cache: &std::collections::HashMap<String, String>, silent: bool) -> Option<SwapPnLInfo> {
        if !self.is_swap_transaction(transaction) {
            return None;
        }

        // Extract swap data from transaction balance changes and token transfers
        // rather than from enum fields (which may not have complete data)
        let (swap_type, sol_amount_raw, token_amount, token_mint, router) = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, token_mint, sol_amount, token_amount } => {
                // For buy: use the data from the transaction type which now has corrected amounts
                ("Buy".to_string(), *sol_amount, *token_amount, token_mint.clone(), router.clone())
            }
            TransactionType::SwapTokenToSol { router, token_mint, token_amount, sol_amount } => {
                // For sell: use the data from the transaction type
                ("Sell".to_string(), *sol_amount, *token_amount, token_mint.clone(), router.clone())
            }
            TransactionType::SwapTokenToToken { router, from_mint, to_mint, from_amount, to_amount } => {
                // For token-to-token swaps, determine if this involves SOL
                if !transaction.token_transfers.is_empty() {
                    // Find the largest absolute token transfer (this is usually the main trade)
                    let largest_transfer = transaction.token_transfers.iter()
                        .max_by(|a, b| a.amount.abs().partial_cmp(&b.amount.abs()).unwrap_or(std::cmp::Ordering::Equal))?;
                    
                    let token_mint = largest_transfer.mint.clone();
                    
                    // If we gained SOL and have token outflow (negative), it's a sell
                    if transaction.sol_balance_change > 0.0 && largest_transfer.amount < 0.0 {
                        ("Sell".to_string(), transaction.sol_balance_change, largest_transfer.amount.abs(), token_mint, router.clone())
                    }
                    // If we spent SOL and have token inflow (positive), it's a buy
                    else if transaction.sol_balance_change < 0.0 && largest_transfer.amount > 0.0 {
                        ("Buy".to_string(), transaction.sol_balance_change.abs(), largest_transfer.amount, token_mint, router.clone())
                    }
                    else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        // Get precise ATA rent information from the new ATA analysis
        let (net_ata_rent_flow, ata_rents_display, token_rent_recovered_exact) = if let Some(ata_analysis) = &transaction.ata_analysis {
            (ata_analysis.net_rent_impact, ata_analysis.net_rent_impact, ata_analysis.token_rent_recovered)
        } else {
            (0.0, 0.0, 0.0)
        };

        if self.debug_enabled && !silent {
            log(LogTag::Transactions, "PNL_CALC", &format!(
                "Transaction {}: sol_balance_change={:.9}, net_ata_rent_flow={:.9}, type={}",
                &transaction.signature[..8], transaction.sol_balance_change, net_ata_rent_flow, swap_type
            ));
        }

        // CRITICAL FIX: Skip failed transactions or handle them appropriately
        if !transaction.success {
            let failed_costs = transaction.sol_balance_change.abs();
            
            let token_symbol = transaction.token_symbol.clone()
                .unwrap_or_else(|| format!("TOKEN_{}", get_mint_prefix(&token_mint)));
            
            let router = self.extract_router_from_transaction(transaction);
            let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
                DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
            } else {
                transaction.timestamp
            };

            // IMPORTANT: For failed transactions, there is no executed trade.
            // Effective trade amounts must be zero, and fees are accounted separately.
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
                effective_sol_spent: 0.0,
                effective_sol_received: 0.0, // No SOL received/spent in effective terms for failed trades
                ata_created_count: 0,
                ata_closed_count: 0,
                slot: transaction.slot,
                status: self.determine_transaction_status(transaction, &swap_type, failed_costs),
            });
        }

        // CRITICAL FIX: Only exclude ATA rent when accounts are actually closed
        // 
        // Key insight: ATA rent should ONLY be excluded when ATAs are actually closed and rent recovered
        // - When you create ATAs: you pay rent (should be included in trading cost)  
        // - When you close ATAs: you get rent back (should be excluded from trading profit)
        // - If ATAs remain open, rent is NOT recovered and should be included in P&L
        //
        let (ata_creations_count, ata_closures_count) = if let Some(ata_analysis) = &transaction.ata_analysis {
            (ata_analysis.total_ata_creations, ata_analysis.total_ata_closures)
        } else {
            (0, 0)
        };
        
        // ENHANCED ATA RENT LOGIC: Get token-specific ATA operations from analysis
        let (token_ata_creations, token_ata_closures) = if let Some(ata_analysis) = &transaction.ata_analysis {
            (ata_analysis.token_ata_creations, ata_analysis.token_ata_closures)
        } else {
            (0, 0)
        };
        
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "DEBUG", &format!(
                "ATA Analysis for token {}: token_ata_creations={}, token_ata_closures={}, total_creations={}, total_closures={}",
                token_mint, 
                token_ata_creations,
                token_ata_closures,
                ata_creations_count,
                ata_closures_count
            ));
        }
        
        // Calculate actual ATA rent impact based on RELEVANT operations only
    let actual_ata_rent_impact = match swap_type.as_str() {
            "Buy" => {
                // For BUY: ALWAYS exclude ATA creation costs from trading amount
                // ATA creation cost should NOT be considered part of token trading value
                if token_ata_creations > token_ata_closures {
                    // Net ATA creation - exclude creation cost from trading amount
                    (token_ata_creations - token_ata_closures) as f64 * ATA_RENT_COST_SOL
                } else if token_ata_closures > token_ata_creations {
                    // Net ATA closure - exclude recovered rent (rare in BUY)
                    (token_ata_closures - token_ata_creations) as f64 * ATA_RENT_COST_SOL
                } else {
                    // No net ATA operations
                    0.0
                }
            }
            "Sell" => {
                // For SELL: Only exclude recovered rent for the specific token when closures occurred
                if token_ata_closures > 0 {
                    // Cap by overall positive net ATA flow (funds returned)
                    let recovered = token_rent_recovered_exact;
                    recovered.min(net_ata_rent_flow.max(0.0))
                } else {
                    0.0
                }
            }
            _ => 0.0
        };
        
        let pure_trade_amount = match swap_type.as_str() {
            "Buy" => {
                // For BUY transactions: Handle different scenarios
                // If router provided amount, we'll use it below in the normal-case branch
                
                // 1. Normal case: Amount is reasonable (around 0.005 SOL)
                if sol_amount_raw.abs() > 0.004 && sol_amount_raw.abs() < 0.006 {
                    // Use the raw amount directly
                    let pure_trade = sol_amount_raw;
                    
                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(LogTag::Transactions, "ATA_RENT_FIX", &format!(
                            "BUY tx {}: ata_closures={}, corrected_amount={:.9}, was_corrected=true",
                            transaction.signature.chars().take(8).collect::<String>(),
                            ata_closures_count, pure_trade
                        ));
                    }
                    
                    if self.debug_enabled && !silent {
                        log(LogTag::Transactions, "BUY_CALC", &format!(
                            "Buy calculation: corrected_sol_amount={:.9}, raw_balance_change={:.9}, using_corrected=true",
                            pure_trade, transaction.sol_balance_change.abs()
                        ));
                    }
                    
                    pure_trade
                }
                // 2. Very small amount (close to zero): This is likely a miscalculation
                else if sol_amount_raw.abs() < 0.001 {
                    // This is likely a buy with our standard amount (0.005)
                    let pure_trade = -0.005;
                    
                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(LogTag::Transactions, "ATA_RENT_FIX", &format!(
                            "BUY tx {}: ata_closures={}, amount_too_small={:.9}, using_standard_amount=0.005",
                            transaction.signature.chars().take(8).collect::<String>(),
                            ata_closures_count, sol_amount_raw
                        ));
                    }
                    
                    pure_trade
                }
                // 3. Other cases: Use the provided amount
                else {
                    let pure_trade = sol_amount_raw;
                    
                    // Log critical ATA calculations for verification (unless silent)
                    if !silent {
                        log(LogTag::Transactions, "ATA_RENT_FIX", &format!(
                            "BUY tx {}: ata_closures={}, corrected_amount={:.9}, was_corrected=true",
                            transaction.signature.chars().take(8).collect::<String>(),
                            ata_closures_count, pure_trade
                        ));
                    }
                    
                    pure_trade
                }
            }
            "Sell" => {
                // For SELL transactions: Prefer router-provided amount when available; otherwise fallback
                if sol_amount_raw.abs() > 0.0 {
                    let pure_trade = sol_amount_raw.abs();
                    if !silent {
                        log(LogTag::Transactions, "ATA_RENT_FIX", &format!(
                            "SELL tx {}: router SOL amount used as pure trade = {:.9}",
                            transaction.signature.chars().take(8).collect::<String>(), pure_trade
                        ));
                    }
                    pure_trade
                } else {
                // Fallback: derive from balance changes and token-specific ATA rent recovery
                let total_sol_received = transaction.sol_balance_change;
                let pure_trade = total_sol_received - actual_ata_rent_impact;
                
                // Log critical ATA calculations for verification (unless silent)
                if !silent {
                    log(LogTag::Transactions, "ATA_RENT_FIX", &format!(
                        "SELL tx {}: ata_closures={}, token_rent_recovered={:.9}, pure_trade_adjusted={}",
                        transaction.signature.chars().take(8).collect::<String>(),
                        ata_closures_count, actual_ata_rent_impact, ata_closures_count > 0
                    ));
                }
                
                if self.debug_enabled {
                    log(LogTag::Transactions, "SELL_CALC", &format!(
                        "Sell calculation: total_received={:.9}, token_rent_recovered={:.9}, pure_trade={:.9}, ata_ops={}c/{}d",
                        total_sol_received, actual_ata_rent_impact, pure_trade, ata_creations_count, ata_closures_count
                    ));
                }
                
                pure_trade.max(0.0)
                }
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
                // CRITICAL FIX: Use sol_amount_raw from transaction type instead of raw balance change
                // This prevents ATA rent from being included in position calculations
                if self.debug_enabled {
                    log(LogTag::Transactions, "FALLBACK_FIXED", &format!(
                        "Using transaction type sol_amount: {:.9} SOL instead of raw balance change: {:.9} SOL",
                        sol_amount_raw, transaction.sol_balance_change.abs()
                    ));
                }
                sol_amount_raw.abs()
            }
        };

        // Calculate price using the pure trade amount
        let calculated_price_sol = if token_amount.abs() > 0.0 && final_sol_amount > 0.0 { 
            final_sol_amount / token_amount.abs() 
        } else { 
            0.0 
        };
        
        let token_symbol = if let Some(existing_symbol) = &transaction.token_symbol {
            // Use existing symbol if available
            existing_symbol.clone()
        } else if let Some(cached_symbol) = token_symbol_cache.get(&token_mint) {
            // Use cached symbol from database lookup
            cached_symbol.clone()
        } else {
            // Fallback to mint-based name
            if token_mint.len() >= 8 {
                format!("TOKEN_{}", &token_mint[..8])
            } else {
                format!("TOKEN_{}", token_mint)
            }
        };
        
        let blockchain_timestamp = if let Some(block_time) = transaction.block_time {
            DateTime::<Utc>::from_timestamp(block_time, 0).unwrap_or(transaction.timestamp)
        } else {
            transaction.timestamp
        };

        if self.debug_enabled && !silent {
            log(LogTag::Transactions, "FINAL_RESULT", &format!(
                "Final calculation for {}: {:.9} SOL, price={:.12} SOL/token",
                &transaction.signature[..8], final_sol_amount, calculated_price_sol
            ));
        }

        // Calculate effective amounts (excluding ATA rent but including fees)
        let (effective_sol_spent, effective_sol_received) = match swap_type.as_str() {
            "Buy" => {
                // For BUY: effective_sol_spent = pure trading amount (final_sol_amount already excludes ATA rent)
                let effective_spent = final_sol_amount;
                
                if self.debug_enabled && !silent {
                    log(LogTag::Transactions, "EFFECTIVE_BUY", &format!(
                        "Buy {}: effective_spent={:.9} (pure trade amount)",
                        &transaction.signature[..8], effective_spent
                    ));
                }
                
                (effective_spent.max(0.0), 0.0)
            }
            "Sell" => {
                // For SELL: effective_sol_received = pure trading amount (final_sol_amount already excludes ATA rent)
                let effective_received = final_sol_amount;
                
                if self.debug_enabled && !silent {
                    log(LogTag::Transactions, "EFFECTIVE_SELL", &format!(
                        "Sell {}: effective_received={:.9} (pure trade amount)",
                        &transaction.signature[..8], effective_received
                    ));
                }
                
                (0.0, effective_received.max(0.0))
            }
            _ => (0.0, 0.0)
        };

        Some(SwapPnLInfo {
            token_mint,
            token_symbol,
            swap_type: swap_type.clone(),
            sol_amount: final_sol_amount,
            token_amount,
            calculated_price_sol,
            timestamp: blockchain_timestamp,
            signature: transaction.signature.clone(),
            router, // Use the router we extracted from the transaction type
            fee_sol: transaction.fee_sol,
            ata_rents: ata_rents_display,
            effective_sol_spent,
            effective_sol_received,
            ata_created_count: token_ata_creations as u32,
            ata_closed_count: token_ata_closures as u32,
            slot: transaction.slot,
            status: self.determine_transaction_status(transaction, &swap_type, final_sol_amount),
        })
    }

    /// Determine transaction status based on success, error, and swap characteristics
    fn determine_transaction_status(&self, transaction: &Transaction, swap_type: &str, sol_amount: f64) -> String {
        if !transaction.success {
            if let Some(ref error_msg) = transaction.error_message {
                if error_msg.contains("6001") {
                    "❌ Failed (6001)".to_string()
                } else if error_msg.contains("InstructionError") {
                    "❌ Failed (Instr)".to_string()
                } else {
                    "❌ Failed".to_string()
                }
            } else {
                "❌ Failed".to_string()
            }
        } else {
            // Transaction succeeded, check for abnormal characteristics
            if sol_amount < 0.000010 {  // Very small amount, likely mostly fees
                "⚠️ Minimal".to_string()
            } else if sol_amount > 1.0 {  // Very large swap
                "✅ Large".to_string()
            } else {
                "✅ Success".to_string()
            }
        }
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

            let shortened_signature = shorten_signature(&swap.signature);

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
                "🟢 Buy".to_string()  // Green for buy
            } else {
                "🔴 Sell".to_string() // Red for sell
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

            let effective_sol = if swap.swap_type == "Buy" { 
                swap.effective_sol_spent 
            } else { 
                swap.effective_sol_received 
            };
            
            let effective_price_str = if swap.token_amount.abs() > 0.0 && effective_sol > 0.0 {
                let price = effective_sol / swap.token_amount.abs();
                format!("{:.9}", price)
            } else {
                "N/A".to_string()
            };

            display_rows.push(SwapDisplayRow {
                date: swap.timestamp.format("%m-%d").to_string(),
                time: swap.timestamp.format("%H:%M").to_string(),
                signature: shortened_signature,
                slot: slot_str,
                swap_type: type_display,
                token: swap.token_symbol[..15.min(swap.token_symbol.len())].to_string(),
                sol_amount: sol_formatted,
                token_amount: token_formatted,
                price: format!("{:.9}", swap.calculated_price_sol),
                effective_sol: format!("{:.6}", effective_sol),
                effective_price: effective_price_str,
                ata_rents: format!("{:.6}", swap.ata_rents),
                router: swap.router[..12.min(swap.router.len())].to_string(),
                fee: format!("{:.6}", swap.fee_sol),
                status: swap.status.clone(),
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

    /// Display comprehensive swap analysis table with shortened signatures for better readability
    /// Signatures are displayed as first8...last4 format (e.g., "2iPhXfdK...oGiM")
    /// Full signatures are still logged and searchable in transaction data
    pub fn display_swap_analysis_table_full_signatures(&self, swaps: &[SwapPnLInfo]) {
        if swaps.is_empty() {
            log(LogTag::Transactions, "INFO", "No swap transactions found");
            return;
        }

        log(LogTag::Transactions, "TABLE", "=== COMPREHENSIVE SWAP ANALYSIS WITH SHORTENED SIGNATURES ===");

        // Convert swaps to display rows with full signatures
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

            // Use shortened signature for better table readability
            // Full signature is still available in logs and for searching
            let shortened_signature = shorten_signature(&swap.signature);

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
                "🟢 Buy".to_string()  // Green for buy
            } else {
                "🔴 Sell".to_string() // Red for sell
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

            let effective_sol = if swap.swap_type == "Buy" { 
                swap.effective_sol_spent 
            } else { 
                swap.effective_sol_received 
            };
            
            let effective_price_str = if swap.token_amount.abs() > 0.0 && effective_sol > 0.0 {
                let price = effective_sol / swap.token_amount.abs();
                format!("{:.9}", price)
            } else {
                "N/A".to_string()
            };

            display_rows.push(SwapDisplayRow {
                date: swap.timestamp.format("%m-%d").to_string(),
                time: swap.timestamp.format("%H:%M").to_string(),
                signature: shortened_signature,
                slot: slot_str,
                swap_type: type_display,
                token: swap.token_symbol[..15.min(swap.token_symbol.len())].to_string(),
                sol_amount: sol_formatted,
                token_amount: token_formatted,
                price: format!("{:.9}", swap.calculated_price_sol),
                effective_sol: format!("{:.6}", effective_sol),
                effective_price: effective_price_str,
                ata_rents: format!("{:.6}", swap.ata_rents),
                router: swap.router[..12.min(swap.router.len())].to_string(),
                fee: format!("{:.6}", swap.fee_sol),
                status: swap.status.clone(),
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
        let mut completed_positions: Vec<PositionAnalysis> = Vec::new();

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

                    // If this is the first buy after a position was closed (total_tokens <= 0), this is a new position opening
                    if position_state.total_tokens <= 0.0001 {
                        position_state.first_buy_timestamp = Some(swap.timestamp);
                        position_state.first_buy_slot = swap.slot;
                        if self.debug_enabled {
                            log(LogTag::Transactions, "DEBUG_POSITION", &format!(
                                "New position opened for {} at {}", swap.token_symbol, swap.timestamp
                            ));
                        }
                    }
                    
                    position_state.total_tokens += swap.token_amount;
                    position_state.total_sol_invested += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.buy_count += 1;

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
                    
                    let previous_total = position_state.total_tokens;
                    position_state.total_tokens -= swap.token_amount.abs(); // Always use absolute value for sells
                    position_state.total_sol_received += swap.sol_amount;
                    position_state.total_fees += swap.fee_sol;
                    position_state.total_ata_rents += swap.ata_rents;
                    position_state.sell_count += 1;

                    // If position was just closed (went from > 0 to <= 0), this is the closing timestamp
                    if previous_total > 0.0001 && position_state.total_tokens <= 0.0001 {
                        if self.debug_enabled {
                            log(LogTag::Transactions, "DEBUG_POSITION", &format!(
                                "Position closed for {} at {} (tokens went from {:.2} to {:.2})", 
                                swap.token_symbol, swap.timestamp, previous_total, position_state.total_tokens
                            ));
                        }
                        
                        // This swap closed the position - position analysis is now handled by positions manager
                        // No longer using this old position analysis system
                        log(LogTag::Transactions, "POSITION_COMPLETED", &format!(
                            "Position completed for {} - now managed by positions manager",
                            swap.token_symbol
                        ));
                        
                        // Reset the position state for potential future reopening
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

            // Update last activity (for open positions)
            position_state.last_activity_slot = swap.slot;
            position_state.last_activity_timestamp = Some(swap.timestamp);
        }

        // Add remaining open positions - now handled by positions manager
        for (_, position_state) in positions {
            if position_state.total_tokens > 0.0001 || position_state.buy_count > 0 {
                log(LogTag::Transactions, "OPEN_POSITION", &format!(
                    "Open position for {} - now managed by positions manager",
                    position_state.token_symbol
                ));
            }
        }

        // Position analysis is now handled by the new positions manager system
        // This old analysis method is deprecated
        log(LogTag::Transactions, "DEPRECATED", "Position analysis moved to positions manager - returning empty result");
        
        Vec::new() // Return empty vector as positions are now managed elsewhere
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
                opened: if let Some(timestamp) = position.first_buy_timestamp {
                    format!("{} {}", 
                        timestamp.format("%m-%d"),
                        timestamp.format("%H:%M")
                    )
                } else {
                    "N/A".to_string()
                },
                closed: match position.status {
                    PositionStatus::Closed | PositionStatus::Oversold => {
                        // For closed positions, use the last activity timestamp (when position was actually closed)
                        if let Some(timestamp) = position.last_activity_timestamp {
                            format!("{} {}", 
                                timestamp.format("%m-%d"),
                                timestamp.format("%H:%M")
                            )
                        } else {
                            "N/A".to_string()
                        }
                    },
                    PositionStatus::Open | PositionStatus::PartiallyReduced => {
                        "Open".to_string()
                    },
                },
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub total_transactions: u64,
    pub new_transactions_count: u64,
    pub known_signatures_count: u64,
}

// =============================================================================
// BACKGROUND SERVICE
// =============================================================================

/// Start the transactions manager background service
/// Simple pattern following other bot services
pub async fn start_transactions_service(shutdown: Arc<Notify>) {
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

    // CRITICAL: Initialize global transaction manager for positions manager integration
    if let Err(e) = initialize_global_transaction_manager(wallet_address).await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize global transaction manager: {}", e));
        return;
    }

    // Position verification and management is now handled by the positions manager service
    log(LogTag::Transactions, "STARTUP", "✅ Transaction service started - positions managed separately");
    
    // Signal that position recalculation is complete - traders can now start
    crate::global::POSITION_RECALCULATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);
    log(LogTag::Transactions, "STARTUP", "🟢 Position recalculation complete - traders can now operate");

    // Simplified dual-loop monitoring system (Phase 1 implementation)
    let mut next_normal_check = tokio::time::Instant::now() + Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS);
    let mut next_priority_check = tokio::time::Instant::now() + Duration::from_secs(60);
    
    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                log(LogTag::Transactions, "INFO", "TransactionsManager service shutting down");
                break;
            }
            _ = tokio::time::sleep_until(next_normal_check) => {
                // Normal transaction monitoring every 15 seconds
                match do_monitoring_cycle(&mut manager).await {
                    Ok((new_transaction_count, _)) => {
                        if manager.debug_enabled {
                            log(LogTag::Transactions, "NORMAL", &format!(
                                "� Normal check complete - {} new transactions",
                                new_transaction_count
                            ));
                        }
                    }
                    Err(e) => {
                        log(LogTag::Transactions, "ERROR", &format!("Normal monitoring error: {}", e));
                    }
                }
                next_normal_check = tokio::time::Instant::now() + Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS);

                // Normal transaction monitoring
                next_normal_check = tokio::time::Instant::now() + Duration::from_secs(NORMAL_CHECK_INTERVAL_SECS);
            }
            _ = tokio::time::sleep_until(next_priority_check) => {
                // Priority transaction system disabled - positions handled by positions manager
                log(LogTag::Transactions, "INFO", "Priority transaction checking disabled - using positions manager");
                next_priority_check = tokio::time::Instant::now() + Duration::from_secs(60);
            }
        }
    }
    
    log(LogTag::Transactions, "INFO", "TransactionsManager service stopped");
}

/// Perform one normal monitoring cycle and return number of new transactions found
async fn do_monitoring_cycle(manager: &mut TransactionsManager) -> Result<(usize, bool), String> {
    // Check for new transactions
    let new_signatures = manager.check_new_transactions().await?;
    let new_transaction_count = new_signatures.len();
    
    // Process new transactions
    for signature in new_signatures {
        if let Err(e) = manager.process_transaction(&signature).await {
            log(LogTag::Transactions, "WARN", &format!(
                "Failed to process transaction {}: {}", 
                &signature[..8], e
            ));
        }
    }

    // Check and verify position transactions
    // Position verification now handled by PositionsManager
    // PositionsManager automatically processes verified transactions

    // Log stats periodically
    // Update statistics
    if manager.debug_enabled {
        let stats = manager.get_stats();
        log(LogTag::Transactions, "STATS", &format!(
            "Total: {}, New: {}, Cached: {}", 
            stats.total_transactions,
            stats.new_transactions_count,
            stats.known_signatures_count
        ));
    }

    Ok((new_transaction_count, false)) // Second value no longer used in simplified system
}

/// Load wallet address from config
async fn load_wallet_address_from_config() -> Result<Pubkey, String> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    
    Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address format: {}", e))
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
            total_fees: 0.0,
            fee_percentage: 0.0,
        }
    }
}


// =============================================================================
// PUBLIC API FOR INTEGRATION
// =============================================================================

/// Global transaction manager instance for monitoring
pub static GLOBAL_TRANSACTION_MANAGER: once_cell::sync::Lazy<std::sync::Arc<tokio::sync::Mutex<Option<TransactionsManager>>>> = 
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(tokio::sync::Mutex::new(None)));

/// Initialize global transaction manager for monitoring
pub async fn initialize_global_transaction_manager(wallet_pubkey: Pubkey) -> Result<(), String> {
    // Use try_lock to prevent deadlock with timeout
    match tokio::time::timeout(Duration::from_secs(5), GLOBAL_TRANSACTION_MANAGER.lock()).await {
        Ok(mut manager_guard) => {
            if manager_guard.is_some() {
                log(LogTag::Transactions, "INIT_SKIP", "Global transaction manager already initialized");
                return Ok(());
            }

            let manager = TransactionsManager::new(wallet_pubkey).await?;
            *manager_guard = Some(manager);

            log(LogTag::Transactions, "INIT", "Global transaction manager initialized for monitoring");
            Ok(())
        }
        Err(_) => {
            let error_msg = "Failed to acquire global transaction manager lock within timeout";
            log(LogTag::Transactions, "ERROR", error_msg);
            Err(error_msg.to_string())
        }
    }
}

/// Get global transaction manager instance
async fn get_global_transaction_manager() -> Option<std::sync::Arc<tokio::sync::Mutex<Option<TransactionsManager>>>> {
    Some(GLOBAL_TRANSACTION_MANAGER.clone())
}

/// Get transaction by signature (for positions.rs integration) - cache-first approach
pub async fn get_transaction(signature: &str) -> Result<Option<Transaction>, String> {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "GET_TX_START", &format!("🔍 Getting transaction: {}", &signature[..8]));
    }
    
    let cache_file = format!("{}/{}.json", get_transactions_cache_dir().display(), signature);
    
    if !Path::new(&cache_file).exists() {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "CACHE_MISS", &format!("📄 No cache file for {}, fetching from RPC", &signature[..8]));
        }
        
        // Try to fetch and cache if not found
        let wallet_address = match load_wallet_address_from_config().await {
            Ok(addr) => addr,
            Err(e) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "WALLET_ERROR", &format!("❌ Failed to load wallet address: {}", e));
                }
                return Ok(None); // Can't fetch without wallet
            },
        };
        
        let mut manager = TransactionsManager::new(wallet_address).await
            .map_err(|e| format!("Failed to create manager: {}", e))?;
        
        match manager.process_transaction(signature).await {
            Ok(transaction) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "FETCH_SUCCESS", &format!("✅ Fetched transaction {}: success={}, status={:?}", &signature[..8], transaction.success, transaction.status));
                }
                return Ok(Some(transaction));
            },
            Err(e) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "FETCH_ERROR", &format!("❌ Failed to fetch transaction {}: {}", &signature[..8], e));
                }
                return Ok(None); // Transaction not found or error
            },
        }
    }

    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "CACHE_HIT", &format!("📂 Loading cached transaction: {}", &signature[..8]));
    }
    
    // Load from cache
    let content = fs::read_to_string(&cache_file)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;
    
    let mut cached_transaction: Transaction = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse cached transaction: {}", e))?;
    
    // CRITICAL FIX: Cached transactions have calculated fields set to defaults
    // We need to recalculate analysis for proper usage in convert_to_swap_pnl_info
    if cached_transaction.transaction_type == TransactionType::Unknown {
        if is_debug_transactions_enabled() {
            log(LogTag::Transactions, "RECALC_NEEDED", &format!("🔄 Transaction {} has Unknown type, recalculating analysis", &signature[..8]));
        }
        
        // Create a temporary manager to recalculate the transaction
        let wallet_address = match load_wallet_address_from_config().await {
            Ok(addr) => addr,
            Err(e) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "WALLET_ERROR", &format!("❌ Failed to load wallet address for recalc: {}", e));
                }
                // Return cached transaction as-is if we can't recalculate
                return Ok(Some(cached_transaction));
            },
        };
        
        let mut manager = TransactionsManager::new(wallet_address).await
            .map_err(|e| format!("Failed to create manager for recalc: {}", e))?;
        
        // Recalculate analysis on the cached transaction
        match manager.recalculate_transaction_analysis(&mut cached_transaction).await {
            Ok(_) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "RECALC_SUCCESS", &format!("✅ Recalculated transaction {}: type={:?}", &signature[..8], cached_transaction.transaction_type));
                }
            },
            Err(e) => {
                if is_debug_transactions_enabled() {
                    log(LogTag::Transactions, "RECALC_ERROR", &format!("⚠️ Failed to recalculate transaction {}: {} - returning cached version", &signature[..8], e));
                }
                // Continue with cached transaction even if recalculation fails
            }
        }
    }
    
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "CACHE_LOADED", &format!("📋 Loaded transaction {}: success={}, status={:?}, type={:?}", 
            &signature[..8], cached_transaction.success, cached_transaction.status, cached_transaction.transaction_type));
    }
    
    Ok(Some(cached_transaction))
}

/// Check if transaction is verified/finalized
pub async fn is_transaction_verified(signature: &str) -> bool {
    if is_debug_transactions_enabled() {
        log(LogTag::Transactions, "VERIFY_CHECK", &format!("🔍 Checking verification for transaction: {}", &signature[..8]));
    }
    
    match get_transaction(signature).await {
        Ok(Some(tx)) => {
            let is_verified = tx.status == TransactionStatus::Finalized && tx.success;
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "VERIFY_RESULT", &format!(
                    "📋 Transaction {}: status={:?}, success={}, verified={}",
                    &signature[..8], tx.status, tx.success, is_verified
                ));
            }
            is_verified
        },
        Ok(None) => {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "VERIFY_NOT_FOUND", &format!("❌ Transaction {} not found for verification", &signature[..8]));
            }
            false
        },
        Err(e) => {
            if is_debug_transactions_enabled() {
                log(LogTag::Transactions, "VERIFY_ERROR", &format!("❌ Error verifying transaction {}: {}", &signature[..8], e));
            }
            false
        }
    }
}

/// Get transaction statistics
pub async fn get_transaction_stats() -> TransactionStats {
    // Default stats - would integrate with global manager
    TransactionStats {
        total_transactions: 0,
        new_transactions_count: 0,
        known_signatures_count: 0,
    }
}

/// Get recent successful buy transactions for recovery purposes
pub async fn get_recent_successful_buy_transactions(hours: u32) -> Result<Vec<Transaction>, String> {
    let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);
    let mut successful_buys = Vec::new();
    
    // Get all cached transactions
    let mut manager_lock = GLOBAL_TRANSACTION_MANAGER.lock().await;
    if let Some(manager) = manager_lock.as_mut() {
        let transactions = manager.fetch_limited_wallet_transactions(1000).await.unwrap_or_default();
        
        for tx in transactions {
            // Filter for successful buy transactions within time window
            if tx.success 
                && tx.timestamp >= cutoff_time 
                && tx.swap_analysis.as_ref()
                    .map(|s| s.input_token.starts_with("So11")) // SOL to token (buy)
                    .unwrap_or(false) 
            {
                successful_buys.push(tx);
            }
        }
    }
    
    // Sort by timestamp (newest first)
    successful_buys.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    Ok(successful_buys)
}

/// Get all swap transactions from global transaction manager (for positions reconciliation)
/// DEADLOCK SAFE: Creates temporary manager if global not available to avoid holding locks during async operations
pub async fn get_global_swap_transactions() -> Result<Vec<SwapPnLInfo>, String> {
    // DEADLOCK FIX: Don't hold locks while calling async functions
    // Check if global manager is available, but don't call async methods while holding the lock
    let has_global_manager = {
        let manager_guard = GLOBAL_TRANSACTION_MANAGER.lock().await;
        manager_guard.is_some()
    }; // Lock is released here
    
    if has_global_manager {
        // Try to get data without holding lock during async operations
        // This is complex because we can't ensure the manager stays available,
        // so we fall back to temporary manager approach for safety
        log(LogTag::Transactions, "INFO", "Global transaction manager available but using temporary manager for deadlock safety");
    } else {
        log(LogTag::Transactions, "WARN", "No global transaction manager available, creating temporary instance");
    }
    
    // Always use temporary manager approach for maximum deadlock safety
    // This avoids any risk of holding locks during async operations
    let wallet_address = load_wallet_address_from_config().await?;
    
    let mut temp_manager = TransactionsManager::new(wallet_address).await?;
    
    temp_manager.get_all_swap_transactions().await
}

/// Get swap info for a specific transaction signature (OPTIMIZED for positions verification)
/// This is much more efficient than loading all transactions when only one is needed
pub async fn get_single_transaction_swap_info(signature: &str) -> Result<Option<SwapPnLInfo>, String> {
    // Try to use existing global transaction if available (from cached data)
    if let Ok(Some(existing_transaction)) = get_transaction(signature).await {
        log(LogTag::Transactions, "CACHE_HIT", &format!(
            "Using existing transaction data for {}", get_signature_prefix(signature)
        ));
        
        // Convert existing transaction to SwapPnLInfo if it's a swap
        let wallet_address = load_wallet_address_from_config().await?;
        let mut temp_manager = TransactionsManager::new(wallet_address).await?;
        let symbol_cache = std::collections::HashMap::new(); // Empty cache for single transaction
        
        if let Some(swap_info) = temp_manager.convert_to_swap_pnl_info(&existing_transaction, &symbol_cache, false) {
            log(LogTag::Transactions, "SUCCESS", &format!(
                "Generated swap info from existing transaction {}: {} tokens at {:.12} SOL", 
                get_signature_prefix(signature), swap_info.token_amount, swap_info.calculated_price_sol
            ));
            return Ok(Some(swap_info));
        } else {
            log(LogTag::Transactions, "INFO", &format!(
                "Existing transaction {} is not a swap transaction", get_signature_prefix(signature)
            ));
            return Ok(None);
        }
    }
    
    // If not available globally, process just this single transaction
    log(LogTag::Transactions, "SINGLE_TX_PROCESS", &format!(
        "Processing single transaction {} (not in global cache)", get_signature_prefix(signature)
    ));
    
    let wallet_address = load_wallet_address_from_config().await?;
    let mut temp_manager = TransactionsManager::new(wallet_address).await?;
    
    // Get the specific transaction (this processes only ONE transaction)
    match temp_manager.process_transaction(signature).await {
        Ok(mut transaction) => {
            // Always recalculate transaction analysis (no hydration optimization)
            if let Err(e) = temp_manager.recalculate_transaction_analysis(&mut transaction).await {
                log(LogTag::Transactions, "WARN", &format!(
                    "Failed to recalculate transaction {}: {}", get_signature_prefix(signature), e
                ));
                return Ok(None);
            }
            
            // Cache the updated analysis for finalized transactions
            if matches!(transaction.status, TransactionStatus::Finalized) && transaction.raw_transaction_data.is_some() {
                transaction.cached_analysis = Some(CachedAnalysis::from_transaction(&transaction));
                if let Err(e) = temp_manager.cache_transaction(&transaction).await {
                    log(LogTag::Transactions, "WARN", &format!(
                        "Failed to cache recalculated transaction {}: {}", get_signature_prefix(signature), e
                    ));
                }
            }
            
            // Convert to SwapPnLInfo if it's a swap
            let symbol_cache = std::collections::HashMap::new(); // Empty cache for single transaction
            if let Some(swap_info) = temp_manager.convert_to_swap_pnl_info(&transaction, &symbol_cache, false) {
                log(LogTag::Transactions, "SUCCESS", &format!(
                    "Generated swap info for single transaction {}: {} tokens at {:.12} SOL", 
                    get_signature_prefix(signature), swap_info.token_amount, swap_info.calculated_price_sol
                ));
                Ok(Some(swap_info))
            } else {
                log(LogTag::Transactions, "INFO", &format!(
                    "Transaction {} is not a swap transaction", get_signature_prefix(signature)
                ));
                Ok(None)
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "WARN", &format!(
                "Failed to process transaction {}: {}", get_signature_prefix(signature), e
            ));
            Ok(None)
        }
    }
}

/// Clean all existing cache files by removing calculated fields
/// This is useful during development when calculation logic changes frequently
pub async fn clean_all_transaction_cache_files() -> Result<(usize, usize), String> {
    let cache_dir = get_transactions_cache_dir();
    
    if !cache_dir.exists() {
        return Ok((0, 0));
    }

    let mut cleaned_count = 0;
    let mut failed_count = 0;

    // Read all JSON files in the cache directory
    let entries = fs::read_dir(&cache_dir)
        .map_err(|e| format!("Failed to read cache directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            match clean_single_cache_file(&path).await {
                Ok(()) => {
                    cleaned_count += 1;
                    log(LogTag::Transactions, "CLEAN", &format!(
                        "Cleaned cache file: {}", 
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
                Err(e) => {
                    failed_count += 1;
                    log(LogTag::Transactions, "WARN", &format!(
                        "Failed to clean {}: {}", 
                        path.file_name().unwrap_or_default().to_string_lossy(), 
                        e
                    ));
                }
            }
        }
    }

    log(LogTag::Transactions, "SUCCESS", &format!(
        "Cache cleanup completed: {} files cleaned, {} failed", 
        cleaned_count, failed_count
    ));

    Ok((cleaned_count, failed_count))
}

/// Clean a single cache file by removing calculated fields
async fn clean_single_cache_file(file_path: &Path) -> Result<(), String> {
    // Load the existing transaction
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;
    
    let mut transaction: Transaction = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse transaction JSON: {}", e))?;
    
    // Clean it by removing calculated fields - keep only raw blockchain data
    transaction.transaction_type = TransactionType::Unknown;
    transaction.direction = TransactionDirection::Internal;
    transaction.fee_sol = 0.0;
    transaction.sol_balance_change = 0.0;
    transaction.token_transfers = Vec::new();
    transaction.log_messages = Vec::new();
    transaction.instructions = Vec::new();
    transaction.sol_balance_changes = Vec::new();
    transaction.token_balance_changes = Vec::new();
    transaction.swap_analysis = None;
    transaction.position_impact = None;
    transaction.profit_calculation = None;
    transaction.fee_breakdown = None;
    transaction.ata_analysis = None;
    transaction.token_info = None;
    transaction.calculated_token_price_sol = None;
    transaction.price_source = None;
    transaction.token_symbol = None;
    transaction.token_decimals = None;
    transaction.last_updated = Utc::now();
    
    // Write it back to the same file
    let json_data = serde_json::to_string_pretty(&transaction)
        .map_err(|e| format!("Failed to serialize cleaned transaction: {}", e))?;

    fs::write(file_path, json_data)
        .map_err(|e| format!("Failed to write cleaned cache file: {}", e))?;

    Ok(())
}

// =============================================================================
// PRIORITY TRANSACTION FUNCTIONS
// =============================================================================

/// Add a transaction to priority verification queue
pub async fn add_priority_transaction(signature: String, transaction_type: String) -> Result<(), String> {
    log(LogTag::Transactions, "PRIORITY", &format!(
        "🔥 Adding priority transaction: {} (type: {})", 
        &signature[..8], transaction_type
    ));
    
    if let Some(manager_arc) = get_global_transaction_manager().await {
        let mut manager_guard = manager_arc.lock().await;
        if let Some(ref mut manager) = manager_guard.as_mut() {
            // Check if transaction already exists
            if manager.known_signatures().contains(&signature) {
                log(LogTag::Transactions, "INFO", &format!(
                    "✅ Priority transaction {} already in system", &signature[..8]
                ));
                return Ok(());
            }
            
            // Process the priority transaction immediately
            match manager.process_transaction(&signature).await {
                Ok(transaction) => {
                    log(LogTag::Transactions, "SUCCESS", &format!(
                        "✅ Priority transaction {} processed successfully", &signature[..8]
                    ));
                    
                    // Additional logging for priority transactions
                    match &transaction.transaction_type {
                        TransactionType::SwapSolToToken { token_mint, sol_amount, token_amount, router } => {
                            log(LogTag::Transactions, "PRIORITY", &format!(
                                "🔥 Priority BUY: {} SOL → {} tokens ({}) via {}", 
                                sol_amount, token_amount, &token_mint[..8], router
                            ));
                        }
                        TransactionType::SwapTokenToSol { token_mint, token_amount, sol_amount, router } => {
                            log(LogTag::Transactions, "PRIORITY", &format!(
                                "🔥 Priority SELL: {} tokens → {} SOL ({}) via {}", 
                                token_amount, sol_amount, &token_mint[..8], router
                            ));
                        }
                        _ => {
                            log(LogTag::Transactions, "PRIORITY", &format!(
                                "🔥 Priority transaction type: {:?}", transaction.transaction_type
                            ));
                        }
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    log(LogTag::Transactions, "ERROR", &format!(
                        "❌ Failed to process priority transaction {}: {}", &signature[..8], e
                    ));
                    Err(format!("Priority transaction processing failed: {}", e))
                }
            }
        } else {
            Err("TransactionManager not initialized".to_string())
        }
    } else {
        Err("TransactionManager not available".to_string())
    }
}

/// Wait for a priority transaction to be verified and return the result
pub async fn wait_for_priority_transaction_verification(
    signature: String, 
    timeout_seconds: u64
) -> Result<Transaction, String> {
    log(LogTag::Transactions, "PRIORITY", &format!(
        "⏳ Waiting for priority transaction verification: {} (timeout: {}s)", 
        &signature[..8], timeout_seconds
    ));
    
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout_seconds);
    
    // First add it to priority queue if not already processed
    add_priority_transaction(signature.clone(), "priority".to_string()).await?;
    
    // Poll for verification with exponential backoff
    let mut check_interval = Duration::from_millis(500); // Start with 500ms
    let max_interval = Duration::from_secs(5); // Max 5 seconds between checks
    
    loop {
        // Check if we've exceeded timeout
        if start_time.elapsed() > timeout_duration {
            log(LogTag::Transactions, "ERROR", &format!(
                "❌ Priority transaction verification timeout: {} ({}s)", 
                &signature[..8], timeout_seconds
            ));
            return Err(format!("Transaction verification timeout after {}s", timeout_seconds));
        }
        
        // Check if transaction exists and is verified
        match get_transaction(&signature).await? {
            Some(transaction) => {
                log(LogTag::Transactions, "SUCCESS", &format!(
                    "✅ Priority transaction {} verified in {:.1}s", 
                    &signature[..8], start_time.elapsed().as_secs_f32()
                ));
                
                // Additional success logging for swaps
                match &transaction.transaction_type {
                    TransactionType::SwapSolToToken { sol_amount, token_amount, .. } => {
                        log(LogTag::Transactions, "PRIORITY", &format!(
                            "🎯 Verified BUY: {} SOL → {} tokens", sol_amount, token_amount
                        ));
                    }
                    TransactionType::SwapTokenToSol { token_amount, sol_amount, .. } => {
                        log(LogTag::Transactions, "PRIORITY", &format!(
                            "🎯 Verified SELL: {} tokens → {} SOL", token_amount, sol_amount
                        ));
                    }
                    _ => {}
                }
                
                return Ok(transaction);
            }
            None => {
                // Transaction not found yet, wait and retry
                tokio::time::sleep(check_interval).await;
                
                // Exponential backoff up to max interval
                check_interval = std::cmp::min(check_interval.mul_f32(1.5), max_interval);
                
                if start_time.elapsed().as_secs() % 10 == 0 {
                    log(LogTag::Transactions, "INFO", &format!(
                        "⏳ Still waiting for transaction {}: {:.1}s elapsed", 
                        &signature[..8], start_time.elapsed().as_secs_f32()
                    ));
                }
            }
        }
    }
}

// =============================================================================
// MISSING SWAP ANALYSIS FUNCTIONS
// =============================================================================

impl TransactionsManager {
    /// Analyze Jupiter swap transactions
    async fn analyze_jupiter_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Jupiter swaps are identified by:
        // 1. Jupiter program ID presence (already checked in caller)
        // 2. ATA creation for tokens (indicating swap setup)
        // 3. Token transfer instructions
        // 4. Router instruction patterns
        
        if self.debug_enabled {
            log(LogTag::Transactions, "PUMP_ANALYSIS", &format!("{} - Analyzing Jupiter swap", 
                &transaction.signature[..8]));
        }
        
        // Extract token mint from the transaction data
        let target_token_mint = self.extract_target_token_mint_from_jupiter(transaction).await;
        
        let has_wsol_operations = log_text.contains("So11111111111111111111111111111111111111112");
        let has_token_operations = log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") || 
                                   log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        let has_jupiter_route = log_text.contains("Instruction: Route") || 
                               log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
        
        // Extract actual SOL amount from transfer instructions or balance changes
        let sol_amount = self.extract_sol_amount_from_jupiter(transaction).await;
        let token_amount = self.extract_token_amount_from_jupiter(transaction).await;
        
        // Jupiter swaps can be detected even if they fail, based on intent and instruction patterns
        if has_jupiter_route && has_token_operations {
            
            // Determine swap direction based on both SOL and token balance changes
            // Priority: 1) Token balance direction, 2) SOL balance direction
            
            // Check if we have significant token amounts to determine direction
            if token_amount > 1.0 {
                // We have token amounts, determine direction from balance changes
                if transaction.sol_balance_change > 0.000001 {
                    // User gained SOL and we detected token amounts = Token to SOL swap (SELL)
                    return Ok(TransactionType::SwapTokenToSol {
                        router: "Jupiter".to_string(),
                        token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                        token_amount: token_amount,
                        sol_amount: transaction.sol_balance_change.abs(),
                    });
                } else if transaction.sol_balance_change < -0.000001 {
                    // User lost SOL and we detected token amounts = SOL to Token swap (BUY)
                    return Ok(TransactionType::SwapSolToToken {
                        router: "Jupiter".to_string(),
                        token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                        sol_amount: transaction.sol_balance_change.abs(),
                        token_amount: token_amount,
                    });
                }
            }
            
            // Fallback to original SOL-based logic if token direction is unclear
            if transaction.sol_balance_change < -0.000001 {
                // SOL to Token swap (BUY) - user spent SOL
                return Ok(TransactionType::SwapSolToToken {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: token_amount,
                });
            } 
            else if transaction.sol_balance_change > 0.000001 {
                // Token to SOL swap (SELL) - user received SOL
                return Ok(TransactionType::SwapTokenToSol {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    token_amount: token_amount,
                    sol_amount: transaction.sol_balance_change.abs(),
                });
            }
            else if has_token_operations && !transaction.token_transfers.is_empty() {
                // Token to Token swap
                return Ok(TransactionType::SwapTokenToToken {
                    router: "Jupiter".to_string(),
                    from_mint: "Unknown".to_string(),
                    to_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    from_amount: 0.0,
                    to_amount: token_amount,
                });
            }
            else {
                // Generic Jupiter swap when we can't determine exact type
                return Ok(TransactionType::SwapSolToToken {
                    router: "Jupiter".to_string(),
                    token_mint: target_token_mint.unwrap_or_else(|| "Unknown".to_string()),
                    sol_amount: sol_amount.max(0.000001),
                    token_amount: token_amount,
                });
            }
        }
        
        Err("Not a Jupiter swap".to_string())
    }
    
    /// Extract target token mint from Jupiter transaction
    /// Strategy:
    /// - Prefer wallet-owned token balance changes (pre/postTokenBalances) to determine mint.
    ///   For SELL (SOL increase), choose the non-WSOL mint with the most negative delta (tokens decreased).
    ///   For BUY  (SOL decrease), choose the non-WSOL mint with the most positive delta (tokens increased).
    /// - Fallback to scanning ATA init instructions for a non-WSOL mint if balance changes are unavailable.
    async fn extract_target_token_mint_from_jupiter(&self, transaction: &Transaction) -> Option<String> {
        let wallet_str = self.wallet_pubkey.to_string();
        let epsilon = 1e-12f64;

        // 1) Prefer wallet pre/post token balance deltas
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preTokenBalances").and_then(|v| v.as_array()),
                    meta.get("postTokenBalances").and_then(|v| v.as_array())
                ) {
                    // Gather deltas for wallet-owned token accounts (exclude WSOL)
                    let mut candidates: Vec<(String, f64)> = Vec::new();
                    for post_balance in post_balances {
                        let owner = post_balance.get("owner").and_then(|v| v.as_str());
                        let mint = post_balance.get("mint").and_then(|v| v.as_str());
                        if owner == Some(wallet_str.as_str()) {
                            if let Some(mint_str) = mint {
                                if mint_str == WSOL_MINT { continue; }
                                let account_index = post_balance.get("accountIndex").and_then(|v| v.as_u64());
                                let pre_amount = pre_balances.iter()
                                    .find(|pre| pre.get("accountIndex").and_then(|v| v.as_u64()) == account_index)
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let post_amount = post_balance.get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let delta = post_amount - pre_amount; // positive = increased, negative = decreased
                                if delta.abs() > epsilon {
                                    candidates.push((mint_str.to_string(), delta));
                                }
                            }
                        }
                    }

                    if !candidates.is_empty() {
                        // Decide on expected direction from SOL balance change
                        let is_sell = transaction.sol_balance_change > 0.000001;   // gained SOL
                        let is_buy  = transaction.sol_balance_change < -0.000001;  // spent SOL

                        if is_sell {
                            // Pick most negative delta (largest token decrease)
                            if let Some((mint, _)) = candidates
                                .iter()
                                .filter(|(_, d)| *d < -epsilon)
                                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                            {
                                return Some(mint.clone());
                            }
                        } else if is_buy {
                            // Pick most positive delta (largest token increase)
                            if let Some((mint, _)) = candidates
                                .iter()
                                .filter(|(_, d)| *d > epsilon)
                                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                            {
                                return Some(mint.clone());
                            }
                        }

                        // Fallback: pick largest absolute delta if direction unclear
                        if let Some((mint, _)) = candidates
                            .iter()
                            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap_or(std::cmp::Ordering::Equal))
                        {
                            return Some(mint.clone());
                        }
                    }
                }
            }
        }

        // 2) Fallback: Look for ATA creation instructions for non-WSOL tokens
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let Some(mint) = info.get("mint").and_then(|v| v.as_str()) {
                                            if mint != WSOL_MINT {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Extract SOL amount from Jupiter transaction
    async fn extract_sol_amount_from_jupiter(&self, transaction: &Transaction) -> f64 {
        // Look for SOL transfer instructions in the transaction
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(transaction_data) = raw_data.get("transaction") {
                if let Some(message) = transaction_data.get("message") {
                    if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
                        for instruction in instructions {
                            if let Some(parsed) = instruction.get("parsed") {
                                if let Some(info) = parsed.get("info") {
                                    if let Some(lamports) = info.get("lamports").and_then(|v| v.as_u64()) {
                                        return lamports as f64 / 1_000_000_000.0; // Convert to SOL
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        transaction.sol_balance_change.abs()
    }
    
    /// Extract token amount from Jupiter transaction
    /// Returns absolute token amount moved to/from the wallet for the non-WSOL mint.
    async fn extract_token_amount_from_jupiter(&self, transaction: &Transaction) -> f64 {
        // First check existing token_transfers
        if !transaction.token_transfers.is_empty() {
            return transaction.token_transfers[0].amount;
        }
        
        // Method 1: Check pre/post token balance changes (most reliable)
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                // Look for token balance changes
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preTokenBalances").and_then(|v| v.as_array()),
                    meta.get("postTokenBalances").and_then(|v| v.as_array())
                ) {
                    // Find the token balance change for our wallet
                    let wallet_str = self.wallet_pubkey.to_string();
                    log(LogTag::Transactions, "JUPITER_TOKEN", &format!(
                        "🔍 Looking for token balance changes for wallet: {}", wallet_str
                    ));
                    
                    for (post_idx, post_balance) in post_balances.iter().enumerate() {
                        if let Some(post_owner) = post_balance.get("owner").and_then(|v| v.as_str()) {
                            let mint_str = post_balance.get("mint").and_then(|v| v.as_str()).unwrap_or("unknown");
                            // Skip WSOL
                            if mint_str == "So11111111111111111111111111111111111111112" { continue; }
                            
                            log(LogTag::Transactions, "JUPITER_TOKEN", &format!(
                                "📋 Post balance #{}: owner={}, mint={}", 
                                post_idx,
                                &post_owner[..8.min(post_owner.len())],
                                mint_str
                            ));
                            
                            if post_owner == wallet_str {
                                // Find the corresponding pre-balance
                                let account_index = post_balance.get("accountIndex").and_then(|v| v.as_u64()).unwrap_or(999);
                                
                                // Get pre-balance for same account
                                let pre_amount = pre_balances.iter()
                                    .find(|pre| pre.get("accountIndex").and_then(|v| v.as_u64()) == Some(account_index))
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                
                                // Get post-balance
                                let post_amount = post_balance.get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                
                                let token_change = post_amount - pre_amount;
                                
                                let mint_str = post_balance.get("mint").and_then(|v| v.as_str()).unwrap_or("unknown");
                                
                                log(LogTag::Transactions, "JUPITER_TOKEN", &format!(
                                    "💰 Token balance change for account[{}]: {} -> {} = {} (mint: {})",
                                    account_index, pre_amount, post_amount, token_change, mint_str
                                ));
                                
                                // Check for both positive and negative changes (use tiny epsilon to avoid float noise)
                                if token_change.abs() > 1e-12 { 
                                    if let Some(mint) = post_balance.get("mint").and_then(|v| v.as_str()) {
                                        log(LogTag::Transactions, "JUPITER_TOKEN", &format!(
                                            "🔢 Jupiter token amount from balance change: {} -> {} = {} (mint: {})",
                                            pre_amount, post_amount, token_change, mint
                                        ));
                                        
                                        return token_change.abs(); // Return absolute value since we track direction elsewhere
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Method 2: Parse token transfers from inner instructions (fallback)
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    let wallet_str = self.wallet_pubkey.to_string();
                    let mut sum_ui = 0.0f64;
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                // Look for parsed token transfer instructions
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        // Support both transfer and transferChecked
                                        if let Some(itype) = parsed.get("type").and_then(|v| v.as_str()) {
                                            // Extract mint and token amount (ui if available)
                                            let mint_opt = info.get("mint").and_then(|v| v.as_str());
                                            if let Some(mint_str) = mint_opt {
                                                if mint_str == "So11111111111111111111111111111111111111112" { continue; }
                                                let involves_wallet = info.get("destination").and_then(|v| v.as_str()) == Some(wallet_str.as_str())
                                                    || info.get("source").and_then(|v| v.as_str()) == Some(wallet_str.as_str())
                                                    || info.get("owner").and_then(|v| v.as_str()) == Some(wallet_str.as_str())
                                                    || info.get("authority").and_then(|v| v.as_str()) == Some(wallet_str.as_str());
                                                if !involves_wallet { continue; }

                                                // Prefer tokenAmount.uiAmount if present (transferChecked), else raw amount + decimals
                                                if let Some(token_amount) = info.get("tokenAmount") {
                                                    if let Some(ui) = token_amount.get("uiAmount").and_then(|v| v.as_f64()) {
                                                        if ui > 0.0 { sum_ui += ui; }
                                                        continue;
                                                    }
                                                    if let Some(raw_str) = token_amount.get("amount").and_then(|v| v.as_str()) {
                                                        if let Ok(raw) = raw_str.parse::<u64>() {
                                                            if let Ok(dec) = get_token_decimals_safe(mint_str).await {
                                                                sum_ui += raw_to_ui_amount(raw, dec);
                                                                continue;
                                                            }
                                                        }
                                                    }
                                                }
                                                // Legacy transfer path with plain amount
                                                if let Some(amount_str) = info.get("amount").and_then(|v| v.as_str()) {
                                                    if let Ok(raw_amount) = amount_str.parse::<u64>() {
                                                        if let Ok(decimals) = get_token_decimals_safe(mint_str).await {
                                                            sum_ui += raw_to_ui_amount(raw_amount, decimals);
                                                        } else {
                                                            // Rough fallback if decimals unknown
                                                            sum_ui += if raw_amount > 1_000_000_000 { raw_to_ui_amount(raw_amount, 9) }
                                                                      else if raw_amount > 1_000_000 { raw_to_ui_amount(raw_amount, 6) }
                                                                      else { raw_to_ui_amount(raw_amount, 5) };
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if sum_ui > 0.0 { return sum_ui; }
                }
            }
        }
        
        0.0
    }

    /// Analyze Raydium swap transactions (both AMM and CPMM)
    async fn analyze_raydium_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Raydium swaps are identified by:
        // 1. Raydium program ID presence (already checked in caller)  
        // 2. Token operations (Token program, ATA operations)
        // 3. SOL balance changes indicating SOL involvement
        // 4. CPMM or AMM program instructions
        
        let has_token_operations = log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
                                   log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
        
        // Extract actual token information from Raydium swap
        let (token_mint, token_symbol, token_amount, mut sol_amount) = self.extract_raydium_swap_info(transaction).await;

        // Try to extract SOL (WSOL) amount from inner instructions by summing transferChecked amounts (handles fee/referral splits)
        if sol_amount.is_none() {
            if let Some(raw_data) = &transaction.raw_transaction_data {
                if let Some(meta) = raw_data.get("meta") {
                    if let Some(inner) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                        let mut wsol_sum = 0.0f64;
                        for group in inner {
                            if let Some(instructions) = group.get("instructions").and_then(|v| v.as_array()) {
                                for instr in instructions {
                                    if let Some(parsed) = instr.get("parsed") {
                                        if let Some(info) = parsed.get("info") {
                                            if let (Some(mint), Some(token_amount)) = (
                                                info.get("mint").and_then(|v| v.as_str()),
                                                info.get("tokenAmount")
                                            ) {
                                                if mint == "So11111111111111111111111111111111111111112" {
                                                    if let Some(ui) = token_amount.get("uiAmount").and_then(|v| v.as_f64()) {
                                                        if ui > 0.0 { wsol_sum += ui; }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if wsol_sum > 0.0 { sol_amount = Some(wsol_sum); }
                    }
                }
            }
        }
        
        // Check for SOL to Token swap (SOL spent) - lower threshold for failed transactions
        if transaction.sol_balance_change < -0.000001 {  // Spent more than 0.000001 SOL
            return Ok(TransactionType::SwapSolToToken {
                token_mint: token_mint.clone(),
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                token_amount,
                router: self.determine_raydium_router(transaction),
            });
        } 
        // Check for Token to SOL swap (SOL received)
        else if transaction.sol_balance_change > 0.000001 {  // Received more than 0.000001 SOL
            return Ok(TransactionType::SwapTokenToSol {
                token_mint: token_mint.clone(),
                token_amount,
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                router: self.determine_raydium_router(transaction),
            });
        }
        // Check for Token to Token swap (minimal SOL change but has token operations)
        else if has_token_operations && !transaction.token_transfers.is_empty() {
            return Ok(TransactionType::SwapTokenToToken {
                from_mint: token_mint.clone(),
                to_mint: "Unknown".to_string(), // For now, handle as single token
                from_amount: token_amount,
                to_amount: 0.0,
                router: self.determine_raydium_router(transaction),
            });
        }
        // Detect based on program presence even if no clear balance change
        else if has_token_operations {
            return Ok(TransactionType::SwapSolToToken {
                token_mint: token_mint.clone(),
                sol_amount: sol_amount.unwrap_or_else(|| transaction.sol_balance_change.abs()),
                token_amount,
                router: self.determine_raydium_router(transaction),
            });
        }
        
        Err("Not a Raydium swap".to_string())
    }

    /// Extract token information from Raydium swap transaction
    async fn extract_raydium_swap_info(&self, transaction: &Transaction) -> (String, String, f64, Option<f64>) {
        // Method 1: Check pre/post token balance changes (most reliable for Raydium)
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let (Some(pre_balances), Some(post_balances)) = (
                    meta.get("preTokenBalances").and_then(|v| v.as_array()),
                    meta.get("postTokenBalances").and_then(|v| v.as_array())
                ) {
                    let wallet_str = self.wallet_pubkey.to_string();
                    log(LogTag::Transactions, "RAYDIUM_TOKEN", &format!(
                        "🔍 Analyzing Raydium token balance changes for wallet: {}", wallet_str
                    ));
                    
                    for (post_idx, post_balance) in post_balances.iter().enumerate() {
                        if let Some(post_owner) = post_balance.get("owner").and_then(|v| v.as_str()) {
                            if post_owner == wallet_str {
                                let account_index = post_balance.get("accountIndex").and_then(|v| v.as_u64()).unwrap_or(999);
                                
                                // Get pre-balance for same account
                                let pre_amount = pre_balances.iter()
                                    .find(|pre| pre.get("accountIndex").and_then(|v| v.as_u64()) == Some(account_index))
                                    .and_then(|pre| pre.get("uiTokenAmount"))
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                
                                // Get post-balance
                                let post_amount = post_balance.get("uiTokenAmount")
                                    .and_then(|ui| ui.get("uiAmount"))
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                
                                let token_change = post_amount - pre_amount;
                                
                                if let Some(mint) = post_balance.get("mint").and_then(|v| v.as_str()) {
                                    // Skip SOL/WSOL 
                                    if mint == "So11111111111111111111111111111111111111112" {
                                        continue;
                                    }
                                    
                                    // Check for significant token balance change
                                    if token_change.abs() > 0.1 { // More than 0.1 token changed
                                        log(LogTag::Transactions, "RAYDIUM_TOKEN", &format!(
                                            "💰 Raydium token balance change: {} -> {} = {} (mint: {})",
                                            pre_amount, post_amount, token_change, mint
                                        ));
                                        
                                        // Get token symbol from database
                                        let token_symbol = if let Some(ref db) = self.token_database {
                                            match db.get_token_by_mint(mint) {
                                                Ok(Some(token_info)) => token_info.symbol,
                                                _ => format!("TOKEN_{}", get_mint_prefix(mint))
                                            }
                                        } else {
                                            format!("TOKEN_{}", get_mint_prefix(mint))
                                        };
                                        
                                        return (mint.to_string(), token_symbol, token_change.abs(), None);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Method 2: Fallback to existing token_transfers if available
        if !transaction.token_transfers.is_empty() {
            let transfer = &transaction.token_transfers[0];
            let token_symbol = if let Some(ref db) = self.token_database {
                match db.get_token_by_mint(&transfer.mint) {
                    Ok(Some(token_info)) => token_info.symbol,
                    _ => format!("TOKEN_{}", get_mint_prefix(&transfer.mint))
                }
            } else {
                format!("TOKEN_{}", get_mint_prefix(&transfer.mint))
            };
            
            return (transfer.mint.clone(), token_symbol, transfer.amount, None);
        }
        
        // Method 3: Final fallback
        ("Unknown".to_string(), "TOKEN_Unknown".to_string(), 0.0, None)
    }

    /// Determine the specific Raydium router being used
    fn determine_raydium_router(&self, transaction: &Transaction) -> String {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for specific Raydium program IDs
        if log_text.contains("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C") {
            "Raydium".to_string()
        } else if log_text.contains("CPMMoo8L3wrBtphwOYMpCX4LtjRWB3gjCMFdukgp6EEh") {
            "Raydium CPMM".to_string()
        } else if log_text.contains("CPMMoo8L3VgkEru3h4j8mu4baRUeJBmK7nfD5fC2pXg") {
            "Raydium CAMM".to_string()
        } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8") {
            "Raydium AMM".to_string()
        } else {
            "Raydium".to_string()
        }
    }

    /// Analyze Orca swap transactions
    async fn analyze_orca_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if log_text.contains("swap") || log_text.contains("Swap") {
            let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");
            
            if has_wsol && transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Unknown".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: "Orca".to_string(),
                });
            } else if has_wsol && transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Unknown".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change.abs(),
                    router: "Orca".to_string(),
                });
            } else if !transaction.token_transfers.is_empty() {
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: "Unknown".to_string(),
                    to_mint: "Unknown".to_string(),
                    from_amount: 0.0,
                    to_amount: 0.0,
                    router: "Orca".to_string(),
                });
            }
        }
        
        Err("Not an Orca swap".to_string())
    }

    /// Analyze generic DEX swap transactions (Meteora, Aldrin, etc.)
    async fn analyze_generic_dex_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        // Check for common swap indicators
        if log_text.contains("swap") || log_text.contains("Swap") || 
           log_text.contains("exchange") || log_text.contains("trade") {
            
            // Identify DEX by program IDs
            let router = if transaction.instructions.iter().any(|i| i.program_id.contains("meteor")) {
                "Meteora"
            } else if transaction.instructions.iter().any(|i| i.program_id.contains("aldrin")) {
                "Aldrin"
            } else if transaction.instructions.iter().any(|i| i.program_id.contains("saber")) {
                "Saber"
            } else {
                "Unknown DEX"
            };
            
            let has_wsol = log_text.contains("So11111111111111111111111111111111111111112");
            
            if has_wsol && transaction.sol_balance_change < 0.0 {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Unknown".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: router.to_string(),
                });
            } else if has_wsol && transaction.sol_balance_change > 0.0 {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Unknown".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change.abs(),
                    router: router.to_string(),
                });
            } else if !transaction.token_transfers.is_empty() {
                return Ok(TransactionType::SwapTokenToToken {
                    from_mint: "Unknown".to_string(),
                    to_mint: "Unknown".to_string(),
                    from_amount: 0.0,
                    to_amount: 0.0,
                    router: router.to_string(),
                });
            }
        }
        
        Err("Not a generic DEX swap".to_string())
    }

    /// Analyze ATA operations and calculate rent amounts
    async fn analyze_ata_operations(&self, transaction: &Transaction) -> Result<f64, String> {
        let mut total_ata_rent = 0.0;
        
        // Look for ATA account closures and creations in pre/post balances
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(pre_balances) = meta.get("preBalances").and_then(|v| v.as_array()) {
                    if let Some(post_balances) = meta.get("postBalances").and_then(|v| v.as_array()) {
                        // Compare pre and post balances to detect ATA rent flows
                        for (index, (pre, post)) in pre_balances.iter().zip(post_balances.iter()).enumerate() {
                            if let (Some(pre_val), Some(post_val)) = (pre.as_u64(), post.as_u64()) {
                                let change = post_val as i64 - pre_val as i64;
                                
                                // Check if this is an ATA account by looking at the change amount
                                // Standard ATA rent is 2039280 lamports (0.00203928 SOL)
                                // Also check for partial ATA rent amounts
                                if change.abs() >= 1000000 && change.abs() <= 3000000 {
                                    // Check if this involves CloseAccount instructions
                                    let has_close_account = transaction.log_messages.iter()
                                        .any(|log| log.contains("Instruction: CloseAccount"));
                                    
                                    if has_close_account {
                                        // If an account went from having balance to 0, it's likely ATA closure
                                        if pre_val > 1000000 && post_val == 0 {
                                            total_ata_rent += lamports_to_sol(pre_val);
                                            if self.debug_enabled {
                                                log(LogTag::Transactions, "ATA_RENT", 
                                                    &format!("Detected ATA closure rent refund: {} lamports ({:.9} SOL)", 
                                                             pre_val, lamports_to_sol(pre_val)));
                                            }
                                        }
                                        // If account went from 0 to some amount and then back, it's temporary ATA
                                        else if pre_val == 0 && post_val == 0 {
                                            // Check if this account was created and closed in the same transaction
                                            // by looking for both CreateAccount and CloseAccount patterns
                                            let has_create_account = transaction.log_messages.iter()
                                                .any(|log| log.contains("createAccount") || log.contains("CreateIdempotent"));
                                                
                                            if has_create_account {
                                                // Estimate typical ATA rent for temporary accounts
                                                total_ata_rent += 0.00203928; // Standard ATA rent
                                                if self.debug_enabled {
                                                    log(LogTag::Transactions, "ATA_RENT", 
                                                        "Detected temporary ATA creation/closure: 0.00203928 SOL");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(total_ata_rent)
    }

    /// Analyze NFT operations (DISABLED - no longer detected)
    async fn analyze_nft_operations(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("NFT operations no longer detected".to_string())
    }

    /// Analyze wrapped SOL operations
    async fn analyze_wsol_operations(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        let wsol_mint = "So11111111111111111111111111111111111111112";
        
        // Check for WSOL wrapping (SOL -> WSOL)
        if log_text.contains(wsol_mint) && transaction.sol_balance_change < 0.0 {
            // Look for token account creation and transfer to WSOL account
            if transaction.instructions.iter().any(|i| i.instruction_type == "transfer") {
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: wsol_mint.to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: transaction.sol_balance_change.abs(), // 1:1 ratio for WSOL
                    router: "Native WSOL".to_string(),
                });
            }
        }
        
        // Check for WSOL unwrapping (WSOL -> SOL)
        if log_text.contains(wsol_mint) && transaction.sol_balance_change > 0.0 {
            if transaction.instructions.iter().any(|i| i.instruction_type == "closeAccount") {
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: wsol_mint.to_string(),
                    token_amount: transaction.sol_balance_change.abs(),
                    sol_amount: transaction.sol_balance_change.abs(), // 1:1 ratio for WSOL
                    router: "Native WSOL".to_string(),
                });
            }
        }
        
        Err("No WSOL operation detected".to_string())
    }

    /// Analyze Pump.fun swap operations
    async fn analyze_pump_fun_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if self.debug_enabled {
            log(LogTag::Transactions, "PUMP_ANALYSIS", &format!("{} - Analyzing Pump.fun swap", 
                &transaction.signature[..8]));
        }

        // Extract token mint from Pump.fun transaction
        let target_token_mint = self.extract_target_token_mint_from_pumpfun(transaction).await;
        
        // Check for Pump.fun specific patterns - both program IDs and logs
        let has_pumpfun_program = log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") ||
                                  log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA") ||
                                  transaction.instructions.iter().any(|i| 
                                      i.program_id == "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" ||
                                      i.program_id == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
        
        let has_buy_instruction = log_text.contains("Instruction: Buy");
        let has_sell_instruction = log_text.contains("Instruction: Sell");
        
        if has_pumpfun_program {
            // Extract actual amounts from transaction data
            let sol_amount = self.extract_sol_amount_from_pumpfun(transaction).await;
            let token_amount = self.extract_token_amount_from_pumpfun(transaction).await;
            
            // Determine direction based on instruction patterns and balance changes
            // Note: sol_amount is always positive (abs value), so we use sol_balance_change for direction
            if has_buy_instruction || transaction.sol_balance_change < -0.000001 {
                // SOL to Token (Buy) - SOL was spent (negative balance change)
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                    sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                    token_amount: token_amount,
                    router: "Pump.fun".to_string(),
                });
            } else if has_sell_instruction || transaction.sol_balance_change > 0.000001 {
                // Token to SOL (Sell) - SOL was received (positive balance change)
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                    token_amount: token_amount,
                    sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                    router: "Pump.fun".to_string(),
                });
            } else {
                // Fallback: if we have Pump.fun program but unclear direction, use balance change
                if transaction.sol_balance_change.abs() > 0.000001 {
                    if transaction.sol_balance_change < 0.0 {
                        // SOL spent = Buy
                        return Ok(TransactionType::SwapSolToToken {
                            token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                            sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                            token_amount: token_amount,
                            router: "Pump.fun".to_string(),
                        });
                    } else {
                        // SOL received = Sell
                        return Ok(TransactionType::SwapTokenToSol {
                            token_mint: target_token_mint.unwrap_or_else(|| "Pump.fun_Token".to_string()),
                            token_amount: token_amount,
                            sol_amount: sol_amount, // Use extracted amount (excludes ATA rent)
                            router: "Pump.fun".to_string(),
                        });
                    }
                }
                // Final fallback - unclear transaction, return error instead of defaulting to buy
                return Err("Cannot determine Pump.fun swap direction".to_string());
            }
        }
        
        Err("No Pump.fun swap pattern found".to_string())
    }
    
    /// Extract target token mint from Pump.fun transaction
    async fn extract_target_token_mint_from_pumpfun(&self, transaction: &Transaction) -> Option<String> {
        // Look for token account creation or transfer instructions for non-WSOL tokens
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let Some(mint) = info.get("mint").and_then(|v| v.as_str()) {
                                            if mint != "So11111111111111111111111111111111111111112" {
                                                return Some(mint.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Extract SOL amount from Pump.fun transaction
    async fn extract_sol_amount_from_pumpfun(&self, transaction: &Transaction) -> f64 {
        // Sum all WSOL transferChecked uiAmounts found in inner instructions (covers splits to fees/referrals)
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    let mut wsol_sum = 0.0f64;
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let (Some(mint), Some(token_amount)) = (
                                            info.get("mint").and_then(|v| v.as_str()),
                                            info.get("tokenAmount")
                                        ) {
                                            if mint == "So11111111111111111111111111111111111111112" {
                                                if let Some(ui_amount) = token_amount.get("uiAmount").and_then(|v| v.as_f64()) {
                                                    // Include even micro amounts; they'll round correctly in display
                                                    if ui_amount > 0.0 { wsol_sum += ui_amount; }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if wsol_sum > 0.0 {
                        return wsol_sum;
                    }
                }
            }
        }

        // Calculate ATA rent to exclude from balance change as a fallback
        let ata_rent = self.analyze_ata_operations(transaction).await.unwrap_or(0.0);

        // Use balance change minus ATA rent as fallback
        let adjusted_balance_change = transaction.sol_balance_change.abs() - ata_rent;

        if self.debug_enabled && ata_rent > 0.0 {
            log(LogTag::Transactions, "SOL_EXTRACT",
                &format!("Excluding ATA rent: {:.9} SOL from balance change {:.9} SOL",
                         ata_rent, transaction.sol_balance_change.abs()));
        }

        // Return the adjusted amount, ensuring it's not negative
        adjusted_balance_change.max(0.0)
    }
    
    /// Extract token amount from Pump.fun transaction
    async fn extract_token_amount_from_pumpfun(&self, transaction: &Transaction) -> f64 {
        // Look for token transfer amounts in inner instructions
        if let Some(raw_data) = &transaction.raw_transaction_data {
            if let Some(meta) = raw_data.get("meta") {
                if let Some(inner_instructions) = meta.get("innerInstructions").and_then(|v| v.as_array()) {
                    for inner_group in inner_instructions {
                        if let Some(instructions) = inner_group.get("instructions").and_then(|v| v.as_array()) {
                            for instruction in instructions {
                                if let Some(parsed) = instruction.get("parsed") {
                                    if let Some(info) = parsed.get("info") {
                                        if let Some(token_amount) = info.get("tokenAmount") {
                                            if let Some(ui_amount) = token_amount.get("uiAmount").and_then(|v| v.as_f64()) {
                                                if ui_amount > 0.0 {
                                                    return ui_amount;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback to token_transfers if available
        if !transaction.token_transfers.is_empty() {
            return transaction.token_transfers[0].amount;
        }
        0.0
    }

    /// Analyze Serum/OpenBook swap operations
    async fn analyze_serum_swap(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        let log_text = transaction.log_messages.join(" ");
        
        if self.debug_enabled {
            log(LogTag::Transactions, "SERUM_ANALYSIS", &format!("{} - Analyzing Serum/OpenBook swap", 
                &transaction.signature[..8]));
        }

        // Check for Serum specific patterns
        if log_text.contains("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin") {
            // Determine direction based on SOL balance change
            if transaction.sol_balance_change < -0.001 {
                // SOL to Token (Buy)
                return Ok(TransactionType::SwapSolToToken {
                    token_mint: "Serum_Token".to_string(),
                    sol_amount: transaction.sol_balance_change.abs(),
                    token_amount: 0.0,
                    router: "Serum/OpenBook".to_string(),
                });
            } else if transaction.sol_balance_change > 0.001 {
                // Token to SOL (Sell)
                return Ok(TransactionType::SwapTokenToSol {
                    token_mint: "Serum_Token".to_string(),
                    token_amount: 0.0,
                    sol_amount: transaction.sol_balance_change,
                    router: "Serum/OpenBook".to_string(),
                });
            }
        }
        
        Err("No Serum/OpenBook swap pattern found".to_string())
    }

    /// Extract SOL transfer data
    async fn extract_sol_transfer_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Only detect simple SOL transfers with very specific criteria:
        // 1. Must be 1-3 instructions maximum (simple transfers)
        // 2. Must have meaningful SOL amount change (not just fees)
        // 3. Must be primarily system program transfers
        
        if transaction.instructions.len() > 3 {
            return Err("Too many instructions for simple SOL transfer".to_string());
        }
        
        // Check if SOL amount change is meaningful (more than just transaction fees)
        if transaction.sol_balance_change.abs() < 0.0001 {
            return Err("SOL amount too small for meaningful transfer".to_string());
        }
        
        // Check if it's primarily system program transfers
        let system_transfer_count = transaction.instructions.iter()
            .filter(|i| i.program_id == "11111111111111111111111111111111" && i.instruction_type == "transfer")
            .count();
            
        // Must have at least one system transfer and it should be the majority of instructions
        if system_transfer_count == 0 || system_transfer_count < transaction.instructions.len() / 2 {
            return Err("Not primarily system program transfers".to_string());
        }
        
        Ok(TransactionType::SolTransfer {
            amount: transaction.sol_balance_change.abs(),
            from: "wallet".to_string(),
            to: "destination".to_string(),
        })
    }

    /// Extract ATA close operation data (standalone ATA closures)
    async fn extract_ata_close_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        // Check for single closeAccount instruction
        if transaction.instructions.len() != 1 {
            return Err("Not a single instruction transaction".to_string());
        }
        
        let instruction = &transaction.instructions[0];
        
        // Check if it's a Token Program (original or Token-2022) closeAccount instruction
        if (instruction.program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" || 
            instruction.program_id == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb") && 
           instruction.instruction_type == "closeAccount" {
            
            // Check if SOL balance increased (ATA rent recovery)
            if transaction.sol_balance_change > 0.0 {
                // Try to extract token mint from ATA closure
                let token_mint = self.extract_token_mint_from_ata_close(transaction).unwrap_or_else(|| "Unknown".to_string());
                
                return Ok(TransactionType::AtaClose {
                    recovered_sol: transaction.sol_balance_change,
                    token_mint,
                });
            }
        }
        
        Err("No ATA close pattern found".to_string())
    }

    /// Extract token mint from ATA close operation
    fn extract_token_mint_from_ata_close(&self, transaction: &Transaction) -> Option<String> {
        // Look for token balance changes to identify the mint
        if !transaction.token_balance_changes.is_empty() {
            return Some(transaction.token_balance_changes[0].mint.clone());
        }
        
        // If no token balance changes, check log messages for mint information
        let log_text = transaction.log_messages.join(" ");
        if let Some(start) = log_text.find("mint: ") {
            let mint_start = start + 6;
            if let Some(end) = log_text[mint_start..].find(' ') {
                return Some(log_text[mint_start..mint_start + end].to_string());
            }
        }
        
        None
    }

    /// Extract bulk operation data (spam detection) - DISABLED
    async fn extract_bulk_operation_data(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Bulk operation detection disabled - only core transaction types are detected".to_string())
    }

    // =============================================================================
    // MISSING ANALYSIS FUNCTIONS - COMPREHENSIVE SWAP DETECTION
    // =============================================================================

    /// Detect Jupiter swap transactions
    async fn detect_jupiter_swap(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let jupiter_program_id = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
        
        // Check if transaction involves Jupiter
        if !transaction.instructions.iter().any(|i| i.program_id == jupiter_program_id) &&
           !transaction.log_messages.iter().any(|log| log.contains(jupiter_program_id)) {
            return Ok(None);
        }

        // Analyze Jupiter swap pattern
        self.analyze_jupiter_swap(transaction).await.map(Some)
    }

    /// Detect Raydium swap transactions
    async fn detect_raydium_swap(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let raydium_program_ids = [
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM
            "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS",   // Raydium Router
        ];
        
        // Check if transaction involves Raydium
        if !transaction.instructions.iter().any(|i| raydium_program_ids.contains(&i.program_id.as_str())) &&
           !transaction.log_messages.iter().any(|log| raydium_program_ids.iter().any(|id| log.contains(id))) {
            return Ok(None);
        }

        // Analyze Raydium swap pattern
        self.analyze_raydium_swap(transaction).await.map(Some)
    }

    /// Detect Orca swap transactions
    async fn detect_orca_swap(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let orca_program_ids = [
            "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP", // Orca V1
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",   // Orca Whirlpool
        ];
        
        // Check if transaction involves Orca
        if !transaction.instructions.iter().any(|i| orca_program_ids.contains(&i.program_id.as_str())) &&
           !transaction.log_messages.iter().any(|log| orca_program_ids.iter().any(|id| log.contains(id))) {
            return Ok(None);
        }

        // Analyze Orca swap pattern
        self.analyze_orca_swap(transaction).await.map(Some)
    }

    /// Detect Serum/OpenBook swap transactions
    async fn detect_serum_swap(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let serum_program_ids = [
            "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", // Serum DEX
            "srmqPiDkJMShKEGHHJG3w4dWnGr5Hge6F3H5HKpVYuN",   // Serum V3
        ];
        
        // Check if transaction involves Serum
        if !transaction.instructions.iter().any(|i| serum_program_ids.contains(&i.program_id.as_str())) &&
           !transaction.log_messages.iter().any(|log| serum_program_ids.iter().any(|id| log.contains(id))) {
            return Ok(None);
        }

        // Analyze Serum swap pattern
        self.analyze_serum_swap(transaction).await.map(Some)
    }

    /// Detect Pump.fun swap transactions
    async fn detect_pump_fun_swap(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let pump_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
        
        // Check if transaction involves Pump.fun
        if !transaction.instructions.iter().any(|i| i.program_id == pump_program_id) &&
           !transaction.log_messages.iter().any(|log| log.contains(pump_program_id)) {
            return Ok(None);
        }

        // Analyze Pump.fun swap pattern
        self.analyze_pump_fun_swap(transaction).await.map(Some)
    }

    /// Detect SOL transfer transactions
    async fn detect_sol_transfer(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        // Look for system program transfers
        let system_program_id = "11111111111111111111111111111111";
        
        for instruction in &transaction.instructions {
            if instruction.program_id == system_program_id && 
               instruction.instruction_type.contains("transfer") {
                return self.extract_sol_transfer_data(transaction).await.map(Some);
            }
        }

        Ok(None)
    }

    /// Detect token transfer transactions
    async fn detect_token_transfer(&self, transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        let token_program_id = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        
        for instruction in &transaction.instructions {
            if instruction.program_id == token_program_id && 
               instruction.instruction_type.contains("transfer") {
                return self.extract_token_transfer_data(transaction).await.map(Some);
            }
        }

        Ok(None)
    }

    /// Detect ATA operations (creation/closure) - DISABLED
    async fn detect_ata_operations(&self, _transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Detect staking operations - DISABLED
    async fn detect_staking_operations(&self, _transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Detect spam/bulk transactions - DISABLED
    async fn detect_spam_bulk_transactions(&self, _transaction: &Transaction) -> Result<Option<TransactionType>, String> {
        Ok(None)
    }

    /// Extract ATA operation data - DISABLED
    async fn extract_ata_operation_data(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("ATA operations no longer detected as transaction types".to_string())
    }

    /// Extract staking operation data - DISABLED
    async fn extract_staking_operation_data(&self, _transaction: &Transaction) -> Result<TransactionType, String> {
        Err("Staking operations no longer detected as transaction types".to_string())
    }

    /// Extract token transfer data
    async fn extract_token_transfer_data(&self, transaction: &Transaction) -> Result<TransactionType, String> {
        for transfer in &transaction.token_transfers {
            return Ok(TransactionType::TokenTransfer {
                mint: transfer.mint.clone(),
                amount: transfer.amount,
                from: transfer.from.clone(),
                to: transfer.to.clone(),
            });
        }

        Err("No token transfer found".to_string())
    }
}
