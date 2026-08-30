//! Dashboard-authenticated pairing management (`/api/agent-control/pairings`).
//!
//! These routes stay fully behind the normal dashboard security/auth gates.
//! The create response exposes the one-time pairing secret; every later read
//! returns only non-secret metadata. All DB work runs on a blocking thread so
//! the async runtime is never parked on a connection lock.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::agent_control::{pairing, ToolPermissions};
use crate::logger::{self, LogTag};
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, status_for, success_response};

use super::error_code;

#[derive(Debug, Deserialize)]
pub struct CreatePairingBody {
    pub label: String,
    pub agent_kind: String,
    /// The connection's own per-category policy. Omitted means the default a
    /// new connection gets: full access, limited later from this same tab.
    #[serde(default)]
    pub permissions: Option<ToolPermissions>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePermissionsBody {
    pub permissions: ToolPermissions,
}

/// One-time pairing material plus the exact executable this running app was
/// launched from. Electron packages the backend as a stable resources binary,
/// while headless installs already execute the durable binary directly, so
/// `current_exe` is the canonical stdio command on every supported platform.
#[derive(Serialize)]
struct CreatePairingResponse {
    client_id: String,
    pairing_secret: String,
    binary_path: Option<String>,
}

fn current_binary_path() -> Option<String> {
    std::env::current_exe()
        .ok()?
        .into_os_string()
        .into_string()
        .ok()
}

/// GET /api/agent-control/pairings — list pairings (never the verifier/secret).
pub async fn list(State(_state): State<Arc<AppState>>) -> Response {
    match tokio::task::spawn_blocking(pairing::list).await {
        Ok(Ok(rows)) => success_response(rows),
        Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        Err(_) => internal(),
    }
}

/// POST /api/agent-control/pairings — create a pairing, returning the secret ONCE.
pub async fn create(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<CreatePairingBody>,
) -> Response {
    let CreatePairingBody {
        label,
        agent_kind,
        permissions,
    } = body;
    let result =
        tokio::task::spawn_blocking(move || pairing::create(&label, &agent_kind, permissions))
            .await;

    match result {
        Ok(Ok(new_pairing)) => {
            logger::info(
                LogTag::Security,
                &format!(
                    "agent-control: created pairing {} (kind unchanged, secret shown once)",
                    new_pairing.client_id
                ),
            );
            success_response(CreatePairingResponse {
                client_id: new_pairing.client_id,
                pairing_secret: new_pairing.pairing_secret,
                binary_path: current_binary_path(),
            })
        }
        Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        Err(_) => internal(),
    }
}

#[cfg(test)]
mod tests {
    use super::current_binary_path;

    #[test]
    fn setup_binary_path_is_absolute_when_the_platform_path_is_utf8() {
        if let Some(path) = current_binary_path() {
            assert!(std::path::Path::new(&path).is_absolute());
        }
    }
}

/// DELETE /api/agent-control/pairings/:client_id — revoke; effective on the
/// next bridge request.
pub async fn revoke(
    State(_state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
) -> Response {
    let id_for_log = client_id.clone();
    match tokio::task::spawn_blocking(move || pairing::revoke(&client_id)).await {
        Ok(Ok(true)) => {
            logger::info(
                LogTag::Security,
                &format!("agent-control: revoked pairing {id_for_log}"),
            );
            success_response(serde_json::json!({ "revoked": true }))
        }
        Ok(Ok(false)) => error_response(
            StatusCode::NOT_FOUND,
            "PAIRING_NOT_FOUND",
            "No active pairing with that id",
            None,
        ),
        Ok(Err(e)) => error_response(status_for(&e), error_code(&e), &e.to_string(), None),
        Err(_) => internal(),
    }
}

/// PATCH /api/agent-control/pairings/:client_id/permissions — limit (or widen)
/// what one connection may do. Effective on its next request.
pub async fn update_permissions(
    State(_state): State<Arc<AppState>>,
    Path(client_id): Path<String>,
    Json(body): Json<UpdatePermissionsBody>,
) -> Response {
    let id_for_log = client_id.clone();
    let permissions = body.permissions;
    match tokio::task::spawn_blocking(move || pairing::set_permissions(&client_id, permissions))
        .await
    {
        Ok(Ok(true)) => {
            logger::info(
                LogTag::Security,
                &format!("agent-control: updated permissions for pairing {id_for_log}"),
            );
            success_response(serde_json::json!({ "permissions": permissions }))
        }
        Ok(Ok(false)) => error_response(
            StatusCode::NOT_FOUND,
            "PAIRING_NOT_FOUND",
            "No active pairing with that id",
            None,
        ),
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
