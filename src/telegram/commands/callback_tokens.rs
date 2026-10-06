// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token-related callback handlers
//!
//! Handles token explorer, token lists, token details, buy/blacklist actions.

use super::callbacks::send_with_keyboard;
use crate::i18n::{ids, MessageId, UiArg, UiText};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::telegram::formatters::{self, nested_arg, row, text_arg};
use crate::telegram::keyboards;
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::telegram::{Error, Result};
use crate::trader::manual::manual_add;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

// ============================================================================
// TOKEN EXPLORER
// ============================================================================

/// Send token explorer main menu
pub async fn send_tokens_menu(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = row(
                "❌",
                UiText::new(ids::TELEGRAM_TOKEN_STATS_FAILED)
                    .arg("detail", text_arg(e.to_string())),
            );
            return send_with_keyboard(bot, chat_id, &msg, keyboards::main_menu_compact()).await;
        }
    };

    let msg = row(
        "🔍",
        UiText::new(ids::TELEGRAM_TOKEN_EXPLORER)
            .arg("passed", text_arg(stats.passed_filtering.to_string()))
            .arg(
                "rejected",
                text_arg(
                    stats
                        .total_tokens
                        .saturating_sub(stats.passed_filtering)
                        .to_string(),
                ),
            )
            .arg("priced", text_arg(stats.with_pool_price.to_string()))
            .arg("total", text_arg(stats.total_tokens.to_string())),
    );

    send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
}

/// Send paginated token list for a view
pub async fn send_tokens_list(bot: &Bot, chat_id: ChatId, view: &str) -> Result<()> {
    send_tokens_page(bot, chat_id, view, 1).await
}

/// Send a specific page of tokens
pub(super) async fn send_tokens_page(
    bot: &Bot,
    chat_id: ChatId,
    view: &str,
    page: usize,
) -> Result<()> {
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
            let msg = row(
                "❌",
                UiText::new(ids::TELEGRAM_TOKEN_LIST_FAILED).arg("detail", text_arg(e.to_string())),
            );
            return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
        }
    };

    let view_name = match view {
        "passed" => nested_arg(UiText::new(ids::TELEGRAM_TOKEN_VIEW_PASSED)),
        "rejected" => nested_arg(UiText::new(ids::TELEGRAM_TOKEN_VIEW_REJECTED)),
        "recent" => nested_arg(UiText::new(ids::TELEGRAM_TOKEN_VIEW_RECENT)),
        "all" => nested_arg(UiText::new(ids::TELEGRAM_TOKEN_VIEW_ALL)),
        _ => text_arg(view),
    };

    if result.items.is_empty() {
        let msg = row(
            "📭",
            UiText::new(ids::TELEGRAM_TOKEN_LIST_EMPTY).arg("view", view_name),
        );
        return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
    }

    let view_emoji = match view {
        "passed" => "✅",
        "rejected" => "❌",
        "recent" => "🆕",
        "all" => "📋",
        _ => "📊",
    };

    let mut msg = row(
        view_emoji,
        UiText::new(ids::TELEGRAM_TOKEN_LIST_TITLE)
            .arg("name", view_name)
            .arg("page", text_arg(result.page.to_string()))
            .arg("total", text_arg(result.total_pages.to_string())),
    );
    msg.push_str("\n\n");

    for (i, token) in result.items.iter().enumerate() {
        let idx = (page - 1) * 10 + i + 1;
        let symbol = &token.symbol;
        // Characters, not bytes: the prefix is also the /token_ command argument.
        let mint_short: String = token.mint.chars().take(8).collect();

        let liquidity = match token.liquidity_usd {
            Some(l) => nested_arg(formatters::usd_compact_text(l)),
            None => not_available(),
        };

        let price = if token.price_sol > 0.0 {
            nested_arg(
                UiText::new(ids::TELEGRAM_PRICE_NATIVE)
                    .arg("price", formatters::price_arg(token.price_sol)),
            )
        } else {
            not_available()
        };

        // Add rejection reason for rejected view
        let reason_part = if view == "rejected" {
            result
                .rejection_reasons
                .get(&token.mint)
                .map(|r| format!("\n   └ ⚠️ {}", tg_escape(r)))
                .unwrap_or_default()
        } else {
            String::new()
        };

        msg.push_str(&format!(
            "{idx}. {} ({})",
            formatters::ticker(symbol),
            tg_escape(&mint_short)
        ));
        msg.push_str("\n   ");
        msg.push_str(&tg(&UiText::new(ids::TELEGRAM_TOKEN_LIST_STATS)
            .arg("liquidity", liquidity)
            .arg("price", price)));
        msg.push_str(&format!("{reason_part}\n   /token_{mint_short}\n\n"));
    }

    msg.push_str(&tg_id(ids::TELEGRAM_TOKEN_LIST_HINT));

    let keyboard = keyboards::tokens_list_keyboard(view, page, result.total_pages);
    send_with_keyboard(bot, chat_id, &msg, keyboard).await
}

