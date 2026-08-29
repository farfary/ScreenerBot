//! Bridge handlers. Every one authenticates a pairing credential before it
//! does anything else; the path exemption only removes the GUI token / session
//! cookie, never the pairing check.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::agent_control::{bridge, Error};
use crate::webserver::routes::agent_control::error_code;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, status_for, success_response};

const CLIENT_HEADER: &str = "x-screenerbot-client";
const SECRET_HEADER: &str = "x-screenerbot-pairing-secret";

/// Pull the pairing credential out of the request headers. A missing or
/// non-ASCII header is treated exactly like a wrong secret — one opaque 401,
/// no oracle.
fn credential(headers: &HeaderMap) -> Result<(String, String), Response> {
    let read = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .filter(|s| !s.is_empty())
    };
    match (read(CLIENT_HEADER), read(SECRET_HEADER)) {
        (Some(client), Some(secret)) => Ok((client, secret)),
        _ => Err(reject(&Error::PairingRejected)),
    }
}

fn reject(error: &Error) -> Response {
    error_response(
        status_for(error),
        error_code(error),
        &error.to_string(),
        None,
    )
}

#[derive(Debug, Deserialize)]
pub struct CallToolBody {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
    pub correlation_id: String,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalStatusBody {
    pub approval_id: String,
}

/// POST /api/agent-bridge/ping — liveness + pairing probe for `mcp doctor`.
pub async fn ping(State(_s): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let (client, secret) = match credential(&headers) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match tokio::task::spawn_blocking(move || bridge::ping(&client, &secret)).await {
        Ok(Ok(info)) => success_response(info),
        Ok(Err(e)) => reject(&e),
        Err(_) => task_failed(),
    }
}

/// POST /api/agent-bridge/list-tools — the capabilities this paired client may
/// see. Authenticated against the LIVE app, not a local shortcut.
pub async fn list_tools(State(_s): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let (client, secret) = match credential(&headers) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match tokio::task::spawn_blocking(move || bridge::list_tools(&client, &secret)).await {
        Ok(Ok(tools)) => success_response(serde_json::json!({ "tools": tools })),
        Ok(Err(e)) => reject(&e),
        Err(_) => task_failed(),
    }
}

/// POST /api/agent-bridge/call-tool — execute, deny, or park on the approval
/// queue. Runs the canonical registry tool in the live process.
pub async fn call_tool(
    State(_s): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<CallToolBody>,
) -> Response {
    let (client, secret) = match credential(&headers) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let CallToolBody {
        name,
        arguments,
        correlation_id,
    } = body;
    match bridge::call_tool(&client, &secret, &name, arguments, &correlation_id).await {
        Ok(outcome) => success_response(outcome),
        Err(e) => reject(&e),
    }
}

/// POST /api/agent-bridge/approval-status — poll one approval; owning client
/// only.
pub async fn approval_status(
    State(_s): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ApprovalStatusBody>,
) -> Response {
    let (client, secret) = match credential(&headers) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let approval_id = body.approval_id;
    match tokio::task::spawn_blocking(move || {
        bridge::approval_status(&client, &secret, &approval_id)
    })
    .await
    {
        Ok(Ok(handle)) => success_response(handle),
        Ok(Err(e)) => reject(&e),
        Err(_) => task_failed(),
    }
}

fn task_failed() -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL",
        "agent-control bridge task failed",
        None,
    )
}
