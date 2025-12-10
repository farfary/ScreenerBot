// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

// Analysis cache versioning (bump when snapshot schema changes)
pub const ANALYSIS_CACHE_VERSION: u32 = 2;

/// Deferred retry record for signatures that timed out/dropped
/// Used for both manager-level retries and service-level deferred queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredRetry {
  pub signature: String,
  pub next_retry_at: DateTime<Utc>,
  pub attempts: u32,
  pub current_delay_secs: i64,
  pub last_error: Option<String>,
  pub first_seen: DateTime<Utc>,
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
  // Legacy/raw metrics for compatibility with existing modules
  pub fee_lamports: Option<u64>,
  pub compute_units_consumed: Option<u64>,
  pub instructions_count: usize,
  pub accounts_count: usize,
  #[serde(skip_serializing, default)]
  pub sol_balance_change: f64,
  #[serde(skip_serializing, default)]
  pub wallet_lamport_change: i64,
  #[serde(skip_serializing, default)]
  pub wallet_signed: bool,
  #[serde(skip_serializing, default)]
  pub token_transfers: Vec<TokenTransfer>,

  // Raw Solana data (cached - only raw blockchain data)
  pub raw_transaction_data: Option<serde_json::Value>,
  #[serde(skip_serializing, default)]
  pub log_messages: Vec<String>,
  #[serde(skip_serializing, default)]
  pub instructions: Vec<InstructionInfo>,
  // Compatibility alias used by some modules
  pub instruction_info: Vec<InstructionInfo>,

  // Balance changes - NEVER CACHED - always calculated fresh
  #[serde(skip_serializing, default)]
  pub sol_balance_changes: Vec<SolBalanceChange>,
  #[serde(skip_serializing, default)]
  pub token_balance_changes: Vec<TokenBalanceChange>,

  // Our analysis and calculations - NEVER CACHED - always calculated fresh
  #[serde(skip_serializing, default)]
  pub position_impact: Option<PositionImpact>,
  #[serde(skip_serializing, default)]
  pub profit_calculation: Option<ProfitCalculation>,
  #[serde(skip_serializing, default)]
  pub ata_analysis: Option<AtaAnalysis>, // SINGLE source of truth for ATA operations
  // Compatibility field expected by debug/processor (list of raw ops)
  pub ata_operations: Vec<AtaOperation>,

  // Token information integration - NEVER CACHED - always calculated fresh
  #[serde(skip_serializing, default)]
  pub token_info: Option<TokenSwapInfo>,
  // Compatibility alias used by old code
  pub token_swap_info: Option<TokenSwapInfo>,
  #[serde(skip_serializing, default)]
  pub calculated_token_price_sol: Option<f64>,
  #[serde(skip_serializing, default)]
  pub token_symbol: Option<String>,
  #[serde(skip_serializing, default)]
  pub token_decimals: Option<u8>,

  // Compatibility: swap PnL snapshot attached to transaction
  #[serde(skip_serializing, default)]
  pub swap_pnl_info: Option<SwapPnLInfo>,

  // Analysis timing (used by debug)
  pub analysis_duration_ms: Option<u64>,

  // Cache metadata
  pub last_updated: DateTime<Utc>,

  // Optional persisted analysis snapshot for finalized txs to avoid re-analysis on every load
  #[serde(default)]
  pub cached_analysis: Option<CachedAnalysis>,
}

impl Transaction {
  pub fn new(signature: String) -> Self {
    Self {
      signature,
      slot: None,
      block_time: None,
      timestamp: Utc::now(),
      status: TransactionStatus::Pending,
      transaction_type: TransactionType::default(),
      direction: TransactionDirection::default(),
      success: false,
      error_message: None,
      fee_sol: 0.0,
      fee_lamports: None,
      compute_units_consumed: None,
      instructions_count: 0,
      accounts_count: 0,
      sol_balance_change: 0.0,
      wallet_lamport_change: 0,
      wallet_signed: false,
      token_transfers: Vec::new(),
      raw_transaction_data: None,
      log_messages: Vec::new(),
      instructions: Vec::new(),
      instruction_info: Vec::new(),
      sol_balance_changes: Vec::new(),
      token_balance_changes: Vec::new(),
      position_impact: None,
      profit_calculation: None,
      ata_analysis: None,
      ata_operations: Vec::new(),
      token_info: None,
      token_swap_info: None,
      calculated_token_price_sol: None,
      token_symbol: None,
      token_decimals: None,
      swap_pnl_info: None,
      analysis_duration_ms: None,
      last_updated: Utc::now(),
      cached_analysis: None,
    }
  }
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
  // Legacy simple variants (compatibility)
  Buy,
  Sell,
  Transfer,
  Compute,
  AtaOperation,
  Failed,
  Unknown,

  // New rich variants
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
  Unknown,
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

/// Comprehensive ATA (Associated Token Account) analysis for a transaction
/// This is the SINGLE source of truth for all ATA operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaAnalysis {
  // Raw counts from transaction
  pub total_ata_creations: u32, // Total ATA creations in transaction
  pub total_ata_closures: u32, // Total ATA closures in transaction

  // Token-specific counts (for swap analysis)
  pub token_ata_creations: u32, // ATA creations for specific token
  pub token_ata_closures: u32, // ATA closures for specific token

  // WSOL-specific counts (for SOL wrapping/unwrapping)
  pub wsol_ata_creations: u32, // WSOL ATA creations
  pub wsol_ata_closures: u32, // WSOL ATA closures

