// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Errors produced by the strategies module: validation, evaluation and arming.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};
use crate::strategies::types::StrategyType;

/// Everything that can go wrong while validating or evaluating strategies.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// A required condition parameter is absent from the saved strategy.
    #[error("condition parameter {field} is missing")]
    MissingConditionParameter { field: &'static str },

    /// A condition parameter is present but of the wrong JSON type. Kept
    /// separate from a missing parameter so a caller can tell "never set" from
    /// "set to something unusable": coercing a string "20" to 0 would silently
    /// turn a 20-period moving average into a division by zero.
    #[error("condition parameter {field} must be {expected}")]
    ConditionParameterType {
        field: &'static str,
        expected: &'static str,
    },

    /// A configured condition parameter or enum value is present and of the
    /// right type, but outside the range or set the condition accepts.
    #[error("'{value}' is not a valid {field}")]
    InvalidConditionValue { field: &'static str, value: String },

    /// Required context data was absent for a condition evaluation.
    #[error("{data} is not available")]
    MissingContextData { data: &'static str },

    /// The requested candle timeframe has no data.
    #[error("timeframe {timeframe} has no candle data")]
    NoCandleData { timeframe: String },

    /// The candle series does not reach far enough back for a *time-based*
    /// lookback. Measured in seconds rather than candles: the rule asks for the
    /// price N seconds ago, and whether that can be answered depends on the span
    /// the series covers, not on how many candles fill it.
    #[error(
        "not enough history for {indicator}: {available_seconds}s available, \
         {required_seconds}s needed"
    )]
    InsufficientHistory {
        indicator: &'static str,
        available_seconds: i64,
        required_seconds: i64,
    },

    /// The available candle series is too short for an indicator.
    #[error("not enough candles for {indicator}: have {available}, need {required}")]
    InsufficientCandles {
        indicator: &'static str,
        available: usize,
        required: usize,
    },

    /// The candle series is too old to compare with the live price.
    #[error(
        "timeframe {timeframe} candle data is stale: age {age_seconds}s exceeds {max_age_seconds}s"
    )]
    StaleCandleData {
        timeframe: String,
        age_seconds: i64,
        max_age_seconds: i64,
    },

    /// The transaction counts of the market snapshot are older than the window they
    /// describe, so they no longer say what happened over that window.
    #[error(
        "the {window} transaction counts are stale: age {age_seconds}s exceeds {max_age_seconds}s"
    )]
    StaleMarketData {
        window: String,
        age_seconds: i64,
        max_age_seconds: i64,
    },

    /// A data value reached a condition but no comparison can use it: a zero or
    /// non-finite price or basis. It describes the market, not the configuration.
    #[error("{field} value {value} cannot be used")]
    UnusableData { field: &'static str, value: String },

    /// A rule tree is structurally invalid.
    #[error("invalid rule tree: {reason}")]
    InvalidRuleTree { reason: &'static str },

    /// A condition is used in a strategy type it cannot read the data for.
    #[error("{condition_type} cannot be used in a {strategy_type} strategy")]
    ConditionNotApplicable {
        condition_type: String,
        strategy_type: StrategyType,
    },

    /// Evaluation did not complete before its configured deadline.
    #[error("strategy evaluation timed out after {timeout_ms}ms")]
    EvaluationTimeout { timeout_ms: u64 },

    /// A strategy priority outside the range strategies are ordered in.
    #[error(
        "priority {priority} is outside {min}-{max}",
        min = crate::strategies::types::MIN_PRIORITY,
        max = crate::strategies::types::MAX_PRIORITY
    )]
    PriorityOutOfRange { priority: i32 },

    /// Arming a strategy live needs the exact confirmation phrase.
    #[error("arming a strategy live needs the exact confirmation phrase")]
    LiveConfirmationRequired,

    /// Arming a strategy live is refused while the Auto Trader cannot trade live: setup
    /// incomplete, Explore Mode, or the emergency stop engaged.
    #[error("live trading is unavailable: {reason:?}")]
    LiveUnavailable { reason: crate::trader::LiveBlock },
}

/// Result alias for the strategies module.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Whether the error says the data needed to answer a condition is absent, too short,
    /// stale or unusable, rather than that the strategy itself is wrong. Inside a rule tree
    /// such a condition is unknown; every other error stops the evaluation.
    pub fn is_data_unavailable(&self) -> bool {
        match self {
            Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientHistory { .. }
            | Error::InsufficientCandles { .. }
            | Error::StaleCandleData { .. }
            | Error::StaleMarketData { .. }
            | Error::UnusableData { .. } => true,
            Error::MissingConditionParameter { .. }
            | Error::ConditionParameterType { .. }
            | Error::InvalidConditionValue { .. }
            | Error::InvalidRuleTree { .. }
            | Error::ConditionNotApplicable { .. }
            | Error::EvaluationTimeout { .. }
            | Error::PriorityOutOfRange { .. }
            | Error::LiveConfirmationRequired
            | Error::LiveUnavailable { .. } => false,
        }
    }
}

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        matches!(self, Error::EvaluationTimeout { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::EvaluationTimeout { .. } => Some(Duration::from_millis(250)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::MissingConditionParameter { .. }
            | Error::ConditionParameterType { .. }
            | Error::InvalidConditionValue { .. }
            | Error::InvalidRuleTree { .. }
            | Error::ConditionNotApplicable { .. }
            | Error::PriorityOutOfRange { .. }
            | Error::LiveConfirmationRequired
            | Error::LiveUnavailable { .. } => Severity::Warning,
            Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientCandles { .. }
            | Error::InsufficientHistory { .. }
            | Error::StaleCandleData { .. }
            | Error::StaleMarketData { .. }
            | Error::UnusableData { .. }
            | Error::EvaluationTimeout { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::MissingConditionParameter { .. }
            | Error::ConditionParameterType { .. }
            | Error::InvalidConditionValue { .. }
            | Error::InvalidRuleTree { .. }
            | Error::ConditionNotApplicable { .. }
            | Error::PriorityOutOfRange { .. }
            | Error::LiveConfirmationRequired => 400,
            Error::LiveUnavailable { .. } => 409,
            Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientCandles { .. }
            | Error::InsufficientHistory { .. }
            | Error::StaleCandleData { .. }
            | Error::StaleMarketData { .. }
            | Error::UnusableData { .. } => 424,
            Error::EvaluationTimeout { .. } => 504,
        }
    }
}
