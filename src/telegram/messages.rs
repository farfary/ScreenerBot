//! Notification and position message screens.
//!
//! Each screen is assembled from catalog messages rendered for the Telegram
//! language at send time. Icons are prepended here; values arrive as arguments
//! and are escaped by `tg`.

use crate::i18n::{ids, UiText};
use crate::telegram::formatters::{
    code, duration_text, format_ai_reasoning, format_mint_display, format_pnl, format_pnl_bold,
    format_tokens_f64, nested_arg, pnl_plain, price_arg, row, sol_arg, text_arg, ticker,
};
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::telegram::types::ErrorSeverity;
use crate::trader::closed_reason_text;

/// `$SYMBOL — P&L` subject line.
fn symbol_pnl_line(symbol: &str, pnl: String) -> String {
    format!("{} — {pnl}", ticker(symbol))
}

/// Format position opened notification
pub fn msg_position_opened(
    symbol: &str,
    mint: &str,
    amount_sol: f64,
    entry_price: f64,
    tokens: f64,
    dex: Option<&str>,
    ai_reasoning: &Option<String>,
) -> String {
    let dex = match dex {
        Some(dex) => text_arg(dex),
        None => nested_arg(UiText::new(ids::TELEGRAM_VALUE_UNKNOWN)),
    };
    let rows = [
        row(
            "💰",
            UiText::new(ids::TELEGRAM_NOTIFY_OPENED_SIZE).arg("amount", sol_arg(amount_sol)),
        ),
        row(
            "💎",
            UiText::new(ids::TELEGRAM_NOTIFY_OPENED_PRICE).arg("price", price_arg(entry_price)),
        ),
        row(
            "🪙",
            UiText::new(ids::TELEGRAM_ROW_TOKENS)
                .arg("tokens", text_arg(format_tokens_f64(tokens))),
        ),
        row(
            "📍",
            UiText::new(ids::TELEGRAM_NOTIFY_OPENED_DEX).arg("dex", dex),
        ),
    ];

    format!(
        "{}\n\n{}\n\n{}{}",
        with_icon("🟢", &tg_id(ids::TELEGRAM_NOTIFY_OPENED_TITLE)),
        format!("{} — {}", ticker(symbol), code(&format_mint_display(mint))),
        rows.join("\n"),
        format_ai_reasoning(ai_reasoning),
    )
}

/// Format position closed notification. `reason` is the stored close reason.
pub fn msg_position_closed(
    symbol: &str,
    pnl_sol: f64,
    pnl_pct: f64,
    entry_price: f64,
    exit_price: f64,
    invested: f64,
    received: f64,
    duration_secs: u64,
    reason: &str,
    ai_reasoning: &Option<String>,
) -> String {
    let (header_emoji, title) = if pnl_sol >= 0.0 {
        let emoji = if pnl_pct >= 100.0 {
            "🎉"
        } else if pnl_pct >= 50.0 {
            "🚀"
        } else {
            "🟢"
        };
        (emoji, ids::TELEGRAM_NOTIFY_CLOSED_TITLE_PROFIT)
    } else if pnl_pct <= -50.0 {
        ("💀", ids::TELEGRAM_NOTIFY_CLOSED_TITLE_LOSS)
    } else {
        ("🔴", ids::TELEGRAM_NOTIFY_CLOSED_TITLE_LOSS)
    };

    let rows = [
        row(
            "📈",
            UiText::new(ids::TELEGRAM_ROW_ENTRY).arg("price", price_arg(entry_price)),
        ),
        row(
            "📉",
            UiText::new(ids::TELEGRAM_ROW_EXIT).arg("price", price_arg(exit_price)),
        ),
        row(
            "💵",
            UiText::new(ids::TELEGRAM_ROW_INVESTED).arg("amount", sol_arg(invested)),
        ),
        row(
            "💰",
            UiText::new(ids::TELEGRAM_ROW_RECEIVED).arg("amount", sol_arg(received)),
        ),
        row(
            "⏱️",
            UiText::new(ids::TELEGRAM_ROW_DURATION)
                .arg("duration", nested_arg(duration_text(duration_secs))),
        ),
        row(
            "📋",
            UiText::new(ids::TELEGRAM_ROW_REASON)
                .arg("reason", nested_arg(closed_reason_text(reason))),
        ),
    ];

    format!(
        "{}\n\n{}\n\n{}{}",
        with_icon(header_emoji, &tg_id(title)),
        symbol_pnl_line(symbol, format_pnl_bold(pnl_sol, pnl_pct)),
        rows.join("\n"),
        format_ai_reasoning(ai_reasoning),
    )
}