/// Send filter statistics
pub(super) async fn send_filter_stats(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = row(
                "❌",
                UiText::new(ids::TELEGRAM_TOKEN_STATS_FAILED)
                    .arg("detail", text_arg(e.to_string())),
            );
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

    let count = |id: MessageId, value: usize, percent: Option<f64>| {
        let mut text = UiText::new(id).arg("count", text_arg(value.to_string()));
        if let Some(percent) = percent {
            text = text.arg("percent", text_arg(format!("{percent:.1}")));
        }
        text
    };
    let msg = [
        with_icon("📊", &tg_id(ids::TELEGRAM_TOKEN_FILTER_TITLE)),
        String::new(),
        tg_id(ids::TELEGRAM_TOKEN_FILTER_DISTRIBUTION),
        row(
            "✅",
            count(
                ids::TELEGRAM_TOKEN_FILTER_PASSED,
                stats.passed_filtering,
                Some(passed_pct),
            ),
        ),
        row(
            "❌",
            count(
                ids::TELEGRAM_TOKEN_FILTER_REJECTED,
                rejected_count,
                Some(rejected_pct),
            ),
        ),
        row(
            "🚫",
            count(
                ids::TELEGRAM_TOKEN_FILTER_BLACKLISTED,
                stats.blacklisted,
                None,
            ),
        ),
        String::new(),
        tg_id(ids::TELEGRAM_TOKEN_FILTER_COVERAGE),
        row(
            "💰",
            count(
                ids::TELEGRAM_TOKEN_FILTER_PRICED,
                stats.with_pool_price,
                None,
            ),
        ),
        row(
            "📈",
            count(ids::TELEGRAM_TOKEN_FILTER_OPEN, stats.open_positions, None),
        ),
        row(
            "📋",
            count(ids::TELEGRAM_TOKEN_FILTER_TOTAL, stats.total_tokens, None),
        ),
        String::new(),
        tg_id(ids::TELEGRAM_TOKEN_FILTER_UPDATED),
        row(
            "🕐",
            UiText::new(ids::TELEGRAM_TOKEN_FILTER_TIME).arg(
                "time",
                text_arg(stats.updated_at.format("%H:%M:%S").to_string()),
            ),
        ),
        String::new(),
        tg(&UiText::new(ids::TELEGRAM_TOKEN_FILTER_REFRESH)
            .arg("interval", nested_arg(formatters::duration_text(180)))),
    ]
    .join("\n");

    send_with_keyboard(bot, chat_id, &msg, keyboards::filter_stats_keyboard()).await
}

/// Send token detail view
pub async fn send_token_detail(bot: &Bot, chat_id: ChatId, mint_short: &str) -> Result<()> {
    // Try to find token by mint prefix from the filtering store
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            let msg = with_icon("❌", &tg_id(ids::TELEGRAM_TOKEN_NOT_FOUND_PREFIX));
            return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
        }
    };

    // Check if user has a position
    let has_position = positions::get_open_positions()
        .await
        .iter()
        .any(|p| p.mint == token.mint);

    let usd = |value: Option<f64>| match value {
        Some(v) => text_arg(formatters::format_usd(v)),
        None => not_available(),
    };
    let price_change = match token.price_change_h24 {
        Some(c) => nested_arg(
            UiText::new(ids::TELEGRAM_PERCENT_VALUE).arg("percent", text_arg(format!("{c:+.2}"))),
        ),
        None => not_available(),
    };

    let risk_text = match token.security_score_normalised {
        Some(s) => {
            let emoji = if s <= 30 {
                "🟢"
            } else if s <= 60 {
                "🟡"
            } else {
                "🔴"
            };
            row(
                emoji,
                UiText::new(ids::TELEGRAM_TOKEN_DETAIL_RISK).arg("score", text_arg(s.to_string())),
            )
        }
        None => row("⚪", UiText::new(ids::TELEGRAM_TOKEN_DETAIL_RISK_UNKNOWN)),
    };

    let position_text = if has_position {
        format!(
            "{}\n\n",
            with_icon("✅", &tg_id(ids::TELEGRAM_TOKEN_DETAIL_ACTIVE))
        )
    } else {
        String::new()
    };

    let rows = [
        tg(&UiText::new(ids::TELEGRAM_TOKEN_DETAIL_PRICE)
            .arg("price", formatters::price_arg(token.price_sol))),
        tg(&UiText::new(ids::TELEGRAM_TOKEN_DETAIL_LIQUIDITY)
            .arg("value", usd(token.liquidity_usd))),
        tg(&UiText::new(ids::TELEGRAM_TOKEN_DETAIL_VOLUME).arg("value", usd(token.volume_h24))),
        tg(&UiText::new(ids::TELEGRAM_TOKEN_DETAIL_CHANGE).arg("value", price_change)),
    ];

    let msg = format!(
        "{}\n{}\n\n{}{}\n\n{}\n\n{}",
        with_icon(
            "🪙",
            &format!(
                "{} (${})",
                formatters::bold(&token.name),
                tg_escape(&token.symbol)
            )
        ),
        formatters::code(&formatters::format_mint_display(&token.mint)),
        position_text,
        rows.join("\n"),
        risk_text,
        tg_id(ids::TELEGRAM_TOKEN_DETAIL_ACTION),
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
        Ok(result) => result.items.into_iter().next(),
        _ => None,
    }
}

