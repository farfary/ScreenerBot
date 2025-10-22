use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Strategy definition with rule tree and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy_type: StrategyType,
    pub enabled: bool,
    pub priority: i32,
    pub rules: RuleTree,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: Option<String>,
    pub version: i32,
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
}

impl RuleTree {
    /// Create a leaf node with a condition
    pub fn leaf(condition: Condition) -> Self {
        Self {
            operator: None,
            conditions: None,
            condition: Some(condition),
        }
    }

    /// Create a branch node with operator and children
    pub fn branch(operator: LogicalOperator, conditions: Vec<RuleTree>) -> Self {
        Self {
            operator: Some(operator),
            conditions: Some(conditions),
            condition: None,
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
    pub ohlcv_data: Option<OhlcvData>,
}

/// Position-related data for evaluation
#[derive(Debug, Clone)]
pub struct PositionData {
    pub entry_price: f64,
    pub entry_time: DateTime<Utc>,
    pub current_size_sol: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
}

/// Market-related data for evaluation
#[derive(Debug, Clone)]
pub struct MarketData {
    pub liquidity_sol: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub holder_count: Option<u32>,
    pub token_age_hours: Option<f64>,
}

/// OHLCV data for technical analysis
#[derive(Debug, Clone)]
pub struct OhlcvData {
    pub candles: Vec<Candle>,
    pub timeframe: String,
}

#[derive(Debug, Clone)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
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

/// Performance tracking for strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPerformance {
    pub strategy_id: String,
    pub total_evaluations: u64,
    pub successful_signals: u64,
    pub avg_execution_time_ms: f64,
    pub last_evaluation: DateTime<Utc>,
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
