//! Dashboard route types — data structures for dashboard API responses.

use serde::{Deserialize, Serialize};

// ============================================================================
// Dashboard Overview Types
// ============================================================================

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

// ============================================================================
// Home Dashboard Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct HomeDashboardResponse {
    pub trader: TraderAnalytics,
    pub wallet: WalletAnalytics,
    pub positions: PositionsSnapshot,
    pub system: SystemMetrics,
    pub tokens: TokenStatistics,
    pub trader_status: TraderStatusInfo,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraderStatusInfo {
    pub running: bool,
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
    // Enhanced metrics
    pub avg_position_size_sol: f64,
    pub avg_hold_duration_mins: i64,
    pub best_performer: Option<PositionPerformer>,
    pub worst_performer: Option<PositionPerformer>,
    pub dca_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionPerformer {
    pub symbol: String,
    pub pnl_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_seconds: u64,
    pub uptime_formatted: String,
    pub memory_mb: f64,
    pub memory_percent: f64,
    pub cpu_percent: f64,
    // RPC metrics
    pub rpc_calls_per_min: f64,
    pub rpc_success_rate: f64,
    // Connectivity
    pub websocket_connected: bool,
    pub services_healthy: usize,
    pub services_total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenStatistics {
    pub total_in_database: usize,
    pub with_prices: usize,
    pub passed_filters: usize,
    pub rejected_filters: usize,
    pub blacklisted: usize,
    pub with_ohlcv: usize,
    pub found_today: usize,
    pub found_this_week: usize,
    pub found_this_month: usize,
    pub found_all_time: usize,
}
