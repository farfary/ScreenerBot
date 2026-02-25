//! Telegram route handlers — endpoint implementations for session management and discovery.

use super::types::*;
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::telegram::session::get_session_manager;
use crate::webserver::state::AppState;
use crate::webserver::utils::{error_response, success_response};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    Json,
};
use std::sync::Arc;

/// Get Telegram connection status
pub(super) async fn get_status(State(_state): State<Arc<AppState>>) -> Response {
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
pub(super) async fn get_settings(State(_state): State<Arc<AppState>>) -> Response {
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
                "***".to_owned()
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
            filtering_alerts: config.notify_filtering_alerts,
            trade_alerts: config.notify_trade_alerts,
            daily_summary: config.notify_daily_summary,
        },
        commands_enabled: config.commands_enabled,
        inline_actions: config.inline_actions_enabled,
        sessions,
    };

    success_response(response)
}

/// Update Telegram settings
pub(super) async fn update_settings(
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
                // Validate range: 5-1440 minutes (5 min to 24 hours)
                if timeout >= 5 && timeout <= 1440 {
                    cfg.telegram.session_timeout_minutes = timeout;
                }
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
                if let Some(v) = notif.filtering_alerts {
                    cfg.telegram.notify_filtering_alerts = v;
                }
                if let Some(v) = notif.trade_alerts {
                    cfg.telegram.notify_trade_alerts = v;
                }
                if let Some(v) = notif.daily_summary {
                    cfg.telegram.notify_daily_summary = v;
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
            &format!("Failed to update settings: {e}"),
            None,
        ),
    }
}

/// List all sessions
pub(super) async fn list_sessions(State(_state): State<Arc<AppState>>) -> Response {
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
pub(super) async fn revoke_session(
    State(_state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Response {
    let manager = get_session_manager();
    manager.revoke_session(user_id).await;

    logger::info(
        LogTag::Telegram,
        &format!("Revoked Telegram session for user_id: {user_id}"),
    );

    success_response(serde_json::json!({
        "message": "Session revoked",
        "user_id": user_id
    }))
}

/// Send a test message
pub(super) async fn send_test_message(
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
        return error_response(
            StatusCode::BAD_REQUEST,
            "TELEGRAM_DISABLED",
            "Telegram is not enabled",
            None,
        );
    }

    if bot_token.is_empty() || chat_id.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "NOT_CONFIGURED",
            "Bot token or chat ID not configured",
            None,
        );
    }

    // Create notifier and send
    match crate::telegram::TelegramNotifier::new(&bot_token, &chat_id) {
        Ok(notifier) => {
            let message = req.message.unwrap_or_else(|| {
                "🔔 <b>Test Message</b>\n\nTelegram integration is working!".to_owned()
            });

            match notifier.send_message(&message).await {
                Ok(()) => {
                    logger::info(LogTag::Telegram, "Sent Telegram test message");
                    success_response(serde_json::json!({
                        "message": "Test message sent successfully"
                    }))
                }
                Err(e) => error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "SEND_FAILED",
                    &format!("Failed to send message: {e}"),
                    None,
                ),
            }
        }
        Err(e) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "NOTIFIER_ERROR",
            &format!("Failed to create notifier: {e}"),
            None,
        ),
    }
}

/// GET /totp/status — Check if TOTP is enabled (uses shared lockscreen 2FA)
pub(super) async fn get_totp_status(State(_state): State<Arc<AppState>>) -> Response {
    let totp_configured = with_config(|c| !c.webserver.auth_totp_secret.is_empty());
    let commands_require_2fa = with_config(|c| c.telegram.commands_require_2fa);
    success_response(serde_json::json!({
        "enabled": totp_configured && commands_require_2fa,
        "configured": totp_configured,
        "commands_require_2fa": commands_require_2fa
    }))
}

// ==================== Discovery Handlers ====================

/// POST /discovery/start — Start discovery mode to capture incoming chat IDs
pub(super) async fn start_discovery(State(_state): State<Arc<AppState>>) -> Response {
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
            &format!("Failed to start discovery: {e}"),
            None,
        );
    }

    logger::info(LogTag::Telegram, "Telegram chat discovery mode started");

    success_response(serde_json::json!({
        "message": "Discovery mode started. Send a message to your bot in Telegram.",
        "active": true
    }))
}

/// POST /discovery/stop — Stop discovery mode
pub(super) async fn stop_discovery(State(_state): State<Arc<AppState>>) -> Response {
    // Stop the discovery polling service
    crate::telegram::discovery::stop_discovery().await;

    logger::info(LogTag::Telegram, "Telegram chat discovery mode stopped");

    success_response(serde_json::json!({
        "message": "Discovery mode stopped",
        "active": false
    }))
}

/// GET /discovery/chats — Get list of discovered chats
pub(super) async fn get_discovered_chats(State(_state): State<Arc<AppState>>) -> Response {
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

/// POST /discovery/select/:chat_id — Select a discovered chat as the notification target
pub(super) async fn select_discovered_chat(
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
                &format!("Selected Telegram chat ID: {chat_id}"),
            );

            success_response(serde_json::json!({
                "message": "Chat selected successfully",
                "chat_id": chat_id
            }))
        }
        Err(e) => error_response(
            StatusCode::NOT_FOUND,
            "SELECTION_FAILED",
            &format!("Failed to select chat: {e}"),
            None,
        ),
    }
}

/// POST /discovery/clear — Clear discovered chats list
pub(super) async fn clear_discovered_chats(State(_state): State<Arc<AppState>>) -> Response {
    crate::telegram::discovery::clear_discovered_chats().await;

    success_response(serde_json::json!({
        "message": "Discovered chats cleared"
    }))
}
