// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Dashboard-authenticated approval review and the audit read API
//! (`/api/agent-control/approvals`, `/api/agent-control/audit`).
//!
//! The human sees a pending external-agent request — client label, tool, a
//! redacted argument summary, expiry — and approves or denies it here, inside
//! ScreenerBot. The external caller has no route to these handlers, so it can
//! never approve its own request. Approval claims the stored canonical request
//! exactly once and runs it on its own task in the live process.

use axum::{
    extract::{Path, Query, State},
    response::Response,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::agent_control::{approvals, audit, bridge};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::success_response;

use super::{failure, task_failed};

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
        Ok(Err(e)) => failure(&e),
        Err(_) => task_failed(),
    }
}

/// POST /api/agent-control/approvals/:id/decide — approve or deny exactly once.
pub async fn decide(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<DecideBody>,
) -> Response {
    if body.approve {
        // The approved tool may be a trade. The request is claimed and checked
        // here; it then runs on its own task, so neither a dropped dashboard
        // request nor a slow swap holds the decision. The MCP client reads the
        // outcome from the approval row.
        let approval_id = id.clone();
        match tokio::task::spawn_blocking(move || bridge::start_approved(&approval_id)).await {
            Ok(Ok(started)) => {
                if let Some(approved) = started {
                    let run_id = id.clone();
                    tokio::spawn(async move {
                        if let Err(e) = approved.run().await {
                            logger::error(
                                LogTag::Security,
                                &format!(
                                    "agent-control: approved request {run_id} ran but its outcome was not stored: {e}"
                                ),
                            );
                        }
                    });
                }
                logger::info(
                    LogTag::Security,
                    &format!("agent-control: approved request {id}"),
                );
                success_response(serde_json::json!({ "resolved": "approved" }))
            }
            Ok(Err(e)) => failure(&e),
            Err(_) => task_failed(),
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
            Ok(Err(e)) => failure(&e),
            Err(_) => task_failed(),
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
        Ok(Err(e)) => failure(&e),
        Err(_) => task_failed(),
    }
}
