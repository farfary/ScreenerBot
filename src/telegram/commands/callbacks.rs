//! Callback query handlers for inline keyboard buttons
//!
//! Handles button clicks from inline keyboards.

use super::check_auth;
use super::menu::{send_main_menu, send_positions_menu, send_settings_menu};
use super::status::{handle_balance_command, handle_stats_command, handle_status_command};
use super::trading::{execute_force_stop, handle_pause_entries_command, handle_stop_command};
use crate::config::{update_config_section, with_config};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::telegram::{formatters, keyboards, pagination::PAGINATION_MANAGER};
use crate::trader::manual::{manual_add, manual_sell};
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
        .map_err(|e| format!("Failed to answer callback: {}", e))?;

    let data = query.data.as_deref().unwrap_or("");
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
                        logger::warning(LogTag::Telegram, "Pagination callback without message context");
                        return Ok(());
                    }
                };
                
                if let Some((items, total_pages, total_items)) = PAGINATION_MANAGER.get_page(session_id, page_num) {
                    let text = formatters::format_tokens_page(&items, page_num, total_pages, total_items);
                    let keyboard = keyboards::pagination_keyboard(session_id, page_num, total_pages);

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
                        .map_err(|e| format!("Failed to update pagination: {}", e))?;
                } else {
                    bot.send_message(chat_id, "⚠️ Pagination session expired.")
                        .await
                        .map_err(|e| format!("Failed to send expiry message: {}", e))?;
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
        ["token", "blacklist", mint_short] => send_confirm_token_blacklist(bot, chat_id, mint_short).await,

        // Execute token actions (after confirmation)
        ["exec", "tokenbuy", mint_short, amount_str] => {
            let amount: f64 = amount_str.parse().unwrap_or(0.1);
            execute_token_buy(bot, chat_id, mint_short, amount).await
        }
        ["exec", "tokenbl", mint_short] => execute_token_blacklist(bot, chat_id, mint_short).await,

        _ => {
            logger::debug(LogTag::Telegram, &format!("Unknown callback: {}", data));
            Ok(())
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Send with inline keyboard
async fn send_with_keyboard(
    bot: &Bot,
    chat_id: ChatId,
    message: &str,
    keyboard: InlineKeyboardMarkup,
) -> Result<(), String> {
    bot.send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .map_err(|e| format!("Failed to send: {}", e))?;
    Ok(())
}

// ============================================================================
// POSITION DETAILS
// ============================================================================

async fn send_position_details(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let duration = (chrono::Utc::now() - pos.entry_time).num_seconds() as u64;
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or(0)) as f64;
            let current_price = pos.current_price.unwrap_or(pos.average_entry_price);
            let current_value = tokens * current_price;

            let msg = formatters::msg_position_detail(
                &pos.symbol,
                &pos.mint,
                pos.average_entry_price,
                current_price,
                pos.unrealized_pnl.unwrap_or(0.0),
                pos.unrealized_pnl_percent.unwrap_or(0.0),
                pos.total_size_sol,
                current_value,
                tokens,
                duration,
                pos.dca_count,
            );

            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::position_actions(&pos.mint, &pos.symbol),
            )
            .await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn send_history(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let positions = match positions::db::get_closed_positions().await {
        Ok(pos) => pos,
        Err(e) => {
            logger::warning(
                LogTag::Telegram,
                &format!("Failed to get closed positions: {}", e),
            );
            Vec::new()
        }
    };

    if positions.is_empty() {
        let msg = "📋 <b>Trade History</b>\n\nNo closed positions yet.";
        return send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await;
    }

    let mut msg = String::from("📋 <b>Recent Trades</b>\n\n");
    for pos in positions.iter().take(10) {
        let pnl = pos.pnl.unwrap_or(0.0);
        let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
        let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
        msg.push_str(&format!(
            "{} <b>{}</b>: {}{:.4} SOL\n",
            pnl_emoji, pos.symbol, pnl_sign, pnl
        ));
    }

    if positions.len() > 10 {
        msg.push_str(&format!(
            "\n<i>+{} more trades...</i>",
            positions.len() - 10
        ));
    }

    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
}

// ============================================================================
// CONFIRMATION DIALOGS
// ============================================================================

async fn send_confirm_sell(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    percent: u32,
) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or(0)) as f64;
            let msg = format!(
                "⚠️ <b>Confirm Sell</b>\n\n\
                 Token — {}\n\
                 Amount — {}%\n\
                 Tokens — {:.0}\n\n\
                 <i>Confirm within 30s to execute.</i>",
                pos.symbol,
                percent,
                tokens * (percent as f64 / 100.0)
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_sell(&pos.mint, percent),
            )
            .await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn send_confirm_dca(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let msg = format!(
                "⚠️ <b>Confirm Buy More</b>\n\n\
                 Token — {}\n\
                 Add — {} SOL\n\n\
                 <i>Confirm within 30s to execute.</i>",
                pos.symbol, amount
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_dca(&pos.mint, amount),
            )
            .await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn send_confirm_close(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or(0)) as f64;
            let est_receive = tokens * pos.current_price.unwrap_or(pos.average_entry_price);
            let msg = formatters::msg_confirm_close(
                &pos.symbol,
                pos.unrealized_pnl.unwrap_or(0.0),
                pos.unrealized_pnl_percent.unwrap_or(0.0),
                tokens,
                est_receive,
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_close(&pos.mint, &pos.symbol),
            )
            .await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn send_confirm_close_all(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let positions = positions::get_open_positions().await;
    let msg = format!(
        "⚠️ <b>Close All Positions?</b>\n\n\
         Count — {}\n\n\
         <i>This will market sell all open positions.\nConfirm within 30s.</i>",
        positions.len()
    );
    send_with_keyboard(bot, chat_id, &msg, keyboards::confirm_close_all()).await
}

async fn send_confirm_force_stop(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let msg = "🚨 <b>FORCE STOP</b>\n\n\
         This will immediately halt ALL trading:\n\
         • No new entries\n\
         • No exits\n\
         • No DCA\n\n\
         ⚠️ <b>This is an emergency action.</b>";
    send_with_keyboard(bot, chat_id, msg, keyboards::confirm_force_stop()).await
}

async fn send_confirm_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let msg = format!(
                "🚫 <b>Blacklist Token?</b>\n\n\
                 Token — {}\n\
                 Mint — <code>{}</code>\n\n\
                 <i>This will close the position and prevent future entries.</i>",
                pos.symbol,
                formatters::format_mint_display(&pos.mint)
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_blacklist(&pos.mint, &pos.symbol),
            )
            .await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

// ============================================================================
// EXECUTE ACTIONS
// ============================================================================

async fn execute_sell(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    percent: u32,
) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let msg = format!("⏳ Selling {}% of {}...", percent, pos.symbol);
            let _ = bot
                .send_message(chat_id, &msg)
                .parse_mode(ParseMode::Html)
                .await;

            match manual_sell(&pos.mint, Some(percent as f64)).await {
                Ok(result) => {
                    let msg = format!(
                        "✅ <b>Sell Executed</b>\n\n\
                         Token — {}\n\
                         Sold — {}%\n\
                         Received — {:.4} SOL",
                        pos.symbol,
                        percent,
                        result.executed_size_sol.unwrap_or(0.0)
                    );
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
                Err(e) => {
                    let msg = format!("❌ <b>Sell Failed</b>\n\nError: {}", e);
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
            }
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn execute_dca(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let msg = format!("⏳ Adding {} SOL to {}...", amount, pos.symbol);
            let _ = bot
                .send_message(chat_id, &msg)
                .parse_mode(ParseMode::Html)
                .await;

            match manual_add(&pos.mint, amount).await {
                Ok(_) => {
                    let msg = format!(
                        "✅ <b>DCA Executed</b>\n\n\
                         Token — {}\n\
                         Added — {} SOL",
                        pos.symbol, amount
                    );
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
                Err(e) => {
                    let msg = format!("❌ <b>DCA Failed</b>\n\nError: {}", e);
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
            }
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
}

async fn execute_close(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<(), String> {
    execute_sell(bot, chat_id, mint_short, 100).await
}

async fn execute_close_all(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        let msg = "❌ No positions to close";
        return send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await;
    }

    let _ = bot
        .send_message(chat_id, "⏳ Closing all positions...")
        .parse_mode(ParseMode::Html)
        .await;

    let mut success = 0;
    let mut failed = 0;

    for pos in &positions {
        match manual_sell(&pos.mint, Some(100.0)).await {
            Ok(_) => success += 1,
            Err(_) => failed += 1,
        }
    }

    let msg = format!(
        "📊 <b>Close All Complete</b>\n\n\
         ✅ Closed — {}\n\
         ❌ Failed — {}",
        success, failed
    );
    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
}

async fn execute_force_stop_callback(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let msg = execute_force_stop().await;
    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
}

async fn execute_blacklist(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<(), String> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            // First close the position
            let _ = manual_sell(&pos.mint, Some(100.0)).await;

            // Add to blacklist
            let mint_clone = pos.mint.clone();
            let blacklist_result = tokio::task::spawn_blocking(move || {
                if let Some(db) = crate::tokens::get_global_database() {
                    crate::tokens::cleanup::blacklist_token(
                        &mint_clone,
                        "Blacklisted via Telegram",
                        &db,
                    )
                } else {
                    Err(crate::tokens::TokenError::Database(
                        "Database not available".to_string(),
                    ))
                }
            })
            .await;

            if let Err(e) = blacklist_result {
                logger::warning(LogTag::Telegram, &format!("Failed to blacklist: {}", e));
            } else if let Ok(Err(e)) = blacklist_result {
                logger::warning(LogTag::Telegram, &format!("Failed to blacklist: {}", e));
            }

            let msg = format!(
                "🚫 <b>Token Blacklisted</b>\n\n\
                 Token — {}\n\
                 Status — Closed & Blacklisted",
                pos.symbol
            );
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
        }
        None => {
            let msg = "❌ Position not found";
            send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await
        }
    }
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
            &format!("Failed to toggle {}: {}", setting, e),
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

// ============================================================================
// TOKEN EXPLORER
// ============================================================================

/// Send token explorer main menu
pub async fn send_tokens_menu(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch stats: {}", e);
            return send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await;
        }
    };

    let msg = format!(
        "🔍 <b>Market Explorer</b>\n\n\
         <b>Overview</b>\n\
         Passed Filter — {}\n\
         Rejected — {}\n\
         Active Prices — {}\n\
         Total Discovered — {}\n\n\
         <i>Select a category to browse:</i>",
        stats.passed_filtering,
        stats.total_tokens.saturating_sub(stats.passed_filtering),
        stats.with_pool_price,
        stats.total_tokens
    );

    send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
}

/// Send paginated token list for a view
pub async fn send_tokens_list(bot: &Bot, chat_id: ChatId, view: &str) -> Result<(), String> {
    send_tokens_page(bot, chat_id, view, 1).await
}

/// Send a specific page of tokens
async fn send_tokens_page(
    bot: &Bot,
    chat_id: ChatId,
    view: &str,
    page: usize,
) -> Result<(), String> {
    use crate::filtering::types::{FilteringQuery, FilteringView, SortDirection, TokenSortKey};

    let filtering_view = match view {
        "passed" => FilteringView::Passed,
        "rejected" => FilteringView::Rejected,
        "recent" => FilteringView::Recent,
        "all" => FilteringView::All,
        _ => FilteringView::Passed,
    };

    let query = FilteringQuery {
        view: filtering_view,
        page,
        page_size: 10, // 10 tokens per page for Telegram
        sort_key: TokenSortKey::LiquidityUsd,
        sort_direction: SortDirection::Desc,
        ..Default::default()
    };

    let result = match crate::filtering::query_tokens(query).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("❌ Failed to fetch tokens: {}", e);
            return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
        }
    };

    if result.items.is_empty() {
        let msg = format!("📭 No tokens found in <b>{}</b> view.", view);
        return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
    }

    let view_emoji = match view {
        "passed" => "✅",
        "rejected" => "❌",
        "recent" => "🆕",
        "all" => "📋",
        _ => "📊",
    };

    let view_name = match view {
        "passed" => "Passed Filter",
        "rejected" => "Rejected",
        "recent" => "Recently Added",
        "all" => "All Tokens",
        _ => view,
    };

    let mut msg = format!(
        "{} <b>{}</b> (Page {}/{})\n\n",
        view_emoji,
        view_name,
        result.page,
        result.total_pages
    );

    for (i, token) in result.items.iter().enumerate() {
        let idx = (page - 1) * 10 + i + 1;
        let symbol = &token.symbol;
        let mint_short = &token.mint[..8.min(token.mint.len())];

        // Format liquidity
        let liquidity = token
            .liquidity_usd
            .map(|l| {
                if l >= 1_000_000.0 {
                    format!("${:.1}M", l / 1_000_000.0)
                } else if l >= 1_000.0 {
                    format!("${:.1}K", l / 1_000.0)
                } else {
                    format!("${:.0}", l)
                }
            })
            .unwrap_or_else(|| "N/A".to_string());
        
        // Format price
        let price = if token.price_sol > 0.0 {
             format!("{} SOL", formatters::format_price(token.price_sol))
        } else {
            "N/A".to_string()
        };

        // Add rejection reason for rejected view
        let reason_part = if view == "rejected" {
            result
                .rejection_reasons
                .get(&token.mint)
                .map(|r| format!("\n   └ ⚠️ {}", r))
                .unwrap_or_default()
        } else {
            String::new()
        };

        msg.push_str(&format!(
            "{}. <b>${}</b> ({})\n   Liq: {} • Price: {}{}\n   /token_{}\n\n",
            idx, symbol, mint_short, liquidity, price, reason_part, mint_short
        ));
    }

    msg.push_str("<i>Tap /token_ID to view details</i>");

    let keyboard = keyboards::tokens_list_keyboard(view, page, result.total_pages);
    send_with_keyboard(bot, chat_id, &msg, keyboard).await
}

