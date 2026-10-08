// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Handler for the header metrics endpoint - today's trading P&L, wallet equity and system health aggregation.

use axum::response::Json;

use crate::chains::ChainScope;
use crate::config::with_config;
use crate::connectivity::state::are_critical_endpoints_healthy;
use crate::filtering::{try_fetch_stats, SnapshotState};
use crate::global::are_core_services_ready;
use crate::rpc::{get_global_rpc_stats, RpcStats};
use crate::services::get_service_manager;
use crate::wallet::{get_balance_at_time, get_wallet_worth};

use super::types::*;

pub(super) async fn get_header_metrics() -> Json<HeaderMetricsResponse> {
    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_header_metrics());
    }

    let now = chrono::Utc::now();

    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_default()
        .and_utc();

    // Header collectors are independent. Keep the frequently-polled endpoint bounded by
    // running database/service work concurrently and reusing each result once.
    //
    // The filtering read is the NON-BLOCKING one, for the same reason the home dashboard's
    // is. The header is on screen from the moment the app paints and polls about once a
    // second; the blocking variant waits up to 30 seconds for the FIRST snapshot to be
    // built, so on every launch this endpoint — and with it the whole top bar — stalled for
    // that entire timeout while the initial snapshot ground through the corpus. Counts that
    // are briefly absent cost nothing here; the very next poll picks them up.
    let (today_stats, start_balance, filtering_stats, system) = tokio::join!(
        crate::positions::get_period_trading_stats(today_start, Some(now)),
        get_balance_at_time(today_start),
        try_fetch_stats(ChainScope::All),
        calculate_system_health(),
    );

    // The wallet's worth is served from the live in-memory snapshot, priced now — no
    // database read on this once-a-second endpoint, and the exact same figure the home
    // hero renders.
    let worth = get_wallet_worth();

    let explore = crate::global::is_explore_mode();
    let (trader_enabled, entry_enabled, exit_enabled) = with_config(|cfg| {
        (
            cfg.trader.enabled,
            cfg.trader.entry_monitor_enabled,
            cfg.trader.exit_monitor_enabled,
        )
    });
    let trader_state = if explore {
        TraderHeaderState::Explore
    } else if crate::global::is_force_stopped() {
        TraderHeaderState::ForceStopped
    } else if !trader_enabled {
        TraderHeaderState::Stopped
    } else if !are_core_services_ready() {
        TraderHeaderState::Waiting
    } else if !entry_enabled && !exit_enabled {
        TraderHeaderState::Idle
    } else if entry_enabled && crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
        TraderHeaderState::EntryPaused
    } else {
        TraderHeaderState::Running
    };

    let today_pnl_native = today_stats
        .as_ref()
        .map(|stats| stats.net_pnl_native)
        .unwrap_or_default();
    let start_balance_sol = start_balance.ok().flatten();
    let today_pnl_percent = start_balance_sol
        .filter(|balance| *balance > f64::EPSILON)
        .map(|balance| today_pnl_native / balance * 100.0)
        .unwrap_or_default();

    let trader = TraderHeaderInfo {
        enabled: !explore && trader_enabled,
        state: trader_state,
        today_pnl_native,
        today_pnl_percent,
    };

    let wallet = WalletHeaderInfo {
        sol_balance: worth.native_balance,
        tokens_worth_native: worth.tokens_worth_native,
        total_equity_native: worth.total_equity_native,
        change_today_native: start_balance_sol.map(|start| worth.total_equity_native - start),
        change_today_percent: start_balance_sol
            .filter(|start| *start > f64::EPSILON)
            .map(|start| (worth.total_equity_native - start) / start * 100.0),
        token_count: worth.token_count,
        last_updated: worth.updated_at.to_rfc3339(),
    };

    let rpc = rpc_header_info(get_global_rpc_stats().as_ref());

    // Absent, not zeroed, while the first snapshot builds: a top bar reading "0 monitored,
    // 0 passed, refreshed just now" is a wrong answer, where "—" is an honest one.
    let filtering = FilteringHeaderInfo {
        snapshot_state: SnapshotState::of(&filtering_stats),
        monitoring_count: filtering_stats.as_ref().map(|stats| stats.total_tokens),
        passed_count: filtering_stats.as_ref().map(|stats| stats.passed_filtering),
        rejected_count: filtering_stats
            .as_ref()
            .map(|stats| stats.total_tokens.saturating_sub(stats.passed_filtering)),
        last_refresh: filtering_stats
            .as_ref()
            .map(|stats| stats.updated_at.to_rfc3339()),
    };

    // SOL/USD price for the header price card.
    let sol = SolHeaderInfo {
        price_usd: crate::native_price::get_native_price(),
        change_24h_percent: crate::native_price::get_native_price_info()
            .and_then(|info| info.chain)
            .and_then(crate::ohlcvs::native_usd_chart::change_24h_percent),
    };

    Json(HeaderMetricsResponse {
        trader,
        wallet,
        rpc,
        filtering,
        system,
        sol,
        copy: copy_header().await,
        timestamp: now.to_rfc3339(),
    })
}

