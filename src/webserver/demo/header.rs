//! Demo generator for the top header metrics bar.

use chrono::Utc;

use crate::filtering::SnapshotState;
use crate::webserver::routes::header::{
    FilteringHeaderInfo, HeaderMetricsResponse, RpcHeaderInfo, SolHeaderInfo, SystemHeaderInfo,
    TraderHeaderInfo, TraderHeaderState, WalletHeaderInfo,
};

use super::aggregates;
use super::data::*;
use super::DEMO_SOL_PRICE_FALLBACK;

/// Live SOL/USD price if the price service has a fresh quote, else the fallback.
/// Demo mode starts a lightweight SOL price service so this is real when online.
fn live_sol_price() -> f64 {
    let live = crate::sol_price::get_sol_price();
    if live > 0.0 {
        live
    } else {
        DEMO_SOL_PRICE_FALLBACK
    }
}

/// Generate demo header metrics response.
pub fn get_demo_header_metrics() -> HeaderMetricsResponse {
    let now = Utc::now();
    let open = aggregates::open_agg();
    let trades = aggregates::closed_trades(now);
    let today = aggregates::within_hours(&trades, now, 24);

    let today_pnl_percent = today.net_pnl_sol / DEMO_START_BALANCE * 100.0;
    let trader = TraderHeaderInfo {
        enabled: true,
        state: TraderHeaderState::Running,
        today_pnl_sol: today.net_pnl_sol,
        today_pnl_percent,
    };

    // Demo must model the real thing: the headline is WORTH (cash + holdings), measured
    // against the start-of-day worth — identical to the home hero's demo numbers. Showing
    // cash here while the hero showed equity made the two cards disagree on screen.
    let demo_equity = DEMO_SOL_BALANCE + open.current_value_sol;
    let change_today_sol = demo_equity - DEMO_START_BALANCE;
    let wallet = WalletHeaderInfo {
        sol_balance: DEMO_SOL_BALANCE,
        tokens_worth_sol: open.current_value_sol,
        total_equity_sol: demo_equity,
        change_today_sol: Some(change_today_sol),
        change_today_percent: Some(change_today_sol / DEMO_START_BALANCE * 100.0),
        token_count: open.count,
        last_updated: now.to_rfc3339(),
    };

    let rpc = RpcHeaderInfo {
        success_rate_percent: 99.7,
        avg_latency_ms: 142,
        calls_per_minute: 284.5,
        healthy: true,
    };

    let filtering = FilteringHeaderInfo {
        snapshot_state: SnapshotState::Ready,
        monitoring_count: Some(DEMO_TOKENS_TRACKED),
        passed_count: Some(347),
        rejected_count: Some(2500),
        last_refresh: Some(now.to_rfc3339()),
    };

    let system = SystemHeaderInfo {
        all_services_healthy: true,
        unhealthy_services: vec![],
        critical_degraded: false,
    };

    HeaderMetricsResponse {
        trader,
        wallet,
        rpc,
        filtering,
        system,
        sol: SolHeaderInfo {
            price_usd: live_sol_price(),
            change_24h_percent: crate::ohlcvs::sol_usd_chart::change_24h_percent().or(Some(2.3)),
        },
        timestamp: now.to_rfc3339(),
    }
}
