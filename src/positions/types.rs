use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ==================== POSITION STRUCTURES ====================

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
    pub entry_size_sol: f64,  // Initial SOL spent on first entry
    pub total_size_sol: f64,   // Cumulative SOL invested (includes DCA)
    pub price_highest: f64,
    pub price_lowest: f64,
    // Transaction signatures
    pub entry_transaction_signature: Option<String>,
    pub exit_transaction_signature: Option<String>,
    pub token_amount: Option<u64>, // Initial amount of tokens bought (first entry)
    pub effective_entry_price: Option<f64>, // Initial entry price (deprecated, use average_entry_price)
    pub effective_exit_price: Option<f64>, // Final exit price (deprecated, use average_exit_price)
    pub sol_received: Option<f64>, // Total SOL received after all exits
    // Profit targets
    pub profit_target_min: Option<f64>, // Minimum profit target percentage
    pub profit_target_max: Option<f64>, // Maximum profit target percentage
    pub liquidity_tier: Option<String>, // Liquidity tier for reference
    // Verification status
    pub transaction_entry_verified: bool, // Whether entry transaction is fully verified
    pub transaction_exit_verified: bool,  // Whether exit transaction is fully verified
    // Fee tracking
    pub entry_fee_lamports: Option<u64>, // Actual entry transaction fee
    pub exit_fee_lamports: Option<u64>,  // Actual exit transaction fee
    // Price tracking
    pub current_price: Option<f64>, // Current market price (updated by monitoring system)
    pub current_price_updated: Option<DateTime<Utc>>, // When current_price was last updated
    // Phantom position handling
    pub phantom_remove: bool,
    pub phantom_confirmations: u32, // How many times we confirmed zero wallet balance while still open
    pub phantom_first_seen: Option<DateTime<Utc>>, // When first confirmed phantom
    pub synthetic_exit: bool,       // True if we synthetically closed due to missing exit tx
    pub closed_reason: Option<String>, // Optional reason for closure (e.g., "synthetic_phantom_closure")
    
    // ==================== PARTIAL EXIT & DCA SUPPORT ====================
    // Partial exit tracking
    pub remaining_token_amount: Option<u64>, // Current holdings after partial exits
    pub total_exited_amount: u64,            // Cumulative tokens sold
    pub average_exit_price: Option<f64>,     // Weighted average exit price
    pub partial_exit_count: u32,             // Number of partial exits executed
    
    // DCA tracking
    pub dca_count: u32,                      // Number of additional entries (DCA)
    pub average_entry_price: f64,            // Weighted average entry price (all entries)
    pub last_dca_time: Option<DateTime<Utc>>, // Last DCA timestamp for cooldown
}

// ==================== EXIT & ENTRY HISTORY ====================

/// Record of a single exit (partial or full)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExitRecord {
    pub id: Option<i64>,              // Database ID
    pub position_id: i64,             // Parent position ID
    pub timestamp: DateTime<Utc>,
    pub amount: u64,                  // Tokens sold
    pub price: f64,                   // Exit price per token
    pub sol_received: f64,            // SOL received
    pub transaction_signature: String,
    pub is_partial: bool,             // true if partial, false if full exit
    pub percentage: f64,              // % of position sold at this exit
    pub fees_lamports: Option<u64>,  // Transaction fee
}

/// Record of a single entry (initial or DCA)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EntryRecord {
    pub id: Option<i64>,              // Database ID
    pub position_id: i64,             // Parent position ID
    pub timestamp: DateTime<Utc>,
    pub amount: u64,                  // Tokens bought
    pub price: f64,                   // Entry price per token
    pub sol_spent: f64,               // SOL spent
    pub transaction_signature: String,
    pub is_dca: bool,                 // true if DCA, false if initial entry
    pub fees_lamports: Option<u64>,  // Transaction fee
}
