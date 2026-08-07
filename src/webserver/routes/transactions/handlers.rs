//! Route handlers for transactions API.

use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;

use crate::transactions::{get_transaction, get_transaction_database};
use crate::webserver::state::AppState;

use super::types::*;

/// POST /api/transactions/list - List transactions with filters and pagination
pub(super) async fn list_transactions(
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
pub(super) async fn get_transaction_detail(
    State(_state): State<Arc<AppState>>,
    Path(signature): Path<String>,
) -> Json<Option<TransactionDetailResponse>> {
    match get_transaction(&signature).await {
        Ok(Some(tx)) => Json(Some(TransactionDetailResponse::from(tx))),
        _ => Json(None),
    }
}

/// POST /api/transactions/summary - Get transaction summary/KPIs
pub(super) async fn get_summary(
    State(state): State<Arc<AppState>>,
) -> Json<TransactionSummaryResponse> {
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
    let success_count = db
        .get_successful_transactions_count()
        .await
        .unwrap_or_default();
    let failed_count = db.get_failed_transactions_count().await.unwrap_or_default();

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

    // Get newest/oldest known signatures. These are scoped to a subject, and this
    // summary is our own transaction history -- not any other wallet we may watch.
    let own_subject = crate::transactions::Subject::own().ok();
    let newest_known_signature = match own_subject {
        Some(subject) => db.get_newest_known_signature(subject).await.ok().flatten(),
        None => None,
    };
    let oldest_known_signature = match own_subject {
        Some(subject) => db.get_oldest_known_signature(subject).await.ok().flatten(),
        None => None,
    };

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
