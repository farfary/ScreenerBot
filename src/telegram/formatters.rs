//! Value formatters and Telegram screen helpers.
//!
//! Numbers keep their fixed precision here; the words and units around them
//! come from `locales/en/telegram.ftl`. Message screens that combine these
//! values live in `messages.rs`.
//!
//! Emoji conventions (added in Rust, never in catalogs):
//! - 🟢 profit/success, 🔴 loss/error, 🟡 pending/warning
//! - 📈 buy/increase, 📉 sell/decrease
//! - 💰 balance, 💎 value, 🎯 target, 🛡️ protection

use crate::filtering::types::PassedToken;
use crate::i18n::{ids, LanguageIdentifier, UiArg, UiText};
use crate::telegram::text::{
    locale, tg, tg_escape, tg_id, tg_plain, tg_plain_id, tg_plain_with, with_icon,
};

/// Escape HTML special characters
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Text argument.
pub(crate) fn text_arg(value: impl Into<String>) -> UiArg {
    UiArg::Text(value.into())
}

/// SOL amount argument with the fixed Telegram precision.
pub(crate) fn sol_arg(amount: f64) -> UiArg {
    UiArg::Text(format_sol(amount))
}

/// Price argument with the adaptive Telegram precision.
pub(crate) fn price_arg(price: f64) -> UiArg {
    UiArg::Text(format_price(price))
}

/// Nested message argument.
pub(crate) fn nested_arg(text: UiText) -> UiArg {
    UiArg::Nested(Box::new(text))
}

/// Rendered message line led by an icon.
pub(crate) fn row(icon: &str, text: UiText) -> String {
    with_icon(icon, &tg(&text))
}

/// Keep the first `head` and last `tail` characters of a value longer than
/// `max` characters. Counts characters, not bytes, so multibyte input never
/// splits.
pub fn shorten_middle(value: &str, head: usize, tail: usize, max: usize) -> String {
    let count = value.chars().count();
    if count <= max {
        return value.to_owned();
    }
    let start: String = value.chars().take(head).collect();
    let end: String = value.chars().skip(count - tail).collect();
    format!("{start}...{end}")
}

/// Format a mint address for display (first 4...last 4)
pub fn format_mint_display(mint: &str) -> String {
    shorten_middle(mint, 4, 4, 12)
}

/// Format a price with appropriate precision
/// - Very small (<1e-9): scientific notation
/// - Small (<0.000001): 9 decimals
/// - Medium (<0.01): 6 decimals
/// - Fractional (<1): 4 decimals
/// - Large: 2 decimals
pub fn format_price(price: f64) -> String {
    if price == 0.0 {
        "0".to_owned()
    } else if price.abs() < 1e-9 {
        format!("{:.2e}", price)
    } else if price.abs() < 0.000001 {
        format!("{:.9}", price)
    } else if price.abs() < 0.01 {
        format!("{:.6}", price)
    } else if price.abs() < 1.0 {
        format!("{:.4}", price)
    } else if price.abs() < 1000.0 {
        format!("{:.2}", price)
    } else {
        format!("{:.0}", price)
    }
}

/// Format SOL amount with 4 decimal places
pub fn format_sol(amount: f64) -> String {
    if amount.abs() < 0.0001 {
        format!("{:.6}", amount)
    } else {
        format!("{:.4}", amount)
    }
}

/// Format token amount with comma separators
pub fn format_tokens(amount: u64) -> String {
    let s = amount.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

/// Format token amount from f64 with comma separators
pub fn format_tokens_f64(amount: f64) -> String {
    format_tokens(amount as u64)
}

/// Emoji for a P&L result.
fn pnl_icon(pnl_sol: f64, pnl_pct: f64) -> &'static str {
    if pnl_sol >= 0.0 {
        if pnl_pct >= 100.0 {
            "🎉"
        } else if pnl_pct >= 50.0 {
            "🚀"
        } else {
            "🟢"
        }
    } else if pnl_pct <= -50.0 {
        "💀"
    } else {
        "🔴"
    }
}

/// Signed P&L amount and percent, without markup or emoji.
fn pnl_text(pnl_sol: f64, pnl_pct: f64) -> UiText {
    let sign = if pnl_sol >= 0.0 { "+" } else { "" };
    UiText::new(ids::TELEGRAM_PNL)
        .arg("sol", text_arg(format!("{sign}{}", format_sol(pnl_sol))))
        .arg("percent", text_arg(format!("{sign}{pnl_pct:.1}")))
}

