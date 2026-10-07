// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Shared agent-control API (`/api/agent-control`).
//!
//! Owns the capability registry's tool list, the per-category permission
//! policy, durable client pairings, the external-agent approval queue and the
//! audit read API. Every route here stays behind the normal dashboard
//! security/auth gates. The live-app bridge that external agents actually call
//! is a separate, narrowly-exempted module (`routes::agent_bridge`).

use axum::{
    response::{IntoResponse as _, Response},
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

use crate::errors::ErrorClass;
use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::state::AppState;

mod approvals;
mod handlers;
mod pairings;

use handlers::{get_permissions, list_tools, update_permissions};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tools", get(list_tools))
        .route("/permissions", get(get_permissions))
        .route("/permissions", patch(update_permissions))
        .route("/pairings", get(pairings::list).post(pairings::create))
        .route("/pairings/{client_id}", delete(pairings::revoke))
        .route(
            "/pairings/{client_id}/permissions",
            patch(pairings::update_permissions),
        )
        .route("/approvals", get(approvals::list_pending))
        .route("/approvals/{id}/decide", post(approvals::decide))
        .route("/audit", get(approvals::list_audit))
}

/// Response for an agent-control error. The category and HTTP status come from
/// the error value (`http_status`), never from matching prose; the message is
/// the catalog entry for the failed operation and the error's own text goes to
/// `details`.
pub(crate) fn failure(error: &crate::agent_control::Error) -> Response {
    use crate::agent_control::Error;
    let (code, id) = match error {
        Error::Config(_) => (
            ApiErrorCode::for_status(error.http_status()),
            ids::ERRORS_AGENT_CONFIG_FAILED,
        ),
        Error::InvalidParameters { .. } => (
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AGENT_INVALID_PARAMETERS,
        ),
        Error::SecretPath { .. } => (
            ApiErrorCode::Forbidden,
            ids::ERRORS_AGENT_WALLET_KEY_MATERIAL,
        ),
        Error::Database(_) => (
            ApiErrorCode::for_status(error.http_status()),
            ids::ERRORS_AGENT_STORE_FAILED,
        ),
        Error::InvalidPairingRequest { .. } => (
            ApiErrorCode::InvalidInput,
            ids::ERRORS_AGENT_INVALID_PAIRING_REQUEST,
        ),
        Error::PairingRejected => (
            ApiErrorCode::Unauthorized,
            ids::ERRORS_AGENT_PAIRING_REJECTED,
        ),
        Error::Disabled => (
            ApiErrorCode::AgentControlDisabled,
            ids::ERRORS_AGENT_DISABLED,
        ),
        Error::ApprovalNotPending => (
            ApiErrorCode::Conflict,
            ids::ERRORS_AGENT_APPROVAL_NOT_PENDING,
        ),
        Error::ApprovalNotFound => (ApiErrorCode::NotFound, ids::ERRORS_AGENT_APPROVAL_NOT_FOUND),
        Error::TaskEnded { .. } => (ApiErrorCode::Internal, ids::ERRORS_AGENT_TASK_FAILED),
    };
    ApiError::new(code, id)
        .details(error.to_string())
        .into_response()
}

/// Response for a blocking agent-control task that failed to complete.
pub(crate) fn task_failed() -> Response {
    ApiError::new(ApiErrorCode::Internal, ids::ERRORS_AGENT_TASK_FAILED).into_response()
}
