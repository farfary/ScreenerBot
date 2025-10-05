use axum::{
    extract::State,
    http::StatusCode,
    response::{ IntoResponse, Response },
    routing::get,
    Json,
    Router,
};
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;
use sysinfo::System;

use crate::{
    global::{
        are_core_services_ready,
        get_pending_services,
        POOL_SERVICE_READY,
        POSITIONS_SYSTEM_READY,
        SECURITY_ANALYZER_READY,
        TOKENS_SYSTEM_READY,
        TRANSACTIONS_SYSTEM_READY,
    },
    logger::{ log, LogTag },
    rpc::get_global_rpc_stats,
    webserver::{ state::AppState, utils::{ format_duration, success_response } },
};

// ================================================================================================
// Response Types
// ================================================================================================

/// Complete system status response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusResponse {
    pub timestamp: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub services: ServiceStatusResponse,
    pub metrics: SystemMetricsResponse,
    pub trading_enabled: bool,
}

/// Service readiness status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusResponse {
    pub tokens_system: ServiceState,
    pub positions_system: ServiceState,
    pub pool_service: ServiceState,
    pub security_analyzer: ServiceState,
    pub transactions_system: ServiceState,
    pub all_ready: bool,
}

/// Individual service state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub ready: bool,
    pub last_check: DateTime<Utc>,
    pub error: Option<String>,
}

/// System resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    // Backward-compat (kept for existing UI):
    pub memory_usage_mb: u64, // maps to system used memory
    pub cpu_usage_percent: f32, // maps to system CPU usage

    // Detailed metrics (new):
    pub system_memory_used_mb: u64,
    pub system_memory_total_mb: u64,
    pub process_memory_mb: u64,
    pub cpu_system_percent: f32,
    pub cpu_process_percent: f32,

    // Others
    pub active_threads: usize,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub rpc_success_rate: f32,
    pub ws_connections: usize,
}

/// Simple health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
}

// ================================================================================================
// Route Handlers
// ================================================================================================

/// Create status routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(system_status))
        .route("/status/services", get(service_status))
        .route("/status/metrics", get(system_metrics))
}

/// GET /api/health
/// Simple health check endpoint for load balancers and monitoring
async fn health_check() -> Response {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    success_response(response)
}

/// GET /api/status
/// Complete system status including services and metrics
async fn system_status(State(state): State<Arc<AppState>>) -> Response {
    log(LogTag::Webserver, "DEBUG", "Fetching complete system status");

    let uptime = state.uptime_seconds();
    let services = get_service_status_internal();
    let metrics = get_system_metrics_internal(&state).await;
    let trading_enabled = are_core_services_ready();

    let response = SystemStatusResponse {
        timestamp: Utc::now(),
        uptime_seconds: uptime,
        uptime_formatted: format_duration(uptime),
        services,
        metrics,
        trading_enabled,
    };

    success_response(response)
}

/// GET /api/status/services
/// Detailed service readiness status
async fn service_status() -> Response {
    let response = get_service_status_internal();
    success_response(response)
}

/// GET /api/status/metrics
/// System resource metrics
async fn system_metrics(State(state): State<Arc<AppState>>) -> Response {
    let response = get_system_metrics_internal(&state).await;
    success_response(response)
}

// ================================================================================================
// Internal Helper Functions
// ================================================================================================

/// Get service status from global flags
fn get_service_status_internal() -> ServiceStatusResponse {
    let now = Utc::now();

    let tokens_ready = TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
    let positions_ready = POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);
    let pool_ready = POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst);
    let security_ready = SECURITY_ANALYZER_READY.load(std::sync::atomic::Ordering::SeqCst);
    let transactions_ready = TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);

    let all_ready = are_core_services_ready();

    // Get pending services for error messages
    let pending = get_pending_services();
    let error_msg = if !pending.is_empty() {
        Some(format!("Waiting for: {}", pending.join(", ")))
    } else {
        None
    };

    ServiceStatusResponse {
        tokens_system: ServiceState {
            ready: tokens_ready,
            last_check: now,
            error: if !tokens_ready {
                error_msg.clone()
            } else {
                None
            },
        },
        positions_system: ServiceState {
            ready: positions_ready,
            last_check: now,
            error: if !positions_ready {
                error_msg.clone()
            } else {
                None
            },
        },
        pool_service: ServiceState {
            ready: pool_ready,
            last_check: now,
            error: if !pool_ready {
                error_msg.clone()
            } else {
                None
            },
        },
        security_analyzer: ServiceState {
            ready: security_ready,
            last_check: now,
            error: if !security_ready {
                error_msg.clone()
            } else {
                None
            },
        },
        transactions_system: ServiceState {
            ready: transactions_ready,
            last_check: now,
            error: if !transactions_ready {
                error_msg
            } else {
                None
            },
        },
        all_ready,
    }
}

/// Get system metrics
async fn get_system_metrics_internal(state: &AppState) -> SystemMetricsResponse {
    // Get RPC stats
    let rpc_stats = get_global_rpc_stats();
    let total_calls: u64 = rpc_stats
        .as_ref()
        .map(|s| s.calls_per_url.values().sum())
        .unwrap_or(0);
    let success_rate = 100.0; // RpcStats doesn't track failures separately

    // Get system info
    let mut sys = System::new_all();
    sys.refresh_all();

    // Global (system) CPU and memory
    let cpu_system_percent = sys.global_cpu_info().cpu_usage();
    let system_memory_total_mb = (sys.total_memory() / 1024 / 1024) as u64;
    let system_memory_used_mb = (sys.used_memory() / 1024 / 1024) as u64;

    // Current process info
    let pid = sysinfo::get_current_pid().ok();
    let (process_memory_mb, cpu_process_percent) = if let Some(pid) = pid {
        if let Some(process) = sys.process(pid) {
            let mem = (process.memory() / 1024 / 1024) as u64; // MB
            let cpu = process.cpu_usage();
            (mem, cpu)
        } else {
            (0, 0.0)
        }
    } else {
        (0, 0.0)
    };

    // Count active threads (approximate)
    let thread_count = std::thread
        ::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    // Get WebSocket connections
    let ws_connections = state.ws_connection_count().await;

    SystemMetricsResponse {
        // Backward-compat fields map to system-wide view
        memory_usage_mb: system_memory_used_mb,
        cpu_usage_percent: cpu_system_percent,

        // Detailed
        system_memory_used_mb,
        system_memory_total_mb,
        process_memory_mb,
        cpu_system_percent,
        cpu_process_percent,
        active_threads: thread_count,
        rpc_calls_total: total_calls,
        rpc_calls_failed: 0, // RpcStats doesn't track failures separately
        rpc_success_rate: success_rate,
        ws_connections,
    }
}
