//! Demo generator for the top header metrics bar.

use chrono::Utc;

use crate::webserver::routes::header::{
    FilteringHeaderInfo, HeaderMetricsResponse, PositionsHeaderInfo, RpcHeaderInfo, SolHeaderInfo,
    SystemHeaderInfo, TraderHeaderInfo, WalletHeaderInfo,
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
        running: true,
        enabled: true,
        today_pnl_sol: today.net_pnl_sol,
        today_pnl_percent,
        uptime_seconds: 3 * 24 * 3600 + 7 * 3600 + 23 * 60 + 45,
    };

    let change_24h_sol = DEMO_SOL_BALANCE - DEMO_START_BALANCE;
    let wallet = WalletHeaderInfo {
        sol_balance: DEMO_SOL_BALANCE,
        change_24h_sol,
        change_24h_percent: change_24h_sol / DEMO_START_BALANCE * 100.0,
        token_count: open.count,
        tokens_worth_sol: open.current_value_sol,
        last_updated: now.to_rfc3339(),
    };

    let positions = PositionsHeaderInfo {
        open_count: open.count as i64,
        unrealized_pnl_sol: open.unrealized_pnl_sol,
        unrealized_pnl_percent: open.unrealized_pnl_percent,
        total_invested_sol: open.invested_sol,
    };

    let rpc = RpcHeaderInfo {
        success_rate_percent: 99.7,
        avg_latency_ms: 142,
        calls_per_minute: 284.5,
        healthy: true,
    };

    let filtering = FilteringHeaderInfo {
        monitoring_count: DEMO_TOKENS_TRACKED,
        passed_count: 347,
        rejected_count: 2500,
        last_refresh: now.to_rfc3339(),
    };

    let system = SystemHeaderInfo {
        all_services_healthy: true,
        unhealthy_services: vec![],
        critical_degraded: false,
    };

    HeaderMetricsResponse {
        trader,
        wallet,
        positions,
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
