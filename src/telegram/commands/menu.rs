//! Menu command handlers
//!
//! Handles the interactive menu and navigation.

use crate::telegram::keyboards;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

/// Handle /menu command
pub async fn handle_menu_command(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    send_main_menu(bot, chat_id).await
}

/// Send the main menu to the user
pub async fn send_main_menu(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let message = "🤖 <b>ScreenerBot Control Panel</b>\n\n\
        Select an option below to view information or control the bot.";

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboards::main_menu())
        .await
        .map_err(|e| format!("Failed to send menu: {}", e))?;

    Ok(())
}

/// Send positions menu
pub async fn send_positions_menu(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let positions = crate::positions::get_open_positions().await;

    if positions.is_empty() {
        let keyboard = keyboards::main_menu_compact();
        bot.send_message(
            chat_id,
            "📊 <b>No Open Positions</b>\n\nYou have no active positions.",
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| format!("Failed to send positions: {}", e))?;
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
                p.unrealized_pnl_percent.unwrap_or(0.0),
            )
        })
        .collect();

    let keyboard = keyboards::positions_list(&pos_list);

    let mut message = format!("📊 <b>Open Positions ({})</b>\n\n", positions.len());
    for (i, pos) in positions.iter().take(10).enumerate() {
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or(0.0);
        let emoji = if pnl_pct >= 0.0 { "🟢" } else { "🔴" };
        let sign = if pnl_pct >= 0.0 { "+" } else { "" };
        message.push_str(&format!(
            "{}. {} ${} ({}{:.1}%)\n",
            i + 1,
            emoji,
            pos.symbol,
            sign,
            pnl_pct
        ));
    }
    message.push_str("\nClick a position for actions.");

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| format!("Failed to send positions: {}", e))?;

    Ok(())
}

/// Send settings menu
pub async fn send_settings_menu(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let message = "⚙️ <b>Settings</b>\n\n\
        Configure notifications and trading controls.";

    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboards::settings_menu())
        .await
        .map_err(|e| format!("Failed to send settings: {}", e))?;

    Ok(())
}
