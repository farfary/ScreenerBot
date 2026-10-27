// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Domain values for paper and confirmation-gated live copy trading.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::chains::ChainId;
use crate::trader::policy::{ExitPolicy, ExitPolicyOverrides};

use crate::trader::paper::{
    confirm_live, PaperFillError, PaperModeError, PaperSellFill, TradingMode, LIVE_ARM_CONFIRMATION,
};

mod paper;
pub use paper::{PaperContext, PaperDecision, PaperExitRule, PaperPosition};

/// The lowest slippage a task or the copy default may run with; the one bound
/// behind task validation, the pipeline's precheck and the config section.
pub const MIN_COPY_SLIPPAGE_PCT: f64 = 0.1;

/// How this bot derives its SOL input from the target's SOL input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SizingMode {
    Fixed {
        sol: f64,
    },
    RatioOfTarget {
        pct: f64,
    },
    /// Reserved for V2: a current target portfolio is required to size safely.
    PercentOfTargetPortfolio {
        pct: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitMode {
    BuyOnly,
    Mirror,
    Hybrid,
}

/// Why a task stopped copying, stored with the pause so the task list can tell
/// a user pause from a guard that stood the task down.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopyPauseReason {
    User,
    LatencyKillSwitch {
        average_ms: u64,
        threshold_ms: u64,
    },
    WatchDetached,
    WatchBudgetExceeded {
        page_budget: usize,
        signatures_checked: usize,
    },
    HeliusUnavailable,
    WatchProcessingFailed,
    /// The copy store opened with restored or recreated contents, so budgets, claims and
    /// task settings may predate the failure. Cleared only by an explicit resume.
    StorageRestored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTask {
    pub id: i64,
    pub chain: ChainId,
    pub target_address: String,
    pub label: Option<String>,
    pub enabled: bool,
    pub mode: TradingMode,
    pub sizing: SizingMode,
    pub exit_mode: ExitMode,
    pub exit_policy_overrides: ExitPolicyOverrides,
    pub max_native_per_trade: f64,
    pub max_native_per_token: f64,
    pub total_budget_native: f64,
    pub min_target_trade_native: Option<f64>,
    pub max_target_trade_native: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Per-task override of `copy_trading.require_filter_pass`; `None` inherits.
    #[serde(default)]
    pub require_filter_pass: Option<bool>,
    /// Per-task override of `copy_trading.copy_stable_quoted`; `None` inherits.
    #[serde(default)]
    pub copy_stable_quoted: Option<bool>,
    #[serde(default)]
    pub pause_reason: Option<CopyPauseReason>,
    #[serde(default)]
    pub paused_at: Option<DateTime<Utc>>,
}

impl CopyTask {
    pub fn requires_filter_pass(&self, global: bool) -> bool {
        self.require_filter_pass.unwrap_or(global)
    }

    /// Whether wallet buys paid in a stable coin are copied.
    pub fn copies_stable_quoted(&self, global: bool) -> bool {
        self.copy_stable_quoted.unwrap_or(global)
    }
}

/// API/repository creation input. Server-assigned identity and timestamps cannot be
/// forged by callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTaskInput {
    pub target_address: String,
    pub label: Option<String>,
    pub enabled: bool,
    pub mode: TradingMode,
    pub sizing: SizingMode,
    pub exit_mode: ExitMode,
    #[serde(default)]
    pub exit_policy_overrides: ExitPolicyOverrides,
    pub max_native_per_trade: f64,
    pub max_native_per_token: f64,
    pub total_budget_native: f64,
    pub min_target_trade_native: Option<f64>,
    pub max_target_trade_native: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
    #[serde(default)]
    pub require_filter_pass: Option<bool>,
    #[serde(default)]
    pub copy_stable_quoted: Option<bool>,
}

impl CopyTaskInput {
    /// Validate a new task. `exit_base` is the global exit policy the task's exit
    /// overrides are layered onto; the merged policy must pass its own validation.
    pub fn into_task(
        self,
        chain: ChainId,
        now: DateTime<Utc>,
        exit_base: ExitPolicy,
    ) -> Result<CopyTask, CopySkip> {
        if self.mode != TradingMode::Paper {
            return Err(CopySkip::ModeTransitionRequired);
        }
        self.into_task_with_mode(chain, now, TradingMode::Paper, Some(exit_base))
    }

    /// Validate an update of a stored task. The exit overrides are checked over
    /// `exit_base` only when they differ from `stored_overrides`: a global change can
    /// leave stored overrides that no longer fit the policy they inherit, and that task
    /// must still pause, resume and take every other edit.
    pub fn into_task_for_update(
        self,
        chain: ChainId,
        now: DateTime<Utc>,
        current_mode: TradingMode,
        stored_overrides: &ExitPolicyOverrides,
        exit_base: ExitPolicy,
    ) -> Result<CopyTask, CopySkip> {
        if self.mode != current_mode {
            return Err(CopySkip::ModeTransitionRequired);
        }
        let exit_base = (self.exit_policy_overrides != *stored_overrides).then_some(exit_base);
        self.into_task_with_mode(chain, now, current_mode, exit_base)
    }