/// Send filter statistics
async fn send_filter_stats(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch stats: {}", e);
            return send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await;
        }
    };

    let rejected_count = stats.total_tokens.saturating_sub(stats.passed_filtering);
    let passed_pct = if stats.total_tokens > 0 {
        (stats.passed_filtering as f64 / stats.total_tokens as f64) * 100.0
    } else {
        0.0
    };
    let rejected_pct = if stats.total_tokens > 0 {
        (rejected_count as f64 / stats.total_tokens as f64) * 100.0
    } else {
        0.0
    };

    let msg = format!(
        "📊 <b>Filter Analysis</b>\n\n\
         <b>Distribution</b>\n\
         ✅ Passed — {} ({:.1}%)\n\
         ❌ Rejected — {} ({:.1}%)\n\
         🚫 Blacklisted — {}\n\n\
         <b>Coverage</b>\n\
         💰 With Pool Price — {}\n\
         📈 Open Positions — {}\n\
         📋 Total Discovered — {}\n\n\
         <b>Last Updated</b>\n\
         🕐 {}\n\n\
         <i>Auto-refreshes every 3m</i>",
        stats.passed_filtering,
        passed_pct,
        rejected_count,
        rejected_pct,
        stats.blacklisted,
        stats.with_pool_price,
        stats.open_positions,
        stats.total_tokens,
        stats.updated_at.format("%H:%M:%S UTC")
    );

    send_with_keyboard(bot, chat_id, &msg, keyboards::filter_stats_keyboard()).await
}

