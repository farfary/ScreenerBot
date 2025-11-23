use crate::strategies::conditions::{get_candles_for_timeframe, get_param_f64, get_param_string, get_param_string_optional, validate_timeframe_param, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Check for consecutive green (bullish) or red (bearish) candles
pub struct ConsecutiveCandlesCondition;

#[async_trait]
impl ConditionEvaluator for ConsecutiveCandlesCondition {
    fn condition_type(&self) -> &'static str {
        "ConsecutiveCandles"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let count = get_param_f64(condition, "count")? as usize;
        let direction = get_param_string(condition, "direction")?;
        let minimum_change = get_param_f64(condition, "minimum_change")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.len() < count {
            return Err(format!(
                "Not enough candles: {} < {}",
                candles.len(),
                count
            ));
        }

        // Get the most recent candles
        let recent_candles = &candles[candles.len() - count..];

        // Check for consecutive pattern
        let mut consecutive_count = 0;
        for candle in recent_candles {
            let price_change_pct = ((candle.close - candle.open) / candle.open) * 100.0;

            let is_match = match direction.as_str() {
                "GREEN" => price_change_pct >= minimum_change,
                "RED" => price_change_pct <= -minimum_change,
                _ => return Err(format!("Invalid direction: {}", direction)),
            };

            if is_match {
                consecutive_count += 1;
            } else {
                // Reset if pattern breaks
                consecutive_count = 0;
            }
        }

        Ok(consecutive_count >= count)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let count = get_param_f64(condition, "count")?;;
        if count < 2.0 || count > 20.0 {
            return Err("Count must be between 2 and 20".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["GREEN", "RED"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        let minimum_change = get_param_f64(condition, "minimum_change")?;
        if minimum_change < 0.0 {
            return Err("Minimum change must be non-negative".to_string());
        }
        if minimum_change > 50.0 {
            return Err("Minimum change must be 50% or less".to_string());
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "ConsecutiveCandles",
            "name": "Consecutive Candles",
            "category": "Candle Patterns",
            "tags": ["candles", "pattern", "momentum", "streak"],
            "icon": "icon-chart-candlestick",
            "origin": "strategy",
            "description": "Detect consecutive green (bullish) or red (bearish) candles with minimum size filter",
            "parameters": {
                "timeframe": {
                    "type": "enum",
                    "name": "Timeframe",
                    "description": "Candle timeframe to analyze (defaults to strategy timeframe if not set)",
                    "default": null,
                    "optional": true,
                    "options": [
                        { "value": "1m", "label": "1 Minute" },
                        { "value": "5m", "label": "5 Minutes" },
                        { "value": "15m", "label": "15 Minutes" },
                        { "value": "1h", "label": "1 Hour" },
                        { "value": "4h", "label": "4 Hours" },
                        { "value": "12h", "label": "12 Hours" },
                        { "value": "1d", "label": "1 Day" }
                    ]
                },
                "count": {
                    "type": "number",
                    "name": "Candle Count",
                    "description": "Number of consecutive candles required",
                    "default": 3,
                    "min": 2,
                    "max": 20,
                    "step": 1
                },
                "direction": {
                    "type": "enum",
                    "name": "Candle Direction",
                    "description": "Color/direction of consecutive candles",
                    "default": "GREEN",
                    "options": [
                        { "value": "GREEN", "label": "Green (Bullish)" },
                        { "value": "RED", "label": "Red (Bearish)" }
                    ]
                },
                "minimum_change": {
                    "type": "percent",
                    "name": "Minimum Change %",
                    "description": "Minimum % change for each candle (filters noise)",
                    "default": 0.5,
                    "min": 0.1,
                    "max": 50.0,
                    "step": 0.1
                }
            }
        })
    }
}
