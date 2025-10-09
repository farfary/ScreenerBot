use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    logger::{log, LogTag},
    webserver::ws::{hub::WsHub, topics},
};

// ============================================================================
// STATUS SNAPSHOT TYPES (moved from snapshots.rs)
// ============================================================================

#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub trading_enabled: bool,
    pub trader_mode: String,
    pub open_positions: usize,
    pub closed_positions_today: usize,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub services: HashMap<String, String>,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub rpc_requests_total: u64,
    pub rpc_errors_total: u64,
    pub ws_connections: usize,
    pub ohlcv_stats: Option<OhlcvStatsSnapshot>,
    pub rpc_stats: Option<RpcStatsSnapshot>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OhlcvStatsSnapshot {
    pub total_tokens: usize,
    pub critical_tokens: usize,
    pub high_tokens: usize,
    pub medium_tokens: usize,
    pub low_tokens: usize,
    pub cache_hit_rate: f64,
    pub api_calls_per_minute: f64,
    pub queue_size: usize,
}

#[derive(Clone, Debug, Serialize, serde::Deserialize)]
pub struct RpcStatsSnapshot {
    pub total_calls: u64,
    pub total_errors: u64,
    pub success_rate: f32,
    pub calls_per_second: f64,
    pub average_response_time_ms: f64,
    pub calls_per_url: HashMap<String, u64>,
    pub errors_per_url: HashMap<String, u64>,
    pub calls_per_method: HashMap<String, u64>,
    pub errors_per_method: HashMap<String, u64>,
    pub uptime_seconds: i64,
}

/// Gather current status snapshot (aggregates data from multiple sources)
pub async fn gather_status_snapshot() -> StatusSnapshot {
    // Trader state
    let trading_enabled = config::with_config(|cfg| cfg.trader.enabled);
    let trader_mode = "Normal".to_string(); // Simplified for now

    // Positions (count from DB)
    let open_positions = crate::positions::db::get_open_positions()
        .await
        .map(|positions| positions.len())
        .unwrap_or(0);
    let closed_positions_today = 0; // TODO: implement count_closed_positions_today

    // Wallet balances
    let (sol_balance, usdc_balance) = (0.0, 0.0); // TODO: get from wallet module

    // Services health
    let services = if let Some(manager_ref) = crate::services::get_service_manager().await {
        if let Some(manager) = manager_ref.read().await.as_ref() {
            let health_map = manager.get_health().await;
            health_map
                .into_iter()
                .map(|(name, health)| {
                    let status = match health {
                        crate::services::ServiceHealth::Healthy => "healthy".to_string(),
                        crate::services::ServiceHealth::Degraded(reason) => {
                            format!("degraded: {}", reason)
                        }
                        crate::services::ServiceHealth::Unhealthy(reason) => {
                            format!("unhealthy: {}", reason)
                        }
                        crate::services::ServiceHealth::Starting => "starting".to_string(),
                        crate::services::ServiceHealth::Stopping => "stopping".to_string(),
                    };
                    (name.to_string(), status)
                })
                .collect()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    // System metrics (from sysinfo)
    let sys = sysinfo::System::new_all();
    let cpu_percent = sys.global_cpu_info().cpu_usage();
    let memory_mb = (sys.used_memory() / 1024 / 1024) as u64;

    // RPC stats (from global stats)
    let rpc_stats_opt = crate::rpc::get_global_rpc_stats();
    let (rpc_requests_total, rpc_errors_total, rpc_stats) = if let Some(stats) = rpc_stats_opt {
        let uptime_seconds = Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds();
        let rpc_snapshot = RpcStatsSnapshot {
            total_calls: stats.total_calls(),
            total_errors: stats.total_errors(),
            success_rate: stats.success_rate(),
            calls_per_second: stats.calls_per_second(),
            average_response_time_ms: stats.average_response_time_ms_global(),
            calls_per_url: stats.calls_per_url.clone(),
            errors_per_url: stats.errors_per_url.clone(),
            calls_per_method: stats.calls_per_method.clone(),
            errors_per_method: stats.errors_per_method.clone(),
            uptime_seconds,
        };
        (
            stats.total_calls(),
            stats.total_errors(),
            Some(rpc_snapshot),
        )
    } else {
        (0, 0, None)
    };

    // OHLCV stats (not critical yet)
    let ohlcv_stats = None;

    // WebSocket connections (from hub)
    let ws_connections = if let Some(state) = crate::webserver::state::get_app_state().await {
        state.ws_hub().active_connections().await
    } else {
        0
    };

    StatusSnapshot {
        trading_enabled,
        trader_mode,
        open_positions,
        closed_positions_today,
        sol_balance,
        usdc_balance,
        services,
        cpu_percent,
        memory_mb,
        rpc_requests_total,
        rpc_errors_total,
        ws_connections,
        ohlcv_stats,
        rpc_stats,
        timestamp: Utc::now(),
    }
}

pub fn start(hub: Arc<WsHub>) {
    tokio::spawn(run(hub));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "ws.sources.status started");
    }
}

async fn run(hub: Arc<WsHub>) {
    // Phase 1 cleanup: slow cadence to 10s until Phase 2 demand-gating is wired.
    // TODO(Phase 2): restore dynamic cadence with explicit subscription tracking instead of fixed sleep.
    let mut ticker = interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        let snapshot = gather_status_snapshot().await;
        let seq = hub.next_seq("system.status").await;
        let envelope = topics::status::status_to_envelope(&snapshot, seq);
        hub.broadcast(envelope).await;
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "ws.sources.status snapshot: positions={}, ws_connections={}",
                    snapshot.open_positions, snapshot.ws_connections
                ),
            );
        }
    }
}
