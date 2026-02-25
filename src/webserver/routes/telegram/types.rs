//! Telegram route types — request/response structs for Telegram session management.

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct TelegramStatusResponse {
    pub enabled: bool,
    pub connected: bool,
    pub bot_configured: bool,
    pub totp_configured: bool,
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

#[derive(Deserialize)]
pub struct TestMessageRequest {
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct TelegramSettingsResponse {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
    pub totp_configured: bool,
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
    pub filtering_alerts: bool,
    pub trade_alerts: bool,
    pub daily_summary: bool,
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
    pub filtering_alerts: Option<bool>,
    pub trade_alerts: Option<bool>,
    pub daily_summary: Option<bool>,
}

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
