// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy data types — conditions, templates, and evaluation context.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::i18n::UiText;
// Import unified types from OHLCV module
use crate::ohlcvs::TimeframeBundle;
use crate::tokens::Token;
use crate::trader::paper::TradingMode;
use crate::utils::canonical_json::canonicalize;

/// Strategy type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum StrategyType {
    Entry,
    Exit,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StrategyType::Entry => write!(f, "ENTRY"),
            StrategyType::Exit => write!(f, "EXIT"),
        }
    }
}

/// The data a condition reads, published in its schema as `reads` so the editor can say
/// what a condition depends on without inferring it from the category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConditionData {
    /// OHLCV candles of a timeframe.
    Candles,
    /// The open position the exit strategy manages.
    Position,
    /// The live pool the price is read from.
    Pool,
    /// The token's market snapshot (transaction counts per window, creation time).
    Market,
    /// The evaluation instant alone.
    Clock,
}

/// The lowest priority a strategy takes; strategies are evaluated lowest priority first.
pub const MIN_PRIORITY: i32 = 1;
/// The highest priority a strategy takes.
pub const MAX_PRIORITY: i32 = 100;

/// Strategy definition with rule tree and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: StrategyType,
    pub enabled: bool,
    pub priority: i32,
    pub timeframe: String, // Which timeframe to use for OHLCV conditions ("1m", "5m", etc.)
    pub rules: RuleTree,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Option<String>,
    pub version: i32,
    /// Exit strategies only: the entry strategies whose positions this one manages.
    /// Empty manages every position, including those no entry strategy opened.
    #[serde(default)]
    pub entry_strategy_ids: Vec<String>,
    /// Paper strategies trade the paper book; only live ones open or close real trades.
    pub mode: TradingMode,
    /// When the rules last changed: the moment [`Strategy::rules_fingerprint`] took its
    /// stored value. Written only by the strategies store: an insert stamps the creation
    /// time and an update the moment the fingerprint changed; the value on a `Strategy`
    /// handed to an insert or an update is ignored.
    pub rules_changed_at: DateTime<Utc>,
}

impl Strategy {
    /// The binding a strategy of `strategy_type` keeps from the requested entry strategy
    /// ids: each id once, in the order given, for an exit strategy; none for an entry
    /// strategy, which manages no positions.
    pub fn entry_binding(strategy_type: StrategyType, requested: Vec<String>) -> Vec<String> {
        if strategy_type != StrategyType::Exit {
            return Vec::new();
        }
        let mut binding: Vec<String> = Vec::with_capacity(requested.len());
        for id in requested {
            let id = id.trim().to_owned();
            if !id.is_empty() && !binding.contains(&id) {
                binding.push(id);
            }
        }
        binding
    }

    /// Refuse a `priority` outside [`MIN_PRIORITY`]..=[`MAX_PRIORITY`].
    pub fn check_priority(priority: i32) -> super::Result<()> {
        if (MIN_PRIORITY..=MAX_PRIORITY).contains(&priority) {
            Ok(())
        } else {
            Err(super::Error::PriorityOutOfRange { priority })
        }
    }

    /// Whether this strategy still trades on the book of `mode`: it is enabled and its mode
    /// is `mode`. Each executor asks it of the deciding strategy, read again just before
    /// the trade, so a strategy switched off or moved to the other book after its decision
    /// trades nothing more on the book it left.
    pub fn trades_on(&self, mode: TradingMode) -> bool {
        self.enabled && self.mode == mode
    }

    /// What the strategy decides on, as one value: its type, timeframe and rule tree (each
    /// node's operator, child order and `disabled` flag, each condition's type and each
    /// parameter's value), and for an exit strategy the set of entry strategies it
    /// manages. Name, description, priority, on/off, mode, author, version, timestamps,
    /// the strategy-level parameter map and a parameter's default and constraints are
    /// not part of it. The `1:` prefix names this projection; a different projection takes
    /// a new prefix.
    pub fn rules_fingerprint(&self) -> String {
        let mut projection = serde_json::Map::new();
        projection.insert(
            "type".to_owned(),
            serde_json::Value::String(self.strategy_type.to_string()),
        );
        projection.insert(
            "timeframe".to_owned(),
            serde_json::Value::String(self.timeframe.clone()),
        );
        projection.insert("rules".to_owned(), rule_projection(&self.rules));
        if self.strategy_type == StrategyType::Exit {
            let mut managed: Vec<&str> =
                self.entry_strategy_ids.iter().map(String::as_str).collect();
            managed.sort_unstable();
            managed.dedup();
            projection.insert("manages".to_owned(), serde_json::json!(managed));
        }
        let canonical = canonicalize(&serde_json::Value::Object(projection));
        format!(
            "{RULES_FINGERPRINT_FORMAT}:{:x}",
            Sha256::digest(canonical.as_bytes())
        )
    }

