//! Status and information commands
//!
//! Commands for viewing bot status, positions, balance, and stats.

use crate::chains::solana::assets::ata::get_sol_balance;
use crate::config::with_config;
use crate::i18n::{ids, UiArg, UiText};
use crate::positions;
use crate::sol_price;
use crate::telegram::formatters::{format_duration, format_mint_display, format_sol};
use crate::telegram::text::{tg, tg_escape, tg_id, with_icon};
use crate::version::VERSION;

/// Handle /status command
pub async fn handle_status_command() -> String {
    let trading_enabled = with_config(|cfg| cfg.trader.enabled);
    let entry_enabled = with_config(|cfg| cfg.trader.entry_monitor_enabled);
    let exit_enabled = with_config(|cfg| cfg.trader.exit_monitor_enabled);
    let open_positions = positions::get_open_positions_count().await;
    let uptime = (chrono::Utc::now() - *crate::global::STARTUP_TIME)
        .num_seconds()
        .max(0) as u64;
    let force_stopped = crate::global::is_force_stopped();

    let status_emoji = if force_stopped {
        "🔴"
    } else if trading_enabled {
        "🟢"
    } else {
        "🟡"
    };

    let state = if force_stopped {
        ids::TELEGRAM_STATUS_STATE_STOPPED
    } else if trading_enabled {
        ids::TELEGRAM_STATUS_STATE_ACTIVE
    } else {
        ids::TELEGRAM_STATUS_STATE_PAUSED
    };
    let on_off = |enabled: bool| {
        UiArg::Nested(Box::new(UiText::new(if enabled {
            ids::TELEGRAM_STATUS_ON
        } else {
            ids::TELEGRAM_STATUS_OFF
        })))
    };

    with_icon(
        status_emoji,
        &tg(&UiText::new(ids::TELEGRAM_STATUS_BODY)
            .arg("state", UiArg::Nested(Box::new(UiText::new(state))))
            .arg("uptime", UiArg::Text(format_duration(uptime)))
            .arg("version", UiArg::Text(VERSION.to_owned()))
            .arg("entries", on_off(entry_enabled))
            .arg("exits", on_off(exit_enabled))
            .arg("positions", UiArg::Text(open_positions.to_string()))),
    )
}

/// Handle /positions command
pub async fn handle_positions_command() -> String {
    let positions = positions::get_open_positions().await;

    if positions.is_empty() {
        return with_icon("📦", &tg_id(ids::TELEGRAM_POSITIONS_EMPTY));
    }

    let mut response = with_icon(
        "📦",
        &tg(&UiText::new(ids::TELEGRAM_POSITIONS_TITLE)
            .arg("count", UiArg::Text(positions.len().to_string()))),
    );
    response.push_str("\n\n");

    let mut total_invested = 0.0;
    let mut total_pnl = 0.0;

    for (_i, pos) in positions.iter().take(10).enumerate() {
        let pnl_pct = pos.unrealized_pnl_percent.unwrap_or_default();
        let pnl_sol = pos.unrealized_pnl.unwrap_or_default();
        let pnl_emoji = if pnl_pct >= 0.0 { "🟢" } else { "🔴" };
        let sign = if pnl_pct >= 0.0 { "+" } else { "" };
        // Count characters, not bytes: symbols may be multibyte.
        let symbol = if pos.symbol.chars().count() > 6 {
            format!("{}..", pos.symbol.chars().take(5).collect::<String>())
        } else {
            pos.symbol.clone()
        };

        response.push_str(&with_icon(
            pnl_emoji,
            &tg(&UiText::new(ids::TELEGRAM_POSITIONS_ROW)
                .arg("symbol", UiArg::Text(symbol))
                .arg(
                    "pnl_sol",
                    UiArg::Text(format!("{sign}{}", format_sol(pnl_sol))),
                )
                .arg("pnl_pct", UiArg::Text(format!("{sign}{pnl_pct:.1}")))),
        ));
        response.push('\n');

        total_invested += pos.total_size_sol;
        total_pnl += pnl_sol;
    }

    if positions.len() > 10 {
        response.push('\n');
        response.push_str(&tg(&UiText::new(ids::TELEGRAM_POSITIONS_MORE)
            .arg("count", UiArg::Text((positions.len() - 10).to_string()))));
        response.push('\n');
    }

    let sign = if total_pnl >= 0.0 { "+" } else { "" };
    response.push('\n');
    response.push_str(&tg(&UiText::new(ids::TELEGRAM_POSITIONS_SUMMARY)
        .arg("invested", UiArg::Text(format_sol(total_invested)))
        .arg(
            "pnl",
            UiArg::Text(format!("{sign}{}", format_sol(total_pnl))),
        )));

    response
}

/// Handle /balance command
pub async fn handle_balance_command() -> String {
    let wallet_address = match crate::utils::get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => return with_icon("❌", &tg_escape(&e.to_string())),
    };

    let sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => balance,
        Err(e) => return with_icon("❌", &tg_escape(&e.to_string())),
    };

    let sol_price_usd = sol_price::get_sol_price();
    let usd_value = sol_balance * sol_price_usd;

    format!(
        "{}\n\n<a href=\"https://solscan.io/account/{}\">{}</a>",
        with_icon(
            "💰",
            &tg(&UiText::new(ids::TELEGRAM_BALANCE_BODY)
                .arg("sol", UiArg::Text(format_sol(sol_balance)))
                .arg("usd", UiArg::Usd(format!("{usd_value:.2}")))),
        ),
        wallet_address,
        format_mint_display(&wallet_address),
    )
}

/// Handle /stats command
pub async fn handle_stats_command() -> String {
    let positions = positions::get_open_positions().await;

    let mut total_invested = 0.0;
    let mut total_pnl = 0.0;

    for pos in &positions {
        total_invested += pos.total_size_sol;
        total_pnl += pos.unrealized_pnl.unwrap_or_default();
    }

    let pnl_emoji = if total_pnl >= 0.0 { "🟢" } else { "🔴" };
    let sign = if total_pnl >= 0.0 { "+" } else { "" };

    format!(
        "{} {pnl_emoji}",
        with_icon(
            "📈",
            &tg(&UiText::new(ids::TELEGRAM_STATS_BODY)
                .arg("positions", UiArg::Text(positions.len().to_string()))
                .arg("invested", UiArg::Text(format_sol(total_invested)))
                .arg(
                    "pnl",
                    UiArg::Text(format!("{sign}{}", format_sol(total_pnl)))
                )),
        )
    )
}
