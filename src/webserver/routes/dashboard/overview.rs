//! Dashboard overview — aggregates key metrics for the dashboard home view.

use axum::{extract::State, response::Json};
use std::collections::HashMap;
use std::sync::Arc;

use crate::global::{
    POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY, TRANSACTIONS_SYSTEM_READY,
};
use crate::positions;
use crate::rpc::get_global_rpc_stats;
use crate::tokens::cleanup::get_blacklist_summary;
use crate::tokens::database::get_global_database;
use crate::wallet::get_current_wallet_status;
use crate::webserver::demo;
use crate::webserver::snapshot::get_cached_system_metrics;
use crate::webserver::state::AppState;

use super::types::*;
use super::utils::format_uptime;

/// GET /api/dashboard/overview
/// Get comprehensive dashboard overview
pub async fn get_dashboard_overview(State(state): State<Arc<AppState>>) -> Json<DashboardOverview> {
    // Return demo data if demo mode is enabled
    if demo::is_demo_mode() {
        return Json(demo::get_demo_dashboard_overview());
    }

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

    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();

    // Use SQL aggregation for closed positions stats (optimized)
    let epoch_start = chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let all_time_stats = positions::get_period_trading_stats(epoch_start, None)
        .await
        .unwrap_or_else(|_| positions::PeriodTradingStats {
            buys: 0,
            sells: 0,
            profit_sol: 0.0,
            loss_sol: 0.0,
            net_pnl_sol: 0.0,
            drawdown_percent: 0.0,
            win_rate: 0.0,
        });

    let total_pnl = all_time_stats.net_pnl_sol;
    let win_rate = all_time_stats.win_rate;
    let closed_positions_count = positions::get_db_closed_positions_count_since(epoch_start)
        .await
        .unwrap_or_default();

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
        total_positions: (open_positions.len() as i64 + closed_positions_count),
        open_positions: open_positions.len() as i64,
        closed_positions: closed_positions_count,
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
        transactions_system: TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::Relaxed),
    };

    let all_services_ready = services.tokens_system
        && services.positions_system
        && services.pool_service
        && services.transactions_system;

    let uptime_seconds = state.uptime_seconds();
    let uptime_formatted = format_uptime(uptime_seconds);

    // Get cached system metrics (5s cache, non-blocking)
    let cached_metrics = get_cached_system_metrics().await;

    // Use process memory (bot only) instead of system memory
    let memory_mb = cached_metrics.process_memory_mb as f64;
    // Use process CPU instead of system CPU
    let cpu_percent = cached_metrics.cpu_process_percent as f64;
    let active_threads = cached_metrics.active_threads;

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
                .num_seconds().max(0) as u64;
            let recent_calls_per_second = rpc_stats.calls_per_minute_recent(5) / 60.0;
            let fallback_cps = rpc_stats.calls_per_second();
            RpcInfo {
                total_calls: rpc_stats.total_calls(),
                calls_per_second: if recent_calls_per_second > 0.0 {
                    recent_calls_per_second
                } else {
                    fallback_cps
                },
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
                let mut by_reason = HashMap::new();
                by_reason.insert("Manual".to_owned(), summary.manual_count);
                by_reason.insert("MintAuthority".to_owned(), summary.authority_mint_count);
                by_reason.insert(
                    "FreezeAuthority".to_owned(),
                    summary.authority_freeze_count,
                );
                if summary.non_authority_auto_count > 0 {
                    by_reason.insert(
                        "NonAuthorityAuto".to_owned(),
                        summary.non_authority_auto_count,
                    );
                    for (reason, count) in summary.non_authority_breakdown.iter() {
                        by_reason.insert(format!("NonAuthority::{reason}"), *count);
                    }
                }

                BlacklistInfo {
                    total_blacklisted: summary.total_count,
                    by_reason,
                }
            }
            Err(_) => BlacklistInfo {
                total_blacklisted: 0,
                by_reason: HashMap::new(),
            },
        }
    } else {
        BlacklistInfo {
            total_blacklisted: 0,
            by_reason: HashMap::new(),
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
