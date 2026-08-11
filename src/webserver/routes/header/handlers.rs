use axum::response::Json;

use crate::config::with_config;
use crate::connectivity::state::are_critical_endpoints_healthy;
use crate::filtering::global_store;
use crate::global::are_core_services_ready;
use crate::rpc::get_global_rpc_stats;
use crate::services::{get_service_manager, ServiceHealth};
use crate::wallet::{get_balance_at_time, get_wallet_worth};

use super::types::*;

pub(super) async fn get_header_metrics() -> Json<HeaderMetricsResponse> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return Json(crate::webserver::demo::get_demo_header_metrics());
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
    let store = global_store();
    let (today_stats, start_balance, filtering_stats, system) = tokio::join!(
        crate::positions::get_period_trading_stats(today_start, Some(now)),
        get_balance_at_time(today_start),
        store.stats_if_ready(),
        calculate_system_health(),
    );

    // The wallet's worth is served from the live in-memory snapshot, priced now — no
    // database read on this once-a-second endpoint, and the exact same figure the home
    // hero renders.
    let worth = get_wallet_worth();

    let preview = crate::global::is_preview_mode();
    let (trader_enabled, entry_enabled, exit_enabled) = with_config(|cfg| {
        (
            cfg.trader.enabled,
            cfg.trader.entry_monitor_enabled,
            cfg.trader.exit_monitor_enabled,
        )
    });
    let trader_state = if preview {
        TraderHeaderState::Preview
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

    let today_pnl_sol = today_stats
        .as_ref()
        .map(|stats| stats.net_pnl_sol)
        .unwrap_or_default();
    let start_balance_sol = start_balance.ok().flatten();
    let today_pnl_percent = start_balance_sol
        .filter(|balance| *balance > f64::EPSILON)
        .map(|balance| today_pnl_sol / balance * 100.0)
        .unwrap_or_default();

    let trader = TraderHeaderInfo {
        enabled: !preview && trader_enabled,
        state: trader_state,
        today_pnl_sol,
        today_pnl_percent,
    };

    let wallet = WalletHeaderInfo {
        sol_balance: worth.sol_balance,
        tokens_worth_sol: worth.tokens_worth_sol,
        total_equity_sol: worth.total_equity_sol,
        change_today_sol: start_balance_sol.map(|start| worth.total_equity_sol - start),
        change_today_percent: start_balance_sol
            .filter(|start| *start > f64::EPSILON)
            .map(|start| (worth.total_equity_sol - start) / start * 100.0),
        token_count: worth.token_count,
        last_updated: worth.updated_at.to_rfc3339(),
    };

    // RPC info
    let rpc = if let Some(stats) = get_global_rpc_stats() {
        let recent_cpm = stats.calls_per_minute_recent(5);
        let uptime_secs = chrono::Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds()
            .max(0) as u64;
        let fallback_cpm = (stats.total_calls() as f64 / uptime_secs.max(1) as f64) * 60.0;

        RpcHeaderInfo {
            success_rate_percent: stats.success_rate(),
            avg_latency_ms: stats.average_response_time_ms_global() as u64,
            calls_per_minute: if recent_cpm > 0.0 {
                recent_cpm
            } else {
                fallback_cpm
            },
            healthy: stats.success_rate() > 90.0,
        }
    } else {
        RpcHeaderInfo {
            success_rate_percent: 0.0,
            avg_latency_ms: 0,
            calls_per_minute: 0.0,
            healthy: false,
        }
    };

    let filtering = match filtering_stats {
        Some(stats) => FilteringHeaderInfo {
            monitoring_count: stats.total_tokens,
            passed_count: stats.passed_filtering,
            rejected_count: stats.total_tokens.saturating_sub(stats.passed_filtering),
            last_refresh: stats.updated_at.to_rfc3339(),
        },
        None => FilteringHeaderInfo {
            monitoring_count: 0,
            passed_count: 0,
            rejected_count: 0,
            last_refresh: now.to_rfc3339(),
        },
    };

    // SOL/USD price for the header price card.
    let sol = SolHeaderInfo {
        price_usd: crate::sol_price::get_sol_price(),
        change_24h_percent: crate::ohlcvs::sol_usd_chart::change_24h_percent(),
    };

    Json(HeaderMetricsResponse {
        trader,
        wallet,
        rpc,
        filtering,
        system,
        sol,
        timestamp: now.to_rfc3339(),
    })
}

async fn calculate_system_health() -> SystemHeaderInfo {
    let mut unhealthy_services = Vec::new();
    let mut critical_degraded = false;

    // In preview mode the wallet/RPC services are intentionally not running, so
    // "core services" being incomplete is expected and must not be flagged as a problem.
    let preview = crate::global::is_preview_mode();

    // Check core services readiness (skipped in preview mode)
    if !preview && !are_core_services_ready() {
        unhealthy_services.push("Core Services".to_owned());
        critical_degraded = true;
    }

    // Check critical endpoints
    if !are_critical_endpoints_healthy().await {
        unhealthy_services.push("Critical Endpoints".to_owned());
        critical_degraded = true;
    }

    // Check service manager health. Only enabled services can be "unhealthy" — a
    // disabled service (e.g. trading/pools in preview mode, or any service
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
