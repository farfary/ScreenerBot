// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trading control commands
//!
//! Commands for enabling/disabling trading, force stop, pause/resume.

use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::keyboards;
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::telegram::{Error, Result};
use crate::trader;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Message with a technical error detail.
fn failure_message(icon: &str, id: MessageId, error: &impl std::fmt::Display) -> String {
    with_icon(
        icon,
        &tg(&UiText::new(id).arg("detail", UiArg::Text(error.to_string()))),
    )
}

/// The chat message of a trader start the setup or emergency-stop gate refused, or of a
/// start that failed; `None` when the trader runs.
fn start_refusal(
    result: std::result::Result<(), trader::Error>,
    failed: MessageId,
) -> Option<String> {
    match result {
        Ok(()) | Err(trader::Error::AlreadyRunning) => None,
        Err(trader::Error::TraderUnavailable) => Some(with_icon(
            "⚠️",
            &tg_id(ids::TELEGRAM_TRADER_START_UNAVAILABLE),
        )),
        Err(trader::Error::ForceStopActive) => Some(with_icon(
            "🚨",
            &tg_id(ids::TELEGRAM_TRADER_START_FORCE_STOPPED),
        )),
        Err(e) => Some(failure_message("❌", failed, &e)),
    }
}

/// Handle /start command - Welcome message and enable trading through the same gates
/// as every other trader start; a refusal is the reply instead of the welcome.
pub async fn handle_start_command() -> String {
    let started = trader::start_trader_checked().await.map(|_| ());
    if let Some(refusal) = start_refusal(started, ids::TELEGRAM_START_FAILED) {
        logger::warning(LogTag::Telegram, "Trading not enabled by the start command");
        return refusal;
    }

    with_icon("🚀", &tg_id(ids::TELEGRAM_START_READY))
}

/// Handle /stop command - Disable trading
pub async fn handle_stop_command() -> String {
    match trader::stop_trader_gracefully().await {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Trading disabled via Telegram command");
            with_icon("🛑", &tg_id(ids::TELEGRAM_STOP_DONE))
        }
        Err(trader::Error::AlreadyStopped) => with_icon("✅", &tg_id(ids::TELEGRAM_STOP_ALREADY)),
        Err(e) => failure_message("❌", ids::TELEGRAM_STOP_FAILED, &e),
    }
}

/// Handle /pause or /pause_entries command
pub async fn handle_pause_entries_command() -> String {
    match trader::set_monitor_enabled(trader::Monitor::Entry, false) {
        Ok(()) => {
            logger::info(LogTag::Telegram, "Entry monitor paused via Telegram");
            with_icon("⏸️", &tg_id(ids::TELEGRAM_PAUSE_DONE))
        }
        Err(e) => failure_message("❌", ids::TELEGRAM_PAUSE_FAILED, &e),
    }
}

/// Handle /resume or /resume_entries command
pub async fn handle_resume_entries_command() -> String {
    if let Err(e) = trader::set_monitor_enabled(trader::Monitor::Entry, true) {
        return failure_message("❌", ids::TELEGRAM_RESUME_FAILED, &e);
    }
    // Resuming entries also turns the trader on, so a stopped trader starts entering.
    let started = trader::start_trader_checked().await.map(|_| ());
    if let Some(refusal) = start_refusal(started, ids::TELEGRAM_RESUME_FAILED) {
        return refusal;
    }
    logger::info(LogTag::Telegram, "Entry monitor resumed via Telegram");
    with_icon("▶️", &tg_id(ids::TELEGRAM_RESUME_DONE))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn configure() {
        let _ = crate::config::utils::CONFIG
            .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
    }

    #[test]
    fn each_refused_start_names_its_gate_and_a_running_trader_none() {
        configure();
        assert_eq!(start_refusal(Ok(()), ids::TELEGRAM_START_FAILED), None);
        assert_eq!(
            start_refusal(
                Err(trader::Error::AlreadyRunning),
                ids::TELEGRAM_START_FAILED
            ),
            None
        );
        assert_eq!(
            start_refusal(
                Err(trader::Error::TraderUnavailable),
                ids::TELEGRAM_START_FAILED
            ),
            Some(with_icon(
                "⚠️",
                &tg_id(ids::TELEGRAM_TRADER_START_UNAVAILABLE)
            ))
        );
        assert_eq!(
            start_refusal(
                Err(trader::Error::ForceStopActive),
                ids::TELEGRAM_START_FAILED
            ),
            Some(with_icon(
                "🚨",
                &tg_id(ids::TELEGRAM_TRADER_START_FORCE_STOPPED)
            ))
        );
    }

    #[tokio::test]
    async fn the_start_command_refuses_before_setup_and_leaves_the_trader_off() {
        configure();
        assert!(!crate::global::is_initialization_complete());

        let reply = handle_start_command().await;

        assert_eq!(
            reply,
            with_icon("⚠️", &tg_id(ids::TELEGRAM_TRADER_START_UNAVAILABLE))
        );
        assert!(!crate::config::with_config(|cfg| cfg.trader.enabled));
    }
}
