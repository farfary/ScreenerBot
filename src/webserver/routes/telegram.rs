//! Telegram session management API routes
//!
//! Provides endpoints for:
//! - Telegram connection status
//! - Session listing and management
//! - Password authentication
//! - TOTP two-factor authentication
//! - Test message sending

use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::telegram::session::{get_session_manager, TelegramSessionManager};
use crate::webserver::state::AppState;
use crate::webserver::totp;
use crate::webserver::utils::{error_response, success_response};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// === RESPONSE TYPES ===

#[derive(Serialize)]
pub struct TelegramStatusResponse {
    pub enabled: bool,
    pub connected: bool,
    pub bot_configured: bool,
    pub totp_configured: bool, // Whether lockscreen 2FA is configured (shared with dashboard)
    pub active_sessions: usize,
    pub commands_enabled: bool,
    pub inline_actions_enabled: bool,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub user_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub state: String,
    pub is_authenticated: bool,
    pub last_activity_secs: u64,
    pub created_at_secs: u64,
}

#[derive(Serialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<SessionResponse>,
}

// === REQUEST TYPES ===

#[derive(Deserialize)]
pub struct SetPasswordRequest {
    pub password: String,
    pub current_password: Option<String>, // Required if password already set
}

#[derive(Deserialize)]
pub struct TotpSetupRequest {
    pub password: String, // Require password to setup TOTP
}

#[derive(Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub uri: String,
    pub qr_code: String, // Base64 SVG data URL
}

