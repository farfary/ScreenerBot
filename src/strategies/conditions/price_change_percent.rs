use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_f64, get_param_string, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Price change percentage condition - check if price moved by % from reference price
pub struct PriceChangePercentCondition;

#[async_trait]
impl ConditionEvaluator for PriceChangePercentCondition {
    fn condition_type(&self) -> &'static str {
        "PriceChangePercent"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let percentage = get_param_f64(condition, "percentage")?;
        let direction = get_param_string(condition, "direction")?;
        let time_value = get_param_f64(condition, "time_value")?;
        let time_unit = get_param_string(condition, "time_unit")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        // Convert time period to seconds
        let lookback_seconds = match time_unit.as_str() {
            "SECONDS" => time_value as i64,
            "MINUTES" => (time_value * 60.0) as i64,
            "HOURS" => (time_value * 3600.0) as i64,
            _ => return Err(format!("Invalid time unit: {}", time_unit)),
        };

        // Get candles for specified timeframe (or use strategy default)
        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        // Get current timestamp (use most recent candle timestamp)
        let current_timestamp = candles.last().map(|c| c.timestamp).unwrap_or(0);
        let lookback_timestamp = current_timestamp - lookback_seconds;

        // Find candle closest to lookback timestamp
        let past_candle = candles
            .iter()
            .min_by_key(|c| (c.timestamp - lookback_timestamp).abs())
            .ok_or_else(|| "Failed to find historical candle".to_string())?;

        // Check if we have sufficient data
        if past_candle.timestamp > lookback_timestamp {
            let available_seconds = current_timestamp - candles[0].timestamp;
            let requested_unit = match time_unit.as_str() {
                "SECONDS" => format!("{} seconds", time_value),
                "MINUTES" => format!("{} minutes", time_value),
                "HOURS" => format!("{} hours", time_value),
                _ => "unknown".to_string(),
            };
            return Err(format!(
                "Insufficient historical data: requested {} lookback, only {} seconds available",
                requested_unit, available_seconds
            ));
        }

        let past_price = past_candle.close;

        // Calculate price change percentage
        let price_change_pct = ((current_price - past_price) / past_price) * 100.0;

        let result = match direction.as_str() {
            "ABOVE" => price_change_pct >= percentage,
            "BELOW" => price_change_pct <= -percentage,
            "WITHIN" => price_change_pct.abs() <= percentage,
            _ => return Err(format!("Invalid direction: {}", direction)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let percentage = get_param_f64(condition, "percentage")?;
        if percentage < 0.1 {
            return Err("Percentage must be at least 0.1".to_string());
        }
        if percentage > 1000.0 {
            return Err("Percentage must be 1000 or less".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        let time_value = get_param_f64(condition, "time_value")?;
        if time_value < 1.0 {
            return Err("Time value must be at least 1".to_string());
        }

        let time_unit = get_param_string(condition, "time_unit")?;
        if !["SECONDS", "MINUTES", "HOURS"].contains(&time_unit.as_str()) {
            return Err(format!("Invalid time unit: {}", time_unit));
        }

        // Validate time value ranges based on unit
        match time_unit.as_str() {
            "SECONDS" => {
                if time_value > 3600.0 {
                    return Err(
                        "Time value for seconds must be 3600 or less (1 hour max)".to_string()
                    );
                }
            }
            "MINUTES" => {
                if time_value > 1440.0 {
                    return Err(
                        "Time value for minutes must be 1440 or less (24 hours max)".to_string()
                    );
                }
            }
            "HOURS" => {
                if time_value > 720.0 {
                    return Err(
                        "Time value for hours must be 720 or less (30 days max)".to_string()
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceChangePercent",
            "name": "Price Change %",
            "category": "Price Analysis",
            "tags": ["price", "percentage", "change", "time"],
            "icon": "icon-percent",
            "origin": "strategy",
            "description": "Check if price changed by a percentage threshold within a time period",
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
                "percentage": {
                    "type": "percent",
                    "name": "Change Threshold %",
                    "description": "Percentage price change to trigger (0.1-1000%)",
                    "default": 10.0,
                    "min": 0.1,
                    "max": 1000.0,
                    "step": 0.5
                },
                "direction": {
                    "type": "enum",
                    "name": "Direction",
                    "description": "Price movement direction",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE", "label": "Gain (+%)" },
                        { "value": "BELOW", "label": "Loss (-%)" },
                        { "value": "WITHIN", "label": "Within Range (±%)" }
                    ]
                },
                "time_value": {
                    "type": "number",
                    "name": "Time Period",
                    "description": "Lookback period value (1-3600 for seconds, 1-1440 for minutes, 1-720 for hours)",
                    "default": 5.0,
                    "min": 1.0,
                    "max": 3600.0,
                    "step": 1.0
                },
                "time_unit": {
                    "type": "enum",
                    "name": "Time Unit",
                    "description": "Time unit for lookback period",
                    "default": "MINUTES",
                    "options": [
                        { "value": "SECONDS", "label": "Seconds" },
                        { "value": "MINUTES", "label": "Minutes" },
                        { "value": "HOURS", "label": "Hours" }
                    ]
                }
            }
        })
    }
}
