//! HTML message formatters for Telegram notifications
//!
//! All formatters output HTML-safe strings for Telegram's HTML parse mode.
//! Emoji conventions:
//! - 🟢 profit/success, 🔴 loss/error, 🟡 pending/warning
//! - 📈 buy/increase, 📉 sell/decrease
//! - 💰 balance, 💎 value, 🎯 target, 🛡️ protection

/// Escape HTML special characters
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Format a mint address for display (first 4...last 4)
pub fn format_mint_display(mint: &str) -> String {
    if mint.len() <= 12 {
        mint.to_string()
    } else {
        format!("{}...{}", &mint[..4], &mint[mint.len() - 4..])
    }
}

/// Format a price with appropriate precision
/// - Very small (<1e-9): scientific notation
/// - Small (<0.000001): 9 decimals
/// - Medium (<0.01): 6 decimals
/// - Fractional (<1): 4 decimals
/// - Large: 2 decimals
pub fn format_price(price: f64) -> String {
    if price == 0.0 {
        "0".to_string()
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

/// Format P&L with sign and emoji
pub fn format_pnl(pnl_sol: f64, pnl_pct: f64) -> String {
    let emoji = if pnl_sol >= 0.0 {
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
    };

    let sign = if pnl_sol >= 0.0 { "+" } else { "" };

    format!(
        "{}{} SOL ({}{}%) {}",
        sign,
        format_sol(pnl_sol),
        sign,
        format!("{:.1}", pnl_pct),
        emoji
    )
}

/// Format P&L with bold for emphasis
pub fn format_pnl_bold(pnl_sol: f64, pnl_pct: f64) -> String {
    let emoji = if pnl_sol >= 0.0 {
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
    };

    let sign = if pnl_sol >= 0.0 { "+" } else { "" };

    format!(
        "<b>{}{} SOL ({}{}%)</b> {}",
        sign,
        format_sol(pnl_sol),
        sign,
        format!("{:.1}", pnl_pct),
        emoji
    )
}

/// Format duration in human-readable form
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        if secs > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}m", mins)
        }
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let mins = (seconds % 3600) / 60;
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Format percentage
pub fn format_percent(value: f64) -> String {
    format!("{:.1}%", value)
}

/// Format USD value
pub fn format_usd(amount: f64) -> String {
    if amount.abs() < 0.01 {
        format!("${:.4}", amount)
    } else if amount.abs() < 1.0 {
        format!("${:.2}", amount)
    } else if amount.abs() < 1000.0 {
        format!("${:.2}", amount)
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
        format!("${}", result)
    }
}

// === MESSAGE TEMPLATES ===

/// Format position opened notification
pub fn msg_position_opened(
    symbol: &str,
    mint: &str,
    amount_sol: f64,
    entry_price: f64,
    tokens: f64,
    dex: &str,
) -> String {
    format!(
        r#"🟢 <b>POSITION OPENED</b>
═══════════════════

<b>Token:</b> <code>{}</code>
<b>Mint:</b> <code>{}</code>

💰 <b>Amount:</b> {} SOL
💎 <b>Price:</b> {} SOL
🪙 <b>Tokens:</b> {}
📍 <b>DEX:</b> {}"#,
        html_escape(symbol),
        format_mint_display(mint),
        format_sol(amount_sol),
        format_price(entry_price),
        format_tokens_f64(tokens),
        html_escape(dex),
    )
}

/// Format position closed notification
pub fn msg_position_closed(
    symbol: &str,
    mint: &str,
    pnl_sol: f64,
    pnl_pct: f64,
    entry_price: f64,
    exit_price: f64,
    invested: f64,
    received: f64,
    duration_secs: u64,
    reason: &str,
) -> String {
    let header_emoji = if pnl_sol >= 0.0 { "🟢" } else { "🔴" };
    let result_text = if pnl_sol >= 0.0 { "PROFIT" } else { "LOSS" };

    format!(
        r#"{} <b>POSITION CLOSED - {}</b>
═══════════════════════════

<b>Token:</b> <code>{}</code>

💰 <b>P&L:</b> {}

├─ Entry:    {} SOL
├─ Exit:     {} SOL
├─ Invested: {} SOL
├─ Received: {} SOL
└─ Reason:   {}

⏱️ <b>Duration:</b> {}"#,
        header_emoji,
        result_text,
        html_escape(symbol),
        format_pnl_bold(pnl_sol, pnl_pct),
        format_price(entry_price),
        format_price(exit_price),
        format_sol(invested),
        format_sol(received),
        html_escape(reason),
        format_duration(duration_secs),
    )
}

