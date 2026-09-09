//! Type definitions for trader API

use crate::positions::PositionManagement;
use serde::{Deserialize, Serialize};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TraderStatusResponse {
    pub enabled: bool,
    pub running: bool,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct TraderControlResponse {
    pub success: bool,
    pub message: String,
    pub status: TraderStatusResponse,
}

#[derive(Debug, Deserialize)]
pub struct TraderControlRequest {
    pub enabled: bool,
}

// =============================================================================
// MANUAL TRADING REQUEST/RESPONSE TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ManualBuyRequest {
    pub mint: String,
    #[serde(default)]
    pub size_sol: Option<f64>,
    #[serde(default)]
    pub force: Option<bool>,
    /// Ownership mode for the resulting position. Dashboard buys default to user-only.
    #[serde(default)]
    pub management: Option<PositionManagement>,
    /// Per-trade slippage override in percent. `None` = use the configured slippage
    /// (`swaps.slippage.*`). Manual trading only — the auto-trader is always config-driven.
    #[serde(default)]
    pub slippage_pct: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ManualAddRequest {
    pub mint: String,
    #[serde(default)]
    pub size_sol: Option<f64>,
    /// Per-trade slippage override in percent. `None` = use the configured slippage
    /// (`swaps.slippage.*`). Manual trading only — the auto-trader is always config-driven.
    #[serde(default)]
    pub slippage_pct: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ManualSellRequest {
    pub mint: String,
    #[serde(default)]
    pub percentage: Option<f64>,
    #[serde(default)]
    pub close_all: Option<bool>,
    #[serde(default)]
    pub force: Option<bool>,
    /// Per-trade slippage override in percent. `None` = use the configured slippage
    /// (`swaps.slippage.*`). Manual trading only — the auto-trader is always config-driven.
    #[serde(default)]
    pub slippage_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ManualTradeSuccess {
    pub success: bool,
    pub mint: String,
    pub signature: Option<String>,
    pub effective_price_sol: Option<f64>,
    pub size_sol: Option<f64>,
    pub position_id: Option<String>,
    pub message: String,
    pub timestamp: String,
}

/// Query for `GET /api/trader/stats`. The window is a request parameter, never a
/// constant baked into the dashboard: the Stats tab offers 24h / 7d / 30d and the
/// handler is the only place that decides what a window means.
#[derive(Debug, Deserialize)]
pub struct TraderStatsQuery {
    #[serde(default)]
    pub days: Option<u32>,
}

/// Realized performance over a closed window, plus the live exposure that window
/// does not cover.
///
/// Every derived figure is `Option` and is `None` when the window holds nothing to
/// derive it from. A fresh install has no win rate and no best trade, and `0.0`
/// there is a fabricated claim (a green `+0.0%` "best trade") rather than an empty
/// state, so the absence travels to the dashboard instead of a zero.
#[derive(Debug, Serialize)]
pub struct TraderStatsResponse {
    /// Length of the realized window, echoed back so the UI labels what it shows.
    pub period_days: u32,

    // Live exposure (not window-bound).
    pub open_positions_count: usize,
    pub max_open_positions: usize,
    pub locked_sol: f64,

    // Trade counts.
    pub total_trades: usize,
    pub winners: usize,
    pub losers: usize,
    /// Closed rounds excluded because their cost basis or history is incomplete, so
    /// no honest P&L exists for them. Surfaced rather than silently dropped.
    pub excluded_untrusted: usize,

    // Realized money, in SOL — the single monetary unit.
    pub total_pnl_sol: f64,
    pub gross_profit_sol: f64,
    pub gross_loss_sol: f64,
    pub profit_factor: Option<f64>,
    pub expectancy_sol: Option<f64>,
    pub max_drawdown_sol: f64,

    // Quality.
    pub win_rate_pct: Option<f64>,
    pub avg_win_pct: Option<f64>,
    pub avg_loss_pct: Option<f64>,
    pub avg_hold_time_hours: Option<f64>,
    pub median_hold_time_hours: Option<f64>,
    pub best_trade_pct: Option<f64>,
    pub best_trade_token: Option<String>,
    pub worst_trade_pct: Option<f64>,
    pub worst_trade_token: Option<String>,

    /// One entry per day in the window, oldest first, including days with no trades
    /// so the curve keeps a true time axis.
    pub daily_pnl: Vec<DailyPnlPoint>,
    pub exit_breakdown: Vec<ExitBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct DailyPnlPoint {
    /// UTC calendar day, `YYYY-MM-DD`.
    pub date: String,
    pub net_pnl_sol: f64,
    pub trades: usize,
}

#[derive(Debug, Serialize)]
pub struct ExitBreakdown {
    pub exit_type: String,
    pub count: usize,
    pub avg_profit_pct: f64,
    /// Realized SOL attributable to this exit reason.
    pub net_pnl_sol: f64,
}

// =============================================================================
// FORCE STOP / MONITOR CONTROL / LOSS LIMIT TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct ForceStopRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleMonitorRequest {
    pub enabled: bool,
}

// =============================================================================
// TRAILING STOP PREVIEW TYPES (Phase 2)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TrailingStopPreviewResponse {
    // Position state
    pub position_id: Option<i64>,
    pub symbol: String,
    pub entry_price: f64,
    pub current_price: f64,
    pub peak_price: f64,
    pub current_profit_pct: f64,
    pub unrealized_pnl: f64,

    // Trail state with CURRENT settings
    pub trail_active: bool,
    pub trail_activated_at_pct: Option<f64>,
    pub trail_stop_price: Option<f64>,
    pub distance_to_exit_pct: Option<f64>,
    pub estimated_exit_price: f64,
    pub estimated_exit_profit_pct: f64,

    // What-if scenarios
    pub what_if_scenarios: Vec<WhatIfScenario>,
}

#[derive(Debug, Serialize)]
pub struct WhatIfScenario {
    pub description: String,
    pub activation_pct: f64,
    pub distance_pct: f64,
    pub trail_active: bool,
    pub exit_price: f64,
    pub exit_profit_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct TrailingStopPreviewQuery {
    pub position_id: Option<i64>,
    pub activation_pct: Option<f64>,
    pub distance_pct: Option<f64>,
}

// =============================================================================
// QUOTE PREVIEW TYPES
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct QuotePreviewRequest {
    pub mint: String,
    #[serde(default)]
    pub amount_sol: Option<f64>, // For buy: SOL amount to spend
    #[serde(default)]
    pub amount_tokens: Option<f64>, // For sell: token amount to sell (whole tokens; legacy)
    #[serde(default)]
    pub percentage: Option<f64>, // For sell: percentage of the real on-chain balance (preferred)
    /// Slippage the quote is priced at. `None` = configured default. The dialog sends
    /// whatever the user has selected so the PREVIEW matches what will execute.
    #[serde(default)]
    pub slippage_pct: Option<f64>,
    #[serde(default)]
    pub direction: String, // "buy" or "sell", defaults to "buy"
}

#[derive(Debug, Serialize)]
pub struct QuotePreviewResponse {
    pub success: bool,
    pub router: String,
    pub direction: String,
    // For buy: input_sol is SOL spent, output is tokens received
    // For sell: input is tokens sold, output_sol is SOL received
    pub input_amount: f64,
    pub input_formatted: String,
    pub output_amount: f64,
    /// Authoritative wallet-received minimum supplied by the selected router.
    pub minimum_output_amount: f64,
    pub output_formatted: String,
    pub price_per_token_sol: f64,
    pub price_impact_pct: f64,
    pub platform_fee_pct: f64,
    pub platform_fee_sol: Option<f64>,
    pub network_fee_sol: Option<f64>,
    pub route: String,
    pub slippage_bps: u16,
    pub expires_in_secs: u64,
}

// =============================================================================
// TEMPLATE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<Template>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trading_style: String,
    pub config: TemplateConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemplateConfig {
    pub trailing_stop_enabled: bool,
    pub trailing_stop_activation_pct: f64,
    pub trailing_stop_distance_pct: f64,
    pub roi_exit_enabled: bool,
    pub roi_target_pct: f64,
    pub time_override_enabled: bool,
    pub time_override_duration: f64,
    pub time_override_unit: String,
    pub time_override_loss_threshold_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct ApplyTemplateRequest {
    pub template_id: String,
}