/// Format P&L with sign and emoji
pub fn format_pnl(pnl_sol: f64, pnl_pct: f64) -> String {
    format!(
        "{} {}",
        tg(&pnl_text(pnl_sol, pnl_pct)),
        pnl_icon(pnl_sol, pnl_pct)
    )
}

/// Format P&L with bold for emphasis
pub(crate) fn format_pnl_bold(pnl_sol: f64, pnl_pct: f64) -> String {
    format!(
        "<b>{}</b> {}",
        tg(&pnl_text(pnl_sol, pnl_pct)),
        pnl_icon(pnl_sol, pnl_pct)
    )
}

/// P&L as plain text, for use as a message argument.
pub(crate) fn pnl_plain(pnl_sol: f64, pnl_pct: f64) -> String {
    format!(
        "{} {}",
        tg_plain(&pnl_text(pnl_sol, pnl_pct)),
        pnl_icon(pnl_sol, pnl_pct)
    )
}

/// `$SYMBOL` in bold, escaped.
pub(crate) fn ticker(symbol: &str) -> String {
    format!("<b>${}</b>", tg_escape(symbol))
}

/// Bold, escaped text.
pub(crate) fn bold(value: &str) -> String {
    format!("<b>{}</b>", tg_escape(value))
}

/// Copyable value in code, escaped.
pub(crate) fn code(value: &str) -> String {
    format!("<code>{}</code>", tg_escape(value))
}

/// Duration as catalog text: the two most significant units.
pub(crate) fn duration_text(seconds: u64) -> UiText {
    let number = |value: u64| text_arg(value.to_string());
    if seconds < 60 {
        UiText::new(ids::TELEGRAM_DURATION_SECONDS).arg("seconds", number(seconds))
    } else if seconds < 3600 {
        let (minutes, secs) = (seconds / 60, seconds % 60);
        if secs > 0 {
            UiText::new(ids::TELEGRAM_DURATION_MINUTES_SECONDS)
                .arg("minutes", number(minutes))
                .arg("seconds", number(secs))
        } else {
            UiText::new(ids::TELEGRAM_DURATION_MINUTES).arg("minutes", number(minutes))
        }
    } else if seconds < 86400 {
        let (hours, minutes) = (seconds / 3600, (seconds % 3600) / 60);
        if minutes > 0 {
            UiText::new(ids::TELEGRAM_DURATION_HOURS_MINUTES)
                .arg("hours", number(hours))
                .arg("minutes", number(minutes))
        } else {
            UiText::new(ids::TELEGRAM_DURATION_HOURS).arg("hours", number(hours))
        }
    } else {
        let (days, hours) = (seconds / 86400, (seconds % 86400) / 3600);
        if hours > 0 {
            UiText::new(ids::TELEGRAM_DURATION_DAYS_HOURS)
                .arg("days", number(days))
                .arg("hours", number(hours))
        } else {
            UiText::new(ids::TELEGRAM_DURATION_DAYS).arg("days", number(days))
        }
    }
}

/// Plain-text duration for `locale`.
pub fn format_duration_with(locale: &LanguageIdentifier, seconds: u64) -> String {
    tg_plain_with(locale, &duration_text(seconds))
}

/// Plain-text duration in the Telegram language.
pub fn format_duration(seconds: u64) -> String {
    format_duration_with(&locale(), seconds)
}

