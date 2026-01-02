//! Command handlers module for Telegram bot
//!
//! Organized command handlers for different functionality areas.

mod callbacks;
mod menu;
mod status;
mod trading;

pub use callbacks::handle_callback_query;
pub use menu::{handle_menu_command, send_main_menu};
pub use status::{handle_balance_command, handle_positions_command, handle_stats_command, handle_status_command};
pub use trading::{
    handle_force_stop_command, handle_help_command, handle_login_command,
    handle_pause_entries_command, handle_resume_command, handle_resume_entries_command,
    handle_start_command, handle_stop_command,
};

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::telegram::keyboards;
use crate::telegram::session::get_session_manager;
use crate::telegram::types::SessionState;
use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Map keyboard button text to commands
fn button_to_command(text: &str) -> Option<&'static str> {
    match text {
        "📊 Status" => Some("/status"),
        "💰 Balance" => Some("/balance"),
        "📈 Positions" => Some("/positions"),
        "⏸️ Pause" => Some("/pause"),
        "▶️ Resume" => Some("/resume"),
        "🛑 Stop" => Some("/force_stop"),
        "📉 Stats" => Some("/stats"),
        "⚙️ Menu" => Some("/menu"),
        "❓ Help" => Some("/help"),
        _ => None,
    }
}

/// Handle a single command from text message
pub async fn handle_command(
    bot: &Bot,
    chat_id: ChatId,
    user_id: i64,
    text: &str,
) -> Result<(), String> {
    let text = text.trim();

    // Map keyboard button text to command, or use text directly if it's a command
    let command = if text.starts_with('/') {
        text.split_whitespace().next().unwrap_or("")
    } else if let Some(cmd) = button_to_command(text) {
        cmd
    } else {
        // Not a command or known button text
        return Ok(());
    };

    // Check authentication for sensitive commands
    let is_sensitive = matches!(
        command,
        "/positions"
            | "/balance"
            | "/menu"
            | "/status"
            | "/stats"
            | "/pause"
            | "/pause_entries"
            | "/resume"
            | "/resume_entries"
            | "/force_stop"
            | "/resume_trading"
            | "/start"
            | "/stop"
    );

    if is_sensitive && !check_auth(bot, chat_id, user_id).await {
        return Ok(()); // Auth check failed, message already sent
    }

    // Commands that require special handling (with keyboard)
    match command {
        "/start" => {
            // Send welcome message with reply keyboard
            let response = handle_start_command().await;
            bot.send_message(chat_id, &response)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboards::main_reply_keyboard())
                .await
                .map_err(|e| format!("Failed to send start response: {}", e))?;

            logger::info(LogTag::Telegram, "Handled /start command with reply keyboard");
            return Ok(());
        }
        "/menu" => {
            return handle_menu_command(bot, chat_id).await;
        }
        "/force_stop" => {
            return handle_force_stop_command(bot, chat_id).await;
        }
        "/login" => {
            return handle_login_command(bot, chat_id, user_id).await;
        }
        _ => {}
    }

    let response = match command {
        "/stop" => handle_stop_command().await,
        "/status" => handle_status_command().await,
        "/positions" => handle_positions_command().await,
        "/balance" => handle_balance_command().await,
        "/stats" => handle_stats_command().await,
        "/pause" | "/pause_entries" => handle_pause_entries_command().await,
        "/resume" | "/resume_entries" => handle_resume_entries_command().await,
        "/resume_trading" => handle_resume_command().await,
        "/help" => handle_help_command(),
        _ => format!(
            "❓ Unknown command: {}\n\nUse /help to see available commands.",
            command
        ),
    };

    bot.send_message(chat_id, &response)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| format!("Failed to send response: {}", e))?;

    logger::info(
        LogTag::Telegram,
        &format!("Handled Telegram command: {}", command),
    );

    Ok(())
}

