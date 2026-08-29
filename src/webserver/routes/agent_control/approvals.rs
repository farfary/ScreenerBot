//! Dashboard-authenticated approval review and the audit read API
//! (`/api/agent-control/approvals`, `/api/agent-control/audit`).
//!
//! The human sees a pending external-agent request — client label, tool, a
//! redacted argument summary, expiry — and approves or denies it here, inside
//! ScreenerBot. The external caller has no route to these handlers, so it can
//! never approve its own request. Approval executes the stored canonical
//! request exactly once in the live process.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::agent_control::{approvals, audit, bridge};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, status_for, success_response};

use super::error_code;

#[derive(Debug, Deserialize)]
pub struct DecideBody {
    pub approve: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "one")]
    pub page: u32,
    #[serde(default = "fifty")]
    pub per_page: u32,
}
fn one() -> u32 {
    1
}
fn fifty() -> u32 {
    50
}

/// GET /api/agent-control/approvals — pending external-agent requests.
pub async fn list_pending(State(_state): State<Arc<AppState>>) -> Response {
    match tokio::task::spawn_blocking(approvals::list_pending).await {
        Ok(Ok(rows)) => success_response(rows),
        Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        Err(_) => internal(),
    }
}

/// POST /api/agent-control/approvals/:id/decide — approve or deny exactly once.
pub async fn decide(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<DecideBody>,
) -> Response {
    if body.approve {
        match bridge::execute_approved(&id).await {
            Ok(()) => {
                logger::info(
                    LogTag::Security,
                    &format!("agent-control: approved and executed request {id}"),
                );
                success_response(serde_json::json!({ "resolved": "approved" }))
            }
            Err(e) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        }
    } else {
        let id_for_log = id.clone();
        match tokio::task::spawn_blocking(move || bridge::deny_approval(&id)).await {
            Ok(Ok(())) => {
                logger::info(
                    LogTag::Security,
                    &format!("agent-control: denied request {id_for_log}"),
                );
                success_response(serde_json::json!({ "resolved": "denied" }))
            }
            Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
            Err(_) => internal(),
        }
    }
}

/// GET /api/agent-control/audit — paginated, bounded audit log (newest first).
pub async fn list_audit(
    State(_state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Response {
    let AuditQuery { page, per_page } = q;
    match tokio::task::spawn_blocking(move || audit::list(page, per_page)).await {
        Ok(Ok((rows, total))) => success_response(serde_json::json!({
            "audit": rows,
            "total": total,
            "page": page.max(1),
            "per_page": per_page.clamp(1, 200),
        })),
        Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        Err(_) => internal(),
    }
}

fn internal() -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL",
        "agent-control task failed",
        None,
    )
}