/// Format partial exit notification
pub fn msg_partial_exit(
    symbol: &str,
    exit_pct: f64,
    pnl_sol: f64,
    pnl_pct: f64,
    received_sol: f64,
    remaining_pct: f64,
) -> String {
    let emoji = if pnl_sol >= 0.0 { "🟡" } else { "🟠" };
    let rows = [
        row(
            "💰",
            UiText::new(ids::TELEGRAM_ROW_RECEIVED).arg("amount", sol_arg(received_sol)),
        ),
        row(
            "📊",
            UiText::new(ids::TELEGRAM_ROW_PNL).arg("pnl", text_arg(pnl_plain(pnl_sol, pnl_pct))),
        ),
        row(
            "📦",
            UiText::new(ids::TELEGRAM_ROW_REMAINING)
                .arg("percent", text_arg(format!("{remaining_pct:.0}"))),
        ),
    ];

    format!(
        "{}\n\n{}\n\n{}",
        with_icon(emoji, &tg_id(ids::TELEGRAM_NOTIFY_PARTIAL_TITLE)),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_PARTIAL_SOLD)
            .arg("symbol", text_arg(symbol))
            .arg("percent", text_arg(format!("{exit_pct:.0}")))),
        rows.join("\n"),
    )
}

/// Format DCA executed notification
pub fn msg_dca_executed(
    symbol: &str,
    dca_amount_sol: f64,
    total_invested: f64,
    dca_count: u32,
    new_avg_price: f64,
) -> String {
    let rows = [
        row(
            "➕",
            UiText::new(ids::TELEGRAM_NOTIFY_DCA_ADDED).arg("amount", sol_arg(dca_amount_sol)),
        ),
        row(
            "💰",
            UiText::new(ids::TELEGRAM_ROW_TOTAL).arg("amount", sol_arg(total_invested)),
        ),
        row(
            "💎",
            UiText::new(ids::TELEGRAM_NOTIFY_DCA_AVG).arg("price", price_arg(new_avg_price)),
        ),
    ];

    format!(
        "{}\n\n{}\n\n{}",
        row(
            "📈",
            UiText::new(ids::TELEGRAM_NOTIFY_DCA_TITLE)
                .arg("count", text_arg(dca_count.to_string()))
        ),
        ticker(symbol),
        rows.join("\n"),
    )
}

/// Format system error notification
pub fn msg_system_error(severity: &ErrorSeverity, message: &str) -> String {
    let (emoji, title) = match severity {
        ErrorSeverity::Critical => ("🚨", ids::TELEGRAM_NOTIFY_SEVERITY_CRITICAL),
        ErrorSeverity::Error => ("❌", ids::TELEGRAM_NOTIFY_SEVERITY_ERROR),
        ErrorSeverity::Warning => ("⚠️", ids::TELEGRAM_NOTIFY_SEVERITY_WARNING),
        ErrorSeverity::Info => ("ℹ️", ids::TELEGRAM_NOTIFY_SEVERITY_INFO),
    };

    format!(
        "{}\n\n{}",
        with_icon(emoji, &tg_id(title)),
        tg_escape(message)
    )
}

/// Format bot started notification
pub fn msg_bot_started(version: &str, mode: &str) -> String {
    format!(
        "{}\n\n{}\n{}\n\n{}",
        with_icon("🚀", &tg_id(ids::TELEGRAM_NOTIFY_STARTED_TITLE)),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_STARTED_VERSION).arg("version", text_arg(version))),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_STARTED_MODE).arg("mode", text_arg(mode))),
        with_icon("✅", &tg_id(ids::TELEGRAM_NOTIFY_STARTED_READY)),
    )
}

/// Format bot stopped notification
pub fn msg_bot_stopped(reason: &str) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        with_icon("🛑", &tg_id(ids::TELEGRAM_NOTIFY_STOPPED_TITLE)),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_STOPPED_REASON).arg("reason", text_arg(reason))),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_STOPPED_GOODBYE).arg("icon", text_arg("👋"))),
    )
}