/// Send token detail view
pub async fn send_token_detail(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<(), String> {
    use crate::tokens::get_full_token_async;

    // Try to find token by mint prefix from the filtering store
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = "❌ Token not found. Try searching with a longer prefix.";
            return send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await;
        }
    };

    // Check if user has a position
    let has_position = positions::get_open_positions()
        .await
        .iter()
        .any(|p| p.mint == token.mint);

    // Format token details
    let liquidity = token
        .liquidity_usd
        .map(|l| formatters::format_usd(l))
        .unwrap_or_else(|| "N/A".to_string());
    let volume_24h = token
        .volume_h24
        .map(|v| formatters::format_usd(v))
        .unwrap_or_else(|| "N/A".to_string());
    let price_change = token
        .price_change_h24
        .map(|c| format!("{:+.2}%", c))
        .unwrap_or_else(|| "N/A".to_string());

    let risk_text = token
        .security_score_normalised
        .map(|s| {
            let emoji = if s <= 30 {
                "🟢"
            } else if s <= 60 {
                "🟡"
            } else {
                "🔴"
            };
            format!("{} Risk Assessment: {}/100", emoji, s)
        })
        .unwrap_or_else(|| "⚪ Risk Assessment: Unknown".to_string());

    let position_text = if has_position {
        "✅ <b>Active Position</b>\n\n"
    } else {
        ""
    };

    let msg = format!(
        "🪙 <b>{}</b> (${})\n\
         <code>{}</code>\n\n\
         {}\
         Price — {} SOL\n\
         Liquidity — {}\n\
         24h Volume — {}\n\
         24h Change — {}\n\n\
         {}\n\n\
         <i>Select action:</i>",
        token.name,
        token.symbol,
        formatters::format_mint_display(&token.mint),
        position_text,
        formatters::format_price(token.price_sol),
        liquidity,
        volume_24h,
        price_change,
        risk_text
    );

    send_with_keyboard(
        bot,
        chat_id,
        &msg,
        keyboards::token_detail_keyboard(&token.mint, has_position),
    )
    .await
}

