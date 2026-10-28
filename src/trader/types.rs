// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Core trader types and structures.

use crate::positions::Book;
use crate::trader::paper::{PaperExecution, PaperLegEvidence, PaperLegFill};
use crate::trader::trade_reason::TradeReason;
use chrono::{DateTime, Utc};

/// Represents a decision to trade
#[derive(Debug, Clone)]
pub struct TradeDecision {
    pub position_id: Option<String>,
    pub mint: String,
    pub action: TradeAction,
    pub reason: TradeReason,
    pub strategy_id: Option<String>,
    /// The book the decision trades on. Required with no default, so every producer
    /// chooses: a paper decision is executed only by the paper executor.
    pub book: Book,
    /// When the inputs this decision was decided on were read. A strategy entry carries
    /// the read time of its cycle's strategies, so the paper executor can tell a buy
    /// decided under rules that have since changed.
    pub timestamp: DateTime<Utc>,
    pub priority: TradePriority,
    pub price_native: Option<f64>,
    /// Trade size in SOL — for BUY and DCA only.
    ///
    /// It does NOT carry a sell size: a Sell's size is a percentage of the position, and
    /// that lives in [`TradeDecision::exit_percentage`].
    pub size_native: Option<f64>,
    /// How much of the position a SELL exits, in percent (0, 100].
    ///
    /// `None` = a full exit. This used to be smuggled through `size_native` ("Use size_native for
    /// percentage"), so the same field meant SOL on a buy and a PERCENTAGE on a sell —
    /// silently, with nothing to catch a value put in the wrong one. A 50 in `size_native` was
    /// either half a position or 50 SOL depending only on the action.
    pub exit_percentage: Option<f64>,
    /// The take-profit step a SELL takes (1 is the first step). A partial sell reserves the
    /// step before its swap is sent, so the ladder never sells one step twice; a full sell
    /// reserves nothing and fires again while its level holds until the close is booked.
    /// `None` on every other decision.
    pub take_profit_level: Option<u32>,
    /// Per-trade slippage override, in percent.
    ///
    /// `None` = follow the configured slippage (`trader.slippage.*`), which is what the
    /// AUTO-TRADER always does — it must stay config-driven. Manual and explicitly
    /// configured copy tasks may set a bounded override.
    pub slippage_pct: Option<f64>,
    /// The key a paper fill of this decision is booked under, so the decision books once
    /// however often it is replayed. Set only on paper decisions whose producer can replay
    /// them; `None` books under a fresh reference.
    pub paper_leg_key: Option<String>,
}

/// The entry decisions of one token in one entry cycle: at most one per book. A decision
/// is filed only by its own `book`, so the slot that holds it is always the book it trades
/// on and a live decision can never be routed as a paper one, or the reverse.
#[derive(Debug, Default)]
pub struct EntryDecisions {
    live: Option<TradeDecision>,
    paper: Option<TradeDecision>,
}

impl EntryDecisions {
    /// Files `decision` under its own book, replacing an earlier decision of that book.
    pub fn insert(&mut self, decision: TradeDecision) {
        match decision.book {
            Book::Live => self.live = Some(decision),
            Book::Paper => self.paper = Some(decision),
        }
    }

