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
    /// Trade size in SOL — for BUY and DCA only.
    ///
    /// It does NOT carry a sell size: a Sell's size is a percentage of the position, and
    /// that lives in [`TradeDecision::exit_percentage`].
    pub size_sol: Option<f64>,
    /// How much of the position a SELL exits, in percent (0, 100].
    ///
    /// `None` = a full exit. This used to be smuggled through `size_sol` ("Use size_sol for
    /// percentage"), so the same field meant SOL on a buy and a PERCENTAGE on a sell —
    /// silently, with nothing to catch a value put in the wrong one. A 50 in `size_sol` was
    /// either half a position or 50 SOL depending only on the action.
    pub exit_percentage: Option<f64>,
    /// Per-trade slippage override, in percent.
    ///
    /// `None` = follow the configured slippage (`swaps.slippage.*`), which is what the
    /// AUTO-TRADER always does — it must stay config-driven. Manual and explicitly
    /// configured copy tasks may set a bounded override.
    pub slippage_pct: Option<f64>,
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
    CopyBuy,
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
    CopySell,
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
    /// The swap has a signature but the router confirmation poll timed out. It must
    /// be reconciled, never retried as a fresh buy.
    pub confirmation_pending: bool,
    /// Set when this failure was the global position-capacity guard
    /// (`positions::Error::SlotUnavailable`) rather than an execution failure, so
    /// callers can react to it structurally instead of pattern-matching `error`.
    pub capacity_guard_remaining: Option<usize>,
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
            confirmation_pending: false,
            capacity_guard_remaining: None,
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
            confirmation_pending: false,
            capacity_guard_remaining: None,
        }
    }
}
