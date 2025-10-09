use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use sysinfo::System;
use tokio::time::{interval, Duration, MissedTickBehavior};

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    global::{
        are_core_services_ready, get_pending_services, POOL_SERVICE_READY, POSITIONS_SYSTEM_READY,
        SECURITY_ANALYZER_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
    },
    logger::{log, LogTag},
    trader::is_trader_running,
    wallet::{get_current_wallet_status, get_snapshot_token_balances},
    webserver::{
        state::{get_app_state, AppState},
        utils::format_duration,
        ws::{hub::WsHub, topics},
    },
};

use crate::rpc::get_global_rpc_stats;

const MAX_WALLET_TOKENS: usize = 128;

#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub trading_enabled: bool,
    pub trader_mode: String,
    pub trader_running: bool,
    pub open_positions: usize,
    pub closed_positions_today: usize,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub services: ServiceStatusSnapshot,
    pub metrics: SystemMetricsSnapshot,
    pub rpc_stats: Option<RpcStatsSnapshot>,
    pub wallet: Option<WalletStatusSnapshot>,
    pub ohlcv_stats: Option<OhlcvStatsSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStatusSnapshot {
    pub tokens_system: ServiceStateSnapshot,
    pub positions_system: ServiceStateSnapshot,
    pub pool_service: ServiceStateSnapshot,
    pub security_analyzer: ServiceStateSnapshot,
    pub transactions_system: ServiceStateSnapshot,
    pub all_ready: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceStateSnapshot {
    pub ready: bool,
    pub status: String,
    pub last_check: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemMetricsSnapshot {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub system_memory_used_mb: u64,
    pub system_memory_total_mb: u64,
    pub process_memory_mb: u64,
    pub cpu_system_percent: f32,
    pub cpu_process_percent: f32,
    pub active_threads: usize,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub rpc_success_rate: f32,
    pub ws_connections: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletStatusSnapshot {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub usdc_balance: f64,
    pub total_tokens_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_time: Option<DateTime<Utc>>,
    pub token_balances: Vec<WalletTokenBalanceSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WalletTokenBalanceSnapshot {
    pub mint: String,
    pub balance: u64,
    pub balance_ui: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
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
    let trading_enabled = config::with_config(|cfg| cfg.trader.enabled);
    let trader_mode = "Normal".to_string();
    let trader_running = is_trader_running();

    let open_positions = crate::positions::db::get_open_positions()
        .await
        .map(|positions| positions.len())
        .unwrap_or(0);
    let closed_positions_today = 0;

    let app_state = get_app_state().await;
    let uptime_seconds = app_state
        .as_ref()
        .map(|state| state.uptime_seconds())
        .unwrap_or(0);
    let uptime_formatted = format_duration(uptime_seconds);

    let rpc_stats_raw = get_global_rpc_stats();

    let services = collect_service_status_snapshot();
    let metrics = collect_system_metrics_snapshot(app_state.as_ref(), rpc_stats_raw.as_ref()).await;

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
    });

    let wallet = collect_wallet_snapshot().await;
    let sol_balance = wallet.as_ref().map(|w| w.sol_balance).unwrap_or(0.0);
    let usdc_balance = wallet.as_ref().map(|w| w.usdc_balance).unwrap_or(0.0);

    StatusSnapshot {
        timestamp: Utc::now(),
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
        ohlcv_stats: None,
    }
}

pub fn start(hub: Arc<WsHub>) {
    tokio::spawn(run(hub));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "ws.sources.status started");
    }
}

async fn run(hub: Arc<WsHub>) {
    const TOPIC: &str = "system.status";

    loop {
        hub.wait_for_subscribers(TOPIC).await;

        let active = hub.topic_subscriber_count(TOPIC).await;
        log(
            LogTag::Webserver,
            "INFO",
            &format!(
                "ws.sources.status streaming activated (subscribers={})",
                active
            ),
        );

        publish_snapshot(&hub).await;

        let mut ticker = interval(Duration::from_secs(10));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            ticker.tick().await;

            if !hub.has_subscribers(TOPIC).await {
                let remaining = hub.topic_subscriber_count(TOPIC).await;
                log(
                    LogTag::Webserver,
                    "INFO",
                    &format!(
                        "ws.sources.status streaming paused (subscribers={})",
                        remaining
                    ),
                );
                break;
            }

            publish_snapshot(&hub).await;
        }
    }
}

async fn publish_snapshot(hub: &Arc<WsHub>) {
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
                snapshot.open_positions, snapshot.metrics.ws_connections
            ),
        );
    }
}

