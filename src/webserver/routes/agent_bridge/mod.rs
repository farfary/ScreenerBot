//! The live-app bridge (`/api/agent-bridge/*`).
//!
//! This is the ONLY agent-control surface reachable without the GUI security
//! token / headless session cookie, and the exemption is deliberately narrow:
//! `middleware::is_security_token_exempt_path` and `auth_gate` allow exactly
//! this path prefix and nothing wider.
//!
//! What still protects it:
//! - **Every handler authenticates a mandatory pairing bearer credential
//!   unconditionally, in every mode** — the exemption removes only the dashboard
//!   token/cookie, never the pairing check. This credential, not any network
//!   position, is what guards the surface.
//! - GUI mode: `security_gate` additionally runs its loopback `Host`/`Origin`
//!   check on this path like any other.
//! - Headless mode: `security_gate` and its `Host`/`Origin` check do not run,
//!   and with dashboard password auth enabled the server may be bound to a
//!   non-loopback address — so loopback is NOT guaranteed here. The pairing
//!   credential above is the sole protection in that mode.
//! - `initialization_gate` is NOT exempted: pre-init the bridge returns 503.
//!
//! It is plain JSON, NOT Streamable HTTP MCP — no MCP transport is mounted on
//! the network anywhere.
//!
//! The stdio MCP subprocess reads `agent-runtime.json` only to discover the
//! loopback URL, then calls these routes with its client id + pairing secret
//! (from the environment, never CLI arguments).

use axum::{routing::post, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod handlers;

/// Path prefix, kept in one place so the middleware exemptions and the
/// architecture test reference the same string.
pub const BRIDGE_PREFIX: &str = "/api/agent-bridge/";

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/ping", post(handlers::ping))
        .route("/list-tools", post(handlers::list_tools))
        .route("/call-tool", post(handlers::call_tool))
        .route("/approval-status", post(handlers::approval_status))
}
