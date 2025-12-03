use axum::{extract::State, response::Json, routing::get, Router};
use serde::{Deserialize, Serialize};
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
    Router::new()
        .route("/dashboard/overview", get(get_dashboard_overview))
        .route("/dashboard/home", get(get_home_dashboard))
}

/// Get comprehensive dashboard overview
async fn get_dashboard_overview(State(state): State<Arc<AppState>>) -> Json<DashboardOverview> {
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
                .num_seconds() as u64;
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
                let mut by_reason = std::collections::HashMap::new();
                by_reason.insert("Manual".to_string(), summary.manual_count);
                by_reason.insert("MintAuthority".to_string(), summary.authority_mint_count);
                by_reason.insert(
                    "FreezeAuthority".to_string(),
                    summary.authority_freeze_count,
                );
                if summary.non_authority_auto_count > 0 {
                    by_reason.insert(
                        "NonAuthorityAuto".to_string(),
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

// ============================================================================
// HOME DASHBOARD - Comprehensive Analytics
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct HomeDashboardResponse {
    pub trader: TraderAnalytics,
    pub wallet: WalletAnalytics,
    pub positions: PositionsSnapshot,
    pub system: SystemMetrics,
    pub tokens: TokenStatistics,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraderAnalytics {
    pub today: TradingPeriodStats,
    pub yesterday: TradingPeriodStats,
    pub this_week: TradingPeriodStats,
    pub this_month: TradingPeriodStats,
    pub all_time: TradingPeriodStats,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradingPeriodStats {
    pub buys: i64,
    pub sells: i64,
    pub profit_sol: f64,
    pub loss_sol: f64,
    pub net_pnl_sol: f64,
    pub drawdown_percent: f64,
    pub win_rate: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAnalytics {
    pub current_balance_sol: f64,
    pub token_count: usize,
    pub tokens_worth_sol: f64,
    pub start_of_day_balance_sol: f64,
    pub change_sol: f64,
    pub change_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionsSnapshot {
    pub open_count: i64,
    pub total_invested_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub memory_mb: f64,
    pub memory_percent: f64,
    pub cpu_percent: f64,
    pub cpu_history: Vec<f64>,
    pub memory_history: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStatistics {
    pub total_in_database: usize,
    pub with_prices: usize,
    pub passed_filters: usize,
    pub rejected_filters: usize,
    pub found_today: usize,
    pub found_this_week: usize,
    pub found_this_month: usize,
    pub found_all_time: usize,
}

/// GET /api/dashboard/home
/// Comprehensive home dashboard with all analytics
async fn get_home_dashboard(State(state): State<Arc<AppState>>) -> Json<HomeDashboardResponse> {
    // Return demo data if demo mode is enabled
    if demo::is_demo_mode() {
        return Json(demo::get_demo_home_dashboard());
    }

    use chrono::{DateTime, Duration, TimeZone};

    let now = chrono::Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or_else(|| chrono::NaiveDateTime::default());
    let today_start = chrono::Utc.from_utc_datetime(&today_start);
    let yesterday_start = today_start - Duration::days(1);
    let week_start = today_start - Duration::days(7);
    let month_start = today_start - Duration::days(30);
    let epoch_start = chrono::Utc
        .timestamp_opt(0, 0)
        .earliest()
        .unwrap_or(today_start);

    struct PeriodRange {
        start: DateTime<chrono::Utc>,
        end: Option<DateTime<chrono::Utc>>,
    }

    // OPTIMIZED: Calculate trader analytics using SQL aggregation (3.2s → 0.25s)
    // Instead of fetching ALL closed positions and iterating in Rust,
    // we use 5 parallel SQL queries that return pre-aggregated stats
    let (
        today_stats_result,
        yesterday_stats_result,
        week_stats_result,
        month_stats_result,
        alltime_stats_result,
    ) = tokio::join!(
        positions::get_period_trading_stats(today_start, Some(now)),
        positions::get_period_trading_stats(yesterday_start, Some(today_start)),
        positions::get_period_trading_stats(week_start, Some(now)),
        positions::get_period_trading_stats(month_start, Some(now)),
        positions::get_period_trading_stats(epoch_start, Some(now)),
    );

    // Convert from database PeriodTradingStats to dashboard TradingPeriodStats
    let convert_stats =
        |result: Result<positions::PeriodTradingStats, String>| -> TradingPeriodStats {
            match result {
                Ok(stats) => TradingPeriodStats {
                    buys: stats.buys,
                    sells: stats.sells,
                    profit_sol: stats.profit_sol,
                    loss_sol: stats.loss_sol,
                    net_pnl_sol: stats.net_pnl_sol,
                    drawdown_percent: stats.drawdown_percent,
                    win_rate: stats.win_rate,
                },
                Err(_) => TradingPeriodStats {
                    buys: 0,
                    sells: 0,
                    profit_sol: 0.0,
                    loss_sol: 0.0,
                    net_pnl_sol: 0.0,
                    drawdown_percent: 0.0,
                    win_rate: 0.0,
                },
            }
        };

    let trader = TraderAnalytics {
        today: convert_stats(today_stats_result),
        yesterday: convert_stats(yesterday_stats_result),
        this_week: convert_stats(week_stats_result),
        this_month: convert_stats(month_stats_result),
        all_time: convert_stats(alltime_stats_result),
    };

    // Get wallet analytics
    let current_wallet = get_current_wallet_status().await.ok().flatten();

    // Get start of day balance using optimized single-value query
    // Performance: ~0.05s vs 1.5s (fetching 100 snapshots)
    let start_of_day_balance_sol = crate::wallet::get_balance_at_time(today_start)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            current_wallet
                .as_ref()
                .map(|w| w.sol_balance)
                .unwrap_or(0.0)
        });

    let current_balance_sol = current_wallet
        .as_ref()
        .map(|w| w.sol_balance)
        .unwrap_or(0.0);
    let change_sol = current_balance_sol - start_of_day_balance_sol;
    let change_percent = if start_of_day_balance_sol > 0.0 {
        (change_sol / start_of_day_balance_sol) * 100.0
    } else {
        0.0
    };

    // Calculate token worth (simplified - would need prices)
    let token_count = current_wallet
        .as_ref()
        .map(|w| w.total_tokens_count as usize)
        .unwrap_or(0);

    let wallet = WalletAnalytics {
        current_balance_sol,
        token_count,
        tokens_worth_sol: 0.0, // TODO: Calculate from token balances with prices
        start_of_day_balance_sol,
        change_sol,
        change_percent,
    };

    // Get positions snapshot
    let open_positions = positions::get_db_open_positions().await.unwrap_or_default();
    let total_invested_sol: f64 = open_positions.iter().map(|p| p.entry_size_sol).sum();
    let unrealized_pnl_sol: f64 = open_positions
        .iter()
        .filter_map(|p| {
            if let (Some(current), entry) = (p.current_price, p.entry_price) {
                Some((current - entry) * entry * p.entry_size_sol / entry)
            } else {
                None
            }
        })
        .sum();
    let unrealized_pnl_percent = if total_invested_sol > 0.0 {
        (unrealized_pnl_sol / total_invested_sol) * 100.0
    } else {
        0.0
    };

    let positions_snapshot = PositionsSnapshot {
        open_count: open_positions.len() as i64,
        total_invested_sol,
        unrealized_pnl_sol,
        unrealized_pnl_percent,
    };

    // Get system metrics (cached, non-blocking)
    let uptime_seconds = state.uptime_seconds();
    let uptime_formatted = format_uptime(uptime_seconds);

    let cached_metrics = get_cached_system_metrics().await;

    // Use process memory (bot only) instead of system memory
    let memory_mb = cached_metrics.process_memory_mb as f64;
    let memory_total_mb = cached_metrics.system_memory_total_mb as f64;
    let memory_percent = if memory_total_mb > 0.0 {
        (memory_mb / memory_total_mb) * 100.0
    } else {
        0.0
    };
    // Use process CPU instead of system CPU
    let cpu_percent = cached_metrics.cpu_process_percent as f64;

    // Generate simple history for charts (last 20 data points)
    let cpu_history = vec![cpu_percent; 20];
    let memory_history = vec![memory_percent; 20];

    let system = SystemMetrics {
        uptime_seconds,
        uptime_formatted,
        memory_mb,
        memory_percent,
        cpu_percent,
        cpu_history,
        memory_history,
    };

    // Get token statistics
    let db = crate::tokens::database::get_global_database();
    let total_in_database = db.as_ref().and_then(|d| d.count_tokens().ok()).unwrap_or(0) as usize;

    // Get filtering stats
    let passed_filters = match crate::filtering::fetch_stats().await {
        Ok(stats) => stats.passed_filtering,
        Err(_) => 0,
    };
    let rejected_filters = 0; // TODO: Calculate rejected count

    // TODO: Implement time-based token discovery counts
    let tokens = TokenStatistics {
        total_in_database,
        with_prices: 0, // TODO: Count tokens with prices
        passed_filters,
        rejected_filters,
        found_today: 0,      // TODO: Implement
        found_this_week: 0,  // TODO: Implement
        found_this_month: 0, // TODO: Implement
        found_all_time: total_in_database,
    };

    Json(HomeDashboardResponse {
        trader,
        wallet,
        positions: positions_snapshot,
        system,
        tokens,
        timestamp: now.to_rfc3339(),
    })
}