  // Financial impact (in SOL)
 pub total_rent_spent: f64, // Total SOL spent on ATA creation
  pub total_rent_recovered: f64, // Total SOL recovered from ATA closure
  pub net_rent_impact: f64, // Net impact: recovered - spent (positive = gained SOL, negative = spent SOL)

  // Token-specific financial impact (for accurate swap amounts)
 pub token_rent_spent: f64, // SOL spent on token ATA creation
  pub token_rent_recovered: f64, // SOL recovered from token ATA closure
  pub token_net_rent_impact: f64, // Net token ATA impact

  // WSOL-specific financial impact
 pub wsol_rent_spent: f64, // SOL spent on WSOL ATA creation
  pub wsol_rent_recovered: f64, // SOL recovered from WSOL ATA closure
  pub wsol_net_rent_impact: f64, // Net WSOL ATA impact

  // Detected operations (for debugging)
  pub detected_operations: Vec<AtaOperation>,
}

/// Individual ATA operation detected in transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtaOperation {
  pub operation_type: AtaOperationType,
  pub account_address: String,
  pub token_mint: String, // The mint this ATA is associated with
 pub rent_amount: f64, // SOL amount involved (spent or recovered)
 pub is_wsol: bool, // Whether this is a WSOL ATA
  // Compatibility alias fields for debug tools
  pub mint: String,
  pub rent_cost_sol: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AtaOperationType {
  Creation,
  Closure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSwapInfo {
  // Basic token info (enrichment)
  pub mint: String,
  pub symbol: String,
  pub decimals: u8,
  pub current_price_sol: Option<f64>,
  pub is_verified: bool,

  // Detected swap info (DEX/router level)
  pub router: String,
  pub swap_type: String, // sol_to_token | token_to_sol | token_to_token
  pub input_mint: String,
  pub output_mint: String,
  pub input_amount: u64,
  pub output_amount: u64,
  pub input_ui_amount: f64,
  pub output_ui_amount: f64,
  pub pool_address: Option<String>,
  pub program_id: String,
}

/// SwapPnLInfo - Swap analysis data structure
/// CRITICAL: This struct should NEVER be cached to disk
/// All SwapPnLInfo instances must be calculated fresh on every request
/// This ensures calculations are always current and accurate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPnLInfo {
  pub token_mint: String,
  pub token_symbol: String,
 pub swap_type: String, // "Buy"or "Sell"
  pub sol_amount: f64,
  pub token_amount: f64,
  pub calculated_price_sol: f64,
  pub timestamp: DateTime<Utc>,
  pub signature: String,
  pub router: String,
  pub fee_sol: f64,
  pub ata_rents: f64, // ATA creation and rent costs (in SOL)

  // New fields for effective price calculation (excluding ATA rent but including fees)
  pub effective_sol_spent: f64, // For BUY: SOL spent for tokens (includes fees, excludes ATA rent)
  pub effective_sol_received: f64, // For SELL: SOL received for tokens (includes fees, excludes ATA rent)

  // Token-specific ATA operations for this swap (counts)
  pub ata_created_count: u32,
  pub ata_closed_count: u32,

  pub slot: Option<u64>, // Solana slot number for reliable chronological sorting
 pub status: String, // Transaction status: "Success", "Failed", "Partial", etc.

  // Legacy fields used by debug tools
  pub sol_spent: f64,
  pub sol_received: f64,
  pub tokens_bought: f64,
  pub tokens_sold: f64,
  pub net_sol_change: f64,
  pub estimated_token_value_sol: Option<f64>,
  pub estimated_pnl_sol: Option<f64>,
  pub fees_paid_sol: f64,
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
  pub token_symbol: Option<String>,
  pub token_decimals: Option<u8>,
  // Add missing critical fields for complete caching
  pub log_messages: Vec<String>,
  pub instructions: Vec<InstructionInfo>,
}

impl CachedAnalysis {
  pub fn from_transaction(tx: &Transaction) -> Self {
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
      token_symbol: tx.token_symbol.clone(),
      token_decimals: tx.token_decimals,
      // Include the missing critical fields
      log_messages: tx.log_messages.clone(),
      instructions: tx.instructions.clone(),
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
 Open, // Has remaining tokens, no sells
 Closed, // No remaining tokens, fully sold
  PartiallyReduced, // Has remaining tokens, some sells
 Oversold, // Negative token balance (sold more than bought)
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
  pub effective_sol: String, // Shows effective_sol_spent for buys, effective_sol_received for sells
  #[tabled(rename = "Effective Price")]
  pub effective_price: String, // Price calculated using effective SOL amounts
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransactionStats {
  pub total_transactions: u64,
  pub new_transactions_count: u64,
  pub known_signatures_count: u64,
  pub pending_transactions_count: u64,
  pub failed_transactions_count: u64,
  pub successful_transactions_count: u64,
}

impl TransactionStats {
  /// Create a new empty statistics object
  pub fn new() -> Self {
    Self::default()
  }

  /// Success rate percentage across all tracked transactions
  pub fn success_rate(&self) -> f64 {
    if self.total_transactions == 0 {
      0.0
    } else {
      ((self.successful_transactions_count as f64) / (self.total_transactions as f64)) * 100.0
    }
  }

  /// Failure rate percentage across all tracked transactions
  pub fn failure_rate(&self) -> f64 {
    if self.total_transactions == 0 {
      0.0
    } else {
      ((self.failed_transactions_count as f64) / (self.total_transactions as f64)) * 100.0
    }
  }
}
