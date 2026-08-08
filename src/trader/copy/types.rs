//! Domain values for paper and confirmation-gated live copy trading.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Execution mode. `Live` is persisted for forward-compatible task intent, but the
/// Phase 2c pipeline rejects it before any trading code is reachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyMode {
    Paper,
    Live,
}

/// Explicit acknowledgement required by the dedicated mode-transition endpoint.
pub const LIVE_ARM_CONFIRMATION: &str = "ARM LIVE COPY TRADING";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CopyTask {
    pub id: i64,
    pub target_address: String,
    pub label: Option<String>,
    pub enabled: bool,
    pub mode: CopyMode,
    pub sizing: SizingMode,
    pub exit_mode: ExitMode,
    pub max_sol_per_trade: f64,
    pub max_sol_per_token: f64,
    pub total_budget_sol: f64,
    pub min_target_trade_sol: Option<f64>,
    pub max_target_trade_sol: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub max_sol_per_trade: f64,
    pub max_sol_per_token: f64,
    pub total_budget_sol: f64,
    pub min_target_trade_sol: Option<f64>,
    pub max_target_trade_sol: Option<f64>,
    pub buy_once_per_token: bool,
    pub slippage_pct: f64,
}

impl CopyTaskInput {
    pub fn into_task(self, now: DateTime<Utc>) -> Result<CopyTask, CopySkip> {
        if self.mode != CopyMode::Paper {
            return Err(CopySkip::ModeTransitionRequired);
        }
        self.into_task_with_mode(now, CopyMode::Paper)
    }

    pub fn into_task_for_update(
        self,
        now: DateTime<Utc>,
        current_mode: CopyMode,
    ) -> Result<CopyTask, CopySkip> {
        if self.mode != current_mode {
            return Err(CopySkip::ModeTransitionRequired);
        }
        self.into_task_with_mode(now, current_mode)
    }

    fn into_task_with_mode(self, now: DateTime<Utc>, mode: CopyMode) -> Result<CopyTask, CopySkip> {
        if self.target_address.trim().is_empty()
            || !self.max_sol_per_trade.is_finite()
            || self.max_sol_per_trade <= 0.0
            || !self.max_sol_per_token.is_finite()
            || self.max_sol_per_token <= 0.0
            || !self.total_budget_sol.is_finite()
            || self.total_budget_sol <= 0.0
            || self.max_sol_per_trade > self.max_sol_per_token
            || self.max_sol_per_token > self.total_budget_sol
        {
            return Err(CopySkip::InvalidSizing);
        }
        if matches!(&self.sizing, SizingMode::PercentOfTargetPortfolio { .. }) {
            return Err(CopySkip::UnsupportedSizingMode);
        }
        let valid_optional_limit =
            |value: Option<f64>| value.is_none_or(|amount| amount.is_finite() && amount >= 0.0);
        if !valid_optional_limit(self.min_target_trade_sol)
            || !valid_optional_limit(self.max_target_trade_sol)
            || self
                .min_target_trade_sol
                .zip(self.max_target_trade_sol)
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
        if !self.slippage_pct.is_finite()
            || self.slippage_pct <= 0.0
            || self.slippage_pct > crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT
        {
            return Err(CopySkip::InvalidSlippage {
                maximum_pct: crate::trader::constants::MAX_MANUAL_SLIPPAGE_PCT,
            });
        }
        Ok(CopyTask {
            id: 0,
            target_address: self.target_address,
            label: self.label,
            enabled: self.enabled,
            mode,
            sizing: self.sizing,
            exit_mode: self.exit_mode,
            max_sol_per_trade: self.max_sol_per_trade,
            max_sol_per_token: self.max_sol_per_token,
            total_budget_sol: self.total_budget_sol,
            min_target_trade_sol: self.min_target_trade_sol,
            max_target_trade_sol: self.max_target_trade_sol,
            buy_once_per_token: self.buy_once_per_token,
            slippage_pct: self.slippage_pct,
            created_at: now,
            updated_at: now,
        })
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
    pub total_spent_sol: f64,
    pub token_spent_sol: f64,
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
    pub engine_trade_size_sol: f64,
}

/// Every declined copy is a value suitable for persistence and UI dictionaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CopySkip {
    NotBuySwap,
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
    InvalidPrice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperFill {
    pub input_sol: f64,
    pub market_price_sol: f64,
    pub fill_price_sol: f64,
    pub token_amount: f64,
    pub referral_fee_sol: f64,
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
    pub total_cost_sol: f64,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperDecision {
    pub task_id: i64,
    pub target_address: String,
    pub signature: String,
    pub mint: String,
    pub target_size_sol: f64,
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
    pub sized_sol: f64,
    pub transaction_signature: Option<String>,
    pub error: Option<String>,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CopyOutcome {
    PaperFilled(PaperDecision),
    LiveSubmitted(LiveDecision),
    LiveConfirmed(LiveDecision),
    LiveFailed(LiveDecision),
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
            Self::Skipped { task_id, .. } => *task_id,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
