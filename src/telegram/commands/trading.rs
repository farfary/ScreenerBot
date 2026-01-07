//! Trading control commands
//!
//! Commands for enabling/disabling trading, force stop, pause/resume.

use crate::config::update_config_section;
use crate::logger::{self, LogTag};
use crate::telegram::keyboards;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Check if trader is enabled from config
fn is_trader_enabled() -> bool {
    crate::config::with_config(|cfg| cfg.trader.enabled)
}

/// Handle /start command - Welcome message and enable trading
pub async fn handle_start_command() -> String {
    let currently_enabled = is_trader_enabled();

    if currently_enabled {
        "🚀 <b>ScreenerBot is Ready!</b>\n\n\
        Trading is <b>enabled</b>.\n\n\
        Use the keyboard below to control the bot.\n\
        Type /help for available commands."
            .to_string()
    } else {
        "🚀 <b>Welcome to ScreenerBot!</b>\n\n\
        Trading is currently <b>disabled</b>.\n\n\
        Use the keyboard below to control the bot.\n\
        Press ▶️ Resume to start trading."
            .to_string()
    }
}

/// Handle /stop command - Disable trading
pub async fn handle_stop_command() -> String {
    let currently_enabled = is_trader_enabled();

    if !currently_enabled {
        return "✅ <b>Trading is already disabled</b>".to_string();
    }

    // Disable trading via config
    match update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Trading disabled via Telegram command");
            "🛑 <b>Trading Disabled</b>\n\nThe bot will stop entering new positions.\nExisting positions will continue to be monitored."
                .to_string()
        }
        Err(e) => {
            format!("❌ <b>Failed to disable trading</b>\n\nError: {}", e)
        }
    }
}

/// Handle /pause or /pause_entries command
pub async fn handle_pause_entries_command() -> String {
    match update_config_section(
        |cfg| {
            cfg.trader.entry_monitor_enabled = false;
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Entry monitor paused via Telegram");
            "⏸️ <b>Entry Monitor Paused</b>\n\nNo new positions will be opened.\nExit monitor continues running.".to_string()
        }
        Err(e) => format!("❌ <b>Failed to pause entries</b>\n\nError: {}", e),
    }
}

/// Handle /resume or /resume_entries command
pub async fn handle_resume_entries_command() -> String {
    match update_config_section(
        |cfg| {
            cfg.trader.entry_monitor_enabled = true;
        },
        true,
    ) {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Entry monitor resumed via Telegram");
            "▶️ <b>Entry Monitor Resumed</b>\n\nNow watching for entry signals.".to_string()
        }
        Err(e) => format!("❌ <b>Failed to resume entries</b>\n\nError: {}", e),
    }
}

/// Handle /force_stop command - Show confirmation
pub async fn handle_force_stop_command(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let message = "🚨 <b>FORCE STOP</b>\n\n\
        This will immediately halt ALL trading activity:\n\
        • No new entries\n\
        • No exits (including stop losses)\n\
        • No DCA operations\n\n\
        ⚠️ <b>This is an emergency action!</b>\n\n\
        Are you sure?";

    let keyboard = keyboards::confirm_force_stop();

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| format!("Failed to send force stop confirmation: {}", e))?;

    Ok(())
}

/// Handle /resume_trading command - Clear force stop flag
pub async fn handle_resume_command() -> String {
    if !crate::global::is_force_stopped() {
        return "✅ <b>Trading is not force-stopped</b>\n\nNo action needed.".to_string();
    }

    crate::global::set_force_stopped(false, None);
    logger::info(LogTag::Telegram, "Force stop cleared via Telegram");

    "✅ <b>Trading Resumed</b>\n\nForce stop flag has been cleared.\nNormal trading operations can now resume.".to_string()
}

/// Handle /help command
pub fn handle_help_command() -> String {
    "🤖 <b>ScreenerBot Commands</b>\n\n\
     <b>📊 Info</b>\n\
     /status - Bot status, uptime, and trading state\n\
     /positions - List open positions with P&L\n\
     /balance - Show wallet SOL balance\n\
     /stats - Today's trading statistics\n\n\
     <b>🔍 Tokens</b>\n\
     /tokens - Browse filtered tokens\n\
     /rejected - View rejected tokens\n\n\
     <b>⚡ Controls</b>\n\
     /menu - Open interactive control menu\n\
     /start - Enable trading\n\
     /stop - Disable trading (keeps monitoring)\n\
     /pause - Pause entry monitor only\n\
     /resume - Resume entry monitor\n\
     /force_stop - Emergency halt all trading\n\
     /resume_trading - Clear force stop flag\n\
     /login - Authenticate with 2FA code\n\n\
     <i>Note: Commands only work from the configured chat ID.</i>"
        .to_string()
}

/// Execute force stop action
pub async fn execute_force_stop() -> String {
    crate::global::set_force_stopped(true, Some("Telegram command"));
    logger::warning(LogTag::Telegram, "FORCE STOP activated via Telegram");

    "🚨 <b>FORCE STOP ACTIVATED</b>\n\n\
     All trading has been halted.\n\n\
     Use /resume_trading to clear this flag."
        .to_string()
}

/// Handle /login command - Start the 2FA login flow
pub async fn handle_login_command(bot: &Bot, chat_id: ChatId, user_id: i64) -> Result<(), String> {
    let manager = crate::telegram::session::get_session_manager();

    // Check if 2FA is configured
    let totp_secret = crate::config::with_config(|c| c.webserver.auth_totp_secret.clone());
    if totp_secret.is_empty() {
        // No 2FA configured, just activate the session
        manager.authenticate_session(user_id).await;
        manager.touch_session(user_id).await;

        let _ = bot
            .send_message(
                chat_id,
                "✅ <b>Session Activated</b>\n\n2FA is not configured. Your session is now active.\n\n<i>Tip: Enable 2FA in Security settings for better security.</i>",
            )
            .parse_mode(ParseMode::Html)
            .await;
        return Ok(());
    }

    // Start the login flow
    match manager.start_login(user_id).await {
        Ok(()) => {
            let _ = bot
                .send_message(
                    chat_id,
                    "🔐 <b>Login Required</b>\n\nPlease enter your 6-digit authenticator code:",
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(e) => {
            let _ = bot
                .send_message(chat_id, format!("❌ {}", e))
                .parse_mode(ParseMode::Html)
                .await;
        }
    }

    Ok(())
}