/// Largest prefix of `value` within `max` bytes that ends on a character boundary.
fn truncate_bytes(value: &str, max: usize) -> &str {
    let mut end = max.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// LLM-analysis reasoning block for notifications, or nothing.
pub fn format_ai_reasoning(reasoning: &Option<String>) -> String {
    match reasoning {
        Some(r) if !r.is_empty() => {
            // Truncated before escaping so an entity is never cut in half.
            let truncated = if r.len() > 300 {
                format!("{}...", truncate_bytes(r, 300))
            } else {
                r.clone()
            };
            format!(
                "\n\n{}",
                row(
                    "🤖",
                    UiText::new(ids::TELEGRAM_AI_REASONING).arg("reasoning", text_arg(truncated))
                )
            )
        }
        _ => String::new(),
    }
}

/// Format USD value
pub fn format_usd(amount: f64) -> String {
    let digits = if amount.abs() < 0.01 {
        format!("{:.4}", amount)
    } else if amount.abs() < 1000.0 {
        format!("{:.2}", amount)
    } else {
        // Format with thousand separators manually
        let formatted = format!("{:.0}", amount);
        let chars: Vec<char> = formatted.chars().collect();
        let mut result = String::with_capacity(chars.len() + chars.len() / 3);
        for (i, c) in chars.iter().enumerate() {
            if i > 0 && (chars.len() - i) % 3 == 0 {
                result.push(',');
            }
            result.push(*c);
        }
        result
    };
    tg_plain(&UiText::new(ids::TELEGRAM_AMOUNT_USD).arg("amount", text_arg(digits)))
}

/// Compact USD amount: `$1.5M`, `$2.3K` or `$120`.
pub(crate) fn usd_compact_text(amount: f64) -> UiText {
    if amount >= 1_000_000.0 {
        UiText::new(ids::TELEGRAM_AMOUNT_USD_MILLIONS)
            .arg("amount", text_arg(format!("{:.1}", amount / 1_000_000.0)))
    } else if amount >= 1_000.0 {
        UiText::new(ids::TELEGRAM_AMOUNT_USD_THOUSANDS)
            .arg("amount", text_arg(format!("{:.1}", amount / 1_000.0)))
    } else {
        UiText::new(ids::TELEGRAM_AMOUNT_USD).arg("amount", text_arg(format!("{amount:.0}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn english() -> LanguageIdentifier {
        crate::i18n::source_locale()
            .parse()
            .expect("source locale parses")
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("a<b>c"), "a&lt;b&gt;c");
        assert_eq!(html_escape("a&b"), "a&amp;b");
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(0.0), "0");
        assert_eq!(format_price(1.5), "1.50");
        assert_eq!(format_price(0.00000012), "0.000000120");
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(1000), "1,000");
        assert_eq!(format_tokens(1234567), "1,234,567");
    }

    #[test]
    fn test_format_duration() {
        let en = english();
        assert_eq!(format_duration_with(&en, 30), "30s");
        assert_eq!(format_duration_with(&en, 90), "1m 30s");
        assert_eq!(format_duration_with(&en, 120), "2m");
        assert_eq!(format_duration_with(&en, 3700), "1h 1m");
        assert_eq!(format_duration_with(&en, 7200), "2h");
        assert_eq!(format_duration_with(&en, 90000), "1d 1h");
        assert_eq!(format_duration_with(&en, 172800), "2d");
    }

    #[test]
    fn shortening_counts_characters() {
        assert_eq!(format_mint_display("short"), "short");
        assert_eq!(
            format_mint_display("So11111111111111111111111111111111111111112"),
            "So11...1112"
        );
        assert_eq!(
            shorten_middle("ααααββββγγγγδδδδεεεε", 4, 4, 12),
            "αααα...εεεε"
        );
    }

    #[test]
    fn reasoning_is_cut_on_a_character_boundary() {
        let text = Some("é".repeat(200));
        let block = format_ai_reasoning(&text);
        assert!(block.contains("..."));
        assert!(format_ai_reasoning(&None).is_empty());
    }
}

/// Format a page of tokens for pagination display
pub fn format_tokens_page(
    tokens: &[PassedToken],
    page: usize,
    total_pages: usize,
    total_items: usize,
) -> String {
    let mut text = row(
        "🔍",
        UiText::new(ids::TELEGRAM_FILTER_RESULTS_TITLE)
            .arg("count", text_arg(total_items.to_string())),
    );
    text.push_str("\n\n");

    if tokens.is_empty() {
        text.push_str(&tg_id(ids::TELEGRAM_FILTER_RESULTS_EMPTY));
        return text;
    }

    let link_label = tg_escape(&tg_plain_id(ids::TELEGRAM_BUTTON_DEXSCREENER));
    for token in tokens.iter() {
        let name = match token.name.as_deref() {
            Some(name) => tg_escape(name),
            None => tg_id(ids::TELEGRAM_VALUE_UNKNOWN),
        };
        text.push_str(&format!("• {} ({name})", bold(&token.symbol)));
        text.push_str("\n  ");
        text.push_str(&code(&token.mint));
        text.push_str(&format!(
            "\n  <a href=\"https://dexscreener.com/solana/{}\">{link_label}</a>\n\n",
            token.mint
        ));
    }

    text.push_str(&tg(&UiText::new(ids::TELEGRAM_FILTER_RESULTS_PAGE)
        .arg("page", text_arg((page + 1).to_string()))
        .arg("total", text_arg(total_pages.to_string()))));
    text
}
