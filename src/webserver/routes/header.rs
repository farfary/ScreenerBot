use axum::{response::Json, routing::get, Router};
use serde::Serialize;
use std::sync::Arc;

use crate::config::with_config;
use crate::connectivity::state::are_critical_endpoints_healthy;
use crate::filtering::global_store;
use crate::global::are_core_services_ready;
use crate::positions::state::{get_open_positions, get_position_by_mint};
use crate::rpc::get_global_rpc_stats;
use crate::services::{get_service_manager, ServiceHealth};
use crate::trader::is_trader_running;
use crate::wallet::get_current_wallet_status;
use crate::webserver::state::AppState;

#[derive(Debug, Serialize)]
pub struct HeaderMetricsResponse {
    pub trader: TraderHeaderInfo,
    pub wallet: WalletHeaderInfo,
    pub positions: PositionsHeaderInfo,
    pub rpc: RpcHeaderInfo,
    pub filtering: FilteringHeaderInfo,
    pub system: SystemHeaderInfo,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct TraderHeaderInfo {
    pub running: bool,
    pub enabled: bool,
    pub today_pnl_sol: f64,
    pub today_pnl_percent: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct WalletHeaderInfo {
    pub sol_balance: f64,
    pub change_24h_sol: f64,
    pub change_24h_percent: f64,
    pub token_count: usize,
    pub tokens_worth_sol: f64,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct PositionsHeaderInfo {
    pub open_count: i64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
    pub total_invested_sol: f64,
}

#[derive(Debug, Serialize)]
pub struct RpcHeaderInfo {
    pub success_rate_percent: f32,
    pub avg_latency_ms: u64,
    pub calls_per_minute: f64,
    pub healthy: bool,
}

#[derive(Debug, Serialize)]
pub struct FilteringHeaderInfo {
    pub monitoring_count: usize,
    pub passed_count: usize,
    pub rejected_count: usize,
    pub last_refresh: String,
}

#[derive(Debug, Serialize)]
pub struct SystemHeaderInfo {
    pub all_services_healthy: bool,
    pub unhealthy_services: Vec<String>,
    pub critical_degraded: bool,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/header/metrics", get(get_header_metrics))
}

async fn get_header_metrics() -> Json<HeaderMetricsResponse> {
    // Return demo data if demo mode is enabled
    if crate::webserver::demo::is_demo_mode() {
        return Json(crate::webserver::demo::get_demo_header_metrics());
    }

    let now = chrono::Utc::now();

    // Trader info
    let trader_enabled = with_config(|cfg| cfg.trader.enabled);
    let trader_running = is_trader_running();

    // Calculate today's P&L from positions
    let (today_pnl_sol, today_pnl_percent) = calculate_today_pnl().await;

    // Trader uptime (simplified - would need actual start time tracking)
    let uptime_seconds = if trader_running {
        // TODO: Track actual start time - for now use placeholder
        0
    } else {
        0
    };

    let trader = TraderHeaderInfo {
        running: trader_running,
        enabled: trader_enabled,
        today_pnl_sol,
        today_pnl_percent,
        uptime_seconds,
    };

    // Wallet info
    let wallet = if let Ok(Some(snapshot)) = get_current_wallet_status().await {
        // Calculate 24h change (would need historical data - placeholder for now)
        let change_24h_sol = 0.0; // TODO: Implement 24h delta calculation
        let change_24h_percent = 0.0;

        // Calculate total token worth (simplified)
        let tokens_worth_sol = snapshot.token_balances.len() as f64 * 0.1; // Placeholder

        WalletHeaderInfo {
            sol_balance: snapshot.sol_balance,
            change_24h_sol,
            change_24h_percent,
            token_count: snapshot.token_balances.len(),
            tokens_worth_sol,
            last_updated: snapshot.snapshot_time.to_rfc3339(),
        }
    } else {
        WalletHeaderInfo {
            sol_balance: 0.0,
            change_24h_sol: 0.0,
            change_24h_percent: 0.0,
            token_count: 0,
            tokens_worth_sol: 0.0,
            last_updated: now.to_rfc3339(),
        }
    };

    // Positions info
    let positions_info = calculate_positions_info().await;

    // RPC info
    let rpc = if let Some(stats) = get_global_rpc_stats() {
        let recent_cpm = stats.calls_per_minute_recent(5);
        let uptime_secs = chrono::Utc::now()
            .signed_duration_since(stats.startup_time)
            .num_seconds() as u64;
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

    // Filtering info
    let filtering = {
        let store = global_store();
        match store.get_stats().await {
            Ok(stats) => FilteringHeaderInfo {
                monitoring_count: stats.total_tokens,
                passed_count: stats.passed_filtering,
                rejected_count: stats.total_tokens.saturating_sub(stats.passed_filtering),
                last_refresh: stats.updated_at.to_rfc3339(),
            },
            Err(_) => FilteringHeaderInfo {
                monitoring_count: 0,
                passed_count: 0,
                rejected_count: 0,
                last_refresh: now.to_rfc3339(),
            },
        }
    };

    // System info
    let system = calculate_system_health().await;

    Json(HeaderMetricsResponse {
        trader,
        wallet,
        positions: positions_info,
        rpc,
        filtering,
        system,
        timestamp: now.to_rfc3339(),
    })
}

async fn calculate_today_pnl() -> (f64, f64) {
    // Get all closed positions from today
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| chrono::NaiveDateTime::default());
    let today_start_ts = today_start.and_utc().timestamp();

    // Get all closed positions from state and filter for today
    let all_positions = get_open_positions().await;

    // For now, return 0 until we implement proper P&L tracking
    // TODO: Implement today's P&L calculation from closed positions
    (0.0, 0.0)
}

async fn calculate_positions_info() -> PositionsHeaderInfo {
    let open_positions = get_open_positions().await;
    let open_count = open_positions.len() as i64;

    let total_invested_sol: f64 = open_positions.iter().map(|p| p.total_size_sol).sum();

    let unrealized_pnl_sol: f64 = open_positions.iter().filter_map(|p| p.unrealized_pnl).sum();

    let unrealized_pnl_percent = if total_invested_sol > 0.0 {
        (unrealized_pnl_sol / total_invested_sol) * 100.0
    } else {
        0.0
    };

    PositionsHeaderInfo {
        open_count,
        unrealized_pnl_sol,
        unrealized_pnl_percent,
        total_invested_sol,
    }
}

async fn calculate_system_health() -> SystemHeaderInfo {
    let mut unhealthy_services = Vec::new();
    let mut critical_degraded = false;

    // Check core services readiness
    if !are_core_services_ready() {
        unhealthy_services.push("Core Services".to_string());
        critical_degraded = true;
    }

    // Check critical endpoints
    if !are_critical_endpoints_healthy().await {
        unhealthy_services.push("Critical Endpoints".to_string());
        critical_degraded = true;
    }

    // Check service manager health
    if let Some(manager_arc) = get_service_manager().await {
        let manager = manager_arc.read().await;
        if let Some(manager) = &*manager {
            let health_map = manager.get_health().await;
            for (name, health) in health_map {
                if health != ServiceHealth::Healthy {
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
