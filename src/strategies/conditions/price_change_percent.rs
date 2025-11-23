use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
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

        // Get OHLCV data bundle
        let bundle = context
            .timeframe_bundle
            .as_ref()
            .ok_or_else(|| "OHLCV data not available".to_string())?;

        // Select appropriate timeframe based on lookback period
        let candles = if lookback_seconds <= 3600 {
            // Up to 1 hour: use 1m candles (covers up to 100 minutes)
            &bundle.m1
        } else if lookback_seconds <= 1800 * 60 {
            // Up to 30 hours: use 5m candles (covers up to 500 minutes)
            &bundle.m5
        } else if lookback_seconds <= 90000 {
            // Up to 25 hours: use 15m candles (covers up to 1500 minutes)
            &bundle.m15
        } else if lookback_seconds <= 360000 {
            // Up to 100 hours: use 1h candles (covers up to 100 hours)
            &bundle.h1
        } else if lookback_seconds <= 1440000 {
            // Up to 400 hours: use 4h candles (covers up to 400 hours)
            &bundle.h4
        } else if lookback_seconds <= 4320000 {
            // Up to 1200 hours: use 12h candles (covers up to 1200 hours)
            &bundle.h12
        } else {
            // Beyond: use 1d candles (covers up to 100 days)
            &bundle.d1
        };

        if candles.is_empty() {
            return Err("No OHLCV data available for time period".to_string());
        }

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
                    return Err("Time value for seconds must be 3600 or less (1 hour max)".to_string());
                }
            }
            "MINUTES" => {
                if time_value > 1440.0 {
                    return Err("Time value for minutes must be 1440 or less (24 hours max)".to_string());
                }
            }
            "HOURS" => {
                if time_value > 720.0 {
                    return Err("Time value for hours must be 720 or less (30 days max)".to_string());
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
