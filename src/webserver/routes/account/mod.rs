//! Dashboard routes for the optional ScreenerBot account.
//!
//! ============================================================================
//! WHERE THESE ROUTES LIVE, AND WHY IT MATTERS
//! ============================================================================
//! Everything here is under `/api/account`, which is INSIDE the GUI security
//! token gate (`webserver/middleware.rs`). That placement is load-bearing.
//! The setup screen is exactly where the sign-in panel lives, which makes
//! hanging these endpoints off the initialization namespace an obvious
//! shortcut, but every API that changes state remains inside the token gate.
//!
//! It would also publish the sign-in API to every website the user has open: a
//! page in another tab can POST to `http://127.0.0.1:<port>` all day, and the
//! token header is the only thing that stops it. `initialization_gate` is
//! extended for `/api/account` instead, so these routes work before setup
//! completes while still requiring the token.
//!
//! The ONE exception is the OAuth callback, which is registered on the root
//! router rather than here — see `handlers::oauth_callback` for why it can be
//! exempt safely.

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::webserver::state::AppState;

pub mod handlers;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/status", get(handlers::account_status))
        .route("/signin/browser", post(handlers::start_browser_signin))
        .route("/signup", post(handlers::open_signup))
        .route("/signin/password", post(handlers::signin_with_password))
        .route("/signin/wallet", post(handlers::signin_with_wallet))
        .route("/signin/wallet/check", get(handlers::check_wallet_account))
        .route("/signin/device", post(handlers::start_device_signin))
        .route("/signin/device/poll", post(handlers::poll_device_signin))
        .route("/signout", post(handlers::signout))
        .route("/gateway", post(handlers::set_gateway_enabled))
}