    fn into_task_with_mode(
        self,
        chain: ChainId,
        now: DateTime<Utc>,
        mode: TradingMode,
        exit_base: Option<ExitPolicy>,
    ) -> Result<CopyTask, CopySkip> {
        if self.target_address.trim().is_empty()
            || !self.max_native_per_trade.is_finite()
            || self.max_native_per_trade <= 0.0
            || !self.max_native_per_token.is_finite()
            || self.max_native_per_token <= 0.0
            || !self.total_budget_native.is_finite()
            || self.total_budget_native <= 0.0
            || self.max_native_per_trade > self.max_native_per_token
            || self.max_native_per_token > self.total_budget_native
        {
            return Err(CopySkip::InvalidSizing);
        }
        if matches!(&self.sizing, SizingMode::PercentOfTargetPortfolio { .. }) {
            return Err(CopySkip::UnsupportedSizingMode);
        }
        let valid_optional_limit =
            |value: Option<f64>| value.is_none_or(|amount| amount.is_finite() && amount >= 0.0);
        if !valid_optional_limit(self.min_target_trade_native)
            || !valid_optional_limit(self.max_target_trade_native)
            || self
                .min_target_trade_native
                .zip(self.max_target_trade_native)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            return Err(CopySkip::InvalidSizing);
        }
        match &self.sizing {
            SizingMode::Fixed { sol } if !sol.is_finite() || *sol <= 0.0 => {
                return Err(CopySkip::InvalidSizing);
            }
            SizingMode::RatioOfTarget { pct } if !pct.is_finite() || *pct <= 0.0 => {
                return Err(CopySkip::InvalidSizing);
            }
            _ => {}
        }
        // Sizing clamps every copy to the per-trade cap and refuses one below the
        // minimum trade size, so a smaller cap or fixed size could never copy.
        let minimum_native = crate::trader::constants::MIN_TRADE_SIZE_NATIVE;
        if self.max_native_per_trade < minimum_native
            || matches!(self.sizing, SizingMode::Fixed { sol } if sol < minimum_native)
        {
            return Err(CopySkip::BelowMinimumSize {
                minimum_sol: minimum_native,
            });
        }
        if !self.slippage_pct.is_finite()
            || self.slippage_pct < MIN_COPY_SLIPPAGE_PCT
            || self.slippage_pct > crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT
        {
            return Err(CopySkip::InvalidSlippage {
                maximum_pct: crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT,
            });
        }
        if let Some(Err(fault)) =
            exit_base.map(|base| self.exit_policy_overrides.validate_over(base))
        {
            crate::logger::info(
                crate::logger::LogTag::Trader,
                &format!(
                    "Copy task for {} refused: exit overrides {fault}",
                    self.target_address
                ),
            );
            return Err(CopySkip::InvalidExitPolicy);
        }
        Ok(CopyTask {
            id: 0,
            chain,
            target_address: self.target_address,
            label: self.label,
            enabled: self.enabled,
            mode,
            sizing: self.sizing,
            exit_mode: self.exit_mode,
            exit_policy_overrides: self.exit_policy_overrides,
            max_native_per_trade: self.max_native_per_trade,
            max_native_per_token: self.max_native_per_token,
            total_budget_native: self.total_budget_native,
            min_target_trade_native: self.min_target_trade_native,
            max_target_trade_native: self.max_target_trade_native,
            buy_once_per_token: self.buy_once_per_token,
            slippage_pct: self.slippage_pct,
            created_at: now,
            updated_at: now,
            require_filter_pass: self.require_filter_pass,
            copy_stable_quoted: self.copy_stable_quoted,
            pause_reason: (!self.enabled).then_some(CopyPauseReason::User),
            paused_at: (!self.enabled).then_some(now),
        })
    }
}

impl From<&CopyTask> for CopyTaskInput {
    /// The editable surface of a stored task, so a partial update can be merged
    /// onto it and re-validated through the same path as a full one.
    fn from(task: &CopyTask) -> Self {
        Self {
            target_address: task.target_address.clone(),
            label: task.label.clone(),
            enabled: task.enabled,
            mode: task.mode,
            sizing: task.sizing.clone(),
            exit_mode: task.exit_mode,
            exit_policy_overrides: task.exit_policy_overrides.clone(),
            max_native_per_trade: task.max_native_per_trade,
            max_native_per_token: task.max_native_per_token,
            total_budget_native: task.total_budget_native,
            min_target_trade_native: task.min_target_trade_native,
            max_target_trade_native: task.max_target_trade_native,
            buy_once_per_token: task.buy_once_per_token,
            slippage_pct: task.slippage_pct,
            require_filter_pass: task.require_filter_pass,
            copy_stable_quoted: task.copy_stable_quoted,
        }
    }
}

