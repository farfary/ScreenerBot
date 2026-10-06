// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Domain values for paper and confirmation-gated live copy trading.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::chains::ChainId;
use crate::trader::policy::ExitPolicyOverrides;

/// Execution mode. Entering `Live` requires the dedicated confirmation-gated mode
/// transition before the runtime can submit a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyMode {
    Paper,
    Live,
}

/// Explicit acknowledgement required by the dedicated mode-transition endpoint.
pub const LIVE_ARM_CONFIRMATION: &str = "ARM LIVE COPY TRADING";

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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTask {
    pub id: i64,
    pub chain: ChainId,
    pub target_address: String,
    pub label: Option<String>,
    pub enabled: bool,
    pub mode: CopyMode,
    pub sizing: SizingMode,
    pub exit_mode: ExitMode,
    pub exit_policy_overrides: ExitPolicyOverrides,
    #[serde(rename = "max_sol_per_trade")]
    pub max_native_per_trade: f64,
    #[serde(rename = "max_sol_per_token")]
    pub max_native_per_token: f64,
    #[serde(rename = "total_budget_sol")]
    pub total_budget_native: f64,
    #[serde(rename = "min_target_trade_sol")]
    pub min_target_trade_native: Option<f64>,
    #[serde(rename = "max_target_trade_sol")]
    pub max_target_trade_native: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Per-task override of `copy_trading.require_filter_pass`; `None` inherits.
    #[serde(default)]
    pub require_filter_pass: Option<bool>,
    #[serde(default)]
    pub pause_reason: Option<CopyPauseReason>,
    #[serde(default)]
    pub paused_at: Option<DateTime<Utc>>,
}

impl CopyTask {
    pub fn requires_filter_pass(&self, global: bool) -> bool {
        self.require_filter_pass.unwrap_or(global)
    }
}

/// API/repository creation input. Server-assigned identity and timestamps cannot be
/// forged by callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTaskInput {
    pub target_address: String,
    pub label: Option<String>,
    pub enabled: bool,
    pub mode: CopyMode,
    pub sizing: SizingMode,
    pub exit_mode: ExitMode,
    #[serde(default)]
    pub exit_policy_overrides: ExitPolicyOverrides,
    #[serde(rename = "max_sol_per_trade")]
    pub max_native_per_trade: f64,
    #[serde(rename = "max_sol_per_token")]
    pub max_native_per_token: f64,
    #[serde(rename = "total_budget_sol")]
    pub total_budget_native: f64,
    #[serde(rename = "min_target_trade_sol")]
    pub min_target_trade_native: Option<f64>,
    #[serde(rename = "max_target_trade_sol")]
    pub max_target_trade_native: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
    #[serde(default)]
    pub require_filter_pass: Option<bool>,
}

impl CopyTaskInput {
    pub fn into_task(self, chain: ChainId, now: DateTime<Utc>) -> Result<CopyTask, CopySkip> {
        if self.mode != CopyMode::Paper {
            return Err(CopySkip::ModeTransitionRequired);
        }
        self.into_task_with_mode(chain, now, CopyMode::Paper)
    }

    pub fn into_task_for_update(
        self,
        chain: ChainId,
        now: DateTime<Utc>,
        current_mode: CopyMode,
    ) -> Result<CopyTask, CopySkip> {
        if self.mode != current_mode {
            return Err(CopySkip::ModeTransitionRequired);
        }
        self.into_task_with_mode(chain, now, current_mode)
    }

    fn into_task_with_mode(
        self,
        chain: ChainId,
        now: DateTime<Utc>,
        mode: CopyMode,
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
        if !self.exit_policy_overrides.is_valid() {
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
        }
    }
}

