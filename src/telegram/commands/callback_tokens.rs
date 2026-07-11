//! Token-related callback handlers
//!
//! Handles token explorer, token lists, token details, buy/blacklist actions.

use super::callbacks::send_with_keyboard;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::telegram::{formatters, keyboards};
use crate::trader::manual::manual_add;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

// ============================================================================
// TOKEN EXPLORER
// ============================================================================

/// Send token explorer main menu
pub async fn send_tokens_menu(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch stats: {e}");
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
pub(super) async fn send_tokens_page(
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
            let msg = format!("❌ Failed to fetch tokens: {e}");
            return send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await;
        }
    };

    if result.items.is_empty() {
        let msg = format!("📭 No tokens found in <b>{view}</b> view.");
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
        view_emoji, view_name, result.page, result.total_pages
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
            .unwrap_or_else(|| "N/A".to_owned());

        // Format price
        let price = if token.price_sol > 0.0 {
            format!("{} SOL", formatters::format_price(token.price_sol))
        } else {
            "N/A".to_owned()
        };

        // Add rejection reason for rejected view
        let reason_part = if view == "rejected" {
            result
                .rejection_reasons
                .get(&token.mint)
                .map(|r| format!("\n   └ ⚠️ {r}"))
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
pub(super) async fn send_filter_stats(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let stats = match crate::filtering::fetch_stats().await {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("❌ Failed to fetch stats: {e}");
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
        .unwrap_or_else(|| "N/A".to_owned());
    let volume_24h = token
        .volume_h24
        .map(|v| formatters::format_usd(v))
        .unwrap_or_else(|| "N/A".to_owned());
    let price_change = token
        .price_change_h24
        .map(|c| format!("{:+.2}%", c))
        .unwrap_or_else(|| "N/A".to_owned());

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
            format!("{emoji} Risk Assessment: {s}/100")
        })
        .unwrap_or_else(|| "⚪ Risk Assessment: Unknown".to_owned());

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
        Ok(result) => result.items.into_iter().next(),
        _ => None,
    }
}

/// Send search prompt
pub(super) async fn send_search_prompt(bot: &Bot, chat_id: ChatId) -> Result<(), String> {
    let msg = "🔍 <b>Search Market</b>\n\n\
               Enter symbol or mint address to search:\n\n\
               <i>Example: /token_BONK or /token_So11111</i>";
    send_with_keyboard(bot, chat_id, msg, keyboards::tokens_menu()).await
}

/// Confirmation dialog for buying a token (from token explorer)
pub(super) async fn send_confirm_token_buy(
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
pub(super) async fn send_confirm_token_blacklist(
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
pub(super) async fn execute_token_blacklist(
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
                "Database not available".to_owned(),
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
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {e}"));
            let msg = format!("❌ <b>Blacklist Failed</b>\n\nError: {e}");
            send_with_keyboard(bot, chat_id, &msg, keyboards::tokens_menu()).await
        }
        Err(e) => {
            logger::warning(LogTag::Telegram, &format!("Failed to blacklist token: {e}"));
            let msg = format!("❌ <b>Blacklist Failed</b>\n\nError: {e}");
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
        token.symbol, amount
    );

    bot.send_message(chat_id, &msg)
        .parse_mode(ParseMode::Html)
        .await
        .map_err(|e| format!("Failed to send: {e}"))?;

    // Execute the buy via manual trading system
    match manual_add(&token.mint, amount, None).await {
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