#[derive(Deserialize)]
pub struct TotpVerifyRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct TestMessageRequest {
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct TelegramSettingsResponse {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub totp_configured: bool, // Whether lockscreen 2FA is configured (shared with dashboard)
    pub commands_require_2fa: bool,
    pub session_timeout_minutes: i64,
    pub notifications: NotificationSettings,
    pub commands_enabled: bool,
    pub inline_actions: bool,
    pub sessions: Vec<SessionResponse>,
}

#[derive(Serialize)]
pub struct NotificationSettings {
    pub position_opened: bool,
    pub position_closed: bool,
    pub partial_exit: bool,
    pub dca_executed: bool,
    pub errors: bool,
    pub startup_shutdown: bool,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub enabled: Option<bool>,
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
    pub session_timeout_minutes: Option<i64>,
    pub notifications: Option<UpdateNotificationSettings>,
    pub commands_enabled: Option<bool>,
    pub commands_require_2fa: Option<bool>,
    pub inline_actions: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateNotificationSettings {
    pub position_opened: Option<bool>,
    pub position_closed: Option<bool>,
    pub partial_exit: Option<bool>,
    pub dca_executed: Option<bool>,
    pub errors: Option<bool>,
    pub startup_shutdown: Option<bool>,
}

// === ROUTES ===

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // Status
        .route("/status", get(get_status))
        .route("/test", post(send_test_message))
        // Settings (combined view for dashboard)
        .route("/settings", get(get_settings))
        .route("/settings", post(update_settings))
        // Sessions
        .route("/sessions", get(list_sessions))
        .route("/sessions/:user_id/revoke", post(revoke_session))
        // Password management
        .route("/password", post(set_password))
        .route("/password/verify", post(verify_password))
        // TOTP 2FA
        .route("/totp/status", get(get_totp_status))
        .route("/totp/setup", post(setup_totp))
        .route("/totp/verify", post(verify_totp))
        .route("/totp/disable", post(disable_totp))
        .route("/totp/cancel", post(cancel_totp_setup))
        // Chat Discovery
        .route("/discovery/start", post(start_discovery))
        .route("/discovery/stop", post(stop_discovery))
        .route("/discovery/chats", get(get_discovered_chats))
        .route("/discovery/select/:chat_id", post(select_discovered_chat))
        .route("/discovery/clear", post(clear_discovered_chats))
}

// === HANDLERS ===

/// Get Telegram connection status
async fn get_status(State(_state): State<Arc<AppState>>) -> Response {
    let manager = get_session_manager();

    let (enabled, bot_token, totp_secret, commands, inline) = with_config(|c| {
        (
            c.telegram.enabled,
            c.telegram.bot_token.clone(),
            c.webserver.auth_totp_secret.clone(), // Shared with lockscreen
            c.telegram.commands_enabled,
            c.telegram.inline_actions_enabled,
        )
    });

    let sessions = manager.get_all_sessions().await;

    let response = TelegramStatusResponse {
        enabled,
        connected: enabled && !bot_token.is_empty(),
        bot_configured: !bot_token.is_empty(),
        totp_configured: !totp_secret.is_empty(),
        active_sessions: sessions.iter().filter(|s| s.is_authenticated()).count(),
        commands_enabled: commands,
        inline_actions_enabled: inline,
    };

    success_response(response)
}

/// Get full Telegram settings for dashboard
async fn get_settings(State(_state): State<Arc<AppState>>) -> Response {
    let manager = get_session_manager();

    let config = with_config(|c| c.telegram.clone());

    // Build sessions list
    let sessions: Vec<SessionResponse> = manager
        .get_all_sessions()
        .await
        .into_iter()
        .map(|s| {
            let is_auth = s.is_authenticated();
            let last_activity = s.last_activity.elapsed().as_secs();
            let created_at = s.created_at.elapsed().as_secs();
            SessionResponse {
                user_id: s.user_id,
                username: s.username.clone(),
                first_name: s.first_name.clone(),
                state: format!("{:?}", s.state),
                is_authenticated: is_auth,
                last_activity_secs: last_activity,
                created_at_secs: created_at,
            }
        })
        .collect();

    let totp_secret = with_config(|c| c.webserver.auth_totp_secret.clone());

    let response = TelegramSettingsResponse {
        enabled: config.enabled,
        bot_token: if config.bot_token.is_empty() {
            String::new()
        } else {
            // Mask the token for security (show first 10 chars + ...)
            if config.bot_token.len() > 10 {
                format!("{}...", &config.bot_token[..10])
            } else {
                "***".to_string()
            }
        },
        chat_id: config.chat_id.clone(),
        totp_configured: !totp_secret.is_empty(),
        commands_require_2fa: config.commands_require_2fa,
        session_timeout_minutes: config.session_timeout_minutes,
        notifications: NotificationSettings {
            position_opened: config.notify_position_opened,
            position_closed: config.notify_position_closed,
            partial_exit: config.notify_partial_exit,
            dca_executed: config.notify_dca_executed,
            errors: config.notify_system_errors,
            startup_shutdown: config.notify_on_startup,
        },
        commands_enabled: config.commands_enabled,
        inline_actions: config.inline_actions_enabled,
        sessions,
    };

    success_response(response)
}

/// Update Telegram settings
async fn update_settings(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Response {
    match update_config_section(
        |cfg| {
            if let Some(enabled) = req.enabled {
                cfg.telegram.enabled = enabled;
            }
            if let Some(ref token) = req.bot_token {
                // Only update if not masked value
                if !token.ends_with("...") && !token.is_empty() {
                    cfg.telegram.bot_token = token.clone();
                }
            }
            if let Some(ref chat_id) = req.chat_id {
                cfg.telegram.chat_id = chat_id.clone();
            }
            if let Some(timeout) = req.session_timeout_minutes {
                cfg.telegram.session_timeout_minutes = timeout;
            }
            if let Some(ref notif) = req.notifications {
                if let Some(v) = notif.position_opened {
                    cfg.telegram.notify_position_opened = v;
                }
                if let Some(v) = notif.position_closed {
                    cfg.telegram.notify_position_closed = v;
                }
                if let Some(v) = notif.partial_exit {
                    cfg.telegram.notify_partial_exit = v;
                }
                if let Some(v) = notif.dca_executed {
                    cfg.telegram.notify_dca_executed = v;
                }
                if let Some(v) = notif.errors {
                    cfg.telegram.notify_system_errors = v;
                }
                if let Some(v) = notif.startup_shutdown {
                    cfg.telegram.notify_on_startup = v;
                    cfg.telegram.notify_on_shutdown = v;
                }
            }
            if let Some(commands) = req.commands_enabled {
                cfg.telegram.commands_enabled = commands;
            }
            if let Some(require_2fa) = req.commands_require_2fa {
                cfg.telegram.commands_require_2fa = require_2fa;
            }
            if let Some(inline) = req.inline_actions {
                cfg.telegram.inline_actions_enabled = inline;
            }
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Telegram settings updated via API");
            success_response(serde_json::json!({
                "message": "Settings updated successfully"
            }))
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_ERROR",
            &format!("Failed to update settings: {}", e),
            None,
        ),
    }
}

/// List all sessions
async fn list_sessions(State(_state): State<Arc<AppState>>) -> Response {
    let manager = get_session_manager();

    let sessions: Vec<SessionResponse> = manager
        .get_all_sessions()
        .await
        .into_iter()
        .map(|s| {
            let is_auth = s.is_authenticated();
            let last_activity = s.last_activity.elapsed().as_secs();
            let created_at = s.created_at.elapsed().as_secs();
            SessionResponse {
                user_id: s.user_id,
                username: s.username.clone(),
                first_name: s.first_name.clone(),
                state: format!("{:?}", s.state),
                is_authenticated: is_auth,
                last_activity_secs: last_activity,
                created_at_secs: created_at,
            }
        })
        .collect();

    success_response(SessionsListResponse { sessions })
}

/// Revoke a session
async fn revoke_session(
    State(_state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Response {
    let manager = get_session_manager();
    manager.revoke_session(user_id).await;

    logger::info(
        LogTag::Telegram,
        &format!("Revoked Telegram session for user_id: {}", user_id),
    );

    success_response(serde_json::json!({
        "message": "Session revoked",
        "user_id": user_id
    }))
}

/// Set or update password - DEPRECATED (passwordless flow)
async fn set_password(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<SetPasswordRequest>,
) -> Response {
    error_response(
        StatusCode::GONE,
        "DEPRECATED",
        "Password authentication is no longer used. Telegram now uses passwordless authentication with optional 2FA (configured in Security settings).",
        None,
    )
}

/// Verify a password - DEPRECATED (passwordless flow)
async fn verify_password(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<SetPasswordRequest>,
) -> Response {
    error_response(
        StatusCode::GONE,
        "DEPRECATED",
        "Password authentication is no longer used. Telegram now uses passwordless authentication with optional 2FA.",
        None,
    )
}

/// Send a test message
async fn send_test_message(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<TestMessageRequest>,
) -> Response {
    let (enabled, bot_token, chat_id) = with_config(|c| {
        (
            c.telegram.enabled,
            c.telegram.bot_token.clone(),
            c.telegram.chat_id.clone(),
        )
    });

    if !enabled {
        return error_response(StatusCode::BAD_REQUEST, "TELEGRAM_DISABLED", "Telegram is not enabled", None);
    }

    if bot_token.is_empty() || chat_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "NOT_CONFIGURED", "Bot token or chat ID not configured", None);
    }

    // Create notifier and send
    match crate::telegram::TelegramNotifier::new(&bot_token, &chat_id) {
        Ok(notifier) => {
            let message = req.message.unwrap_or_else(|| {
                "🔔 <b>Test Message</b>\n\nTelegram integration is working!".to_string()
            });

            match notifier.send_message(&message).await {
                Ok(()) => {
                    logger::info(LogTag::Telegram, "Sent Telegram test message");
                    success_response(serde_json::json!({
                        "message": "Test message sent successfully"
                    }))
                }
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "SEND_FAILED", &format!("Failed to send message: {}", e), None),
            }
        }
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "NOTIFIER_ERROR", &format!("Failed to create notifier: {}", e), None),
    }
}