    /// Whether a decision taken on inputs read at `decided_at` was taken under the rules
    /// this strategy holds now: its rules did not change after that moment.
    pub fn decided_under_current_rules(&self, decided_at: DateTime<Utc>) -> bool {
        self.rules_changed_at <= decided_at
    }

    /// Whether this exit strategy manages a position opened by `entry_strategy_id`
    /// (`None` for a position no entry strategy opened).
    pub fn manages(&self, entry_strategy_id: Option<&str>) -> bool {
        self.entry_strategy_ids.is_empty()
            || entry_strategy_id
                .is_some_and(|id| self.entry_strategy_ids.iter().any(|bound| bound == id))
    }
}

/// The projection format [`Strategy::rules_fingerprint`] hashes.
const RULES_FINGERPRINT_FORMAT: &str = "1";

/// The largest magnitude up to which every integer is exactly an `f64`: 2^53.
const EXACT_INTEGER_LIMIT: f64 = 9_007_199_254_740_992.0;

/// The part of a rule tree that decides: operator, children in order, the `disabled` flag,
/// and each condition's type and parameter values.
fn rule_projection(tree: &RuleTree) -> serde_json::Value {
    let mut node = serde_json::Map::new();
    if let Some(operator) = tree.operator {
        node.insert("operator".to_owned(), serde_json::json!(operator));
    }
    if let Some(children) = &tree.conditions {
        node.insert(
            "children".to_owned(),
            serde_json::Value::Array(children.iter().map(rule_projection).collect()),
        );
    }
    if let Some(condition) = &tree.condition {
        let parameters: serde_json::Map<String, serde_json::Value> = condition
            .parameters
            .iter()
            .map(|(name, parameter)| (name.clone(), value_projection(&parameter.value)))
            .collect();
        node.insert(
            "condition".to_owned(),
            serde_json::json!({
                "type": condition.condition_type,
                "parameters": parameters,
            }),
        );
    }
    node.insert(
        "disabled".to_owned(),
        serde_json::Value::Bool(tree.disabled),
    );
    serde_json::Value::Object(node)
}

/// `value` with every number in one form, so equal numbers compare equal: an integral
/// value within ±2^53 as an integer, any other as its shortest round-trip `f64`.
fn value_projection(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Number(number) => {
            let exact_integer = number
                .as_i64()
                .filter(|integer| integer.unsigned_abs() <= EXACT_INTEGER_LIMIT as u64);
            match (exact_integer, number.as_f64()) {
                (Some(integer), _) => serde_json::Value::from(integer),
                (None, Some(float))
                    if float.fract() == 0.0 && float.abs() <= EXACT_INTEGER_LIMIT =>
                {
                    serde_json::Value::from(float as i64)
                }
                (None, Some(float)) => serde_json::Value::from(float),
                (None, None) => value.clone(),
            }
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(value_projection).collect())
        }
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), value_projection(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Rule tree structure supporting logical operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTree {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<LogicalOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditions: Option<Vec<RuleTree>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
    /// A node the user switched off: kept in the strategy, skipped by evaluation.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}

impl RuleTree {
    /// Create a leaf node with a condition
    pub fn leaf(condition: Condition) -> Self {
        Self {
            operator: None,
            conditions: None,
            condition: Some(condition),
            disabled: false,
        }
    }

    /// Create a branch node with operator and children
    pub fn branch(operator: LogicalOperator, conditions: Vec<RuleTree>) -> Self {
        Self {
            operator: Some(operator),
            conditions: Some(conditions),
            condition: None,
            disabled: false,
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.condition.is_some()
    }

    /// Check if this is a branch node
    pub fn is_branch(&self) -> bool {
        self.operator.is_some()
    }
}

/// Logical operators for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

/// Condition definition with type and parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub parameters: HashMap<String, Parameter>,
}

/// Parameter with value and optional constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub value: serde_json::Value,
    pub default: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<ParameterConstraints>,
}

/// Parameter validation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterConstraints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Evaluation context providing data for condition evaluation
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    pub token_mint: String,
    pub current_price: Option<f64>,
    pub position_data: Option<PositionData>,
    pub market_data: Option<MarketData>,
    pub timeframe_bundle: Option<TimeframeBundle>,
    pub strategy_timeframe: String, // Which timeframe this strategy uses (from Strategy.timeframe)
    /// The instant this context describes — the moment `current_price` was read.
    ///
    /// Carried explicitly instead of each condition calling `Utc::now()`, because it is
    /// the only thing that can tell a live series from a dead one: candles are written
    /// only when a trade happens, so the newest candle in a bundle can be hours older
    /// than the price standing next to it. Conditions compare against this, never the
    /// wall clock, which also keeps them pure functions of their input.
    pub evaluated_at: DateTime<Utc>,
}