/// Format partial exit notification
pub fn msg_partial_exit(
    symbol: &str,
    mint: &str,
    exit_pct: f64,
    pnl_sol: f64,
    pnl_pct: f64,
    received_sol: f64,
    remaining_pct: f64,
) -> String {
    let emoji = if pnl_sol >= 0.0 { "🟡" } else { "🟠" };

    format!(
        r#"{} <b>PARTIAL EXIT</b>
═══════════════════

<b>Token:</b> <code>{}</code>

📉 <b>Sold:</b> {}% of position
💰 <b>Received:</b> {} SOL
📊 <b>P&L:</b> {}
📦 <b>Remaining:</b> {}%"#,
        emoji,
        html_escape(symbol),
        format!("{:.0}", exit_pct),
        format_sol(received_sol),
        format_pnl(pnl_sol, pnl_pct),
        format!("{:.0}", remaining_pct),
    )
}

/// Format DCA executed notification
pub fn msg_dca_executed(
    symbol: &str,
    mint: &str,
    dca_amount_sol: f64,
    total_invested: f64,
    dca_count: u32,
    new_avg_price: f64,
) -> String {
    format!(
        r#"📈 <b>DCA EXECUTED</b>
═══════════════════

<b>Token:</b> <code>{}</code>

➕ <b>Added:</b> {} SOL
💰 <b>Total Invested:</b> {} SOL
🔢 <b>DCA Count:</b> {}
💎 <b>New Avg Price:</b> {} SOL"#,
        html_escape(symbol),
        format_sol(dca_amount_sol),
        format_sol(total_invested),
        dca_count,
        format_price(new_avg_price),
    )
}

/// Format system error notification
pub fn msg_system_error(severity: &str, message: &str) -> String {
    let emoji = match severity.to_lowercase().as_str() {
        "critical" => "🚨",
        "error" => "❌",
        "warning" => "⚠️",
        _ => "ℹ️",
    };

    format!(
        r#"{} <b>SYSTEM {}</b>
═══════════════════

{}"#,
        emoji,
        severity.to_uppercase(),
        html_escape(message),
    )
}

/// Format bot started notification
pub fn msg_bot_started(
    version: &str,
    mode: &str,
    wallet_address: &str,
    balance_sol: f64,
) -> String {
    format!(
        r#"🚀 <b>SCREENERBOT STARTED</b>
═══════════════════════════

<b>Version:</b> {}
<b>Mode:</b> {}

<b>Wallet:</b> <code>{}</code>
<b>Balance:</b> {} SOL

✅ Ready for trading!"#,
        html_escape(version),
        html_escape(mode),
        format_mint_display(wallet_address),
        format_sol(balance_sol),
    )
}

