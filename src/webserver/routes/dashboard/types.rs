// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
    pub net_pnl_native: f64,
    /// Gross realized profit (SOL) from winning trades that day.
    pub profit_native: f64,
    /// Gross realized loss (SOL, positive magnitude) from losing trades that day.
    pub loss_native: f64,
    /// Number of positions closed that day.
    pub trades: i64,
    /// Number of profitable positions closed that day.
    pub wins: i64,
    /// End-of-day wallet worth (cash + holdings), if a snapshot exists for that day.
    pub portfolio_value_native: Option<f64>,
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
    pub month_net_pnl_native: f64,
    pub month_trades: i64,
    /// Days that closed with a net realized profit.
    pub month_win_days: u32,
    /// Days that closed with a net realized loss.
    pub month_loss_days: u32,
    /// The day with the largest net realized profit; None without a profitable day.
    pub best_day: Option<CalendarBestDay>,
}

/// The month's most profitable day.
#[derive(Debug, Serialize, Deserialize)]
pub struct CalendarBestDay {
    /// Full date, YYYY-MM-DD (UTC).
    pub date: String,
    pub net_pnl_native: f64,
}

impl PortfolioCalendarResponse {
    /// The month's response with its totals summed from `days`, so the footer and the
    /// cells can never disagree.
    pub fn from_days(year: i32, month: u32, first_weekday: u32, days: Vec<CalendarDay>) -> Self {
        let traded = || days.iter().filter(|day| day.has_data);
        let best_day = traded()
            .filter(|day| day.net_pnl_native > 0.0)
            .max_by(|a, b| a.net_pnl_native.total_cmp(&b.net_pnl_native))
            .map(|day| CalendarBestDay {
                date: day.date.clone(),
                net_pnl_native: day.net_pnl_native,
            });
        Self {
            year,
            month,
            first_weekday,
            days_in_month: days.len() as u32,
            month_net_pnl_native: days.iter().map(|day| day.net_pnl_native).sum(),
            month_trades: days.iter().map(|day| day.trades).sum(),
            month_win_days: traded().filter(|day| day.net_pnl_native > 0.0).count() as u32,
            month_loss_days: traded().filter(|day| day.net_pnl_native < 0.0).count() as u32,
            best_day,
            days,
        }
    }
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
    pub native_balance_raw: u64,
    pub total_tokens_count: usize,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionsSummary {
    pub total_positions: i64,
    pub open_positions: i64,
    pub closed_positions: i64,
    pub total_invested_native: f64,
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
    pub auto_trader: AutoTraderCard,
    pub copy_trading: CopyTradingCard,
    pub timestamp: String,
}

/// The state a Home status card reports for its feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Running,
    Stopped,
    /// Explore Mode, or wallet and RPC setup not complete: the feature cannot run.
    Unavailable,
}

/// The Home Auto Trader card. Figures are absent while the trader is unavailable, and
/// a figure that could not be read is absent too, never zero.
#[derive(Debug, Serialize, Deserialize)]
pub struct AutoTraderCard {
    pub state: FeatureState,
    /// Open positions counted against the limit (wallet-derived rounds excluded).
    pub open_positions: Option<usize>,
    pub max_open_positions: Option<usize>,
    /// Entries and exits since the start of the UTC day.
    pub entries_today: Option<i64>,
    pub exits_today: Option<i64>,
}

/// The Home Copy Trading card. `state` is absent when the copy store could not be
/// read; figures are absent while Copy Trading is unavailable or unreadable.
#[derive(Debug, Serialize, Deserialize)]
pub struct CopyTradingCard {
    pub state: Option<FeatureState>,
    pub followed_wallets: Option<usize>,
    pub active_tasks: Option<usize>,
    pub total_tasks: Option<usize>,
    /// When the newest copied fill was recorded (RFC 3339).
    pub last_copy_at: Option<String>,
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
    pub profit_native: f64,
    pub loss_native: f64,
    pub net_pnl_native: f64,
    pub drawdown_percent: f64,
    pub win_rate: f64,
}