/// Position-related data for evaluation
#[derive(Debug, Clone)]
pub struct PositionData {
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub current_size_sol: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
    /// Highest pool price since the position's average entry price last changed.
    pub peak_price: Option<f64>,
    /// Additions booked on the position after its first entry.
    pub dca_count: u32,
    /// Partial exits booked on the position.
    pub partial_exit_count: u32,
    /// What the tokens still held are worth at the live pool price, in the native coin;
    /// `None` when the token's decimals or a pool price are unknown.
    pub remaining_value_native: Option<f64>,
    /// The pool's native reserves at the price the entry was made at; `None` for a
    /// position opened before that reading was recorded, imported or adopted.
    pub entry_liquidity_native: Option<f64>,
    /// The pool `entry_liquidity_native` was read from.
    pub entry_liquidity_pool: Option<String>,
}

/// Market-related data for evaluation
#[derive(Debug, Clone)]
pub struct MarketData {
    pub liquidity_sol: Option<f64>,
    /// The pool `liquidity_sol` was read from; `None` when the reading names no pool.
    pub liquidity_pool: Option<LiquidityPool>,
    /// Buy and sell counts per window from the cached token snapshot; `None` when the
    /// token is not cached.
    pub transactions: Option<TransactionCounts>,
    /// When the cached token snapshot's market data was fetched; `None` when the token is
    /// not cached.
    pub snapshot_at: Option<DateTime<Utc>>,
    /// When the token was created on-chain; `None` when that time is unknown.
    pub token_created_at: Option<DateTime<Utc>>,
    /// The trade size an entry would spend, in the native coin; `None` for an exit.
    pub reference_size_native: Option<f64>,
}

/// The pool a liquidity reading was taken from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidityPool {
    pub address: String,
    /// Whether the pool's decoder reports its native reserves as virtual reserves, as a
    /// bonding curve does; `None` when the pool's program is not known.
    pub virtual_reserves: Option<bool>,
}

/// Buy and sell transaction counts of one time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WindowCounts {
    pub buys: u64,
    pub sells: u64,
}

impl WindowCounts {
    /// Counts of a window, or `None` when either side is missing or negative: a window
    /// that was never reported is not a window with zero trades.
    pub fn from_reported(buys: Option<i64>, sells: Option<i64>) -> Option<Self> {
        Some(Self {
            buys: u64::try_from(buys?).ok()?,
            sells: u64::try_from(sells?).ok()?,
        })
    }

    /// Buys plus sells.
    pub fn total(&self) -> u64 {
        self.buys.saturating_add(self.sells)
    }
}

/// Buy and sell counts for the four windows the token store carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TransactionCounts {
    pub m5: Option<WindowCounts>,
    pub h1: Option<WindowCounts>,
    pub h6: Option<WindowCounts>,
    pub h24: Option<WindowCounts>,
}

impl TransactionCounts {
    /// The windows of a token snapshot.
    pub fn from_token(token: &Token) -> Self {
        Self {
            m5: WindowCounts::from_reported(token.txns_m5_buys, token.txns_m5_sells),
            h1: WindowCounts::from_reported(token.txns_h1_buys, token.txns_h1_sells),
            h6: WindowCounts::from_reported(token.txns_h6_buys, token.txns_h6_sells),
            h24: WindowCounts::from_reported(token.txns_h24_buys, token.txns_h24_sells),
        }
    }
}

/// Result of strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub strategy_id: String,
    pub result: bool,
    pub confidence: f64,
    pub execution_time_ms: u64,
    pub details: HashMap<String, serde_json::Value>,
}

/// What one node of a strategy's rule tree evaluated to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleVerdict {
    Pass,
    Fail,
    /// The node's data was absent, too short, stale or unusable; it is neither a pass
    /// nor a fail.
    Unknown,
    /// The node is switched off and was not evaluated.
    Off,
}

/// The evaluation of one rule tree node and of every node under it, in the order the
/// tree holds them: a group names its operator, a condition its type.
#[derive(Debug, Clone, Serialize)]
pub struct RuleOutcome {
    pub verdict: RuleVerdict,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<LogicalOperator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_type: Option<String>,
    /// Why the verdict is unknown: the data error the node reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<UiText>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<RuleOutcome>,
}

/// Every evaluation recorded for one strategy, summed. An evaluation is one decided
/// answer, signal or none, that a monitor used; a signal is an evaluation answered true.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EvaluationTotals {
    pub evaluations: u64,
    pub signals: u64,
    /// The evaluation time of every evaluation, in milliseconds.
    pub total_exec_ms: u64,
    pub last_evaluated_at: Option<DateTime<Utc>>,
    pub last_signal_at: Option<DateTime<Utc>>,
}

/// Strategy template for quick creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub risk_level: RiskLevel,
    pub rules: RuleTree,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub author: Option<String>,
}

/// Risk level classification for templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
        }
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
