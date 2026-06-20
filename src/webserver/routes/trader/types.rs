//! Type definitions for trader API

use serde::{Deserialize, Serialize};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TraderStatusResponse {
    pub enabled: bool,
    pub running: bool,
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
    /// Whether the resulting position is manually managed (auto-trader leaves it alone).
    /// Defaults to true: dashboard manual buys are protected unless the user opts out.
    #[serde(default)]
    pub manual_management: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ManualAddRequest {
    pub mint: String,
    #[serde(default)]
    pub size_sol: Option<f64>,
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

#[derive(Debug, Serialize)]
pub struct TraderStatsResponse {
    pub open_positions_count: usize,
    pub locked_sol: f64,
    pub win_rate_pct: f64,
    pub total_trades: usize,
    pub avg_hold_time_hours: f64,
    pub best_trade_pct: f64,
    pub exit_breakdown: Vec<ExitBreakdown>,
}

#[derive(Debug, Serialize)]
pub struct ExitBreakdown {
    pub exit_type: String,
    pub count: usize,
    pub avg_profit_pct: f64,
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
    pub amount_tokens: Option<f64>, // For sell: token amount to sell
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
    pub output_formatted: String,
    pub price_per_token_sol: f64,
    pub price_impact_pct: f64,
    pub platform_fee_pct: f64,
    pub platform_fee_sol: f64,
    pub network_fee_sol: f64,
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
