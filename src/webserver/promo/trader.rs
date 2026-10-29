// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generators for the Auto Trader page: the trader statistics tab, the
//! running state, the monitor controls and the period loss limit panel.

use chrono::{DateTime, Duration, Utc};

use crate::positions::{Book, Opening, Position};
use crate::trader::stats::{RealizedStats, StatsWindow, TraderStats};
use crate::trader::{
    LossLimitSnapshot, MonitorState, MonitorsStatus, TraderRunState, TraderStatus,
};

use super::aggregates;
use super::data::PROMO_OPEN_TOKENS;
use super::strategies;

/// Network fee of each promo buy and sale, in raw units, as the promo positions list
/// reports it.
const PROMO_FEE_RAW: u64 = 5000;

/// Configured period loss limit, in SOL.
const PROMO_LOSS_LIMIT_NATIVE: f64 = 0.5;

/// Length of one loss limit period, the daily default of the live setting.
const PROMO_LOSS_LIMIT_PERIOD_HOURS: i64 = 24;

/// How far into its period the promo session is (19h 20m), so the panel shows a
/// countdown rather than a freshly reset period.
const PROMO_LOSS_LIMIT_ELAPSED_MINUTES: i64 = 19 * 60 + 20;

/// Generate the promo trader status: available and running, the state the promo
/// header and status bar report, with the enabled strategies of the promo roster.
pub fn get_promo_trader_status() -> TraderStatus {
    let (live_strategies, paper_strategies) = strategies::enabled_counts();
    TraderStatus {
        enabled: true,
        running: true,
        available: true,
        unavailable_reason: None,
        live_strategies,
        paper_strategies,
        run_state: TraderRunState::Running,
        live_entry_idle: None,
        paper_entry_idle: None,
    }
}

/// Generate the promo monitor status: both monitors enabled and running under a
/// running trader.
pub fn get_promo_monitors_status() -> MonitorsStatus {
    let trader = get_promo_trader_status();
    let monitor = MonitorState {
        enabled: true,
        running: trader.running,
    };
    MonitorsStatus {
        entry_monitor: monitor.clone(),
        exit_monitor: monitor,
        master_enabled: trader.running,
        force_stopped: false,
        available: trader.available,
        anything_running: trader.running,
    }
}

/// Generate the promo period loss limit status. The period loss is the realized
/// loss of the promo closed trades that exited since the period started, counted
/// the way the live limiter reads the books, so it never contradicts the trade
/// history shown elsewhere.
pub fn get_promo_loss_limit_status() -> LossLimitSnapshot {
    let now = Utc::now();
    let period_start = now - Duration::minutes(PROMO_LOSS_LIMIT_ELAPSED_MINUTES);
    let period_end = period_start + Duration::hours(PROMO_LOSS_LIMIT_PERIOD_HOURS);
    let trades = aggregates::closed_trades(now);
    let current_loss_native =
        aggregates::period_over(trades.iter().filter(|t| t.exit_time >= period_start)).loss_sol;
    let is_limited = current_loss_native >= PROMO_LOSS_LIMIT_NATIVE;

    LossLimitSnapshot {
        enabled: true,
        limit_native: PROMO_LOSS_LIMIT_NATIVE,
        current_loss_native,
        is_limited,
        limited_at: is_limited.then_some(now),
        period_start,
        period_remaining_secs: (period_end - now).num_seconds().max(0),
        progress_percent: (current_loss_native / PROMO_LOSS_LIMIT_NATIVE * 100.0).min(100.0),
    }
}

/// Generate the promo trader stats of `window`: the promo closed trades become closed
/// rounds and go through the same aggregation as a real book, so every figure the Stats
/// tab shows is one the live handler computes. Open exposure comes from the open arrays.
pub fn get_promo_trader_stats(window: StatsWindow) -> TraderStats {
    let now = Utc::now();
    let since = window.since(now);
    let closed: Vec<Position> = promo_closed_rounds(now)
        .into_iter()
        .filter(|round| round.exit_time.is_some_and(|exit| exit >= since))
        .collect();
    let realized = RealizedStats::of(window, closed, now);
    let open = aggregates::open_agg();
    TraderStats {
        open_positions_count: open.count,
        max_open_positions: open.count.max(1),
        locked_native: open.invested_sol,
        unrealized_native: Some(open.unrealized_pnl_sol),
        open_unpriced: 0,
        realized,
    }
}

/// The open rounds of the promo open positions at `now`, with the id, owner, price and
/// unrealized P&L the promo positions list gives each.
pub(super) fn promo_open_rounds(now: DateTime<Utc>) -> Vec<Position> {
    PROMO_OPEN_TOKENS
        .iter()
        .zip(1..)
        .map(
            |(&(symbol, name, mint, _, entry, current, size, hold_minutes), id)| {
                let (origin, management) = super::copy_trading::position_owner(mint);
                let mut position = Position::opening(Opening {
                    mint: mint.to_owned(),
                    symbol: symbol.to_owned(),
                    name: name.to_owned(),
                    entry_price: entry,
                    entry_time: now - Duration::minutes(hold_minutes),
                    size_native: size,
                    entry_signature: None,
                    origin,
                    management,
                    entry_liquidity: None,
                    book: Book::Live,
                });
                position.id = Some(id);
                position.transaction_entry_verified = true;
                position.entry_fee_raw = Some(PROMO_FEE_RAW);
                position.current_price = Some(current);
                position.current_price_updated = Some(now);
                position.unrealized_pnl = Some((current - entry) / entry * size);
                position.unrealized_pnl_percent = Some((current - entry) / entry * 100.0);
                position
            },
        )
        .collect()
}