/// Check if the user is authenticated for sensitive commands
/// Returns true if authenticated, false otherwise (sends auth prompt)
pub async fn check_auth(bot: &Bot, chat_id: ChatId, user_id: i64) -> bool {
    let manager = get_session_manager();
    let session = manager
        .get_or_create_session(user_id, chat_id.0, None, None)
        .await;

    match session.state {
        SessionState::Active => {
            // Check for session timeout
            let timeout_mins = with_config(|c| c.telegram.session_timeout_minutes) as u64;
            if session.last_activity.elapsed() > Duration::from_secs(timeout_mins * 60) {
                // Session expired, invalidate and prompt for re-login
                manager.invalidate_session(user_id).await;

                // Check if 2FA is required for commands
                let commands_require_2fa = with_config(|c| c.telegram.commands_require_2fa);
                let totp_secret = with_config(|c| c.webserver.auth_totp_secret.clone());
                if !commands_require_2fa || totp_secret.is_empty() {
                    // 2FA not required or not configured, auto-reactivate
                    manager.authenticate_session(user_id).await;
                    manager.touch_session(user_id).await;
                    return true;
                }

                let _ = bot
                    .send_message(
                        chat_id,
                        "🔐 <b>Session Expired</b>\n\nUse /login to re-authenticate.",
                    )
                    .parse_mode(ParseMode::Html)
                    .await;
                return false;
            }
            // Update activity timestamp
            manager.touch_session(user_id).await;
            true
        }
        SessionState::Expired => {
            // Check if 2FA is required for commands
            let commands_require_2fa = with_config(|c| c.telegram.commands_require_2fa);
            let totp_secret = with_config(|c| c.webserver.auth_totp_secret.clone());
            if !commands_require_2fa || totp_secret.is_empty() {
                // 2FA not required or not configured, auto-reactivate
                manager.authenticate_session(user_id).await;
                manager.touch_session(user_id).await;
                return true;
            }

            let _ = bot
                .send_message(
                    chat_id,
                    "🔐 <b>Session Expired</b>\n\nUse /login to re-authenticate.",
                )
                .parse_mode(ParseMode::Html)
                .await;
            false
        }
        SessionState::AwaitingTotp => {
            let _ = bot
                .send_message(
                    chat_id,
                    "🔢 <b>2FA Required</b>\n\nPlease enter your 6-digit authenticator code.",
                )
                .parse_mode(ParseMode::Html)
                .await;
            false
        }
        SessionState::Locked { until } => {
            let remaining = until
                .saturating_duration_since(std::time::Instant::now())
                .as_secs();
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "🔒 <b>Account Locked</b>\n\nToo many failed attempts.\nTry again in {} seconds.",
                        remaining
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
            false
        }
    }
}

/// Handle a TOTP entry attempt (passwordless flow - only TOTP after /login)
pub async fn handle_auth_attempt(bot: &Bot, chat_id: ChatId, user_id: i64, text: &str) {
    let manager = get_session_manager();
    let session = manager
        .get_or_create_session(user_id, chat_id.0, None, None)
        .await;

    match session.state {
        SessionState::AwaitingTotp => {
            // Validate format (6 digits)
            if text.len() != 6 || !text.chars().all(|c| c.is_ascii_digit()) {
                let _ = bot
                    .send_message(chat_id, "❌ Please enter a valid 6-digit code.")
                    .parse_mode(ParseMode::Html)
                    .await;
                return;
            }

            match manager.verify_totp(user_id, text).await {
                Ok(true) => {
                    let _ = bot
                        .send_message(
                            chat_id,
                            "✅ <b>Authenticated!</b>\n\nYou now have access to bot commands.",
                        )
                        .parse_mode(ParseMode::Html)
                        .await;
                    let _ = send_main_menu(bot, chat_id).await;

                    logger::info(
                        LogTag::Telegram,
                        &format!(
                            "Telegram session authenticated (2FA) for user_id={}",
                            user_id
                        ),
                    );
                }
                Ok(false) => {
                    let session = manager
                        .get_or_create_session(user_id, chat_id.0, None, None)
                        .await;
                    let max = with_config(|c| c.telegram.max_failed_attempts) as u32;
                    let remaining = max.saturating_sub(session.failed_attempts);
                    let _ = bot
                        .send_message(
                            chat_id,
                            format!("❌ <b>Wrong Code</b>\n\n{} attempts remaining.", remaining),
                        )
                        .parse_mode(ParseMode::Html)
                        .await;

                    logger::warning(
                        LogTag::Telegram,
                        &format!("Failed TOTP attempt for user_id={}", user_id),
                    );
                }
                Err(e) => {
                    let _ = bot
                        .send_message(chat_id, format!("🔒 {}", e))
                        .parse_mode(ParseMode::Html)
                        .await;
                }
            }
        }
        SessionState::Locked { until } => {
            let remaining = until
                .saturating_duration_since(std::time::Instant::now())
                .as_secs();
            let _ = bot
                .send_message(
                    chat_id,
                    format!(
                        "🔒 <b>Account Locked</b>\n\nToo many failed attempts.\nTry again in {} seconds.",
                        remaining
                    ),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        SessionState::Active | SessionState::Expired => {
            // Not expecting auth input in these states, ignore
        }
    }
}
