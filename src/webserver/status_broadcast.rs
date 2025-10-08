use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
};

// Broadcast channel capacity
const STATUS_BROADCAST_CAPACITY: usize = 256;

// Global broadcaster
static STATUS_BROADCAST_TX: OnceCell<broadcast::Sender<StatusSnapshot>> = OnceCell::new();

/// OHLCV statistics snapshot
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

/// RPC statistics snapshot with detailed metrics
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

/// Status snapshot for WebSocket broadcasting
#[derive(Clone, Debug, Serialize)]
pub struct StatusSnapshot {
    pub trading_enabled: bool,
    pub trader_mode: String,
    pub open_positions: usize,
    pub closed_positions_today: usize,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub services: HashMap<String, String>, // service_name -> health_status
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub rpc_requests_total: u64,
    pub rpc_errors_total: u64,
    pub ws_connections: usize,
    pub ohlcv_stats: Option<OhlcvStatsSnapshot>,
    pub rpc_stats: Option<RpcStatsSnapshot>,
    pub timestamp: DateTime<Utc>,
}

/// Initialize the status broadcast system
/// Returns the receiver for the first subscriber (dropped if not used)
pub fn initialize_status_broadcaster() -> broadcast::Receiver<StatusSnapshot> {
    let (tx, rx) = broadcast::channel(STATUS_BROADCAST_CAPACITY);

    match STATUS_BROADCAST_TX.set(tx) {
        Ok(_) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Status broadcaster initialized (capacity: {})",
                        STATUS_BROADCAST_CAPACITY
                    ),
                );
            }
            rx
        }
        Err(_) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    "Status broadcaster already initialized, returning new subscription",
                );
            }
            // Return a new subscription if already initialized
            STATUS_BROADCAST_TX
                .get()
                .expect("Broadcaster exists")
                .subscribe()
        }
    }
}

/// Subscribe to status updates
/// Returns None if broadcaster not initialized
pub fn subscribe() -> Option<broadcast::Receiver<StatusSnapshot>> {
    STATUS_BROADCAST_TX.get().map(|tx| tx.subscribe())
}

/// Start the periodic status broadcaster task
/// Broadcasts status snapshot every `interval_secs` seconds
pub fn start_status_broadcaster(interval_secs: u64) -> tokio::task::JoinHandle<()> {
    // Ensure broadcaster is initialized
    if STATUS_BROADCAST_TX.get().is_none() {
        initialize_status_broadcaster();
    }

    let tx = STATUS_BROADCAST_TX
        .get()
        .expect("Status broadcaster initialized")
        .clone();

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Starting status broadcaster task (interval: {}s)",
                interval_secs
            ),
        );
    }

    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        let mut tick_count: u64 = 0;

        loop {
            ticker.tick().await;
            tick_count += 1;

            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Status broadcaster tick #{}", tick_count),
                );
            }

            // Gather status data from various sources
            let snapshot = gather_status_snapshot().await;

            // Broadcast to all subscribers
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Broadcasting status snapshot (services={}, ws_connections={}, rpc_total={})",
                        snapshot.services.len(),
                        snapshot.ws_connections,
                        snapshot.rpc_requests_total
                    )
                );
            }

            if let Err(error) = tx.send(snapshot) {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "DEBUG",
                        &format!(
                            "No active listeners for status snapshot broadcast: {}",
                            error
                        ),
                    );
                }
            }
        }
    })
}

/// Gather current system status from all sources
async fn gather_status_snapshot() -> StatusSnapshot {
    use crate::positions::{get_closed_positions, get_open_positions};
    use crate::services::get_service_manager;

    // Trading state (placeholder - will be updated when these functions are available)
    let trading_enabled = false; // TODO: Wire to actual trader state
    let trader_mode = "unknown".to_string();

    // Positions
    let open_positions = get_open_positions().await.len();

    let closed_positions_today = {
        let today = Utc::now().date_naive();
        get_closed_positions()
            .await
            .iter()
            .filter(|p| {
                p.exit_time
                    .map(|t| t.date_naive() == today)
                    .unwrap_or(false)
            })
            .count()
    };

    // Balances - get from wallet module
    let sol_balance = crate::wallet::get_current_wallet_status()
        .await
        .ok()
        .flatten()
        .map(|s| s.sol_balance)
        .unwrap_or(0.0);
    let usdc_balance = 0.0; // TODO: Wire USDC balance when available

    // Services health
    let mut services = HashMap::new();

    if let Some(mgr_ref) = get_service_manager().await {
        if let Some(mgr) = mgr_ref.read().await.as_ref() {
            let all_health = mgr.get_health().await;
            for (name, health) in all_health {
                services.insert(name.to_string(), format!("{:?}", health));
            }
        }
    }

    // System metrics
    let (cpu_percent, memory_mb) = get_system_metrics();

    // WebSocket connections
    let ws_connections = crate::webserver::state::get_ws_connection_count().await;

    // OHLCV stats - using OhlcvMetrics which has the basic stats
    // Note: Priority breakdown (critical/high/medium/low) requires MonitorStats
    // which is not directly accessible. We use tokens_monitored as total.
    let metrics = crate::ohlcvs::get_metrics().await;
    let ohlcv_stats = Some(OhlcvStatsSnapshot {
        total_tokens: metrics.tokens_monitored,
        critical_tokens: 0, // TODO: Access monitor stats for priority breakdown
        high_tokens: 0,
        medium_tokens: 0,
        low_tokens: metrics.tokens_monitored, // All tokens shown as low priority for now
        cache_hit_rate: metrics.cache_hit_rate,
        api_calls_per_minute: metrics.api_calls_per_minute,
        queue_size: 0, // Not available in OhlcvMetrics
    });

    // RPC stats - comprehensive metrics from global RPC client
    let rpc_stats = crate::rpc::get_global_rpc_stats().map(|stats| {
        let uptime_seconds = Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds();

        RpcStatsSnapshot {
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
        }
    });

    let snapshot = StatusSnapshot {
        trading_enabled,
        trader_mode,
        open_positions,
        closed_positions_today,
        sol_balance,
        usdc_balance,
        services,
        cpu_percent,
        memory_mb,
        rpc_requests_total: rpc_stats.as_ref().map(|s| s.total_calls).unwrap_or(0),
        rpc_errors_total: rpc_stats.as_ref().map(|s| s.total_errors).unwrap_or(0),
        ws_connections,
        ohlcv_stats,
        rpc_stats,
        timestamp: Utc::now(),
    };

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Status snapshot collected: open_positions={} closed_today={} ws_connections={} rpc_total={} cpu={:.2}% mem={}MB",
                snapshot.open_positions,
                snapshot.closed_positions_today,
                snapshot.ws_connections,
                snapshot.rpc_requests_total,
                snapshot.cpu_percent,
                snapshot.memory_mb
            )
        );
    }

    snapshot
}

/// Get current system metrics (CPU, memory)
fn get_system_metrics() -> (f32, u64) {
    use sysinfo::{Pid, System};

    let mut sys = System::new_all();
    sys.refresh_all();

    let pid = Pid::from_u32(std::process::id());

    if let Some(process) = sys.process(pid) {
        let cpu = process.cpu_usage();
        let memory = process.memory() / 1024; // Convert to MB
        (cpu, memory)
    } else {
        (0.0, 0)
    }
}

/// Get broadcast statistics (subscriber count)
pub fn get_subscriber_count() -> usize {
    STATUS_BROADCAST_TX
        .get()
        .map(|tx| tx.receiver_count())
        .unwrap_or(0)
}
