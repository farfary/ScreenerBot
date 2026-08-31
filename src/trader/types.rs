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
    LlmAnalysisExit, // Model-scored exit recommendation
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
    /// Which step of the trade actually failed, set by the executor that was
    /// running it. `None` on success.
    pub failed_step: Option<TradeStep>,
}

/// The stages a trade passes through, in order.
///
/// The manual-trade action timeline shows the user where a trade stopped. That
/// used to be guessed by five separate copies of the same message search
/// (`error.contains("Quote")`, `contains("No routes")`), which attributed a
/// failure to the wrong step as soon as any wording changed — and reported the
/// SWAP as failed when nothing had been submitted. The executor knows which
/// step it was in, so it says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeStep {
    /// Pre-flight: connectivity, capacity, sizing, blacklist.
    Validation,
    /// Asking the routers what the trade would cost.
    Quote,
    /// Building, signing and submitting the swap.
    Swap,
}

/// An error that can say which step of a trade it ended.
///
/// One mapping, stated once, for every error channel a trade can fail through.
/// The five copies this replaced each searched the message for `"Quote"` and
/// disagreed with each other.
pub trait FailedTradeStep {
    fn trade_step(&self) -> TradeStep;
}

impl FailedTradeStep for crate::positions::Error {
    /// `QuoteFailed` means no router would price the trade, so nothing was
    /// built or submitted; `SwapFailed` means it was. Anything else failed
    /// before either — capacity, sizing, wallet, persistence — and belongs to
    /// validation.
    fn trade_step(&self) -> TradeStep {
        match self {
            crate::positions::Error::QuoteFailed { .. } => TradeStep::Quote,
            crate::positions::Error::SwapFailed { .. } => TradeStep::Swap,
            _ => TradeStep::Validation,
        }
    }
}

impl FailedTradeStep for crate::trader::Error {
    fn trade_step(&self) -> TradeStep {
        match self {
            crate::trader::Error::Positions(e) => e.trade_step(),
            _ => TradeStep::Validation,
        }
    }
}

impl FailedTradeStep for crate::Error {
    fn trade_step(&self) -> TradeStep {
        match self {
            crate::Error::Positions(e) => e.trade_step(),
            crate::Error::Trader(e) => e.trade_step(),
            _ => TradeStep::Validation,
        }
    }
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
            failed_step: None,
        }
    }

    /// Create a failed trade result, naming the step that failed.
    pub fn failure_at(
        decision: TradeDecision,
        step: TradeStep,
        error: String,
        retry_count: u32,
    ) -> Self {
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
            failed_step: Some(step),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trade that never got a quote must not be reported as a failed SWAP.
    /// The prose ladder this replaced did exactly that, telling the user a
    /// transaction had been attempted when nothing had been submitted.
    #[test]
    fn a_quote_failure_is_never_attributed_to_the_swap() {
        let quote_failed = crate::positions::Error::QuoteFailed {
            mint: "So11111111111111111111111111111111111111112".to_owned(),
            detail: "no router would price it".to_owned(),
        };
        assert_eq!(quote_failed.trade_step(), TradeStep::Quote);
        assert_eq!(
            crate::Error::Positions(quote_failed).trade_step(),
            TradeStep::Quote
        );
    }

    /// A failure after submission stays attributed to the swap, whichever
    /// channel it arrives through.
    #[test]
    fn a_swap_failure_is_attributed_to_the_swap_through_every_channel() {
        let mint = "So11111111111111111111111111111111111111112".to_owned();
        let swap_failed = || crate::positions::Error::SwapFailed {
            mint: mint.clone(),
            detail: "submitted and reverted".to_owned(),
        };
        assert_eq!(swap_failed().trade_step(), TradeStep::Swap);
        assert_eq!(
            crate::trader::Error::Positions(swap_failed()).trade_step(),
            TradeStep::Swap
        );
        assert_eq!(
            crate::Error::Trader(crate::trader::Error::Positions(swap_failed())).trade_step(),
            TradeStep::Swap
        );
    }

    /// Anything that failed before quoting belongs to validation — never to a
    /// step the trade did not reach.
    #[test]
    fn a_pre_quote_failure_belongs_to_validation() {
        assert_eq!(
            crate::positions::Error::SlotUnavailable { remaining: 0 }.trade_step(),
            TradeStep::Validation
        );
        assert_eq!(
            crate::positions::Error::DcaDisabled.trade_step(),
            TradeStep::Validation
        );
    }
}
