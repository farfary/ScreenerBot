//! Position-related callback handlers
//!
//! Handles position details, sell/close/DCA confirmations, and execution.

use super::callbacks::send_with_keyboard;
use super::trading::execute_force_stop;
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::telegram::formatters::{self, row, text_arg};
use crate::telegram::text::{tg, tg_id, with_icon};
use crate::telegram::Result;
use crate::telegram::{keyboards, messages};
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

            let msg = messages::msg_position_detail(
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
        None => send_not_found(bot, chat_id).await,
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
        let msg = with_icon("📋", &tg_id(ids::TELEGRAM_POSITION_HISTORY_EMPTY));
        return send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await;
    }

    let mut msg = with_icon("📋", &tg_id(ids::TELEGRAM_POSITION_HISTORY_TITLE));
    msg.push_str("\n\n");
    for pos in positions.iter().take(10) {
        let pnl = pos.pnl.unwrap_or_default();
        let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
        let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
        msg.push_str(&with_icon(
            pnl_emoji,
            &format!(
                "{}: {}",
                formatters::bold(&pos.symbol),
                tg(&UiText::new(ids::TELEGRAM_AMOUNT_SOL)
                    .arg("amount", text_arg(format!("{pnl_sign}{pnl:.4}"))))
            ),
        ));
        msg.push('\n');
    }

    if positions.len() > 10 {
        msg.push('\n');
        msg.push_str(&tg(&UiText::new(ids::TELEGRAM_POSITION_HISTORY_MORE)
            .arg("count", text_arg((positions.len() - 10).to_string()))));
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
            let msg = confirm_screen(
                UiText::new(ids::TELEGRAM_POSITION_CONFIRM_SELL)
                    .arg("symbol", text_arg(pos.symbol.as_str()))
                    .arg("percent", text_arg(percent.to_string()))
                    .arg(
                        "tokens",
                        text_arg(format!("{:.0}", tokens * (percent as f64 / 100.0))),
                    ),
                ids::TELEGRAM_POSITION_CONFIRM_HINT,
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_sell(&pos.mint, percent),
            )
            .await
        }
        None => send_not_found(bot, chat_id).await,
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
            let msg = confirm_screen(
                UiText::new(ids::TELEGRAM_POSITION_CONFIRM_DCA)
                    .arg("symbol", text_arg(pos.symbol.as_str()))
                    .arg("amount", text_arg(amount.to_string())),
                ids::TELEGRAM_POSITION_CONFIRM_HINT,
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_dca(&pos.mint, amount),
            )
            .await
        }
        None => send_not_found(bot, chat_id).await,
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
            let msg = messages::msg_confirm_close(
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
        None => send_not_found(bot, chat_id).await,
    }
}

pub(super) async fn send_confirm_close_all(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let positions = positions::get_open_positions().await;
    let msg = confirm_screen(
        UiText::new(ids::TELEGRAM_POSITION_CONFIRM_CLOSE_ALL)
            .arg("count", text_arg(positions.len().to_string())),
        ids::TELEGRAM_POSITION_CONFIRM_CLOSE_ALL_HINT,
    );
    send_with_keyboard(bot, chat_id, &msg, keyboards::confirm_close_all()).await
}

