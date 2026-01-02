//! Status and information commands
//!
//! Commands for viewing bot status, positions, balance, and stats.

use crate::config::with_config;
use crate::positions;
use crate::sol_price;
use crate::telegram::formatters::{format_duration, format_mint_display, format_sol};
use crate::utils::get_sol_balance;
use crate::version::VERSION;

/// Handle /status command
pub async fn handle_status_command() -> String {
    let trading_enabled = with_config(|cfg| cfg.trader.enabled);
    let entry_enabled = with_config(|cfg| cfg.trader.entry_monitor_enabled);
    let exit_enabled = with_config(|cfg| cfg.trader.exit_monitor_enabled);
    let open_positions = positions::get_open_positions_count().await;
    let uptime = (chrono::Utc::now() - *crate::global::STARTUP_TIME).num_seconds() as u64;
    let force_stopped = crate::global::is_force_stopped();

    let status_emoji = if force_stopped {
        "🚨"
    } else if trading_enabled {
        "🟢"
    } else {
        "🟡"
    };

    let trading_status = if force_stopped {
        "Force Stopped"
    } else if trading_enabled {
        "Active"
    } else {
        "Paused"
    };

    let entry_icon = if entry_enabled { "✅" } else { "⏸️" };
    let exit_icon = if exit_enabled { "✅" } else { "⏸️" };

    format!(
        "{} <b>Status</b>  ·  v{}\n\n\
         <b>Trading:</b> {}\n\
         ├ Entry: {}\n\
         └ Exit: {}\n\n\
         📦 Positions: {}\n\
         ⏱️ Uptime: {}",
        status_emoji,
        VERSION,
        trading_status,
        entry_icon,
        exit_icon,
        open_positions,
        format_duration(uptime),
    )
}

/// Handle /positions command
pub async fn handle_positions_command() -> String {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        return "📦 <b>No Open Positions</b>".to_string();
    }

    let mut response = format!("📦 <b>Positions ({})</b>\n\n", positions.len());

    let mut total_invested = 0.0;
    let mut total_pnl = 0.0;

    for (i, pos) in positions.iter().take(10).enumerate() {
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or(0.0);
        let pnl_sol = pos.unrealized_pnl.unwrap_or(0.0);
        let pnl_emoji = if pnl_pct >= 0.0 { "🟢" } else { "🔴" };
        let sign = if pnl_pct >= 0.0 { "+" } else { "" };

        response.push_str(&format!(
            "{}. <code>${}</code> {} {}{:.1}%  ·  {} SOL\n",
            i + 1,
            pos.symbol,
            pnl_emoji,
            sign,
            pnl_pct,
            format_sol(pos.total_size_sol),
        ));

        total_invested += pos.total_size_sol;
        total_pnl += pnl_sol;
    }

    if positions.len() > 10 {
        response.push_str(&format!("\n<i>+{} more...</i>\n", positions.len() - 10));
    }

    let pnl_emoji = if total_pnl >= 0.0 { "🟢" } else { "🔴" };
    response.push_str(&format!(
        "\n<b>Total:</b> {} SOL  ·  {} SOL {}",
        format_sol(total_invested),
        format_sol(total_pnl),
        pnl_emoji,
    ));

    response
}

/// Handle /balance command
pub async fn handle_balance_command() -> String {
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => return format!("❌ {}", e),
    };

    let sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => balance,
        Err(e) => return format!("❌ {}", e),
    };

    let sol_price_usd = sol_price::get_sol_price();
    let usd_value = sol_balance * sol_price_usd;

    format!(
        "💰 <b>Balance</b>\n\n\
         <code>{}</code>\n\n\
         🪨 <b>{} SOL</b>\n\
         💵 ${:.2}",
        format_mint_display(&wallet_address),
        format_sol(sol_balance),
        usd_value,
    )
}

/// Handle /stats command
pub async fn handle_stats_command() -> String {
    let positions = positions::get_open_positions().await;

    let mut total_invested = 0.0;
    let mut total_pnl = 0.0;

    for pos in &positions {
        total_invested += pos.total_size_sol;
        total_pnl += pos.unrealized_pnl.unwrap_or(0.0);
    }

    let pnl_emoji = if total_pnl >= 0.0 { "🟢" } else { "🔴" };
    let sign = if total_pnl >= 0.0 { "+" } else { "" };

    format!(
        "📈 <b>Stats</b>\n\n\
         📦 Positions: {}\n\
         💵 Invested: {} SOL\n\
         📊 P&L: {}{} SOL {}",
        positions.len(),
        format_sol(total_invested),
        sign,
        format_sol(total_pnl),
        pnl_emoji,
    )
}