/// Send search prompt
pub(super) async fn send_search_prompt(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = with_icon("🔍", &tg_id(ids::TELEGRAM_TOKEN_SEARCH));
    send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
}

/// Confirmation dialog for buying a token (from token explorer)
pub(super) async fn send_confirm_token_buy(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<()> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            return send_token_not_found(bot, chat_id).await;
        }
    };

    let msg = row(
        "💰",
        UiText::new(ids::TELEGRAM_TOKEN_CONFIRM_BUY)
            .arg("symbol", text_arg(token.symbol.as_str()))
            .arg(
                "mint",
                text_arg(formatters::format_mint_display(&token.mint)),
            )
            .arg("amount", text_arg(amount.to_string())),
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
pub(super) async fn send_confirm_token_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<()> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            return send_token_not_found(bot, chat_id).await;
        }
    };

    let msg = row(
        "🚫",
        UiText::new(ids::TELEGRAM_TOKEN_CONFIRM_BLACKLIST)
            .arg("symbol", text_arg(token.symbol.as_str()))
            .arg(
                "mint",
                text_arg(formatters::format_mint_display(&token.mint)),
            ),
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
pub(super) async fn execute_token_blacklist(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
) -> Result<()> {
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            return send_token_not_found(bot, chat_id).await;
        }
    };

    // Add to blacklist using token database
    let mint_clone = token.mint.clone();
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

    match blacklist_result {
        Ok(Ok(())) => {
            let msg = row(
                "🚫",
                UiText::new(ids::TELEGRAM_TOKEN_BLACKLISTED)
                    .arg("symbol", text_arg(token.symbol.as_str())),
            );
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
        Ok(Err(e)) => {
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {e}"));
            let msg = failure_screen(ids::TELEGRAM_TOKEN_BLACKLIST_FAILED, &e.to_string());
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
        Err(e) => {
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {e}"));
            let msg = failure_screen(ids::TELEGRAM_TOKEN_BLACKLIST_FAILED, &e.to_string());
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
    }
}

/// Execute token buy (quick buy from token explorer)
pub(super) async fn execute_token_buy(
    bot: &Bot,
    chat_id: ChatId,
    mint_short: &str,
    amount: f64,
) -> Result<()> {
    // Find token by mint prefix
    let token = match find_token_by_prefix(mint_short).await {
        Some(t) => t,
        None => {
            return send_token_not_found(bot, chat_id).await;
        }
    };

    let msg = row(
        "💰",
        UiText::new(ids::TELEGRAM_TOKEN_BUY_PROCESSING)
            .arg("symbol", text_arg(token.symbol.as_str()))
            .arg("amount", text_arg(amount.to_string())),
    );

    bot.send_message(chat_id, &msg)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| Error::SendFailed {
            chat_id: chat_id.0.to_string(),
            detail: e.to_string(),
        })?;

    // Execute the buy via manual trading system
    match manual_add(&token.mint, amount, None).await {
        Ok(_) => {
            let success_msg = row(
                "✅",
                UiText::new(ids::TELEGRAM_TOKEN_BUY_DONE)
                    .arg("symbol", text_arg(token.symbol.as_str()))
                    .arg("amount", text_arg(amount.to_string())),
            );
            send_with_keyboard(bot, chat_id, &success_msg, keyboards::main_menu_compact()).await
        }
        Err(e) => {
            let error_msg = row(
                "❌",
                UiText::new(ids::TELEGRAM_TOKEN_BUY_FAILED)
                    .arg("symbol", text_arg(token.symbol.as_str()))
                    .arg("detail", text_arg(e.to_string())),
            );
            send_with_keyboard(bot, chat_id, &error_msg, keyboards::tokens_menu()).await
        }
    }
}

/// `N/A` placeholder for a missing value.
fn not_available() -> UiArg {
    nested_arg(UiText::new(ids::TELEGRAM_VALUE_NA))
}

async fn send_token_not_found(bot: &Bot, chat_id: ChatId) -> Result<()> {
    let msg = with_icon("❌", &tg_id(ids::TELEGRAM_TOKEN_NOT_FOUND));
    send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
}

/// Failure title followed by the error detail.
fn failure_screen(title: MessageId, detail: &str) -> String {
    format!(
        "{}\n\n{}",
        with_icon("❌", &tg_id(title)),
        tg(&UiText::new(ids::TELEGRAM_ERROR_LINE).arg("detail", text_arg(detail)))
    )
}