pub fn confirm_mode_transition(
    current: CopyMode,
    requested: CopyMode,
    confirmation: Option<&str>,
) -> Result<CopyMode, CopySkip> {
    if current == requested || requested == CopyMode::Paper {
        return Ok(requested);
    }
    if confirmation != Some(LIVE_ARM_CONFIRMATION) {
        return Err(CopySkip::LiveConfirmationRequired);
    }
    Ok(CopyMode::Live)
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
    /// A gap-fill replayed this trade after downtime; copying it now would trade
    /// on a price that no longer exists.
    StaleObservation {
        arrival_ms: u64,
        threshold_ms: u64,
    },
    /// A replayed observation has no block time, so its age cannot be bounded.
    UnknownObservationTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperFill {
    pub input_sol: f64,
    pub market_price_sol: f64,
    /// `market_price_sol` is the pool's price, not the observed trade's own.
    #[serde(default)]
    pub priced_from_pool: bool,
    pub fill_price_sol: f64,
    pub token_amount: f64,
    pub referral_fee_sol: f64,
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
    pub total_cost_sol: f64,
}

/// A simulated sell against the paper book: the tokens it held, sold at the
/// decision-time price less slippage, the referral fee and network costs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperSellFill {
    pub token_amount: f64,
    pub market_price_sol: f64,
    /// `market_price_sol` is the pool's price, not the observed trade's own.
    #[serde(default)]
    pub priced_from_pool: bool,
    pub fill_price_sol: f64,
    pub gross_sol: f64,
    pub referral_fee_sol: f64,
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
    pub net_proceeds_sol: f64,
}

/// One task's paper holding in one token, accumulated across its buys and sells.
/// `cost_basis_native` is the cost of the tokens still held; realized figures only
/// ever grow.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PaperPosition {
    pub task_id: i64,
    pub mint: String,
    pub token_amount: f64,
    #[serde(rename = "cost_basis_sol")]
    pub cost_basis_native: f64,
    #[serde(rename = "invested_sol")]
    pub invested_native: f64,
    #[serde(rename = "realized_proceeds_sol")]
    pub realized_proceeds_native: f64,
    #[serde(rename = "realized_cost_sol")]
    pub realized_cost_native: f64,
    pub buys: u64,
    pub sells: u64,
    #[serde(rename = "last_price_sol")]
    pub last_price_native: Option<f64>,
    pub last_price_at: Option<DateTime<Utc>>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// Highest pool price seen while this round was open; arms the trailing stop.
    #[serde(rename = "peak_price_sol")]
    pub peak_price_native: Option<f64>,
}

impl PaperPosition {
    pub fn is_open(&self) -> bool {
        self.closed_at.is_none() && self.token_amount > 0.0
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
pub struct PaperDecision {
    pub task_id: i64,
    pub target_address: String,
    pub signature: String,
    pub mint: String,
    pub target_size_sol: f64,
    #[serde(default)]
    pub target_token_amount: f64,
    pub sized_sol: f64,
    pub fill: PaperFill,
    pub telemetry: CopyTelemetry,
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
}

/// The exit-policy rule that sold a paper holding, mirroring the live exit
/// monitor's reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperExitRule {
    StopLoss,
    TrailingStop,
    TakeProfit,
    TimeOverride,
    /// Closed by hand from the dashboard or an agent tool.
    Manual,
}

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
    #[serde(rename = "realized_pnl_sol")]
    pub realized_pnl_native: f64,
    #[serde(rename = "unrealized_pnl_sol")]
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
mod tests {
    use super::*;

    #[test]
    fn watch_budget_pause_exposes_a_distinct_dashboard_reason() {
        let reason = CopyPauseReason::WatchBudgetExceeded {
            page_budget: 8,
            signatures_checked: 800,
        };
        assert_eq!(
            serde_json::to_value(reason).unwrap(),
            serde_json::json!({
                "kind": "watch_budget_exceeded",
                "page_budget": 8,
                "signatures_checked": 800
            })
        );
    }

    #[test]
    fn helius_pause_exposes_a_distinct_dashboard_reason() {
        assert_eq!(
            serde_json::to_value(CopyPauseReason::HeliusUnavailable).unwrap(),
            serde_json::json!({"kind": "helius_unavailable"})
        );
    }