pub(super) async fn send_confirm_force_stop(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = format!(
        "{}\n\n{}",
        with_icon("🚨", &tg_id(ids::TELEGRAM_POSITION_CONFIRM_FORCE_STOP)),
        with_icon(
            "⚠️",
            &tg_id(ids::TELEGRAM_POSITION_CONFIRM_FORCE_STOP_WARNING)
        ),
    );
    send_with_keyboard(bot, chat_id, &msg, keyboards::confirm_force_stop()).await
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
                "{}\n\n{}",
                with_icon(
                    "🚫",
                    &tg(&UiText::new(ids::TELEGRAM_POSITION_CONFIRM_BLACKLIST)
                        .arg("symbol", text_arg(pos.symbol.as_str()))
                        .arg("mint", text_arg(formatters::format_mint_display(&pos.mint))))
                ),
                tg_id(ids::TELEGRAM_POSITION_CONFIRM_BLACKLIST_HINT)
            );
            send_with_keyboard(
                bot,
                chat_id,
                &msg,
                keyboards::confirm_blacklist(&pos.mint, &pos.symbol),
            )
            .await
        }
        None => send_not_found(bot, chat_id).await,
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
            let msg = row(
                "⏳",
                UiText::new(ids::TELEGRAM_POSITION_SELLING)
                    .arg("percent", text_arg(percent.to_string()))
                    .arg("symbol", text_arg(pos.symbol.as_str())),
            );
            let _ = bot
                .send_message(chat_id, &msg)
                .parse_mode(ParseMode::Html)
                .await;

            match manual_sell(&pos.mint, Some(percent as f64), None).await {
                Ok(result) => {
                    let msg = row(
                        "✅",
                        UiText::new(ids::TELEGRAM_POSITION_SELL_DONE)
                            .arg("symbol", text_arg(pos.symbol.as_str()))
                            .arg("percent", text_arg(percent.to_string()))
                            .arg(
                                "amount",
                                text_arg(format!(
                                    "{:.4}",
                                    result.executed_size_sol.unwrap_or_default()
                                )),
                            ),
                    );
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
                Err(e) => {
                    let msg = failure_screen(ids::TELEGRAM_POSITION_SELL_FAILED, &e.to_string());
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
            }
        }
        None => send_not_found(bot, chat_id).await,
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
            let msg = row(
                "⏳",
                UiText::new(ids::TELEGRAM_POSITION_ADDING)
                    .arg("amount", text_arg(amount.to_string()))
                    .arg("symbol", text_arg(pos.symbol.as_str())),
            );
            let _ = bot
                .send_message(chat_id, &msg)
                .parse_mode(ParseMode::Html)
                .await;

            match manual_add(&pos.mint, amount, None).await {
                Ok(_) => {
                    let msg = row(
                        "✅",
                        UiText::new(ids::TELEGRAM_POSITION_DCA_DONE)
                            .arg("symbol", text_arg(pos.symbol.as_str()))
                            .arg("amount", text_arg(amount.to_string())),
                    );
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
                Err(e) => {
                    let msg = failure_screen(ids::TELEGRAM_POSITION_DCA_FAILED, &e.to_string());
                    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
                }
            }
        }
        None => send_not_found(bot, chat_id).await,
    }
}

pub(super) async fn execute_close(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<()> {
    execute_sell(bot, chat_id, mint_short, 100).await
}

pub(super) async fn execute_close_all(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        let msg = with_icon("❌", &tg_id(ids::TELEGRAM_POSITION_NO_POSITIONS));
        return send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await;
    }

    let _ = bot
        .send_message(
            chat_id,
            with_icon("⏳", &tg_id(ids::TELEGRAM_POSITION_CLOSING_ALL)),
        )
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

    let msg = row(
        "📊",
        UiText::new(ids::TELEGRAM_POSITION_CLOSE_ALL_DONE)
            .arg("closed", text_arg(success.to_string()))
            .arg("failed", text_arg(failed.to_string())),
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

            let msg = row(
                "🚫",
                UiText::new(ids::TELEGRAM_POSITION_BLACKLISTED)
                    .arg("symbol", text_arg(pos.symbol.as_str())),
            );
            send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu()).await
        }
        None => send_not_found(bot, chat_id).await,
    }
}

// ============================================================================
// SHARED SCREENS
// ============================================================================

async fn send_not_found(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = with_icon("❌", &tg_id(ids::TELEGRAM_POSITION_NOT_FOUND));
    send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await
}

/// Confirmation prompt: warning icon, body and a closing hint.
fn confirm_screen(body: UiText, hint: crate::i18n::MessageId) -> String {
    format!("{}\n\n{}", with_icon("⚠️", &tg(&body)), tg_id(hint))
}

/// Failure title followed by the error detail.
fn failure_screen(title: crate::i18n::MessageId, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        with_icon("❌", &tg_id(title)),
        tg(&UiText::new(ids::TELEGRAM_ERROR_LINE).arg("detail", text_arg(detail)))
    )
}
