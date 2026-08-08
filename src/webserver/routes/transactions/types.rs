//! Request/Response types for transactions API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::transactions::{
    database::TransactionCursor, database::TransactionListFilters, database::TransactionListRow,
};

#[derive(Debug, Deserialize)]
pub struct ListTransactionsRequest {
    /// Omitted for the bot's own wallet; otherwise a currently watched address.
    pub subject: Option<String>,
    #[serde(default)]
    pub filters: TransactionListFilters,
    #[serde(default)]
    pub pagination: PaginationParams,
}

#[derive(Debug, Default, Deserialize)]
pub struct TransactionSubjectQuery {
    pub subject: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub cursor: Option<TransactionCursor>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: default_limit(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListTransactionsResponse {
    pub items: Vec<TransactionListRow>,
    pub next_cursor: Option<TransactionCursor>,
    pub total_estimate: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TransactionSummaryResponse {
    pub total: u64,
    pub success_count: u64,
    pub failed_count: u64,
    pub pending_global: usize,
    pub pending_local: usize,
    pub deferred_count: usize,
    pub success_rate: f64,
    pub failure_rate: f64,
    pub newest_known_signature: Option<String>,
    pub oldest_known_signature: Option<String>,
    pub db_size_mb: f64,
    pub db_schema_version: u32,
    pub bootstrap_state: BootstrapStateInfo,
}

#[derive(Debug, Serialize)]
pub struct BootstrapStateInfo {
    pub backfill_cursor: Option<String>,
    pub full_history_completed: bool,
}

/// Full transaction detail response - includes all analysis fields
/// This bypasses the skip_serializing attributes on Transaction struct
#[derive(Debug, Serialize)]
pub struct TransactionDetailResponse {
    pub signature: String,
    pub slot: Option<u64>,
    pub block_time: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub status: crate::transactions::TransactionStatus,
    pub transaction_type: crate::transactions::TransactionType,
    pub direction: crate::transactions::TransactionDirection,
    pub success: bool,
    pub error_message: Option<String>,
    pub fee_sol: f64,
    pub fee_lamports: Option<u64>,
    pub compute_units_consumed: Option<u64>,
    pub instructions_count: usize,
    pub accounts_count: usize,
    pub sol_balance_change: f64,
    pub token_transfers: Vec<crate::transactions::TokenTransfer>,
    pub raw_transaction_data: Option<serde_json::Value>,
    pub log_messages: Vec<String>,
    pub instructions: Vec<crate::transactions::InstructionInfo>,
    pub instruction_info: Vec<crate::transactions::InstructionInfo>,
    pub sol_balance_changes: Vec<crate::transactions::SolBalanceChange>,
    pub token_balance_changes: Vec<crate::transactions::TokenBalanceChange>,
    pub ata_operations: Vec<crate::transactions::AtaOperation>,
    pub token_swap_info: Option<crate::transactions::TokenSwapInfo>,
    pub swap_pnl_info: Option<crate::transactions::SwapPnLInfo>,
    pub analysis_duration_ms: Option<u64>,
    pub last_updated: DateTime<Utc>,
}

impl From<crate::transactions::Transaction> for TransactionDetailResponse {
    fn from(tx: crate::transactions::Transaction) -> Self {
        Self {
            signature: tx.signature,
            slot: tx.slot,
            block_time: tx.block_time,
            timestamp: tx.timestamp,
            status: tx.status,
            transaction_type: tx.transaction_type,
            direction: tx.direction,
            success: tx.success,
            error_message: tx.error_message,
            fee_sol: tx.fee_sol,
            fee_lamports: tx.fee_lamports,
            compute_units_consumed: tx.compute_units_consumed,
            instructions_count: tx.instructions_count,
            accounts_count: tx.accounts_count,
            sol_balance_change: tx.sol_balance_change,
            token_transfers: tx.token_transfers,
            raw_transaction_data: tx.raw_transaction_data,
            log_messages: tx.log_messages,
            instructions: tx.instructions.clone(),
            instruction_info: tx.instruction_info,
            sol_balance_changes: tx.sol_balance_changes,
            token_balance_changes: tx.token_balance_changes,
            ata_operations: tx.ata_operations,
            token_swap_info: tx.token_swap_info,
            swap_pnl_info: tx.swap_pnl_info,
            analysis_duration_ms: tx.analysis_duration_ms,
            last_updated: tx.last_updated,
        }
    }
}