    /// The live and the paper decision, in that order.
    pub fn into_books(self) -> (Option<TradeDecision>, Option<TradeDecision>) {
        (self.live, self.paper)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeAction {
    Buy,
    Sell,
    DCA,
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
    pub executed_price_native: Option<f64>,
    pub executed_size_native: Option<f64>,
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
    /// Set when the decision was withdrawn before anything was quoted or sent, because
    /// what decided it no longer trades, so callers record a deliberate stop rather than
    /// an execution failure.
    pub withdrawn: Option<Withdrawal>,
    /// Which step of the trade actually failed, set by the executor that was
    /// running it. `None` on success.
    pub failed_step: Option<TradeStep>,
    /// Why the swap stopped before its transaction was sent, when it did, so
    /// a caller can explain the failure from its type rather than its text.
    pub not_submitted: Option<crate::swaps::NotSubmittedReason>,
    /// What the paper executor booked or why it refused, on every result of a paper
    /// decision; `None` on a live one.
    pub paper: Option<PaperExecution>,
}

/// Why a decision was withdrawn before its trade started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Withdrawal {
    /// The trader, or its automatic entries, were switched off after the decision was
    /// taken.
    TraderStopped,
    /// The deciding strategy was switched off, moved to the other book or removed after
    /// the decision was taken.
    StrategyStoppedTrading,
    /// The deciding strategy's rules changed after the entry was decided, so the round it
    /// would open was decided under rules the strategy no longer has.
    RulesChanged,
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
    /// built or submitted; `SwapFailed` and `PartialExitHeld` mean it was.
    /// Anything else failed before either — capacity, sizing, wallet,
    /// persistence — and belongs to validation.
    fn trade_step(&self) -> TradeStep {
        match self {
            crate::positions::Error::QuoteFailed { .. } => TradeStep::Quote,
            crate::positions::Error::SwapFailed { .. }
            | crate::positions::Error::PartialExitHeld { .. } => TradeStep::Swap,
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
    /// Create a successful trade result. A price or size that is `None`, not finite or not
    /// positive is stored as `None`: the value is not known until verification.
    pub fn success(
        decision: TradeDecision,
        tx_signature: String,
        executed_price_native: Option<f64>,
        executed_size_native: Option<f64>,
        position_id: Option<String>,
    ) -> Self {
        let known = |value: Option<f64>| value.filter(|v| v.is_finite() && *v > 0.0);
        Self {
            decision,
            success: true,
            tx_signature: Some(tx_signature),
            executed_price_native: known(executed_price_native),
            executed_size_native: known(executed_size_native),
            error: None,
            position_id,
            execution_timestamp: Utc::now(),
            retry_count: 0,
            confirmation_pending: false,
            capacity_guard_remaining: None,
            withdrawn: None,
            failed_step: None,
            not_submitted: None,
            paper: None,
        }
    }

    /// A paper leg booked onto the paper book: a success that sent no transaction, so it
    /// carries no signature. The price and size are the leg's fill price and its SOL in
    /// (a buy) or net proceeds (a sale).
    pub fn paper(decision: TradeDecision, evidence: PaperLegEvidence) -> Self {
        let (price, size) = match &evidence.fill {
            PaperLegFill::Buy(fill) => (fill.fill_price_sol, fill.input_sol),
            PaperLegFill::Sell(fill) => (fill.fill_price_sol, fill.net_proceeds_sol),
        };
        let mut result = Self::success(
            decision,
            String::new(),
            Some(price),
            Some(size),
            Some(evidence.position_id.to_string()),
        );
        result.tx_signature = None;
        result.paper = Some(PaperExecution::Filled(evidence));
        result
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
            executed_price_native: None,
            executed_size_native: None,
            error: Some(error),
            position_id: None,
            execution_timestamp: Utc::now(),
            retry_count,
            confirmation_pending: false,
            capacity_guard_remaining: None,
            withdrawn: None,
            failed_step: Some(step),
            not_submitted: None,
            paper: None,
        }
    }

    /// A failed result for a positions error, carrying the step it ended and,
    /// for a swap that was never sent, why.
    pub fn failure_from(
        decision: TradeDecision,
        error: &crate::positions::Error,
        message: String,
    ) -> Self {
        let mut result = Self::failure_at(decision, error.trade_step(), message, 0);
        result.not_submitted = error.not_submitted_reason().cloned();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision() -> TradeDecision {
        TradeDecision {
            position_id: None,
            mint: "Mint".to_owned(),
            action: TradeAction::Buy,
            reason: TradeReason::ManualEntry,
            strategy_id: None,
            book: Book::Live,
            timestamp: Utc::now(),
            priority: TradePriority::Normal,
            price_native: None,
            size_native: None,
            slippage_pct: None,
            exit_percentage: None,
            take_profit_level: None,
            paper_leg_key: None,
        }
    }

    /// Each book's entry decision is filed under that book whatever the order it arrives
    /// in, and a later decision of a book replaces the earlier one.
    #[test]
    fn each_entry_decision_is_filed_under_its_own_book() {
        let of = |book: Book, strategy: &str| TradeDecision {
            book,
            strategy_id: Some(strategy.to_owned()),
            ..decision()
        };
        let mut decisions = EntryDecisions::default();
        decisions.insert(of(Book::Paper, "paper"));
        decisions.insert(of(Book::Live, "live"));
        let (live, paper) = decisions.into_books();
        let live = live.expect("the live decision");
        let paper = paper.expect("the paper decision");
        assert_eq!(
            (live.book, live.strategy_id.as_deref()),
            (Book::Live, Some("live"))
        );
        assert_eq!(
            (paper.book, paper.strategy_id.as_deref()),
            (Book::Paper, Some("paper"))
        );

        let mut paper_only = EntryDecisions::default();
        paper_only.insert(of(Book::Paper, "first"));
        paper_only.insert(of(Book::Paper, "second"));
        let (live, paper) = paper_only.into_books();
        assert!(live.is_none(), "a paper decision reached the live slot");
        assert_eq!(paper.and_then(|d| d.strategy_id).as_deref(), Some("second"));
    }

    /// A fill price or size that is zero, non-finite or unknown is never stored as a value.
    #[test]
    fn a_trade_result_keeps_only_real_prices_and_sizes() {
        let unknown = TradeResult::success(
            decision(),
            "sig".to_owned(),
            Some(0.0),
            Some(f64::NAN),
            None,
        );
        assert_eq!(unknown.executed_price_native, None);
        assert_eq!(unknown.executed_size_native, None);
        let negative = TradeResult::success(decision(), "sig".to_owned(), Some(-1.0), None, None);
        assert_eq!(negative.executed_price_native, None);
        assert_eq!(negative.executed_size_native, None);
        let real = TradeResult::success(
            decision(),
            "sig".to_owned(),
            Some(0.000_5),
            Some(0.2),
            Some("7".to_owned()),
        );
        assert_eq!(real.executed_price_native, Some(0.000_5));
        assert_eq!(real.executed_size_native, Some(0.2));
        assert_eq!(real.position_id.as_deref(), Some("7"));
    }

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
            not_submitted: None,
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
