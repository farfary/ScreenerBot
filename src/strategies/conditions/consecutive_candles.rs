use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
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

        let ohlcv_data = context
            .ohlcv_data
            .as_ref()
            .ok_or_else(|| "OHLCV data not available".to_string())?;

        if ohlcv_data.candles.len() < count {
            return Err(format!(
                "Not enough candles: {} < {}",
                ohlcv_data.candles.len(),
                count
            ));
        }

        // Get the most recent candles
        let recent_candles = &ohlcv_data.candles[ohlcv_data.candles.len() - count..];

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
        let count = get_param_f64(condition, "count")?;
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
            "icon": "🕯️",
            "origin": "strategy",
            "description": "Detect consecutive green (bullish) or red (bearish) candles with minimum size filter",
            "parameters": {
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