/// The copy task's mode transition: the shared live-arming gate with the copy phrase.
pub fn confirm_mode_transition(
    current: TradingMode,
    requested: TradingMode,
    confirmation: Option<&str>,
) -> Result<TradingMode, CopySkip> {
    confirm_live(current, requested, confirmation, LIVE_ARM_CONFIRMATION).map_err(|error| {
        match error {
            PaperModeError::LiveConfirmationRequired => CopySkip::LiveConfirmationRequired,
        }
    })
}

/// Persisted spend state used by risk and sizing. Values are cumulative and must be
/// updated atomically with the successful paper decision.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpendState {
    pub total_spent_native: f64,
    pub token_spent_native: f64,
    pub token_buy_count: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RiskContext {
    pub is_self_wallet: bool,
    pub mint_blacklisted: bool,
    pub filter_passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PipelinePolicy {
    pub require_filter_pass: bool,
    pub engine_trade_size_native: f64,
}

/// Every declined copy is a value suitable for persistence and UI dictionaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopySkip {
    NotBuySwap,
    NotSellSwap,
    ExitModeDisabled,
    ForceStopped,
    LatencyKillSwitch {
        average_ms: u64,
        threshold_ms: u64,
    },
    ClaimReconciledAbandoned,
    CopyPositionNotFound,
    PositionUserOnly,
    PositionManagementMismatch,
    TaskDisabled,
    ModeTransitionRequired,
    LiveConfirmationRequired,
    UnsupportedSizingMode,
    SelfCopy,
    TargetBelowMinimum {
        minimum_sol: f64,
    },
    TargetAboveMaximum {
        maximum_sol: f64,
    },
    AlreadyBought,
    Blacklisted,
    FilterRequired,
    EntryBlocked {
        block: crate::trader::admission::EntryBlock,
    },
    BudgetExhausted,
    TokenCapReached,
    BelowMinimumSize {
        minimum_sol: f64,
    },
    InvalidSizing,
    InvalidSlippage {
        maximum_pct: f64,
    },
    InvalidExitPolicy,
    InvalidPrice,
    PaperQuoteUnavailable,
    PaperQuoteUnfillable,
    PaperMinimumOutputNotMet,
    PaperQueueFull,
    PaperExecutionUnavailable,
    /// A gap-fill replayed this trade after downtime; copying it now would trade
    /// on a price that no longer exists.
    StaleObservation {
        arrival_ms: u64,
        threshold_ms: u64,
    },
    /// A replayed observation has no block time, so its age cannot be bounded.
    UnknownObservationTime,
    /// The wallet paid in a stable coin and the task copies native-quoted buys only.
    StableQuoteDisabled,
    /// A stable-quoted trade could not be sized: no fresh native price.
    StableQuotePriceUnavailable,
    /// The wallet paid with native SOL and a stable coin in one trade.
    SwapMixedQuote,
    /// The wallet traded more than one token in one transaction.
    SwapMultipleTokens,
    /// The wallet's trade amounts could not be read from the transaction.
    SwapUnreadable,
}

/// A fill-model refusal keeps the skip code copy has always persisted for it.
impl From<PaperFillError> for CopySkip {
    fn from(error: PaperFillError) -> Self {
        match error {
            PaperFillError::InvalidSize => Self::InvalidSizing,
            PaperFillError::EmptySell => Self::CopyPositionNotFound,
            PaperFillError::QuoteUnavailable => Self::PaperQuoteUnavailable,
            PaperFillError::QuoteUnfillable => Self::PaperQuoteUnfillable,
            PaperFillError::MinimumOutputNotMet => Self::PaperMinimumOutputNotMet,
        }
    }
}