// === TOTP HANDLERS ===
// NOTE: Telegram TOTP is now managed through Security settings (lockscreen 2FA)
// These routes are kept for backwards compatibility but redirect to the shared 2FA system

/// GET /totp/status - Check if TOTP is enabled (uses shared lockscreen 2FA)
async fn get_totp_status(State(_state): State<Arc<AppState>>) -> Response {
    let totp_configured = with_config(|c| !c.webserver.auth_totp_secret.is_empty());
    let commands_require_2fa = with_config(|c| c.telegram.commands_require_2fa);
    success_response(serde_json::json!({
        "enabled": totp_configured && commands_require_2fa,
        "configured": totp_configured,
        "commands_require_2fa": commands_require_2fa,
        "note": "2FA is now managed in Security settings. Enable lockscreen 2FA to protect Telegram commands."
    }))
}

/// POST /totp/setup - DEPRECATED (use Security settings instead)
async fn setup_totp(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TotpSetupRequest>,
) -> Response {
    error_response(
        StatusCode::GONE,
        "DEPRECATED",
        "Telegram 2FA is now managed through Security settings. Enable lockscreen 2FA in the Security tab to protect Telegram commands.",
        None,
    )
}

/// POST /totp/verify - DEPRECATED (use Security settings instead)
async fn verify_totp(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TotpVerifyRequest>,
) -> Response {
    error_response(
        StatusCode::GONE,
        "DEPRECATED",
        "Telegram 2FA is now managed through Security settings.",
        None,
    )
}

