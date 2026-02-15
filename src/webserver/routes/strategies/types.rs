use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub author: Option<String>,
    pub version: i32,
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
#[derive(Debug, Deserialize)]
pub struct StrategyTestRequest {
    pub token_mint: String,
    pub current_price: f64,
    #[serde(default)]
    pub market_data: Option<TestMarketData>,
    #[serde(default)]
    pub position_data: Option<TestPositionData>,
    #[serde(default)]
    pub ohlcv_data: Option<TestOhlcvData>,
}

#[derive(Debug, Deserialize)]
pub struct TestMarketData {
    pub liquidity_sol: Option<f64>,
    pub volume_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub holder_count: Option<u32>,
    pub token_age_hours: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct TestPositionData {
    pub entry_price: f64,
    pub entry_time: String,
    pub current_size_sol: f64,
    pub unrealized_profit_pct: Option<f64>,
    pub position_age_hours: f64,
}

#[derive(Debug, Deserialize)]
pub struct TestOhlcvData {
    pub candles: Vec<TestCandle>,
    pub timeframe: String,
}

#[derive(Debug, Deserialize)]
pub struct TestCandle {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Strategy test response
#[derive(Debug, Serialize)]
pub struct StrategyTestResponse {
    pub strategy_id: String,
    pub strategy_name: String,
    pub result: bool,
    pub confidence: f64,
    pub execution_time_ms: u64,
    pub details: HashMap<String, serde_json::Value>,
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
}

fn default_enabled() -> bool {
    true
}

fn default_priority() -> i32 {
    10
}

fn default_timeframe() -> String {
    "5m".to_string()
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
    pub timestamp: String,
}

/// Strategy templates list response
#[derive(Debug, Serialize)]
pub struct StrategyTemplatesResponse {
    pub items: Vec<StrategyTemplateItem>,
    pub total: usize,
    pub timestamp: String,
}

/// Strategy template item
#[derive(Debug, Serialize)]
pub struct StrategyTemplateItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub risk_level: String,
    pub rules: serde_json::Value,
    pub parameters: HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub author: Option<String>,
}
