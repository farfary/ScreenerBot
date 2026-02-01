//! Core trader types and structures

use chrono::{DateTime, Utc};

/// Represents a decision to trade
#[derive(Debug, Clone)]
pub struct TradeDecision {
    pub position_id: Option<String>,
    pub mint: String,
    pub action: TradeAction,
    pub reason: TradeReason,
    pub strategy_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub priority: TradePriority,
    pub price_sol: Option<f64>,
    pub size_sol: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeAction {
    Buy,
    Sell,
    DCA,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeReason {
    // Entry reasons
    StrategySignal,
    ManualEntry,
    ForceBuy,
    DCAScheduled,

    // Exit reasons
    TakeProfit,
    StopLoss,
    TrailingStop,
    TimeOverride,
    StrategyExit,
    AiExit, // AI-powered exit recommendation
    ManualExit,
    RiskManagement,
    Blacklisted,
    ForceSell,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TradePriority {
    Emergency, // Immediate execution (stop loss, blacklist)
    High,      // Next available execution slot
    Normal,    // Standard execution
    Low,       // Can be delayed if needed
}

#[derive(Debug, Clone)]
pub struct TradeResult {
    pub decision: TradeDecision,
    pub success: bool,
    pub tx_signature: Option<String>,
    pub executed_price_sol: Option<f64>,
    pub executed_size_sol: Option<f64>,
    pub error: Option<String>,
    pub position_id: Option<String>,
    pub execution_timestamp: DateTime<Utc>,
    pub retry_count: u32,
}

impl TradeResult {
    /// Create a successful trade result
    pub fn success(
        decision: TradeDecision,
        tx_signature: String,
        executed_price_sol: f64,
        executed_size_sol: f64,
        position_id: Option<String>,
    ) -> Self {
        Self {
            decision,
            success: true,
            tx_signature: Some(tx_signature),
            executed_price_sol: Some(executed_price_sol),
            executed_size_sol: Some(executed_size_sol),
            error: None,
            position_id,
            execution_timestamp: Utc::now(),
            retry_count: 0,
        }
    }

    /// Create a failed trade result
    pub fn failure(decision: TradeDecision, error: String, retry_count: u32) -> Self {
        Self {
            decision,
            success: false,
            tx_signature: None,
            executed_price_sol: None,
            executed_size_sol: None,
            error: Some(error),
            position_id: None,
            execution_timestamp: Utc::now(),
            retry_count,
        }
    }
}