impl From<crate::wallets::watch::UnresolvedSwapReason> for CopySkip {
    fn from(reason: crate::wallets::watch::UnresolvedSwapReason) -> Self {
        use crate::wallets::watch::UnresolvedSwapReason;
        match reason {
            UnresolvedSwapReason::MixedQuote => Self::SwapMixedQuote,
            UnresolvedSwapReason::MultipleTokens => Self::SwapMultipleTokens,
            UnresolvedSwapReason::Unreadable => Self::SwapUnreadable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTelemetry {
    pub target_block_time: Option<i64>,
    pub detected_at: DateTime<Utc>,
    pub decoded_at: DateTime<Utc>,
    pub decided_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub target_price_sol: Option<f64>,
    pub fill_price_sol: Option<f64>,
    /// The observation came from a gap-fill replay; its arrival distance measures
    /// downtime and never feeds the latency kill switch.
    #[serde(default)]
    pub backfill: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveDecision {
    pub task_id: i64,
    pub target_address: String,
    pub target_signature: String,
    pub mint: String,
    pub target_size_sol: f64,
    #[serde(default)]
    pub target_token_amount: f64,
    pub sized_sol: f64,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopySellDecision {
    pub task_id: i64,
    pub target_address: String,
    pub target_signature: String,
    pub mint: String,
    pub target_token_amount: f64,
    pub target_sol_amount: f64,
    /// `None` means the V1 full-close path. Proportional target sizing is Phase 5.
    pub exit_percentage: Option<f64>,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub telemetry: CopyTelemetry,
    /// The simulated sell a paper task booked. `None` on live sells.
    #[serde(default)]
    pub paper_fill: Option<PaperSellFill>,
    /// Set when the task's own exit policy closed a paper holding rather than a
    /// mirrored target sell.
    #[serde(default)]
    pub exit_rule: Option<PaperExitRule>,
    /// The take-profit step a policy sale of a paper holding takes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take_profit_level: Option<u32>,
    #[serde(default)]
    pub paper_context: Option<PaperContext>,
}

/// The exit-policy rule that sold a paper holding, mirroring the live exit
/// monitor's reasons.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CopyOutcome {
    PaperFilled(PaperDecision),
    LiveSubmitted(LiveDecision),
    LiveConfirmed(LiveDecision),
    LiveFailed(LiveDecision),
    PaperSellObserved(CopySellDecision),
    LiveSellSubmitted(CopySellDecision),
    LiveSellFailed(CopySellDecision),
    Skipped {
        task_id: i64,
        signature: String,
        mint: Option<String>,
        reason: CopySkip,
        decided_at: DateTime<Utc>,
        #[serde(default)]
        telemetry: Option<CopyTelemetry>,
        #[serde(default)]
        paper: Option<PaperContext>,
    },
}

impl CopyOutcome {
    pub fn task_id(&self) -> i64 {
        match self {
            Self::PaperFilled(decision) => decision.task_id,
            Self::LiveSubmitted(decision)
            | Self::LiveConfirmed(decision)
            | Self::LiveFailed(decision) => decision.task_id,
            Self::PaperSellObserved(decision)
            | Self::LiveSellSubmitted(decision)
            | Self::LiveSellFailed(decision) => decision.task_id,
            Self::Skipped { task_id, .. } => *task_id,
        }
    }

    pub fn telemetry(&self) -> Option<&CopyTelemetry> {
        match self {
            Self::PaperFilled(decision) => Some(&decision.telemetry),
            Self::LiveSubmitted(decision)
            | Self::LiveConfirmed(decision)
            | Self::LiveFailed(decision) => Some(&decision.telemetry),
            Self::PaperSellObserved(decision)
            | Self::LiveSellSubmitted(decision)
            | Self::LiveSellFailed(decision) => Some(&decision.telemetry),
            Self::Skipped { telemetry, .. } => telemetry.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyActivityRow {
    pub id: i64,
    pub task_id: i64,
    pub kind: String,
    pub outcome: CopyOutcome,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ArrivalDistanceStats {
    pub samples: usize,
    pub minimum_ms: Option<u64>,
    pub median_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub maximum_ms: Option<u64>,
    pub average_ms: Option<u64>,
}

/// Which book a task's position and P&L figures come from: its paper ledger while
/// it runs in paper mode, the real positions it opened while live.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyBook {
    Paper,
    #[default]
    Live,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct CopyTaskStats {
    pub task_id: i64,
    pub decisions: usize,
    pub filled_buys: usize,
    /// Sells mirrored from the target wallet.
    pub target_sells: usize,
    /// Paper sells made by the task's own exit policy.
    pub policy_exits: usize,
    /// Paper holdings closed by hand.
    pub manual_closes: usize,
    /// Closed rounds with a positive / non-positive realized result.
    pub wins: usize,
    pub losses: usize,
    pub skipped: usize,
    pub submitted: usize,
    pub failed: usize,
    pub open_positions: usize,
    pub closed_positions: usize,
    pub realized_pnl_native: f64,
    pub unrealized_pnl_native: f64,
    pub book: CopyBook,
    /// Open positions with no price to mark them at; excluded from unrealized P&L.
    pub unpriced_positions: usize,
    pub arrival_distance: ArrivalDistanceStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimReconciliation {
    Confirmed,
    Submitted,
    Abandoned,
}

#[cfg(test)]
#[path = "types/tests.rs"]
mod tests;