async fn copy_header() -> Option<CopyHeaderInfo> {
    let status = crate::trader::copy::control::status().await.ok()?;
    Some(CopyHeaderInfo {
        enabled: status.enabled,
        total_tasks: status.total_tasks,
        paper_tasks: status.paper_tasks,
        live_tasks: status.live_tasks,
        notices: crate::trader::copy::recent_notices(),
    })
}

async fn calculate_system_health() -> SystemHeaderInfo {
    let mut unhealthy_services = Vec::new();
    let mut critical_degraded = false;

    // In Explore Mode the wallet/RPC services are intentionally not running, so
    // "core services" being incomplete is expected and must not be flagged as a problem.
    let explore = crate::global::is_explore_mode();

    // Check core services readiness (skipped in Explore Mode)
    if !explore && !are_core_services_ready() {
        unhealthy_services.push("Core Services".to_owned());
        critical_degraded = true;
    }

    // Check critical endpoints
    if !are_critical_endpoints_healthy().await {
        unhealthy_services.push("Critical Endpoints".to_owned());
        critical_degraded = true;
    }

    // Check service manager health. Only enabled services can be "unhealthy" — a
    // disabled service (e.g. trading/pools in Explore Mode, or any service
    // turned off via config) is intentionally off, not a fault.
    if let Some(manager_arc) = get_service_manager().await {
        let manager = manager_arc.read().await;
        if let Some(manager) = &*manager {
            let health_map = manager.get_health().await;
            for (name, health) in health_map {
                // Only genuine faults count as issues — not transient "starting" or
                // intentionally "disabled" services.
                if health.is_unhealthy() || health.is_degraded() {
                    unhealthy_services.push(name.to_string());
                }
            }
        }
    }

    let all_services_healthy = unhealthy_services.is_empty();

    SystemHeaderInfo {
        all_services_healthy,
        unhealthy_services,
        critical_degraded,
    }
}

/// The header's RPC figures. Without an RPC manager (Explore Mode) there is no rate
/// at all, and before the first call there is no success rate: both are absent,
/// never a 0% or 100% that reads as a measurement.
fn rpc_header_info(stats: Option<&RpcStats>) -> RpcHeaderInfo {
    let Some(stats) = stats else {
        return RpcHeaderInfo {
            success_rate_percent: None,
            avg_latency_ms: 0,
            calls_per_minute: None,
            healthy: false,
        };
    };
    let recent_cpm = stats.calls_per_minute_recent(5);
    let uptime_secs = chrono::Utc::now()
        .signed_duration_since(stats.startup_time)
        .num_seconds()
        .max(0) as u64;
    let fallback_cpm = (stats.total_calls() as f64 / uptime_secs.max(1) as f64) * 60.0;

    RpcHeaderInfo {
        success_rate_percent: (stats.total_calls() > 0).then(|| stats.success_rate()),
        avg_latency_ms: stats.average_response_time_ms_global() as u64,
        calls_per_minute: Some(if recent_cpm > 0.0 {
            recent_cpm
        } else {
            fallback_cpm
        }),
        healthy: stats.success_rate() > 90.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn stats(total_calls: u64, total_errors: u64, success_rate: f32) -> RpcStats {
        RpcStats {
            session_id: String::new(),
            startup_time: chrono::Utc::now(),
            total_calls,
            total_errors,
            success_rate,
            avg_latency_ms: 0.0,
            uptime_secs: 0,
            calls_last_minute: 0,
            provider_count: 1,
            healthy_provider_count: 1,
            calls_per_url: HashMap::new(),
            errors_per_url: HashMap::new(),
            calls_per_method: HashMap::new(),
            errors_per_method: HashMap::new(),
            minute_buckets: Vec::new(),
            last_session: None,
        }
    }

    #[test]
    fn rpc_figures_are_absent_until_they_are_measured() {
        let none = rpc_header_info(None);
        assert_eq!(none.success_rate_percent, None);
        assert_eq!(none.calls_per_minute, None);

        let idle = rpc_header_info(Some(&stats(0, 0, 100.0)));
        assert_eq!(idle.success_rate_percent, None);
        assert_eq!(idle.calls_per_minute, Some(0.0));

        let measured = rpc_header_info(Some(&stats(200, 2, 99.0)));
        assert_eq!(measured.success_rate_percent, Some(99.0));
    }
}
