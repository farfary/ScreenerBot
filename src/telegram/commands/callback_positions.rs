//! Position-related callback handlers
//!
//! Handles position details, sell/close/DCA confirmations, and execution.

use super::callbacks::send_with_keyboard;
use super::trading::execute_force_stop;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::telegram::Result;
use crate::telegram::{formatters, keyboards};
use crate::trader::manual::{manual_add, manual_sell};
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

// ============================================================================
// POSITION DETAILS
// ============================================================================

pub(super) async fn send_position_details(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<()> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let duration = (chrono::Utc::now() - pos.entry_time).num_seconds().max(0) as u64;
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or_default()) as f64;
            let current_price = pos.current_price.unwrap_or(pos.average_entry_price);
            let current_value = tokens * current_price;

            let msg = formatters::msg_position_detail(
                &pos.symbol,
                &pos.mint,
                pos.average_entry_price,
                current_price,
                pos.unrealized_pnl.unwrap_or_default(),
                pos.unrealized_pnl_percent.unwrap_or_default(),
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

pub(super) async fn send_history(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let positions = match positions::db::get_closed_positions().await {
        Ok(pos) => pos,
        Err(e) => {
            logger::warning(
                LogTag::Telegram,
                &format!("Failed to get closed positions: {e}"),
            );
            Vec::new()
        }
    };

    if positions.is_empty() {
        let msg = "📋 <b>Trade History</b>\n\nNo closed positions yet.";
        return send_with_keyboard(bot, chat_id, msg, keyboards::main_menu_compact()).await;
    }

    let mut msg = "📋 <b>Recent Trades</b>\n\n".to_owned();
    for pos in positions.iter().take(10) {
        let pnl = pos.pnl.unwrap_or_default();
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

pub(super) async fn send_confirm_sell(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    percent: u32,
) -> Result<()> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or_default()) as f64;
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

pub(super) async fn send_confirm_dca(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<()> {
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

pub(super) async fn send_confirm_close(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<()> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            let tokens = pos
                .remaining_token_amount
                .unwrap_or(pos.token_amount.unwrap_or_default()) as f64;
            let est_receive = tokens * pos.current_price.unwrap_or(pos.average_entry_price);
            let msg = formatters::msg_confirm_close(
                &pos.symbol,
                pos.unrealized_pnl.unwrap_or_default(),
                pos.unrealized_pnl_percent.unwrap_or_default(),
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

pub(super) async fn send_confirm_close_all(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let positions = positions::get_open_positions().await;
    let msg = format!(
        "⚠️ <b>Close All Positions?</b>\n\n\
         Count — {}\n\n\
         <i>This will market sell all open positions.\nConfirm within 30s.</i>",
        positions.len()
    );
    send_with_keyboard(bot, chat_id, &msg, keyboards::confirm_close_all()).await
}

pub(super) async fn send_confirm_force_stop(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = "🚨 <b>FORCE STOP</b>\n\n\
         This will immediately halt ALL trading:\n\
         • No new entries\n\
         • No exits\n\
         • No DCA\n\n\
         ⚠️ <b>This is an emergency action.</b>";
    send_with_keyboard(bot, chat_id, msg, keyboards::confirm_force_stop()).await
}

pub(super) async fn send_confirm_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<()> {
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

pub(super) async fn execute_sell(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    percent: u32,
) -> Result<()> {
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

            match manual_sell(&pos.mint, Some(percent as f64), None).await {
                Ok(result) => {
                    let msg = format!(
                        "✅ <b>Sell Executed</b>\n\n\
                         Token — {}\n\
                         Sold — {}%\n\
                         Received — {:.4} SOL",
                        pos.symbol,
                        percent,
                        result.executed_size_sol.unwrap_or_default()
                    );
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
                Err(e) => {
                    let msg = format!("❌ <b>Sell Failed</b>\n\nError: {e}");
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

pub(super) async fn execute_dca(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<()> {
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

            match manual_add(&pos.mint, amount, None).await {
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
                    let msg = format!("❌ <b>DCA Failed</b>\n\nError: {e}");
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

pub(super) async fn execute_close(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<()> {
    execute_sell(bot, chat_id, mint_short, 100).await
}

pub(super) async fn execute_close_all(bot: &Bot, chat_id: ChatId) -> Result<()> {
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
        match manual_sell(&pos.mint, Some(100.0), None).await {
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

pub(super) async fn execute_force_stop_callback(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = execute_force_stop().await;
    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
}

pub(super) async fn execute_blacklist(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<()> {
    let positions_list = positions::get_open_positions().await;
    let position = positions_list
        .iter()
        .find(|p| p.mint.starts_with(mint_short));

    match position {
        Some(pos) => {
            // First close the position
            let _ = manual_sell(&pos.mint, Some(100.0), None).await;

            // Add to blacklist
            let mint_clone = pos.mint.clone();
            let blacklist_result = tokio::task::spawn_blocking(move || {
                if let Some(db) = crate::tokens::get_global_database() {
                    crate::tokens::cleanup::blacklist_token(
                        &mint_clone,
                        "Blacklisted via Telegram",
                        "manual",
                        &db,
                    )
                } else {
                    Err(crate::tokens::Error::NotInitialized {
                        resource: "token database".to_owned(),
                    })
                }
            })
            .await;

            if let Err(e) = blacklist_result {
                logger::warning(LogTag::Telegram, &format!("Failed to blacklist: {e}"));
            } else if let Ok(Err(e)) = blacklist_result {
                logger::warning(LogTag::Telegram, &format!("Failed to blacklist: {e}"));
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