/// POST /totp/cancel - Cancel pending TOTP setup (kept for cleanup)
async fn cancel_totp_setup(State(_state): State<Arc<AppState>>) -> Response {
    TelegramSessionManager::cancel_pending_totp("dashboard");
    success_response(serde_json::json!({ "message": "TOTP setup cancelled" }))
}

/// POST /totp/disable - DEPRECATED (use Security settings instead)
async fn disable_totp(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<TotpSetupRequest>,
) -> Response {
    error_response(
        StatusCode::GONE,
        "DEPRECATED",
        "Telegram 2FA is now managed through Security settings. Disable lockscreen 2FA to remove protection from Telegram commands.",
        None,
    )
}

// === DISCOVERY HANDLERS ===

/// Response for discovered chat
#[derive(Serialize)]
pub struct DiscoveredChatResponse {
    pub chat_id: i64,
    pub user_id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub chat_type: String,
    pub message_preview: Option<String>,
    pub discovered_at_secs: u64,
}

/// POST /discovery/start - Start discovery mode to capture incoming chat IDs
async fn start_discovery(State(_state): State<Arc<AppState>>) -> Response {
    let bot_token = with_config(|c| c.telegram.bot_token.clone());
    
    if bot_token.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NO_TOKEN",
            "Bot token must be configured first",
            None,
        );
    }

    // Start the discovery polling service
    if let Err(e) = crate::telegram::discovery::start_discovery().await {
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DISCOVERY_FAILED",
            &format!("Failed to start discovery: {}", e),
            None,
        );
    }
    
    logger::info(LogTag::Telegram, "Telegram chat discovery mode started");
    
    success_response(serde_json::json!({
        "message": "Discovery mode started. Send a message to your bot in Telegram.",
        "active": true
    }))
}

/// POST /discovery/stop - Stop discovery mode
async fn stop_discovery(State(_state): State<Arc<AppState>>) -> Response {
    // Stop the discovery polling service
    crate::telegram::discovery::stop_discovery().await;
    
    logger::info(LogTag::Telegram, "Telegram chat discovery mode stopped");
    
    success_response(serde_json::json!({
        "message": "Discovery mode stopped",
        "active": false
    }))
}

/// GET /discovery/chats - Get list of discovered chats
async fn get_discovered_chats(State(_state): State<Arc<AppState>>) -> Response {
    let is_active = crate::telegram::discovery::is_discovery_running().await;
    let chats = crate::telegram::discovery::get_discovered_chats().await;
    
    let chats_response: Vec<DiscoveredChatResponse> = chats
        .into_iter()
        .map(|c| DiscoveredChatResponse {
            chat_id: c.chat_id,
            user_id: c.user_id,
            username: c.username,
            first_name: c.first_name,
            chat_type: c.chat_type,
            message_preview: c.message_preview,
            discovered_at_secs: c.discovered_at.elapsed().as_secs(),
        })
        .collect();
    
    success_response(serde_json::json!({
        "active": is_active,
        "chats": chats_response
    }))
}

/// POST /discovery/select/:chat_id - Select a discovered chat as the notification target
async fn select_discovered_chat(
    State(_state): State<Arc<AppState>>,
    Path(chat_id): Path<i64>,
) -> Response {
    // Select the chat and save to config
    match crate::telegram::discovery::select_discovered_chat(chat_id).await {
        Ok(()) => {
            // Stop discovery mode after successful selection
            crate::telegram::discovery::stop_discovery().await;
            
            logger::info(
                LogTag::Telegram,
                &format!("Selected Telegram chat ID: {}", chat_id),
            );
            
            success_response(serde_json::json!({
                "message": "Chat selected successfully",
                "chat_id": chat_id
            }))
        }
        Err(e) => error_response(
            StatusCode::NOT_FOUND,
            "SELECTION_FAILED",
            &format!("Failed to select chat: {}", e),
            None,
        ),
    }
}

/// POST /discovery/clear - Clear discovered chats list
async fn clear_discovered_chats(State(_state): State<Arc<AppState>>) -> Response {
    crate::telegram::discovery::clear_discovered_chats().await;
    
    success_response(serde_json::json!({
        "message": "Discovered chats cleared"
    }))
}

