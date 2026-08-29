//! Typed response schemas for structured model decisions (filter, trade, exit).
mod exit_suggestion;
mod filter_decision;
mod trade_decision;

use crate::errors::DataError;
use crate::llm_analysis::error::{Error, Result};

pub use exit_suggestion::{ExitFactor, ExitSuggestion, ExitUrgency};
pub use filter_decision::{FilterAction, FilterDecision, FilterFactor};
pub use trade_decision::{TradeAction, TradeDecision, TradeFactor};

/// Validate JSON response against expected schema
pub fn validate_json_response<T: serde::de::DeserializeOwned>(json_str: &str) -> Result<T> {
    serde_json::from_str(json_str).map_err(|e| {
        Error::Data(DataError::ParseError {
            data_type: "AI response".to_owned(),
            error: e.to_string(),
        })
    })
}
