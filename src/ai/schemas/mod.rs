mod exit_suggestion;
mod filter_decision;
mod trade_decision;

pub use exit_suggestion::{ExitFactor, ExitSuggestion, ExitUrgency};
pub use filter_decision::{FilterAction, FilterDecision, FilterFactor};
pub use trade_decision::{TradeAction, TradeDecision, TradeFactor};

/// Validate JSON response against expected schema
pub fn validate_json_response<T: serde::de::DeserializeOwned>(json_str: &str) -> Result<T, String> {
    serde_json::from_str(json_str).map_err(|e| format!("Failed to parse AI response: {}", e))
}