/// Find a token by mint prefix from the filtering store
async fn find_token_by_prefix(prefix: &str) -> Option<crate::tokens::types::Token> {
    use crate::filtering::types::{FilteringQuery, FilteringView};

    // Search across all tokens
    let query = FilteringQuery {
        view: FilteringView::All,
        search: Some(prefix.to_string()),
        page: 1,
        page_size: 1,
        ..Default::default()
    };

    match crate::filtering::query_tokens(query).await {
        Ok(result) if !result.items.is_empty() => Some(result.items.into_iter().next().unwrap()),
        _ => None,
    }
}

/// Send search prompt
async fn send_search_prompt(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let msg = "🔍 <b>Search Market</b>\n\n\
               Enter symbol or mint address to search:\n\n\
               <i>Example: /token_BONK or /token_So11111</i>";
    send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await
}

/// Confirmation dialog for buying a token (from token explorer)
async fn send_confirm_token_buy(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<(), String> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = "❌ Token not found";
            return send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await;
        }
    };

    let msg = format!(
        "💰 <b>Confirm Direct Buy</b>\n\n\
         Token — ${}\n\
         Mint — <code>{}</code>\n\
         Amount — {} SOL\n\n\
         <i>Confirm within 30s to execute.</i>",
        token.symbol,
        formatters::format_mint_display(&token.mint),
        amount
    );

    send_with_keyboard(
        bot,
        chat_id,
        &msg,
        keyboards::confirm_token_buy(&token.mint, &token.symbol, amount),
    )
    .await
}