fn collect_service_status_snapshot() -> ServiceStatusSnapshot {
    let now = Utc::now();
    let all_ready = are_core_services_ready();
    let pending = get_pending_services();
    let pending_message = if pending.is_empty() {
        None
    } else {
        Some(format!("Waiting for: {}", pending.join(", ")))
    };

    let tokens_ready = TOKENS_SYSTEM_READY.load(Ordering::SeqCst);
    let positions_ready = POSITIONS_SYSTEM_READY.load(Ordering::SeqCst);
    let pool_ready = POOL_SERVICE_READY.load(Ordering::SeqCst);
    let security_ready = SECURITY_ANALYZER_READY.load(Ordering::SeqCst);
    let transactions_ready = TRANSACTIONS_SYSTEM_READY.load(Ordering::SeqCst);

    ServiceStatusSnapshot {
        tokens_system: ServiceStateSnapshot::new(tokens_ready, now, pending_message.clone()),
        positions_system: ServiceStateSnapshot::new(positions_ready, now, pending_message.clone()),
        pool_service: ServiceStateSnapshot::new(pool_ready, now, pending_message.clone()),
        security_analyzer: ServiceStateSnapshot::new(security_ready, now, pending_message.clone()),
        transactions_system: ServiceStateSnapshot::new(transactions_ready, now, pending_message),
        all_ready,
    }
}

impl ServiceStateSnapshot {
    fn new(ready: bool, last_check: DateTime<Utc>, error: Option<String>) -> Self {
        let status = if ready { "healthy" } else { "starting" }.to_string();
        let error = if ready { None } else { error };
        Self {
            ready,
            status,
            last_check,
            error,
        }
    }
}

async fn collect_system_metrics_snapshot(
    app_state: Option<&Arc<AppState>>,
    rpc_stats: Option<&crate::rpc::RpcStats>,
) -> SystemMetricsSnapshot {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_system_percent = sys.global_cpu_info().cpu_usage();
    let system_memory_total_mb = (sys.total_memory() / 1024 / 1024) as u64;
    let system_memory_used_mb = (sys.used_memory() / 1024 / 1024) as u64;

    let (process_memory_mb, cpu_process_percent) = match sysinfo::get_current_pid() {
        Ok(pid) => match sys.process(pid) {
            Some(process) => ((process.memory() / 1024 / 1024) as u64, process.cpu_usage()),
            None => (0, 0.0),
        },
        Err(_) => (0, 0.0),
    };

    let thread_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let ws_connections = if let Some(state) = app_state {
        state.ws_connection_count().await
    } else {
        0
    };

    let (rpc_calls_total, rpc_calls_failed, rpc_success_rate) = if let Some(stats) = rpc_stats {
        (
            stats.total_calls(),
            stats.total_errors(),
            stats.success_rate(),
        )
    } else {
        (0, 0, 100.0)
    };

    SystemMetricsSnapshot {
        memory_usage_mb: system_memory_used_mb,
        cpu_usage_percent: cpu_system_percent,
        system_memory_used_mb,
        system_memory_total_mb,
        process_memory_mb,
        cpu_system_percent,
        cpu_process_percent,
        active_threads: thread_count,
        rpc_calls_total,
        rpc_calls_failed,
        rpc_success_rate,
        ws_connections,
    }
}

async fn collect_wallet_snapshot() -> Option<WalletStatusSnapshot> {
    match get_current_wallet_status().await {
        Ok(Some(snapshot)) => {
            let mut token_balances = Vec::new();

            if let Some(id) = snapshot.id {
                match get_snapshot_token_balances(id).await {
                    Ok(tokens) => {
                        token_balances = tokens
                            .into_iter()
                            .take(MAX_WALLET_TOKENS)
                            .map(|token| WalletTokenBalanceSnapshot {
                                mint: token.mint,
                                balance: token.balance,
                                balance_ui: token.balance_ui,
                                decimals: token.decimals,
                                is_token_2022: token.is_token_2022,
                            })
                            .collect();
                    }
                    Err(err) => {
                        log(
                            LogTag::Webserver,
                            "WARN",
                            &format!("Failed to load wallet token balances: {}", err),
                        );
                    }
                }
            }

            Some(WalletStatusSnapshot {
                sol_balance: snapshot.sol_balance,
                sol_balance_lamports: snapshot.sol_balance_lamports,
                usdc_balance: 0.0,
                total_tokens_count: snapshot.total_tokens_count,
                snapshot_time: Some(snapshot.snapshot_time),
                token_balances,
            })
        }
        Ok(None) => None,
        Err(err) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load current wallet snapshot: {}", err),
            );
            None
        }
    }
}
