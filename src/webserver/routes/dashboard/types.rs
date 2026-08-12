//! Dashboard route types — data structures for dashboard API responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::filtering::SnapshotState;

// ============================================================================
// Portfolio Calendar Types
// ============================================================================

/// A single day cell in the portfolio calendar.
#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarDay {
    /// Day-of-month (1-31).
    pub day: u32,
    /// Full date, YYYY-MM-DD (UTC).
    pub date: String,
    /// Realized net P&L (SOL) from trades closed that day.
    pub net_pnl_sol: f64,
    /// Gross realized profit (SOL) from winning trades that day.
    pub profit_sol: f64,
    /// Gross realized loss (SOL, positive magnitude) from losing trades that day.
    pub loss_sol: f64,
    /// Number of positions closed that day.
    pub trades: i64,
    /// Number of profitable positions closed that day.
    pub wins: i64,
    /// End-of-day wallet SOL balance, if a snapshot exists for that day.
    pub portfolio_value_sol: Option<f64>,
    /// Whether the day has any P&L or trade activity.
    pub has_data: bool,
}

/// Portfolio calendar response for a single month.
#[derive(Debug, Serialize, Deserialize)]
pub struct PortfolioCalendarResponse {
    pub year: i32,
    pub month: u32,
    /// Weekday (0=Sunday..6=Saturday) that the 1st of the month falls on.
    pub first_weekday: u32,
    /// Number of days in the month.
    pub days_in_month: u32,
    pub days: Vec<CalendarDay>,
    pub month_net_pnl_sol: f64,
    pub month_trades: i64,
}

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
    pub by_reason: HashMap<String, usize>,
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

/// The home hero's wallet block. Monetary fields come from `wallet::get_wallet_worth()`,
/// the same call the header makes — the two must never show different numbers.
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAnalytics {
    /// Public address of the active main wallet; empty in preview mode.
    pub wallet_address: String,
    /// Free (uninvested) SOL sitting in the wallet.
    pub current_balance_sol: f64,
    /// Number of distinct fungible tokens held.
    pub token_count: usize,
    /// SOL value of the held tokens.
    pub tokens_worth_sol: f64,
    /// Total portfolio value: cash SOL + token holdings value. The hero headline.
    pub total_equity_sol: f64,
    /// Held tokens with no price available — they contribute 0, so a non-zero count
    /// means the worth is a known-low estimate rather than the whole truth.
    pub unpriced_token_count: usize,
    /// Wallet WORTH at 00:00 UTC today — the change baseline (same quantity as the
    /// headline; using cash here reported a phantom gain the size of the holdings).
    pub start_of_day_balance_sol: f64,
    /// total_equity_sol - start_of_day_balance_sol.
    pub change_sol: f64,
    pub change_percent: f64,
    /// Current SOL/USD price so the client can render an approximate USD value.
    pub sol_price_usd: f64,
    /// Recent wallet WORTH samples, OLDEST first, for a trend sparkline.
    pub balance_history: Vec<f64>,
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
    /// Whether the filtering snapshot the counts below it come from exists yet. The two
    /// database-backed totals (`total_in_database`, `found_all_time`) are always real.
    pub snapshot_state: SnapshotState,
    pub total_in_database: usize,
    pub with_prices: Option<usize>,
    pub passed_filters: Option<usize>,
    pub rejected_filters: Option<usize>,
    pub blacklisted: Option<usize>,
    pub with_ohlcv: Option<usize>,
    pub found_today: usize,
    pub found_this_week: usize,
    pub found_this_month: usize,
    pub found_all_time: usize,
}
