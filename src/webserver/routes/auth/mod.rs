//! Authentication API routes for headless mode
//!
//! Provides REST API endpoints for password-based authentication when running
//! in headless/VPS mode. These routes handle login, logout, session management,
//! and TOTP two-factor authentication.

use axum::{routing::get, routing::post, Router};
use std::sync::Arc;

use crate::webserver::state::AppState;

mod helpers;
mod password_handlers;
mod session_handlers;
mod totp_handlers;
mod types;

// Re-export public helpers for middleware usage
pub use helpers::extract_session_token;
pub use types::SESSION_COOKIE_NAME;

// =============================================================================
// ROUTES
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(session_handlers::login))
        .route("/logout", post(session_handlers::logout))
        .route("/status", get(session_handlers::get_status))
        .route("/set-password", post(password_handlers::set_password))
        // TOTP 2FA routes
        .route("/totp/status", get(totp_handlers::totp_status))
        .route("/totp/setup", post(totp_handlers::totp_setup))
        .route("/totp/verify-setup", post(totp_handlers::totp_verify_setup))
        .route("/totp/disable", post(totp_handlers::totp_disable))
}
