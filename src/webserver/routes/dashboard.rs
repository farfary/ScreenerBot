use axum::{extract::State, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::global::{
    POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, SECURITY_ANALYZER_READY, TOKENS_SYSTEM_READY,
    TRANSACTIONS_SYSTEM_READY,
};
use crate::positions;
use crate::rpc::get_global_rpc_stats;
use crate::tokens::cleanup::get_blacklist_summary;
use crate::tokens::database::get_global_database;
use crate::wallet::get_current_wallet_status;
use crate::webserver::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardOverview {
    pub wallet: WalletInfo,
    pub positions: PositionsSummary,
    pub system: SystemInfo,
    pub rpc: RpcInfo,
    pub blacklist: BlacklistInfo,
    pub monitoring: MonitoringInfo,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: usize,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionsSummary {
    pub total_positions: i64,
    pub open_positions: i64,
    pub closed_positions: i64,
    pub total_invested_sol: f64,
    pub total_pnl: f64,
    pub win_rate: f64,
    pub open_position_details: Vec<OpenPositionDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenPositionDetail {
    pub mint: String,
    pub symbol: String,
    pub entry_price: f64,
    pub current_price: Option<f64>,
    pub pnl_percent: Option<f64>,
    pub hold_duration_minutes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub all_services_ready: bool,
    pub services: ServiceStatus,
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub active_threads: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub tokens_system: bool,
    pub positions_system: bool,
    pub pool_service: bool,
    pub security_analyzer: bool,
    pub transactions_system: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcInfo {
    pub total_calls: u64,
    pub calls_per_second: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlacklistInfo {
    pub total_blacklisted: usize,
    pub by_reason: std::collections::HashMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitoringInfo {
    pub tokens_tracked: usize,
    pub entry_check_interval_secs: u64,
    pub position_monitor_interval_secs: u64,
}

/// Create dashboard routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/dashboard/overview", get(get_dashboard_overview))
}

/// Get comprehensive dashboard overview
async fn get_dashboard_overview(State(state): State<Arc<AppState>>) -> Json<DashboardOverview> {
    // Get wallet info
    let wallet_info = match get_current_wallet_status().await {
        Ok(Some(snapshot)) => WalletInfo {
            sol_balance: snapshot.sol_balance,
            sol_balance_lamports: snapshot.sol_balance_lamports,
            total_tokens_count: snapshot.total_tokens_count as usize,
            last_updated: Some(snapshot.snapshot_time.to_rfc3339()),
        },
        _ => WalletInfo {
            sol_balance: 0.0,
            sol_balance_lamports: 0,
            total_tokens_count: 0,
            last_updated: None,
        },
    };

    // Get positions summary
    let open_positions = positions::get_db_open_positions().await.unwrap_or_default();
    let closed_positions = positions::get_db_closed_positions()
        .await
        .unwrap_or_default();

    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();

    let total_pnl: f64 = closed_positions
        .iter()
        .filter_map(|p| {
            if let (Some(sol_received), entry_size) = (p.sol_received, p.entry_size_sol) {
                Some(sol_received - entry_size)
            } else {
                None
            }
        })
        .sum();

    // Calculate win rate
    let win_rate = if closed_positions.len() > 0 {
        let profitable = closed_positions
            .iter()
            .filter(|p| {
                if let (Some(entry), Some(exit)) = (p.effective_entry_price, p.effective_exit_price)
                {
                    exit > entry
                } else {
                    false
                }
            })
            .count();
        ((profitable as f64) / (closed_positions.len() as f64)) * 100.0
    } else {
        0.0
    };

    // Get open position details
    let open_position_details: Vec<OpenPositionDetail> = open_positions
        .iter()
        .map(|p| {
            let hold_duration = chrono::Utc::now()
                .signed_duration_since(p.entry_time)
                .num_minutes();

            let pnl_percent = if let (Some(current), entry) = (p.current_price, p.entry_price) {
                Some(((current - entry) / entry) * 100.0)
            } else {
                None
            };

            OpenPositionDetail {
                mint: p.mint.clone(),
                symbol: p.symbol.clone(),
                entry_price: p.entry_price,
                current_price: p.current_price,
                pnl_percent,
                hold_duration_minutes: hold_duration,
            }
        })
        .collect();

    let positions_summary = PositionsSummary {
        total_positions: (open_positions.len() + closed_positions.len()) as i64,
        open_positions: open_positions.len() as i64,
        closed_positions: closed_positions.len() as i64,
        total_invested_sol,
        total_pnl,
        win_rate,
        open_position_details,
    };

    // Get system info
    let services = ServiceStatus {
        tokens_system: TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
        positions_system: POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
        pool_service: POOL_SERVICE_READY.load(std::sync::atomic::Ordering::Relaxed),
        security_analyzer: SECURITY_ANALYZER_READY.load(std::sync::atomic::Ordering::Relaxed),
        transactions_system: TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
    };

    let all_services_ready = services.tokens_system
        && services.positions_system
        && services.pool_service
        && services.security_analyzer
        && services.transactions_system;

    let uptime_seconds = state.uptime_seconds();
    let uptime_formatted = format_uptime(uptime_seconds);

    // Get system metrics (simplified version)
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();

    let memory_mb = (sys.used_memory() as f64) / 1024.0 / 1024.0;
    let cpu_percent = sys.global_cpu_info().cpu_usage() as f64;
    let active_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let system_info = SystemInfo {
        all_services_ready,
        services,
        uptime_seconds,
        uptime_formatted,
        memory_mb,
        cpu_percent,
        active_threads,
    };

    // Get RPC stats
    let rpc_info = match get_global_rpc_stats() {
        Some(rpc_stats) => {
            let rpc_uptime = chrono::Utc::now()
                .signed_duration_since(rpc_stats.startup_time)
                .num_seconds() as u64;
            RpcInfo {
                total_calls: rpc_stats.total_calls(),
                calls_per_second: rpc_stats.calls_per_second(),
                uptime_seconds: rpc_uptime,
            }
        }
        None => RpcInfo {
            total_calls: 0,
            calls_per_second: 0.0,
            uptime_seconds: 0,
        },
    };

    // Get blacklist info
    let blacklist_info = if let Some(db) = get_global_database() {
        match get_blacklist_summary(&db) {
            Ok(summary) => {
                let mut by_reason = std::collections::HashMap::new();
                by_reason.insert("LowLiquidity".to_string(), summary.low_liquidity_count);
                by_reason.insert("NoRoute".to_string(), summary.no_route_count);
                by_reason.insert("ApiError".to_string(), summary.api_error_count);
                by_reason.insert("SystemToken".to_string(), summary.system_token_count);
                by_reason.insert("Manual".to_string(), summary.manual_count);
                by_reason.insert(
                    "PoorPerformance".to_string(),
                    summary.poor_performance_count,
                );
                by_reason.insert("SecurityIssue".to_string(), summary.security_count);

                BlacklistInfo {
                    total_blacklisted: summary.total_count,
                    by_reason,
                }
            }
            Err(_) => BlacklistInfo {
                total_blacklisted: 0,
                by_reason: std::collections::HashMap::new(),
            },
        }
    } else {
        BlacklistInfo {
            total_blacklisted: 0,
            by_reason: std::collections::HashMap::new(),
        }
    };

    // Get monitoring info (use hardcoded constants from trader module)
    let monitoring_info = MonitoringInfo {
        tokens_tracked: crate::pools::get_available_tokens().len(),
        entry_check_interval_secs: crate::trader::ENTRY_MONITOR_INTERVAL_SECS,
        position_monitor_interval_secs: crate::trader::POSITION_MONITOR_INTERVAL_SECS,
    };

    Json(DashboardOverview {
        wallet: wallet_info,
        positions: positions_summary,
        system: system_info,
        rpc: rpc_info,
        blacklist: blacklist_info,
        monitoring: monitoring_info,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Format uptime duration into human-readable string
fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, secs)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}
