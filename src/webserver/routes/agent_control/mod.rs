//! Shared agent-control API (`/api/agent-control`).
//!
//! Owns the capability registry's tool list, the per-category permission
//! policy, durable client pairings, the external-agent approval queue and the
//! audit read API. Every route here stays behind the normal dashboard
//! security/auth gates. The live-app bridge that external agents actually call
//! is a separate, narrowly-exempted module (`routes::agent_bridge`).

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use std::sync::Arc;

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
        .route("/pairings/:client_id", delete(pairings::revoke))
        .route(
            "/pairings/:client_id/permissions",
            patch(pairings::update_permissions),
        )
        .route("/approvals", get(approvals::list_pending))
        .route("/approvals/:id/decide", post(approvals::decide))
        .route("/audit", get(approvals::list_audit))
}

/// Stable error-code string for an agent-control error, for the API envelope.
/// The HTTP status still comes from `status_for` (the error value), never from
/// matching prose.
pub(crate) fn error_code(error: &crate::agent_control::Error) -> &'static str {
    use crate::agent_control::Error;
    match error {
        Error::Config(_) => "CONFIG_ERROR",
        Error::InvalidParameters { .. } => "INVALID_PARAMETERS",
        Error::SecretPath { .. } => "WALLET_KEY_MATERIAL",
        Error::Database(_) => "STORE_ERROR",
        Error::InvalidPairingRequest { .. } => "INVALID_PAIRING_REQUEST",
        Error::PairingRejected => "PAIRING_REJECTED",
        Error::Disabled => "AGENT_CONTROL_DISABLED",
        Error::ApprovalNotPending => "APPROVAL_NOT_PENDING",
        Error::ApprovalNotFound => "APPROVAL_NOT_FOUND",
    }
}