/// The closed rounds of the promo closed trades at `now`, in the order of the promo
/// positions list.
pub(super) fn promo_closed_rounds(now: DateTime<Utc>) -> Vec<Position> {
    aggregates::closed_trades(now)
        .iter()
        .enumerate()
        .map(|(index, trade)| promo_closed_round(index, trade))
        .collect()
}

/// The closed round of the `index`-th promo closed trade, with the id, owner and fees the
/// promo positions list gives it.
fn promo_closed_round(index: usize, trade: &aggregates::ClosedTrade) -> Position {
    let (origin, management) = super::copy_trading::position_owner(trade.mint);
    let mut position = Position::opening(Opening {
        mint: trade.mint.to_owned(),
        symbol: trade.symbol.to_owned(),
        name: trade.name.to_owned(),
        entry_price: trade.entry_price,
        entry_time: trade.entry_time,
        size_native: trade.size_sol,
        entry_signature: None,
        origin,
        management,
        entry_liquidity: None,
        book: Book::Live,
    });
    position.id = i64::try_from(PROMO_OPEN_TOKENS.len() + index + 1).ok();
    position.total_size_native = trade.size_sol;
    position.transaction_exit_verified = true;
    position.entry_fee_raw = Some(PROMO_FEE_RAW);
    position.exit_fee_raw = Some(PROMO_FEE_RAW);
    position.exit_price = Some(trade.exit_price);
    position.exit_time = Some(trade.exit_time);
    position.pnl = Some(trade.pnl_sol);
    position.pnl_percent = Some(trade.pnl_percent);
    position.closed_reason = Some(trade.reason.to_owned());
    position
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::promo::get_promo_header_metrics;

    #[test]
    fn trader_status_and_monitors_agree_with_the_header() {
        crate::config::utils::install_default_config();
        let header = get_promo_header_metrics().trader;
        let header_running = matches!(header.state, TraderRunState::Running);

        let status = get_promo_trader_status();
        assert!(status.available);
        assert_eq!(status.enabled, header.enabled);
        assert_eq!(status.running, header_running);
        assert!(status.running);

        let monitors = get_promo_monitors_status();
        assert!(monitors.available);
        assert!(!monitors.force_stopped);
        assert_eq!(monitors.master_enabled, status.running);
        assert_eq!(monitors.anything_running, status.running);
        for monitor in [&monitors.entry_monitor, &monitors.exit_monitor] {
            assert!(monitor.enabled);
            assert_eq!(monitor.running, status.running);
        }
    }

    #[test]
    fn loss_limit_is_configured_and_untripped() {
        let limit = get_promo_loss_limit_status();
        assert!(limit.enabled);
        assert!(limit.limit_native > 0.0);
        assert!(!limit.is_limited);
        assert_eq!(limit.limited_at, None);
        assert!(limit.current_loss_native >= 0.0);
        assert!(limit.current_loss_native < limit.limit_native);
        // Below the panel's warning band, so the bar never reads as a near miss.
        assert!(limit.progress_percent < 75.0);
        assert!(limit.period_remaining_secs > 0);
        assert!(limit.period_remaining_secs < PROMO_LOSS_LIMIT_PERIOD_HOURS * 3600);
    }

    #[test]
    fn loss_limit_counts_the_closed_trades_of_the_period() {
        let limit = get_promo_loss_limit_status();
        let trades = aggregates::closed_trades(Utc::now());
        let expected: f64 = trades
            .iter()
            .filter(|t| t.exit_time >= limit.period_start && t.pnl_sol < 0.0)
            .map(|t| -t.pnl_sol)
            .sum();
        assert!((limit.current_loss_native - expected).abs() < 1e-9);
    }

    #[test]
    fn promo_trader_stats_follow_the_window_and_the_shared_aggregation() {
        crate::config::utils::install_default_config();
        let day = get_promo_trader_stats(StatsWindow::Days(1));
        let all = get_promo_trader_stats(StatsWindow::All);
        let trades = aggregates::closed_trades(Utc::now());

        assert_eq!(all.realized.total_trades, trades.len());
        assert!(day.realized.total_trades > 0);
        assert!(day.realized.total_trades < all.realized.total_trades);
        assert_eq!(day.realized.period_days, Some(1));
        assert_eq!(all.realized.period_days, None);
        let net: f64 = trades.iter().map(|t| t.pnl_sol).sum();
        assert!((all.realized.total_pnl_native - net).abs() < 1e-9);

        // Every exit group carries a catalog label, never the stored reason as text.
        for exit in &all.realized.exit_breakdown {
            assert!(exit.label.id.starts_with("trade-reason-"), "{exit:?}");
            assert_ne!(exit.label.id, "trade-reason-stored", "{exit:?}");
        }
        assert_eq!(
            all.realized
                .exit_breakdown
                .iter()
                .map(|exit| exit.count)
                .sum::<usize>(),
            all.realized.total_trades
        );
        assert_eq!(all.open_positions_count, PROMO_OPEN_TOKENS.len());
        assert_eq!(all.open_unpriced, 0);
        assert!(all.unrealized_native.is_some());
    }
}
