//! Transaction database types — row structs for SQLite serialization.
//
// Type definitions for database operations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// =============================================================================
// LIST/FILTER TYPES FOR UI
// =============================================================================

/// Cursor for pagination (timestamp desc, signature desc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCursor {
    pub timestamp: String, // RFC3339 format
    pub signature: String,
}

/// Filters for listing transactions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionListFilters {
    /// Transaction types to include: ["buy", "sell", "swap", "transfer", "ata", "failed", "unknown"]
    #[serde(default)]
    pub types: Vec<String>,

    /// Filter by token mint (partial match)
    pub mint: Option<String>,

    /// Only confirmed/finalized transactions
    pub only_confirmed: Option<bool>,

    /// Filter by direction: "Incoming", "Outgoing", "Internal", "Unknown"
    pub direction: Option<String>,

    /// Filter by status: "Pending", "Confirmed", "Finalized", "Failed"
    pub status: Option<String>,

    /// Filter by signature (partial match)
    pub signature: Option<String>,

    /// Time range (RFC3339)
    pub time_from: Option<DateTime<Utc>>,
    pub time_to: Option<DateTime<Utc>>,

    /// Filter by router (partial match)
    pub router: Option<String>,

    /// SOL delta range
    pub min_sol: Option<f64>,
    pub max_sol: Option<f64>,
}

/// Lightweight transaction row for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListRow {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub slot: Option<u64>,
    pub status: String,
    pub success: bool,
    pub direction: Option<String>,
    pub transaction_type: Option<String>,
    pub token_mint: Option<String>,
    pub token_symbol: Option<String>,
    pub router: Option<String>,
    pub sol_delta: f64,
    pub token_amount: Option<f64>,
    pub fee_sol: f64,
    pub fee_lamports: Option<u64>,
    pub ata_rents: f64,
    pub instructions_count: usize,
}

/// Result of list_transactions query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionListResult {
    pub items: Vec<TransactionListRow>,
    pub next_cursor: Option<TransactionCursor>,
    pub total_estimate: Option<u64>,
}

// =============================================================================
// DATABASE STATISTICS AND REPORTING
// =============================================================================

/// Statistics about database operations and contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_raw_transactions: u64,
    pub total_processed_transactions: u64,
    pub total_known_signatures: u64,
    pub total_deferred_retries: u64,
    pub total_pending_transactions: u64,
    pub database_size_bytes: u64,
    pub schema_version: u32,
    pub last_updated: DateTime<Utc>,
}

/// Database integrity check results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub raw_transactions_count: u64,
    pub processed_transactions_count: u64,
    pub orphaned_processed_transactions: u64,
    pub missing_processed_transactions: u64,
    pub schema_version_correct: bool,
    pub foreign_key_violations: u64,
    pub index_integrity_ok: bool,
    pub pending_transactions_count: u64,
}

/// Minimal row for wallet flow cache export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletFlowExportRow {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub sol_delta: f64,
}
