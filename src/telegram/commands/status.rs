//! Status and information commands
//!
//! Commands for viewing bot status, positions, balance, and stats.

use crate::config::with_config;
use crate::positions;
use crate::sol_price;
use crate::telegram::formatters::{format_duration, format_mint_display};
use crate::utils::get_sol_balance;
use crate::version::VERSION;

/// Handle /status command
pub async fn handle_status_command() -> String {
    let trading_enabled = with_config(|cfg| cfg.trader.enabled);
    let open_positions = positions::get_open_positions_count().await;
    let uptime = (chrono::Utc::now() - *crate::global::STARTUP_TIME).num_seconds() as u64;
    let services_ready = crate::global::are_core_services_ready();

    let uptime_str = format_duration(uptime);
    let status_emoji = if trading_enabled && services_ready {
        "🟢"
    } else if services_ready {
        "🟡"
    } else {
        "🔴"
    };
    let trading_status = if trading_enabled { "Enabled" } else { "Disabled" };

    format!(
        "{} <b>ScreenerBot Status</b>\n\n\
         Version: {}\n\
         Uptime: {}\n\
         Trading: {}\n\
         Open Positions: {}\n\
         Services Ready: {}",
        status_emoji,
        VERSION,
        uptime_str,
        trading_status,
        open_positions,
        if services_ready { "Yes ✅" } else { "No ❌" }
    )
}

/// Handle /positions command
pub async fn handle_positions_command() -> String {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        return "📊 <b>No Open Positions</b>\n\nYou have no active positions.".to_string();
    }

    let mut response = format!("📊 <b>Open Positions ({})</b>\n\n", positions.len());

    for (i, pos) in positions.iter().take(10).enumerate() {
        let pnl_emoji = if pos.unrealized_pnl.unwrap_or(0.0) >= 0.0 {
            "🟢"
        } else {
            "🔴"
        };
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or(0.0);
        let pnl_sign = if pnl_pct >= 0.0 { "+" } else { "" };

        response.push_str(&format!(
            "{}. <code>${}</code> {}\n   Size: {:.4} SOL | P&L: {}{:.2}%\n\n",
            i + 1,
            pos.symbol,
            pnl_emoji,
            pos.total_size_sol,
            pnl_sign,
            pnl_pct
        ));
    }

    if positions.len() > 10 {
        response.push_str(&format!("... and {} more", positions.len() - 10));
    }

    response
}

/// Handle /balance command
pub async fn handle_balance_command() -> String {
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => return format!("❌ <b>Error</b>\n\nFailed to get wallet address: {}", e),
    };

    let sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => balance,
        Err(e) => return format!("❌ <b>Error</b>\n\nFailed to get SOL balance: {}", e),
    };

    // Get SOL price for USD value
    let sol_price_usd = sol_price::get_sol_price();
    let usd_value = sol_balance * sol_price_usd;

    format!(
        "💰 <b>Wallet Balance</b>\n\n\
         Address: <code>{}</code>\n\
         SOL: {:.4}\n\
         USD: ${:.2}",
        format_mint_display(&wallet_address),
        sol_balance,
        usd_value
    )
}

/// Handle /stats command
pub async fn handle_stats_command() -> String {
    let positions = positions::get_open_positions().await;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Calculate totals from open positions
    let mut total_invested = 0.0;
    let mut total_unrealized_pnl = 0.0;

    for pos in &positions {
        total_invested += pos.total_size_sol;
        total_unrealized_pnl += pos.unrealized_pnl.unwrap_or(0.0);
    }

    let pnl_emoji = if total_unrealized_pnl >= 0.0 {
        "🟢"
    } else {
        "🔴"
    };
    let pnl_sign = if total_unrealized_pnl >= 0.0 { "+" } else { "" };

    format!(
        "📈 <b>Trading Statistics</b>\n\n\
         Date: {}\n\n\
         <b>Open Positions:</b> {}\n\
         <b>Total Invested:</b> {:.4} SOL\n\
         <b>Unrealized P&L:</b> {}{:.4} SOL {}\n\n\
         <i>Use /positions to see individual positions</i>",
        today,
        positions.len(),
        total_invested,
        pnl_sign,
        total_unrealized_pnl,
        pnl_emoji
    )
}
