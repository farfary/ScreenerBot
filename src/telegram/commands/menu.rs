//! Menu command handlers
//!
//! Handles the interactive menu and navigation.

use crate::i18n::{ids, UiArg, UiText};
use crate::telegram::keyboards;
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::telegram::{Error, Result};
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Handle /menu command
pub async fn handle_menu_command(bot: &Bot, chat_id: ChatId) -> Result<()> {
    send_main_menu(bot, chat_id).await
}

/// Send the main menu to the user
pub async fn send_main_menu(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let message = with_icon("🤖", &tg_id(ids::TELEGRAM_MENU_TITLE));

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboards::main_menu())
        .await
        .map_err(|e| Error::SendFailed {
            chat_id: chat_id.0.to_string(),
            detail: e.to_string(),
        })?;

    Ok(())
}

/// Send positions menu
pub async fn send_positions_menu(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let positions = crate::positions::get_open_positions().await;

    if positions.is_empty() {
        let keyboard = keyboards::main_menu_compact();
        bot.send_message(
            chat_id,
            with_icon("📦", &tg_id(ids::TELEGRAM_MENU_POSITIONS_EMPTY)),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| Error::SendFailed {
            chat_id: chat_id.0.to_string(),
            detail: e.to_string(),
        })?;
        return Ok(());
    }

    // Build position list for keyboard
    let pos_list: Vec<(String, String, f64)> = positions
        .iter()
        .take(10)
        .map(|p| {
            (
                p.symbol.clone(),
                p.mint.clone(),
                p.unrealized_pnl_percent.unwrap_or_default(),
            )
        })
        .collect();

    let keyboard = keyboards::positions_list(&pos_list);

    let mut message = with_icon(
        "📊",
        &tg(&UiText::new(ids::TELEGRAM_MENU_POSITIONS_TITLE)
            .arg("count", UiArg::Text(positions.len().to_string()))),
    );
    message.push_str("\n\n");
    for (i, pos) in positions.iter().take(10).enumerate() {
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or_default();
        let emoji = if pnl_pct >= 0.0 { "🟢" } else { "🔴" };
        let sign = if pnl_pct >= 0.0 { "+" } else { "" };
        message.push_str(&format!(
            "{}. {} <b>${}</b> ({}{:.1}%)\n",
            i + 1,
            emoji,
            tg_escape(&pos.symbol),
            sign,
            pnl_pct
        ));
    }
    message.push('\n');
    message.push_str(&tg_id(ids::TELEGRAM_MENU_POSITIONS_HINT));

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

/// Send settings menu
pub async fn send_settings_menu(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let message = with_icon("⚙️", &tg_id(ids::TELEGRAM_MENU_SETTINGS));

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboards::settings_menu())
        .await
        .map_err(|e| Error::SendFailed {
            chat_id: chat_id.0.to_string(),
            detail: e.to_string(),
        })?;

    Ok(())
}
