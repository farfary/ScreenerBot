//! Telegram session management API routes
//!
//! Provides endpoints for:
//! - Telegram connection status
//! - Session listing and management
//! - Password authentication
//! - TOTP two-factor authentication
//! - Test message sending

mod handlers;
pub mod types;

use crate::webserver::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub use types::*;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Status
        .route("/status", get(handlers::get_status))
        .route("/test", post(handlers::send_test_message))
        // Settings (combined view for dashboard)
        .route("/settings", get(handlers::get_settings))
        .route("/settings", post(handlers::update_settings))
        // Sessions
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/:user_id/revoke", post(handlers::revoke_session))
        // TOTP 2FA (status only - setup is in Security settings)
        .route("/totp/status", get(handlers::get_totp_status))
        // Chat Discovery
        .route("/discovery/start", post(handlers::start_discovery))
        .route("/discovery/stop", post(handlers::stop_discovery))
        .route("/discovery/chats", get(handlers::get_discovered_chats))
        .route(
            "/discovery/select/:chat_id",
            post(handlers::select_discovered_chat),
        )
        .route("/discovery/clear", post(handlers::clear_discovered_chats))
}