/// Format daily summary notification
pub fn msg_daily_summary(
    date: &str,
    total_trades: u32,
    winning: u32,
    losing: u32,
    total_pnl_sol: f64,
    open_positions: u32,
) -> String {
    let win_rate = if total_trades > 0 {
        (winning as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    let emoji = if total_pnl_sol >= 0.0 { "📈" } else { "📉" };
    let pnl_emoji = if total_pnl_sol >= 0.0 { "🟢" } else { "🔴" };

    let lines = [
        tg_id(ids::TELEGRAM_NOTIFY_SUMMARY_PERFORMANCE),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_SUMMARY_TRADES)
            .arg("total", text_arg(total_trades.to_string()))
            .arg("wins", text_arg(winning.to_string()))
            .arg("win_icon", text_arg("🟢"))
            .arg("losses", text_arg(losing.to_string()))
            .arg("loss_icon", text_arg("🔴"))),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_SUMMARY_WIN_RATE)
            .arg("percent", text_arg(format!("{win_rate:.0}")))),
        tg(&UiText::new(ids::TELEGRAM_NOTIFY_SUMMARY_PNL)
            .arg("amount", sol_arg(total_pnl_sol))
            .arg("icon", text_arg(pnl_emoji))),
    ];

    format!(
        "{}\n\n{}\n\n{}",
        with_icon(
            emoji,
            &tg(&UiText::new(ids::TELEGRAM_NOTIFY_SUMMARY_TITLE).arg("date", text_arg(date)))
        ),
        lines.join("\n"),
        row(
            "📦",
            UiText::new(ids::TELEGRAM_NOTIFY_SUMMARY_OPEN)
                .arg("count", text_arg(open_positions.to_string()))
        ),
    )
}

/// Format single position details
pub fn msg_position_detail(
    symbol: &str,
    mint: &str,
    entry_price: f64,
    current_price: f64,
    pnl_sol: f64,
    pnl_pct: f64,
    invested: f64,
    value: f64,
    tokens: f64,
    duration_secs: u64,
    dca_count: u32,
) -> String {
    let emoji = if pnl_pct >= 0.0 { "📈" } else { "📉" };
    let mut rows = vec![
        row(
            "📈",
            UiText::new(ids::TELEGRAM_ROW_ENTRY).arg("price", price_arg(entry_price)),
        ),
        row(
            "📉",
            UiText::new(ids::TELEGRAM_ROW_CURRENT).arg("price", price_arg(current_price)),
        ),
        row(
            "💵",
            UiText::new(ids::TELEGRAM_ROW_INVESTED).arg("amount", sol_arg(invested)),
        ),
        row(
            "💰",
            UiText::new(ids::TELEGRAM_ROW_VALUE).arg("amount", sol_arg(value)),
        ),
        row(
            "🪙",
            UiText::new(ids::TELEGRAM_ROW_TOKENS)
                .arg("tokens", text_arg(format_tokens_f64(tokens))),
        ),
    ];
    if dca_count > 0 {
        rows.push(row(
            "🔢",
            UiText::new(ids::TELEGRAM_ROW_DCA).arg("count", text_arg(dca_count.to_string())),
        ));
    }
    rows.push(row(
        "⏱️",
        UiText::new(ids::TELEGRAM_ROW_DURATION)
            .arg("duration", nested_arg(duration_text(duration_secs))),
    ));

    format!(
        "{}\n{}\n\n{}\n\n{}",
        with_icon(emoji, &ticker(symbol)),
        code(&format_mint_display(mint)),
        format_pnl_bold(pnl_sol, pnl_pct),
        rows.join("\n"),
    )
}

/// Format confirmation message for close position
pub fn msg_confirm_close(
    symbol: &str,
    pnl_sol: f64,
    pnl_pct: f64,
    tokens: f64,
    est_receive: f64,
) -> String {
    format!(
        "{}\n\n{}\n\n{}\n{}\n\n{}",
        with_icon("⚠️", &tg_id(ids::TELEGRAM_POSITION_CONFIRM_CLOSE_TITLE)),
        symbol_pnl_line(symbol, format_pnl(pnl_sol, pnl_pct)),
        tg(&UiText::new(ids::TELEGRAM_POSITION_CONFIRM_CLOSE_SELLING)
            .arg("tokens", text_arg(format_tokens_f64(tokens)))),
        tg(&UiText::new(ids::TELEGRAM_POSITION_CONFIRM_CLOSE_ESTIMATED)
            .arg("amount", sol_arg(est_receive))),
        with_icon("⏰", &tg_id(ids::TELEGRAM_POSITION_CONFIRM_CLOSE_HINT)),
    )
}