    #[test]
    fn processing_pause_exposes_a_distinct_dashboard_reason() {
        assert_eq!(
            serde_json::to_value(CopyPauseReason::WatchProcessingFailed).unwrap(),
            serde_json::json!({"kind": "watch_processing_failed"})
        );
    }

    fn input() -> CopyTaskInput {
        CopyTaskInput {
            target_address: "target".to_owned(),
            label: None,
            enabled: true,
            mode: CopyMode::Paper,
            sizing: SizingMode::Fixed { sol: 0.05 },
            exit_mode: ExitMode::BuyOnly,
            exit_policy_overrides: Default::default(),
            max_native_per_trade: 0.1,
            max_native_per_token: 0.5,
            total_budget_native: 2.0,
            min_target_trade_native: None,
            max_target_trade_native: None,
            buy_once_per_token: true,
            slippage_pct: 2.0,
            require_filter_pass: None,
        }
    }

    fn validate(input: CopyTaskInput) -> Result<CopyTask, CopySkip> {
        input.into_task(ChainId::Solana, Utc::now())
    }

    #[test]
    fn a_task_that_could_never_place_a_copy_is_refused() {
        let minimum_native = crate::trader::constants::MIN_TRADE_SIZE_NATIVE;
        assert!(validate(input()).is_ok());
        let tiny_size = CopyTaskInput {
            sizing: SizingMode::Fixed { sol: 0.0005 },
            ..input()
        };
        assert_eq!(
            validate(tiny_size),
            Err(CopySkip::BelowMinimumSize {
                minimum_sol: minimum_native
            })
        );
        let tiny_cap = CopyTaskInput {
            sizing: SizingMode::RatioOfTarget { pct: 10.0 },
            max_native_per_trade: 0.0005,
            ..input()
        };
        assert_eq!(
            validate(tiny_cap),
            Err(CopySkip::BelowMinimumSize {
                minimum_sol: minimum_native
            })
        );
    }

    #[test]
    fn slippage_below_the_copy_minimum_is_refused() {
        let low = CopyTaskInput {
            slippage_pct: MIN_COPY_SLIPPAGE_PCT / 2.0,
            ..input()
        };
        assert!(matches!(
            validate(low),
            Err(CopySkip::InvalidSlippage { .. })
        ));
        let lowest = CopyTaskInput {
            slippage_pct: MIN_COPY_SLIPPAGE_PCT,
            ..input()
        };
        assert!(validate(lowest).is_ok());
    }

    #[test]
    fn legacy_skip_without_telemetry_still_decodes() {
        let json = r#"{
            "outcome":"skipped",
            "task_id":1,
            "signature":"target-signature",
            "mint":"mint",
            "reason":{"kind":"task_disabled"},
            "decided_at":"2026-01-01T00:00:00Z"
        }"#;
        let outcome: CopyOutcome = serde_json::from_str(json).unwrap();
        let CopyOutcome::Skipped { telemetry, .. } = outcome else {
            panic!("expected skipped outcome")
        };
        assert_eq!(telemetry, None);
    }

