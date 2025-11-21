use crate::strategies::conditions::{get_candles_from_context, get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Detect price breakouts from recent high/low levels
pub struct PriceBreakoutCondition;

#[async_trait]
impl ConditionEvaluator for PriceBreakoutCondition {
    fn condition_type(&self) -> &'static str {
        "PriceBreakout"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let lookback = get_param_f64(condition, "lookback")? as usize;
        let direction = get_param_string(condition, "direction")?;
        let confirmation = get_param_f64(condition, "confirmation")?;

        let candles = get_candles_from_context(context)?;

        if candles.len() < lookback {
            return Err(format!(
                "Not enough candles: {} < {}",
                candles.len(),
                lookback
            ));
        }

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        // Get lookback candles (excluding current)
        let end_idx = candles.len().saturating_sub(1);
        let start_idx = end_idx.saturating_sub(lookback);
        let lookback_candles = &candles[start_idx..end_idx];

        if lookback_candles.is_empty() {
            return Err("No lookback candles available".to_string());
        }

        // Find highest high and lowest low in lookback period
        let period_high = lookback_candles
            .iter()
            .map(|c| c.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let period_low = lookback_candles
            .iter()
            .map(|c| c.low)
            .fold(f64::INFINITY, f64::min);

        let result = match direction.as_str() {
            "UPWARD" => {
                // Upward breakout: price breaks above period high + confirmation%
                let breakout_level = period_high * (1.0 + confirmation / 100.0);
                current_price >= breakout_level
            }
            "DOWNWARD" => {
                // Downward breakout: price breaks below period low - confirmation%
                let breakout_level = period_low * (1.0 - confirmation / 100.0);
                current_price <= breakout_level
            }
            _ => return Err(format!("Invalid direction: {}", direction)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let lookback = get_param_f64(condition, "lookback")?;
        if lookback < 2.0 || lookback > 100.0 {
            return Err("Lookback must be between 2 and 100".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["UPWARD", "DOWNWARD"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        let confirmation = get_param_f64(condition, "confirmation")?;
        if confirmation < 0.0 || confirmation > 20.0 {
            return Err("Confirmation must be between 0 and 20%".to_string());
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceBreakout",
            "name": "Price Breakout",
            "category": "Price Analysis",
            "tags": ["breakout", "resistance", "support", "momentum"],
            "icon": "🚀",
            "origin": "strategy",
            "description": "Detect price breaking above resistance (period high) or below support (period low)",
            "parameters": {
                "lookback": {
                    "type": "number",
                    "name": "Lookback Period",
                    "description": "Number of candles to find support/resistance level",
                    "default": 20,
                    "min": 2,
                    "max": 100,
                    "step": 1
                },
                "direction": {
                    "type": "enum",
                    "name": "Breakout Direction",
                    "description": "Direction of the breakout",
                    "default": "UPWARD",
                    "options": [
                        { "value": "UPWARD", "label": "Upward (Resistance Break)" },
                        { "value": "DOWNWARD", "label": "Downward (Support Break)" }
                    ]
                },
                "confirmation": {
                    "type": "percent",
                    "name": "Confirmation %",
                    "description": "How far past the level to confirm breakout (avoids false signals)",
                    "default": 1.0,
                    "min": 0.0,
                    "max": 20.0,
                    "step": 0.5
                }
            }
        })
    }
}
