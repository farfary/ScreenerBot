//! Callback query handlers for inline keyboard buttons
//!
//! Handles button clicks from inline keyboards.

use super::callback_positions::{
    execute_blacklist, execute_close, execute_close_all, execute_dca,
    execute_force_stop_callback, execute_sell, send_confirm_blacklist, send_confirm_close,
    send_confirm_close_all, send_confirm_dca, send_confirm_force_stop, send_confirm_sell,
    send_history, send_position_details,
};
use super::callback_tokens::{
    execute_token_blacklist, execute_token_buy, send_confirm_token_blacklist,
    send_confirm_token_buy, send_filter_stats, send_search_prompt, send_token_detail,
    send_tokens_list, send_tokens_menu, send_tokens_page,
};
use super::check_auth;
use super::menu::{send_main_menu, send_positions_menu, send_settings_menu};
use super::status::{handle_balance_command, handle_stats_command, handle_status_command};
use super::trading::{handle_pause_entries_command, handle_stop_command};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::telegram::{formatters, keyboards, pagination::PAGINATION_MANAGER};
use teloxide::prelude::*;
use teloxide::types::{ChatId, InlineKeyboardMarkup, ParseMode};

/// Handle callback query from inline keyboard button
pub async fn handle_callback_query(
    bot: &Bot,
    chat_id: ChatId,
    user_id: i64,
    query: teloxide::types::CallbackQuery,
) -> Result<(), String> {
    // Always answer callback query first to remove loading indicator
    bot.answer_callback_query(&query.id)
        .await
        .map_err(|e| format!("Failed to answer callback: {e}"))?;

    let data = query.data.as_deref().unwrap_or_default();
    let parts: Vec<&str> = data.split(':').collect();

    // Check authentication for sensitive callbacks
    let is_sensitive_callback = !parts.is_empty()
        && (parts[0].starts_with("exec")
            || parts[0].starts_with("confirm")
            || parts[0] == "sell"
            || parts[0] == "dca"
            || parts[0] == "close"
            || parts[0] == "bl"
            || parts[0] == "toggle"
            || parts[0] == "token");

    if is_sensitive_callback && !check_auth(bot, chat_id, user_id).await {
        return Ok(()); // Auth check failed, message already sent
    }

    match parts.as_slice() {
        // Menu navigation
        ["menu", "main"] => send_main_menu(bot, chat_id).await,

        // Pagination
        ["page", session_id, page_num_str, ..] => {
            if let Ok(page_num) = page_num_str.parse::<usize>() {
                // Get message ID - required for editing
                let message_id = match query.message.as_ref() {
                    Some(msg) => msg.id(),
                    None => {
                        logger::warning(
                            LogTag::Telegram,
                            "Pagination callback without message context",
                        );
                        return Ok(());
                    }
                };

                if let Some((items, total_pages, total_items)) =
                    PAGINATION_MANAGER.get_page(session_id, page_num)
                {
                    let text =
                        formatters::format_tokens_page(&items, page_num, total_pages, total_items);
                    let keyboard =
                        keyboards::pagination_keyboard(session_id, page_num, total_pages);

                    // Update the message
                    bot.edit_message_text(chat_id, message_id, text)
                        .parse_mode(ParseMode::Html)
                        .link_preview_options(teloxide::types::LinkPreviewOptions {
                            is_disabled: true,
                            url: None,
                            prefer_small_media: false,
                            prefer_large_media: false,
                            show_above_text: false,
                        })
                        .reply_markup(keyboard)
                        .await
                        .map_err(|e| format!("Failed to update pagination: {e}"))?;
                } else {
                    bot.send_message(chat_id, "⚠️ Pagination session expired.")
                        .await
                        .map_err(|e| format!("Failed to send expiry message: {e}"))?;
                }
            }
            Ok(())
        }

        ["noop"] => Ok(()),

        ["menu", "positions"] => send_positions_menu(bot, chat_id).await,
        ["menu", "settings"] => send_settings_menu(bot, chat_id).await,
        ["menu", "refresh"] => send_main_menu(bot, chat_id).await,

        // Commands
        ["cmd", "status"] => {
            let msg = handle_status_command().await;
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
        }
        ["cmd", "balance"] => {
            let msg = handle_balance_command().await;
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
        }
        ["cmd", "stats"] => {
            let msg = handle_stats_command().await;
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
        }
        ["cmd", "stop_trader"] => {
            let msg = handle_stop_command().await;
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
        }
        ["cmd", "pause_entries"] => {
            let msg = handle_pause_entries_command().await;
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
        }
        ["cmd", "history"] => send_history(bot, chat_id).await,

        // Authentication
        ["auth", "cancel"] => send_main_menu(bot, chat_id).await,
        ["auth", "start"] => {
            let msg = "🔑 <b>Authentication Required</b>\n\n\
                       Please enter your password to continue.\n\n\
                       <i>Type your password and send it.</i>";
            send_with_keyboard(bot, chat_id, msg, keyboards::auth_prompt()).await
        }

        // Position actions
        ["pos", mint_short] => send_position_details(bot, chat_id, mint_short).await,
        ["sell", mint_short, percent] => {
            let pct: u32 = percent.parse().unwrap_or(100);
            send_confirm_sell(bot, chat_id, mint_short, pct).await
        }
        ["dca", mint_short, amount] => {
            let amt: f64 = amount.parse().unwrap_or(0.1);
            send_confirm_dca(bot, chat_id, mint_short, amt).await
        }

        // Confirmations
        ["confirm", "close", mint_short] => send_confirm_close(bot, chat_id, mint_short).await,
        ["confirm", "closeall"] => send_confirm_close_all(bot, chat_id).await,
        ["confirm", "force_stop"] => send_confirm_force_stop(bot, chat_id).await,

        // Execute actions
        ["exec", "close", mint_short] => execute_close(bot, chat_id, mint_short).await,
        ["exec", "closeall"] => execute_close_all(bot, chat_id).await,
        ["exec", "sell", mint_short, percent] => {
            let pct: u32 = percent.parse().unwrap_or(100);
            execute_sell(bot, chat_id, mint_short, pct).await
        }
        ["exec", "dca", mint_short, amount] => {
            let amt: f64 = amount.parse().unwrap_or(0.1);
            execute_dca(bot, chat_id, mint_short, amt).await
        }
        ["exec", "force_stop"] => execute_force_stop_callback(bot, chat_id).await,
        ["exec", "bl", mint_short] => execute_blacklist(bot, chat_id, mint_short).await,

        // Blacklist
        ["bl", mint_short] => send_confirm_blacklist(bot, chat_id, mint_short).await,

        // Cancel actions
        ["cancel", _, _] | ["cancel", _] => send_main_menu(bot, chat_id).await,

        // Settings toggles
        ["toggle", setting] => handle_toggle(bot, chat_id, setting).await,
        ["settings", section] => handle_settings_section(bot, chat_id, section).await,

        // Token Explorer navigation
        ["menu", "tokens"] => send_tokens_menu(bot, chat_id).await,
        ["tokens", "menu"] => send_tokens_menu(bot, chat_id).await,
        ["tokens", "passed"] => send_tokens_list(bot, chat_id, "passed").await,
        ["tokens", "rejected"] => send_tokens_list(bot, chat_id, "rejected").await,
        ["tokens", "recent"] => send_tokens_list(bot, chat_id, "recent").await,
        ["tokens", "all"] => send_tokens_list(bot, chat_id, "all").await,
        ["tokens", "stats"] => send_filter_stats(bot, chat_id).await,
        ["tokens", "stats", "refresh"] => send_filter_stats(bot, chat_id).await,
        ["tokens", "search"] => send_search_prompt(bot, chat_id).await,
        ["tokens", "page", view, page_str] => {
            let page = page_str.parse::<usize>().unwrap_or(1);
            send_tokens_page(bot, chat_id, view, page).await
        }
        ["tokens", "refresh", view] => send_tokens_list(bot, chat_id, view).await,

        // Token detail & actions
        ["token", "view", mint_short] => send_token_detail(bot, chat_id, mint_short).await,
        ["token", "buy", mint_short, amount_str] => {
            let amount: f64 = amount_str.parse().unwrap_or(0.1);
            send_confirm_token_buy(bot, chat_id, mint_short, amount).await
        }
        ["token", "blacklist", mint_short] => {
            send_confirm_token_blacklist(bot, chat_id, mint_short).await
        }

        // Execute token actions (after confirmation)
        ["exec", "tokenbuy", mint_short, amount_str] => {
            let amount: f64 = amount_str.parse().unwrap_or(0.1);
            execute_token_buy(bot, chat_id, mint_short, amount).await
        }
        ["exec", "tokenbl", mint_short] => execute_token_blacklist(bot, chat_id, mint_short).await,

        _ => {
            logger::debug(LogTag::Telegram, &format!("Unknown callback: {data}"));
            Ok(())
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Send with inline keyboard
pub(super) async fn send_with_keyboard(
    bot: &Bot,
    chat_id: ChatId,
    message: &str,
    keyboard: InlineKeyboardMarkup,
) -> Result<(), String> {
    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| format!("Failed to send: {e}"))?;
    Ok(())
}

// ============================================================================
// SETTINGS HANDLERS
// ============================================================================

async fn handle_toggle(bot: &Bot, chat_id: ChatId, setting: &str) -> Result<(), String> {
    let result = match setting {
        "entry_monitor" => update_config_section(
            |cfg| {
                cfg.trader.entry_monitor_enabled = !cfg.trader.entry_monitor_enabled;
            },
            true,
        ),
        "exit_monitor" => update_config_section(
            |cfg| {
                cfg.trader.exit_monitor_enabled = !cfg.trader.exit_monitor_enabled;
            },
            true,
        ),
        "notify_opened" => update_config_section(
            |cfg| {
                cfg.telegram.notify_position_opened = !cfg.telegram.notify_position_opened;
            },
            true,
        ),
        "notify_closed" => update_config_section(
            |cfg| {
                cfg.telegram.notify_position_closed = !cfg.telegram.notify_position_closed;
            },
            true,
        ),
        "notify_partial" => update_config_section(
            |cfg| {
                cfg.telegram.notify_partial_exit = !cfg.telegram.notify_partial_exit;
            },
            true,
        ),
        "notify_dca" => update_config_section(
            |cfg| {
                cfg.telegram.notify_dca_executed = !cfg.telegram.notify_dca_executed;
            },
            true,
        ),
        "notify_errors" => update_config_section(
            |cfg| {
                cfg.telegram.notify_system_errors = !cfg.telegram.notify_system_errors;
            },
            true,
        ),
        _ => return Ok(()),
    };

    if let Err(e) = result {
        logger::warning(
            LogTag::Telegram,
            &format!("Failed to toggle {setting}: {e}"),
        );
    }

    // Refresh the settings menu
    send_settings_menu(bot, chat_id).await
}

async fn handle_settings_section(bot: &Bot, chat_id: ChatId, section: &str) -> Result<(), String> {
    match section {
        "notifications" => {
            let config = with_config(|c| c.telegram.clone());
            let keyboard = keyboards::notification_settings(
                config.notify_position_opened,
                config.notify_position_closed,
                config.notify_partial_exit,
                config.notify_dca_executed,
                config.notify_system_errors,
            );
            let msg = "🔔 <b>Notification Settings</b>\n\n\
                       Toggle notifications on/off:";
            send_with_keyboard(bot, chat_id, msg, keyboard).await
        }
        "trading" => {
            let config = with_config(|c| c.trader.clone());
            let keyboard = keyboards::trading_controls(
                config.entry_monitor_enabled,
                config.exit_monitor_enabled,
                config.enabled,
            );
            let msg = "⚡ <b>Trading Controls</b>\n\n\
                       Toggle trading features:";
            send_with_keyboard(bot, chat_id, msg, keyboard).await
        }
        _ => send_settings_menu(bot, chat_id).await,
    }
}
