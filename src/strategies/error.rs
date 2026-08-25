//! Errors produced by the strategies module.

use std::time::Duration;

use crate::errors::{ErrorClass, Severity};

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

    /// A rule tree is structurally invalid.
    #[error("invalid rule tree: {reason}")]
    InvalidRuleTree { reason: &'static str },

    /// Evaluation did not complete before its configured deadline.
    #[error("strategy evaluation timed out after {timeout_ms}ms")]
    EvaluationTimeout { timeout_ms: u64 },
}

/// Result alias for the strategies module.
pub type Result<T> = std::result::Result<T, Error>;

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
            | Error::InvalidRuleTree { .. } => Severity::Warning,
            Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientCandles { .. }
            | Error::InsufficientHistory { .. }
            | Error::StaleCandleData { .. }
            | Error::EvaluationTimeout { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::MissingConditionParameter { .. }
            | Error::ConditionParameterType { .. }
            | Error::InvalidConditionValue { .. }
            | Error::InvalidRuleTree { .. } => 400,
            Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientCandles { .. }
            | Error::InsufficientHistory { .. }
            | Error::StaleCandleData { .. } => 424,
            Error::EvaluationTimeout { .. } => 504,
        }
    }
}
