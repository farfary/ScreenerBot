//! Real-time system state snapshot assembly for the dashboard.
mod collectors;
mod types;

use chrono::Utc;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::time::Instant;

use crate::{
    config,
    rpc::get_global_rpc_stats,
    trader::is_trader_running,
    webserver::{state::get_app_state, utils::format_duration},
};

// Re-export public types
pub use types::*;

// Re-export public collector functions
pub use collectors::{collect_service_status_snapshot, get_cached_system_metrics};

const MAX_WALLET_TOKENS: usize = 128;
const MAX_PENDING_QUEUE_SAMPLE: usize = 10;

/// Cache duration for system metrics (expensive sysinfo calls)
const SYSTEM_METRICS_CACHE_SECS: u64 = 5;

/// Cached system metrics to avoid expensive sysinfo calls on every request
struct CachedSystemMetrics {
    metrics: SystemMetricsSnapshot,
    last_updated: Instant,
}

static SYSTEM_METRICS_CACHE: LazyLock<RwLock<Option<CachedSystemMetrics>>> =
    LazyLock::new(|| RwLock::new(None));

/// Gather current status snapshot (aggregates data from multiple sources)
pub async fn gather_status_snapshot() -> StatusSnapshot {
    // In Explore Mode trading is disabled outright — report it as off regardless
    // of the persisted config value so the status bar reflects reality.
    let explore = crate::global::is_explore_mode();
    let trading_enabled = !explore && config::with_config(|cfg| cfg.trader.enabled);
    let trader_mode = "Normal".to_owned();
    let trader_running = !explore && is_trader_running();

    let day_start_naive = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| Utc::now().naive_utc());
    let day_start = chrono::DateTime::<Utc>::from_naive_utc_and_offset(day_start_naive, Utc);

    let (open_positions_result, closed_positions_result) = tokio::join!(
        crate::positions::db::get_open_positions(),
        crate::positions::get_db_closed_positions_count_since(day_start),
    );

    let open_positions = open_positions_result
        .map(|positions| positions.len())
        .unwrap_or_default();
    let closed_positions_today = closed_positions_result
        .map(|count| count.max(0) as usize)
        .unwrap_or_default();

    let app_state = get_app_state().await;
    let uptime_seconds = app_state
        .as_ref()
        .map(|state| state.uptime_seconds())
        .unwrap_or_default();
    let uptime_formatted = format_duration(uptime_seconds);

    let rpc_stats_raw = get_global_rpc_stats();
    let rpc_metrics_summary = rpc_stats_raw.as_ref().map(types::RpcMetricsSummary::from);

    let services = collectors::collect_service_status_snapshot();

    let (
        metrics,
        wallet,
        ohlcv_stats,
        pools,
        discovery,
        events,
        transactions,
        dexscreener,
        geckoterminal,
    ) = tokio::join!(
        collectors::collect_system_metrics_snapshot(rpc_metrics_summary),
        collectors::collect_wallet_snapshot(),
        collectors::collect_ohlcv_stats_snapshot(),
        async { collectors::collect_pool_service_snapshot() },
        collectors::collect_token_discovery_snapshot(),
        collectors::collect_events_snapshot(),
        collectors::collect_transactions_snapshot(),
        collectors::collect_dexscreener_status_snapshot(),
        collectors::collect_gecko_terminal_status_snapshot(),
    );

    let rpc_stats = rpc_stats_raw.as_ref().map(|stats| RpcStatsSnapshot {
        total_calls: stats.total_calls(),
        total_errors: stats.total_errors(),
        success_rate: stats.success_rate(),
        calls_per_second: stats.calls_per_second(),
        average_response_time_ms: stats.average_response_time_ms_global(),
        calls_per_url: stats.calls_per_url.clone(),
        errors_per_url: stats.errors_per_url.clone(),
        calls_per_method: stats.calls_per_method.clone(),
        errors_per_method: stats.errors_per_method.clone(),
        uptime_seconds: Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds(),
        session_id: stats.session_id.clone(),
        session_started_at: stats.startup_time,
        recent_calls_per_minute: stats.calls_per_minute_recent(5),
        minute_buckets: stats.get_minute_buckets(),
        last_session: stats.last_session.clone(),
    });

    let sol_balance = wallet.as_ref().map(|w| w.sol_balance).unwrap_or_default();
    let usdc_balance = wallet.as_ref().map(|w| w.usdc_balance).unwrap_or_default();

    StatusSnapshot {
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        uptime_formatted,
        trading_enabled,
        trader_mode,
        trader_running,
        open_positions,
        closed_positions_today,
        sol_balance,
        usdc_balance,
        services,
        metrics,
        rpc_stats,
        wallet,
        ohlcv_stats,
        pools,
        discovery,
        events,
        transactions,
        dexscreener,
        geckoterminal,
    }
}
