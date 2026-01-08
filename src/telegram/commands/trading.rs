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
    // Ideally we'd check for errors, but for the start command we want to be seamless
    let _ = update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    );

    "🚀 <b>ScreenerBot is Ready!</b>\n\n\
    Trading is <b>enabled</b>.\n\n\
    Use the keyboard below to control the bot.\n\
    Type /help for available commands."
        .to_string()
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
            "🛑 <b>Trading Disabled</b>\n\nAll trading monitors (entries & exits) are stopped.\nUse /pause to stop only entries."
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
            // Ensure master switch is on so resume actually works if previously stopped
            if !cfg.trader.enabled {
                cfg.trader.enabled = true;
            }
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
    "🤖 <b>ScreenerBot Help</b>\n\n\
     <b>📊 Dashboard</b>\n\
     /status — System status & uptime\n\
     /stats — Daily performance\n\
     /balance — Wallet balance\n\
     /positions — Open positions\n\n\
     <b>🔍 Market</b>\n\
     /tokens — Token explorer\n\
     /rejected — Filtered tokens\n\n\
     <b>⚡ Trading</b>\n\
     /start — Enable trading system\n\
     /stop — Disable trading system\n\
     /pause — Pause new entries\n\
     /resume — Resume new entries\n\
     /menu — Interactive menu\n\n\
     <b>🚨 Safety</b>\n\
     /force_stop — <b>EMERGENCY HALT</b>\n\
     /resume_trading — Clear emergency status\n\n\
     <b>⚙️ System</b>\n\
     /login — 2FA Authentication\n\n\
     <i>Tip: Tap a command to run it.</i>"
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