/// Confirmation dialog for blacklisting a token (from token explorer - not position)
async fn send_confirm_token_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<(), String> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = "❌ Token not found";
            return send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await;
        }
    };

    let msg = format!(
        "🚫 <b>Blacklist Token?</b>\n\n\
         Token — ${}\n\
         Mint — <code>{}</code>\n\n\
         <i>This will prevent this token from satisfying filters.</i>",
        token.symbol,
        formatters::format_mint_display(&token.mint)
    );

    send_with_keyboard(
        bot,
        chat_id,
        &msg,
        keyboards::confirm_token_blacklist(&token.mint, &token.symbol),
    )
    .await
}

/// Execute token blacklist (from token explorer - not position)
async fn execute_token_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<(), String> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = "❌ Token not found";
            return send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await;
        }
    };

    // Add to blacklist using token database
    let mint_clone = token.mint.clone();
    let blacklist_result = tokio::task::spawn_blocking(move || {
        if let Some(db) = crate::tokens::get_global_database() {
            crate::tokens::cleanup::blacklist_token(&mint_clone, "Blacklisted via Telegram", &db)
        } else {
            Err(crate::tokens::TokenError::Database(
                "Database not available".to_string(),
            ))
        }
    })
    .await;

    match blacklist_result {
        Ok(Ok(())) => {
            let msg = format!(
                "🚫 <b>Token Blacklisted</b>\n\n\
                 Token — ${}\n\
                 Status — Added to blacklist",
                token.symbol
            );
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
        Ok(Err(e)) => {
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {}", e));
            let msg = format!("❌ <b>Blacklist Failed</b>\n\nError: {}", e);
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
        Err(e) => {
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {}", e));
            let msg = format!("❌ <b>Blacklist Failed</b>\n\nError: {}", e);
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
    }
}

/// Execute token buy (quick buy from token explorer)
async fn execute_token_buy(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<(), String> {
    // Find token by mint prefix
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = "❌ Token not found";
            return send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await;
        }
    };

    let msg = format!(
        "💰 <b>Processing Buy...</b>\n\n\
         Token — ${}\n\
         Amount — {} SOL",
        token.symbol,
        amount
    );

    bot.send_message(chat_id, &msg)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| format!("Failed to send: {}", e))?;

    // Execute the buy via manual trading system
    match manual_add(&token.mint, amount).await {
        Ok(_) => {
            let success_msg = format!(
                "✅ <b>Buy Successful</b>\n\n\
                 Token — ${}\n\
                 Amount — {} SOL\n\n\
                 <i>View details in /positions</i>",
                token.symbol, amount
            );
            send_with_keyboard(bot, chat_id, &success_msg, keyboards::main_menu_compact()).await
        }
        Err(e) => {
            let error_msg = format!(
                "❌ <b>Buy Failed</b>\n\n\
                 Token — ${}\n\
                 Error — {}",
                token.symbol, e
            );
            send_with_keyboard(bot, chat_id, &error_msg, keyboards::tokens_menu()).await
        }
    }
}
