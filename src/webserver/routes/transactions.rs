// Transactions API routes
//
// Provides endpoints for listing, filtering, and viewing transaction details

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::transactions::{
    database::{TransactionCursor, TransactionListFilters, TransactionListRow},
    get_transaction, get_transaction_database,
};
use crate::webserver::state::AppState;

// =============================================================================
// REQUEST/RESPONSE TYPES (inline per repo conventions)
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ListTransactionsRequest {
    #[serde(default)]
    pub filters: TransactionListFilters,
    #[serde(default)]
    pub pagination: PaginationParams,
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

// =============================================================================
// ROUTE HANDLERS
// =============================================================================

/// POST /api/transactions/list - List transactions with filters and pagination
async fn list_transactions(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ListTransactionsRequest>,
) -> Json<ListTransactionsResponse> {
    let db = match get_transaction_database().await {
        Some(db) => db,
        None => {
            return Json(ListTransactionsResponse {
                items: vec![],
                next_cursor: None,
                total_estimate: Some(0),
            });
        }
    };

    let result = match db
        .list_transactions(
            &request.filters,
            request.pagination.cursor.as_ref(),
            request.pagination.limit,
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return Json(ListTransactionsResponse {
                items: vec![],
                next_cursor: None,
                total_estimate: Some(0),
            });
        }
    };

    Json(ListTransactionsResponse {
        items: result.items,
        next_cursor: result.next_cursor,
        total_estimate: result.total_estimate,
    })
}

/// GET /api/transactions/:signature - Get full transaction details
async fn get_transaction_detail(
    State(_state): State<Arc<AppState>>,
    Path(signature): Path<String>,
) -> Json<Option<TransactionDetailResponse>> {
    match get_transaction(&signature).await {
        Ok(Some(tx)) => Json(Some(TransactionDetailResponse::from(tx))),
        _ => Json(None),
    }
}

/// POST /api/transactions/summary - Get transaction summary/KPIs
async fn get_summary(State(state): State<Arc<AppState>>) -> Json<TransactionSummaryResponse> {
    let db = match get_transaction_database().await {
        Some(db) => db,
        None => {
            return Json(TransactionSummaryResponse {
                total: 0,
                success_count: 0,
                failed_count: 0,
                pending_global: 0,
                pending_local: 0,
                deferred_count: 0,
                success_rate: 0.0,
                failure_rate: 0.0,
                newest_known_signature: None,
                oldest_known_signature: None,
                db_size_mb: 0.0,
                db_schema_version: 0,
                bootstrap_state: BootstrapStateInfo {
                    backfill_cursor: None,
                    full_history_completed: false,
                },
            });
        }
    };

    // Get DB stats
    let db_stats = match db.get_stats().await {
        Ok(s) => s,
        Err(_) => {
            return Json(TransactionSummaryResponse {
                total: 0,
                success_count: 0,
                failed_count: 0,
                pending_global: 0,
                pending_local: 0,
                deferred_count: 0,
                success_rate: 0.0,
                failure_rate: 0.0,
                newest_known_signature: None,
                oldest_known_signature: None,
                db_size_mb: 0.0,
                db_schema_version: 0,
                bootstrap_state: BootstrapStateInfo {
                    backfill_cursor: None,
                    full_history_completed: false,
                },
            });
        }
    };

    // Get counts
    let total = db_stats.total_raw_transactions;
    let success_count = db.get_successful_transactions_count().await.unwrap_or(0);
    let failed_count = db.get_failed_transactions_count().await.unwrap_or(0);

    let success_rate = if total > 0 {
        (success_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let failure_rate = if total > 0 {
        (failed_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Get bootstrap state
    let bootstrap = db.get_bootstrap_state().await.unwrap_or_default();

    // Get pending counts from DB
    let pending_global = db_stats.total_pending_transactions as usize;
    let pending_local = 0; // TODO: Get from TransactionsManager if exposed
    let deferred_count = db_stats.total_deferred_retries as usize;

    // Get newest/oldest known signatures
    let newest_known_signature = db.get_newest_known_signature().await.ok().flatten();
    let oldest_known_signature = db.get_oldest_known_signature().await.ok().flatten();

    Json(TransactionSummaryResponse {
        total,
        success_count,
        failed_count,
        pending_global,
        pending_local,
        deferred_count,
        success_rate,
        failure_rate,
        newest_known_signature,
        oldest_known_signature,
        db_size_mb: db_stats.database_size_bytes as f64 / (1024.0 * 1024.0),
        db_schema_version: db_stats.schema_version,
        bootstrap_state: BootstrapStateInfo {
            backfill_cursor: bootstrap.backfill_before_cursor,
            full_history_completed: bootstrap.full_history_completed,
        },
    })
}

// =============================================================================
// ROUTER SETUP
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/list", post(list_transactions))
        .route("/summary", post(get_summary))
        .route("/:signature", get(get_transaction_detail))
}