/// Format bot stopped notification
pub fn msg_bot_stopped(
    reason: &str,
    uptime_secs: u64,
    trades_executed: u32,
    total_pnl: f64,
) -> String {
    format!(
        r#"🛑 <b>SCREENERBOT STOPPED</b>
═══════════════════════════

<b>Reason:</b> {}
<b>Uptime:</b> {}

<b>Session Summary:</b>
├ Trades: {}
└ P&L: {} SOL

Goodbye! 👋"#,
        html_escape(reason),
        format_duration(uptime_secs),
        trades_executed,
        format_sol(total_pnl),
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

    let emoji = if total_pnl_sol >= 0.0 { "📊" } else { "📉" };

    format!(
        r#"{} <b>DAILY SUMMARY</b>
═══════════════════

<b>Date:</b> {}

<b>Performance:</b>
├ Trades: {}
├ Wins: {} | Losses: {}
├ Win Rate: {}%
└ <b>P&L: {} SOL</b>

<b>Open Positions:</b> {}"#,
        emoji,
        html_escape(date),
        total_trades,
        winning,
        losing,
        format!("{:.1}", win_rate),
        format_sol(total_pnl_sol),
        open_positions,
    )
}

/// Format status message
pub fn msg_status(
    version: &str,
    uptime_secs: u64,
    trading_active: bool,
    entry_enabled: bool,
    exit_enabled: bool,
    open_positions: u32,
    balance_sol: f64,
    today_pnl: f64,
) -> String {
    let trading_status = if trading_active {
        "🟢 Active"
    } else {
        "🔴 Stopped"
    };
    let entry_status = if entry_enabled { "🟢" } else { "🔴" };
    let exit_status = if exit_enabled { "🟢" } else { "🔴" };

    format!(
        r#"📊 <b>SCREENERBOT STATUS</b>
═══════════════════════════

<b>Version:</b> {}
<b>Uptime:</b> {}
<b>Trading:</b> {}

<b>Monitors:</b>
├ Entry: {}
└ Exit: {}

<b>Portfolio:</b>
├ Balance: {} SOL
├ Positions: {}
└ Today P&L: {} SOL"#,
        html_escape(version),
        format_duration(uptime_secs),
        trading_status,
        entry_status,
        exit_status,
        format_sol(balance_sol),
        open_positions,
        format_sol(today_pnl),
    )
}

/// Format balance message
pub fn msg_balance(sol_balance: f64, usd_value: f64, positions_value: f64) -> String {
    let total = sol_balance + positions_value;

    format!(
        r#"💰 <b>WALLET BALANCE</b>
═══════════════════════════

<b>SOL Balance:</b> {} SOL
<b>USD Value:</b> {}

<b>Positions Value:</b> {} SOL
<b>Total Value:</b> {} SOL"#,
        format_sol(sol_balance),
        format_usd(usd_value),
        format_sol(positions_value),
        format_sol(total),
    )
}

/// Format positions list message
pub fn msg_positions_list(positions: &[(String, f64, f64, String)]) -> String {
    // positions: [(symbol, pnl_pct, value_sol, duration)]
    if positions.is_empty() {
        return "📊 <b>NO OPEN POSITIONS</b>\n\nYou have no open positions.".to_string();
    }

    let mut lines = vec!["📊 <b>OPEN POSITIONS</b>".to_string()];
    lines.push("═══════════════════════════".to_string());
    lines.push(String::new());

    let mut total_value = 0.0;
    let mut total_pnl = 0.0;

    for (i, (symbol, pnl_pct, value_sol, duration)) in positions.iter().enumerate() {
        let emoji = if *pnl_pct >= 0.0 { "🟢" } else { "🔴" };
        let sign = if *pnl_pct >= 0.0 { "+" } else { "" };

        lines.push(format!(
            "{}. {} <code>{}</code> │ {}{}% │ {} SOL │ {}",
            i + 1,
            emoji,
            html_escape(symbol),
            sign,
            format!("{:.1}", pnl_pct),
            format_sol(*value_sol),
            duration,
        ));

        total_value += value_sol;
        total_pnl += value_sol * (pnl_pct / 100.0);
    }

    lines.push(String::new());
    lines.push(format!(
        "<b>Total:</b> {} SOL │ P&L: {} SOL",
        format_sol(total_value),
        format_sol(total_pnl),
    ));

    lines.join("\n")
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

    format!(
        r#"{} <b>{} POSITION</b>
═══════════════════════════

<b>Mint:</b> <code>{}</code>

💎 <b>P&L:</b> {}

├─ Entry:    {} SOL
├─ Current:  {} SOL
├─ Invested: {} SOL
├─ Value:    {} SOL
├─ Tokens:   {}
└─ Duration: {}

🔢 <b>DCA Count:</b> {}"#,
        emoji,
        html_escape(symbol),
        format_mint_display(mint),
        format_pnl_bold(pnl_sol, pnl_pct),
        format_price(entry_price),
        format_price(current_price),
        format_sol(invested),
        format_sol(value),
        format_tokens_f64(tokens),
        format_duration(duration_secs),
        dca_count,
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
        r#"⚠️ <b>CONFIRM: Close Position</b>
═══════════════════════════

<b>Token:</b> {}
<b>Current P&L:</b> {}

This will sell ALL {} tokens.
<b>Estimated receive:</b> {} SOL

⏰ <i>Confirm within 30 seconds</i>"#,
        html_escape(symbol),
        format_pnl(pnl_sol, pnl_pct),
        format_tokens_f64(tokens),
        format_sol(est_receive),
    )
}

/// Format PIN prompt
pub fn msg_pin_prompt() -> String {
    "🔒 <b>AUTHENTICATION REQUIRED</b>\n\nPlease enter your PIN to access ScreenerBot:"
        .to_string()
}

/// Format PIN success
pub fn msg_pin_success(timeout_mins: u32) -> String {
    format!(
        "✅ <b>Authenticated</b>\n\nSession active for {} minutes.\n\nUse /help to see available commands.",
        timeout_mins
    )
}

/// Format PIN failure
pub fn msg_pin_failure(attempts_remaining: u32) -> String {
    format!(
        "❌ <b>Invalid PIN</b>\n\n{} attempts remaining.",
        attempts_remaining
    )
}

/// Format lockout message
pub fn msg_locked_out(minutes: u32) -> String {
    format!(
        "🔒 <b>Account Locked</b>\n\nToo many failed attempts. Try again in {} minutes.",
        minutes
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3700), "1h 1m");
        assert_eq!(format_duration(90000), "1d 1h");
    }
}