    /// Catalog key of the label for each skip kind. The match is exhaustive, so a
    /// new variant fails to compile until it is mapped here and in `SKIP_LABELS`
    /// (pages/copy/format.js).
    fn skip_label_key(skip: &CopySkip) -> &'static str {
        match skip {
            CopySkip::NotBuySwap => "copy-skip-not-buy-swap",
            CopySkip::NotSellSwap => "copy-skip-not-sell-swap",
            CopySkip::ExitModeDisabled => "copy-skip-exit-mode-disabled",
            CopySkip::ForceStopped => "copy-skip-force-stopped",
            CopySkip::LatencyKillSwitch { .. } => "copy-skip-latency-kill-switch",
            CopySkip::ClaimReconciledAbandoned => "copy-skip-claim-reconciled-abandoned",
            CopySkip::CopyPositionNotFound => "copy-skip-copy-position-not-found",
            CopySkip::PositionUserOnly => "copy-skip-position-user-only",
            CopySkip::PositionManagementMismatch => "copy-skip-position-management-mismatch",
            CopySkip::TaskDisabled => "copy-skip-task-disabled",
            CopySkip::ModeTransitionRequired => "copy-skip-mode-transition-required",
            CopySkip::LiveConfirmationRequired => "copy-skip-live-confirmation-required",
            CopySkip::UnsupportedSizingMode => "copy-skip-unsupported-sizing-mode",
            CopySkip::SelfCopy => "copy-skip-self-copy",
            CopySkip::TargetBelowMinimum { .. } => "copy-skip-target-below-minimum",
            CopySkip::TargetAboveMaximum { .. } => "copy-skip-target-above-maximum",
            CopySkip::AlreadyBought => "copy-skip-already-bought",
            CopySkip::Blacklisted => "copy-skip-blacklisted",
            CopySkip::FilterRequired => "copy-skip-filter-required",
            CopySkip::EntryBlocked { .. } => "copy-skip-entry-blocked",
            CopySkip::BudgetExhausted => "copy-skip-budget-exhausted",
            CopySkip::TokenCapReached => "copy-skip-token-cap-reached",
            CopySkip::BelowMinimumSize { .. } => "copy-skip-below-minimum-size",
            CopySkip::InvalidSizing => "copy-skip-invalid-sizing",
            CopySkip::InvalidSlippage { .. } => "copy-skip-invalid-slippage",
            CopySkip::InvalidExitPolicy => "copy-skip-invalid-exit-policy",
            CopySkip::InvalidPrice => "copy-skip-invalid-price",
            CopySkip::StaleObservation { .. } => "copy-skip-stale-observation",
            CopySkip::UnknownObservationTime => "copy-skip-unknown-observation-time",
        }
    }

    #[test]
    fn copy_skip_labels_exist_in_the_catalog() {
        let skips = [
            CopySkip::NotBuySwap,
            CopySkip::NotSellSwap,
            CopySkip::ExitModeDisabled,
            CopySkip::ForceStopped,
            CopySkip::LatencyKillSwitch {
                average_ms: 0,
                threshold_ms: 0,
            },
            CopySkip::ClaimReconciledAbandoned,
            CopySkip::CopyPositionNotFound,
            CopySkip::PositionUserOnly,
            CopySkip::PositionManagementMismatch,
            CopySkip::TaskDisabled,
            CopySkip::ModeTransitionRequired,
            CopySkip::LiveConfirmationRequired,
            CopySkip::UnsupportedSizingMode,
            CopySkip::SelfCopy,
            CopySkip::TargetBelowMinimum { minimum_sol: 0.0 },
            CopySkip::TargetAboveMaximum { maximum_sol: 0.0 },
            CopySkip::AlreadyBought,
            CopySkip::Blacklisted,
            CopySkip::FilterRequired,
            CopySkip::EntryBlocked {
                block: crate::trader::admission::EntryBlock::ForceStopped,
            },
            CopySkip::BudgetExhausted,
            CopySkip::TokenCapReached,
            CopySkip::BelowMinimumSize { minimum_sol: 0.0 },
            CopySkip::InvalidSizing,
            CopySkip::InvalidSlippage { maximum_pct: 0.0 },
            CopySkip::InvalidExitPolicy,
            CopySkip::InvalidPrice,
            CopySkip::StaleObservation {
                arrival_ms: 0,
                threshold_ms: 0,
            },
            CopySkip::UnknownObservationTime,
        ];
        for skip in &skips {
            let value = serde_json::to_value(skip).unwrap();
            let id = value["kind"].as_str().expect("skip kind");
            let key = skip_label_key(skip);
            assert_eq!(
                key,
                format!("copy-skip-{}", id.replace('_', "-")),
                "key does not follow the serialized id {id}"
            );
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
        }
    }
}