/// The home hero's wallet block. Monetary fields come from `wallet::get_wallet_worth()`,
/// the same call the header makes — the two must never show different numbers. The worth
/// fields are None until a wallet snapshot exists (always in Explore Mode, which has no
/// wallet): an unknown balance is absent, never a measured zero.
#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAnalytics {
    /// Public address of the active main wallet; empty in Explore Mode.
    pub wallet_address: String,
    /// Free (uninvested) SOL sitting in the wallet.
    pub current_balance_native: Option<f64>,
    /// Number of distinct fungible tokens held.
    pub token_count: Option<usize>,
    /// SOL value of the held tokens.
    pub tokens_worth_native: Option<f64>,
    /// Total portfolio value: cash SOL + token holdings value. The hero headline.
    pub total_equity_native: Option<f64>,
    /// Held tokens with no price available — they contribute 0, so a non-zero count
    /// means the worth is a known-low estimate rather than the whole truth.
    pub unpriced_token_count: Option<usize>,
    /// Wallet WORTH at 00:00 UTC today — the change baseline (same quantity as the
    /// headline; using cash here reported a phantom gain the size of the holdings).
    /// None without a snapshot at or before the start of the day.
    pub start_of_day_balance_native: Option<f64>,
    /// Whether `change_native` could be measured, and if not, why.
    pub change_status: WorthChangeStatus,
    /// total_equity_native - start_of_day_balance_native, set only when both price the
    /// same holdings (`change_status` is `known`): a token entering or leaving pricing
    /// is never a gain or a loss.
    pub change_native: Option<f64>,
    /// None unless the change is known and the baseline is positive.
    pub change_percent: Option<f64>,
    /// Current SOL/USD price so the client can render an approximate USD value.
    pub native_price_usd: f64,
    /// Wallet WORTH since 00:00 UTC today, OLDEST first, for the trend sparkline: the
    /// start-of-day worth, one sample per bucket that priced the same holdings as the
    /// live worth, then the live worth. Its first and last points are the two figures
    /// the day change compares; empty unless the change is known.
    pub balance_history: Vec<f64>,
}

/// Whether the hero's day change could be measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorthChangeStatus {
    /// Like-for-like change against the start of the day.
    Known,
    /// No live worth yet, or no snapshot at or before the start of the day.
    NoBaseline,
    /// The start of the day and now price different holdings, so the two worths are
    /// not comparable.
    Incomparable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionsSnapshot {
    pub open_count: i64,
    pub total_invested_native: f64,
    /// Cost basis plus unrealized P&L of the positions with a cost basis; None while
    /// any of them has no price yet, since its value would be unknown.
    pub current_value_native: Option<f64>,
    pub unrealized_pnl_native: f64,
    /// None while no open position has a cost basis.
    pub unrealized_pnl_percent: Option<f64>,
    /// None with no open position.
    pub avg_position_size_native: Option<f64>,
    /// None with no open position.
    pub avg_hold_duration_mins: Option<i64>,
    pub best_performer: Option<PositionPerformer>,
    pub worst_performer: Option<PositionPerformer>,
    pub dca_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionPerformer {
    /// Database id, which opens the position's details; None only for a row not yet stored.
    pub id: Option<i64>,
    pub mint: String,
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
    /// Tokens with market data in the filtering snapshot: the set the filters evaluate,
    /// and the header's "monitoring" count.
    pub monitored: Option<usize>,
    /// Tokens the pool service holds a live price for right now (the prices trading
    /// uses); None while the pool service is not running.
    pub with_prices: Option<usize>,
    pub passed_filters: Option<usize>,
    /// Monitored tokens that did not pass (blacklisted ones included), the header's
    /// "rejected" count.
    pub rejected_filters: Option<usize>,
    pub blacklisted: Option<usize>,
    pub with_ohlcv: Option<usize>,
    pub found_today: usize,
    pub found_this_week: usize,
    pub found_this_month: usize,
    pub found_all_time: usize,
}
