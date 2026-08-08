//! Route handlers for transactions API.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

use crate::transactions::{get_transaction_database, Subject};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};

use super::types::*;

/// POST /api/transactions/list - List transactions with filters and pagination
pub(super) async fn list_transactions(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<ListTransactionsRequest>,
) -> Response {
    let subject = match resolve_subject(request.subject.as_deref()).await {
        Ok(subject) => subject,
        Err(response) => return response,
    };
    let db = match get_transaction_database().await {
        Some(db) => db,
        None => {
            return success_response(ListTransactionsResponse {
                items: vec![],
                next_cursor: None,
                total_estimate: Some(0),
            });
        }
    };

    let result = match db
        .list_transactions_for_subject(
            subject,
            &request.filters,
            request.pagination.cursor.as_ref(),
            request.pagination.limit,
        )
        .await
    {
        Ok(r) => r,
        Err(_) => {
            return success_response(ListTransactionsResponse {
                items: vec![],
                next_cursor: None,
                total_estimate: Some(0),
            });
        }
    };

    let total_estimate = db
        .count_transactions_for_subject(subject, &request.filters)
        .await
        .ok()
        .or(result.total_estimate);
    success_response(ListTransactionsResponse {
        items: result.items,
        next_cursor: result.next_cursor,
        total_estimate,
    })
}

/// GET /api/transactions/:signature - Get full transaction details
pub(super) async fn get_transaction_detail(
    State(_state): State<Arc<AppState>>,
    Path(signature): Path<String>,
    Query(query): Query<TransactionSubjectQuery>,
) -> Response {
    let subject = match resolve_subject(query.subject.as_deref()).await {
        Ok(subject) => subject,
        Err(response) => return response,
    };
    let Some(db) = get_transaction_database().await else {
        return success_response(Option::<TransactionDetailResponse>::None);
    };
    match db.get_transaction_for_subject(subject, &signature).await {
        Ok(Some(tx)) => success_response(Some(TransactionDetailResponse::from(tx))),
        _ => success_response(Option::<TransactionDetailResponse>::None),
    }
}

/// POST /api/transactions/summary - Get transaction summary/KPIs
pub(super) async fn get_summary(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<TransactionSubjectQuery>,
) -> Response {
    let subject = match resolve_subject(query.subject.as_deref()).await {
        Ok(subject) => subject,
        Err(response) => return response,
    };
    let own_subject = Subject::own().ok();
    let is_own = own_subject == Some(subject);
    let db = match get_transaction_database().await {
        Some(db) => db,
        None => {
            return success_response(TransactionSummaryResponse {
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
            return success_response(TransactionSummaryResponse {
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
    let total = db
        .count_transactions_for_subject(subject, &Default::default())
        .await
        .unwrap_or_default();
    let success_count = db
        .get_successful_transactions_count_for_subject(subject)
        .await
        .unwrap_or_default();
    let failed_count = db
        .get_failed_transactions_count_for_subject(subject)
        .await
        .unwrap_or_default();

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
    let bootstrap = if is_own {
        db.get_bootstrap_state().await.unwrap_or_default()
    } else {
        Default::default()
    };

    // Get pending counts from DB
    let pending_global = db
        .get_pending_transactions_count(subject)
        .await
        .unwrap_or_default() as usize;
    let pending_local = 0; // TODO: Get from TransactionsManager if exposed
    let deferred_count = if is_own {
        db_stats.total_deferred_retries as usize
    } else {
        0
    };

    // Get newest/oldest known signatures for the selected subject.
    let newest_known_signature = db.get_newest_known_signature(subject).await.ok().flatten();
    let oldest_known_signature = db.get_oldest_known_signature(subject).await.ok().flatten();

    success_response(TransactionSummaryResponse {
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

async fn resolve_subject(requested: Option<&str>) -> Result<Subject, Response> {
    let own = Subject::own().map_err(|e| {
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "OWN_WALLET_UNAVAILABLE",
            "The main wallet is not configured",
            Some(&e.to_string()),
        )
    })?;
    let Some(address) = requested.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(own);
    };
    if address == own.address() {
        return Ok(own);
    }

    let pubkey = Pubkey::from_str(address).map_err(|_| {
        error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_SUBJECT",
            "Transaction subject is not a valid Solana address",
            None,
        )
    })?;
    match crate::wallets::watch::get_target_by_address(address).await {
        Ok(Some(_)) => Ok(Subject(pubkey)),
        Ok(None) => Err(error_response(
            StatusCode::FORBIDDEN,
            "SUBJECT_NOT_WATCHED",
            "Transaction subject is not a watched wallet",
            None,
        )),
        Err(e) => Err(error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "WATCH_STORE_UNAVAILABLE",
            "Watched wallets are not available",
            Some(&e),
        )),
    }
}
