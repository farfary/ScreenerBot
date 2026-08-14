//! Promo generator for the trader statistics tab.

use chrono::Utc;

use crate::webserver::routes::trader::types::{ExitBreakdown, TraderStatsResponse};

use super::aggregates;

/// Generate promo trader stats response, fully derived from the closed/open arrays.
pub fn get_promo_trader_stats() -> TraderStatsResponse {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let realized = aggregates::period_over(trades.iter());
    let (best, worst) = aggregates::best_worst(&trades);

    let exit_breakdown = aggregates::reason_breakdown(&trades)
        .into_iter()
        .map(|r| ExitBreakdown {
            exit_type: r.reason,
            count: r.count,
            avg_profit_pct: r.avg_profit_pct,
        })
        .collect();

    TraderStatsResponse {
        open_positions_count: open.count,
        locked_sol: open.invested_sol,
        win_rate_pct: realized.win_rate,
        total_trades: trades.len(),
        avg_hold_time_hours: aggregates::avg_hold_hours(&trades),
        best_trade_pct: best.pnl_percent,
        best_trade_token: Some(best.symbol.to_owned()),
        worst_trade_pct: worst.pnl_percent,
        worst_trade_token: Some(worst.symbol.to_owned()),
        // Trader stats reports REALIZED P&L (closed trades) like the live handler.
        total_pnl_sol: realized.net_pnl_sol,
        exit_breakdown,
    }
}
