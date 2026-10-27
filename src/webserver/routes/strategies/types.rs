// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategies route types — data structures for strategy API responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::i18n::UiText;
use crate::strategies::overview::{RuleExitState, RunNotice, StrategyCounts, StrategySummary};
use crate::strategies::types::{RuleOutcome, Strategy};
use crate::trader::paper::TradingMode;
use crate::trader::TraderRunState;

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
    pub mode: TradingMode,
}

impl From<Strategy> for StrategyItem {
    fn from(strategy: Strategy) -> Self {
        Self {
            id: strategy.id,
            name: strategy.name,
            description: strategy.description,
            strategy_type: strategy.strategy_type.to_string(),
            enabled: strategy.enabled,
            priority: strategy.priority,
            created_at: strategy.created_at.to_rfc3339(),
            updated_at: strategy.updated_at.to_rfc3339(),
            author: strategy.author,
            version: strategy.version,
            entry_strategy_ids: strategy.entry_strategy_ids,
            mode: strategy.mode,
        }
    }
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
    pub mode: TradingMode,
}

/// The strategies overview: the trader-level state once, then each strategy's list item
/// with its status and figures.
#[derive(Debug, Serialize)]
pub struct StrategiesOverviewResponse {
    pub run_state: TraderRunState,
    pub notice: Option<RunNotice>,
    pub rule_exits: Vec<RuleExitState>,
    pub counts: StrategyCounts,
    pub strategies: Vec<StrategyOverviewItem>,
    pub timestamp: String,
}

/// One strategy of the overview: its list item and its summary side by side.
#[derive(Debug, Serialize)]
pub struct StrategyOverviewItem {
    pub item: StrategyItem,
    #[serde(flatten)]
    pub summary: StrategySummary,
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

/// Strategy trading-mode change. Arming live carries the exact confirmation phrase.
#[derive(Debug, Deserialize)]
pub struct StrategyModeRequest {
    pub mode: TradingMode,
    #[serde(default)]
    pub confirmation: Option<String>,
}

/// A strategy's live readiness and the phrase that arms it live.
#[derive(Debug, Serialize)]
pub struct StrategyReadinessResponse {
    #[serde(flatten)]
    pub readiness: crate::trader::paper::readiness::Readiness,
    pub live_confirmation: &'static str,
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
