//! Trading control commands
//!
//! Commands for enabling/disabling trading, force stop, pause/resume.

use crate::config::update_config_section;
use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::keyboards;
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::telegram::{Error, Result};
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Check if trader is enabled from config
fn is_trader_enabled() -> bool {
    crate::config::with_config(|cfg| cfg.trader.enabled)
}

/// Message with a technical error detail.
fn failure_message(icon: &str, id: MessageId, error: &impl std::fmt::Display) -> String {
    with_icon(
        icon,
        &tg(&UiText::new(id).arg("detail", UiArg::Text(error.to_string()))),
    )
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

    with_icon("🚀", &tg_id(ids::TELEGRAM_START_READY))
}

/// Handle /stop command - Disable trading
pub async fn handle_stop_command() -> String {
    let currently_enabled = is_trader_enabled();

    if !currently_enabled {
        return with_icon("✅", &tg_id(ids::TELEGRAM_STOP_ALREADY));
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
            with_icon("🛑", &tg_id(ids::TELEGRAM_STOP_DONE))
        }
        Err(e) => failure_message("❌", ids::TELEGRAM_STOP_FAILED, &e),
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
            with_icon("⏸️", &tg_id(ids::TELEGRAM_PAUSE_DONE))
        }
        Err(e) => failure_message("❌", ids::TELEGRAM_PAUSE_FAILED, &e),
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
            with_icon("▶️", &tg_id(ids::TELEGRAM_RESUME_DONE))
        }
        Err(e) => failure_message("❌", ids::TELEGRAM_RESUME_FAILED, &e),
    }
}

/// Handle /force_stop command - Show confirmation
pub async fn handle_force_stop_command(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let message = format!(
        "{}\n\n{}\n\n{}",
        with_icon("🚨", &tg_id(ids::TELEGRAM_FORCE_STOP_CONFIRM)),
        with_icon("⚠️", &tg_id(ids::TELEGRAM_FORCE_STOP_WARNING)),
        tg_id(ids::TELEGRAM_FORCE_STOP_QUESTION),
    );

    let keyboard = keyboards::confirm_force_stop();

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| Error::SendFailed {
            chat_id: chat_id.0.to_string(),
            detail: e.to_string(),
        })?;

    Ok(())
}

/// Handle /resume_trading command - Clear force stop flag
pub async fn handle_resume_command() -> String {
    if !crate::global::is_force_stopped() {
        return with_icon("✅", &tg_id(ids::TELEGRAM_RESUME_TRADING_NOT_STOPPED));
    }

    crate::global::set_force_stopped(false, None);
    logger::info(LogTag::Telegram, "Force stop cleared via Telegram");

    with_icon("✅", &tg_id(ids::TELEGRAM_RESUME_TRADING_DONE))
}

/// Handle /help command
pub fn handle_help_command() -> String {
    let section = |icon: &str, heading: MessageId, commands: MessageId| {
        format!(
            "<b>{}</b>\n{}",
            with_icon(icon, &tg_id(heading)),
            tg_id(commands)
        )
    };
    [
        with_icon("🤖", &tg_id(ids::TELEGRAM_HELP_TITLE)),
        section(
            "📊",
            ids::TELEGRAM_HELP_HEADING_DASHBOARD,
            ids::TELEGRAM_HELP_COMMANDS_DASHBOARD,
        ),
        section(
            "🔍",
            ids::TELEGRAM_HELP_HEADING_MARKET,
            ids::TELEGRAM_HELP_COMMANDS_MARKET,
        ),
        section(
            "⚡",
            ids::TELEGRAM_HELP_HEADING_TRADING,
            ids::TELEGRAM_HELP_COMMANDS_TRADING,
        ),
        section(
            "🚨",
            ids::TELEGRAM_HELP_HEADING_SAFETY,
            ids::TELEGRAM_HELP_COMMANDS_SAFETY,
        ),
        section(
            "⚙️",
            ids::TELEGRAM_HELP_HEADING_SYSTEM,
            ids::TELEGRAM_HELP_COMMANDS_SYSTEM,
        ),
        tg_id(ids::TELEGRAM_HELP_TIP),
    ]
    .join("\n\n")
}

/// Execute force stop action
pub async fn execute_force_stop() -> String {
    crate::global::set_force_stopped(true, None);
    logger::warning(LogTag::Telegram, "FORCE STOP activated via Telegram");

    with_icon("🚨", &tg_id(ids::TELEGRAM_FORCE_STOP_ACTIVE))
}

/// Handle /login command - Start the 2FA login flow
pub async fn handle_login_command(bot: &Bot, chat_id: ChatId, user_id: i64) -> Result<()> {
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
                with_icon("✅", &tg_id(ids::TELEGRAM_SESSION_ACTIVATED)),
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
                    with_icon("🔐", &tg_id(ids::TELEGRAM_LOGIN_REQUIRED)),
                )
                .parse_mode(ParseMode::Html)
                .await;
        }
        Err(e) => {
            let _ = bot
                .send_message(chat_id, with_icon("❌", &tg_escape(&e.to_string())))
                .parse_mode(ParseMode::Html)
                .await;
        }
    }

    Ok(())
}
