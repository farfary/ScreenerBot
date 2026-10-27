// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies route types — data structures for strategy API responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::i18n::UiText;
use crate::strategies::types::RuleOutcome;

// =============================================================================
// RESPONSE TYPES (Inline with routes as per ScreenerBot patterns)
// =============================================================================

/// Strategy list response
#[derive(Debug, Serialize)]
pub struct StrategyListResponse {
    pub items: Vec<StrategyItem>,
    pub total: usize,
    pub timestamp: String,
}

/// Strategy item in list
#[derive(Debug, Serialize)]
pub struct StrategyItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<String>,
    pub version: i32,
    /// Exit strategies: the entry strategies whose positions it manages; empty = all.
    pub entry_strategy_ids: Vec<String>,
}

/// Strategy detail response
#[derive(Debug, Serialize)]
pub struct StrategyDetailResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    pub enabled: bool,
    pub priority: i32,
    pub timeframe: String,
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<String>,
    pub version: i32,
    /// Exit strategies: the entry strategies whose positions it manages; empty = all.
    pub entry_strategy_ids: Vec<String>,
}

/// Strategy performance response
#[derive(Debug, Serialize)]
pub struct StrategyPerformanceResponse {
    pub strategy_id: String,
    pub total_evaluations: u64,
    pub successful_signals: u64,
    pub success_rate: f64,
    pub avg_execution_time_ms: f64,
    pub last_evaluation: String,
}

/// Strategy test request
///
/// Without `current_price` the strategy is tested in the token's live context, the one
/// the trader would evaluate it in; with it, in the supplied price, market and position
/// data alone.
#[derive(Debug, Deserialize)]
pub struct StrategyTestRequest {
    pub token_mint: String,
    #[serde(default)]
    pub current_price: Option<f64>,
    #[serde(default)]
    pub market_data: Option<TestMarketData>,
    #[serde(default)]
    pub position_data: Option<TestPositionData>,
}

#[derive(Debug, Deserialize)]
pub struct TestMarketData {
    pub liquidity_native: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TestPositionData {
    pub entry_price: f64,
    pub entry_time: String,
    pub current_size_native: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
    #[serde(default)]
    pub peak_price: Option<f64>,
    #[serde(default)]
    pub dca_count: u32,
    #[serde(default)]
    pub partial_exit_count: u32,
    #[serde(default)]
    pub remaining_value_native: Option<f64>,
    #[serde(default)]
    pub entry_liquidity_native: Option<f64>,
    #[serde(default)]
    pub entry_liquidity_pool: Option<String>,
}

/// Strategy test response
#[derive(Debug, Serialize)]
pub struct StrategyTestResponse {
    pub strategy_id: String,
    pub strategy_name: String,
    /// Whether the strategy signals; `None` when a rule it needs had no usable data.
    pub result: Option<bool>,
    /// Why the result is unknown: the data error the undecided rule reached.
    pub unknown_reason: Option<UiText>,
    pub execution_time_ms: u64,
    /// The verdict of every node of the rule tree, the root first.
    pub rules: RuleOutcome,
}

/// Strategy create/update request
#[derive(Debug, Deserialize)]
pub struct StrategyRequest {
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    pub rules: serde_json::Value,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    pub author: Option<String>,
    /// Exit strategies: the entry strategies whose positions it manages; empty = all.
    /// Absent: none on create, the stored binding on update, so a request that omits
    /// the field never widens a binding to every position.
    #[serde(default)]
    pub entry_strategy_ids: Option<Vec<String>>,
}

/// Strategy enabled-state update request.
#[derive(Debug, Deserialize)]
pub struct StrategyEnabledRequest {
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    10
}

/// Timeframe of a strategy saved without one, and the one an unset condition timeframe falls back to.
pub(super) fn default_timeframe() -> String {
    "5m".to_owned()
}

/// Query parameters for strategy list
#[derive(Debug, Deserialize)]
pub struct StrategyListQuery {
    #[serde(rename = "type")]
    pub strategy_type: Option<String>,
    pub enabled: Option<bool>,
}

/// Condition schemas response
#[derive(Debug, Serialize)]
pub struct ConditionSchemasResponse {
    pub schemas: serde_json::Value,
    /// Timeframe a new strategy is saved with; an unset condition timeframe uses the strategy's.
    pub default_timeframe: String,
    pub timestamp: String,
}
