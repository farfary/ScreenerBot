//! Price change percent condition — triggers on percentage price moves within a time window.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_f64, get_param_string,
    get_param_string_optional, usable_basis, validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Price change percentage condition - check if price moved by % from reference price
pub struct PriceChangePercentCondition;

#[async_trait]
impl ConditionEvaluator for PriceChangePercentCondition {
    fn condition_type(&self) -> &'static str {
        "PriceChangePercent"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let percentage = get_param_f64(condition, "percentage")?;
        let direction = get_param_string(condition, "direction")?;
        let time_value = get_param_f64(condition, "time_value")?;
        let time_unit = get_param_string(condition, "time_unit")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let current_price = get_current_price(context)?;

        // Convert time period to seconds
        let lookback_seconds = match time_unit.as_str() {
            "SECONDS" => time_value as i64,
            "MINUTES" => (time_value * 60.0) as i64,
            "HOURS" => (time_value * 3600.0) as i64,
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "time unit",
                    value: time_unit,
                })
            }
        };

        // Get candles for specified timeframe (or use strategy default)
        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        // Anchor "now" on the newest candle. That is only sound because
        // `get_candles_for_timeframe` has already refused a series too far behind
        // `context.evaluated_at` — otherwise this window would float backwards with the
        // series while `current_price` stayed live, and an hours-old move would be
        // reported as a move of `time_value` `time_unit`.
        let current_timestamp = candles
            .iter()
            .map(|c| c.timestamp)
            .max()
            .unwrap_or_default();
        let lookback_timestamp = current_timestamp - lookback_seconds;

        // The reference is the NEWEST candle at or before the lookback instant. Taking
        // the candle CLOSEST to that instant instead could select one inside the window
        // and then reject it for being too recent — an "insufficient data" error that
        // reported more seconds available than the lookback had asked for.
        let past_candle = candles
            .iter()
            .filter(|c| c.timestamp <= lookback_timestamp)
            .max_by_key(|c| c.timestamp);

        let past_candle = match past_candle {
            Some(candle) => candle,
            None => {
                let oldest_timestamp = candles
                    .iter()
                    .map(|c| c.timestamp)
                    .min()
                    .unwrap_or(current_timestamp);
                let available_seconds = current_timestamp - oldest_timestamp;
                return Err(Error::InsufficientHistory {
                    indicator: "price change lookback",
                    available_seconds: available_seconds.max(0),
                    required_seconds: lookback_seconds.max(0),
                });
            }
        };

        let past_price = usable_basis("reference candle close", past_candle.close)?;

        // Calculate price change percentage
        let price_change_pct = ((current_price - past_price) / past_price) * 100.0;

        let result = match direction.as_str() {
            "ABOVE" => price_change_pct >= percentage,
            "BELOW" => price_change_pct <= -percentage,
            "WITHIN" => price_change_pct.abs() <= percentage,
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "direction",
                    value: direction,
                })
            }
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let percentage = get_param_f64(condition, "percentage")?;
        if percentage < 0.1 {
            return Err(Error::InvalidConditionValue {
                field: "percentage",
                value: percentage.to_string(),
            });
        }
        if percentage > 1000.0 {
            return Err(Error::InvalidConditionValue {
                field: "percentage",
                value: percentage.to_string(),
            });
        }

        let direction = get_param_string(condition, "direction")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&direction.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "direction",
                value: direction,
            });
        }

        let time_value = get_param_f64(condition, "time_value")?;
        if time_value < 1.0 {
            return Err(Error::InvalidConditionValue {
                field: "time value",
                value: time_value.to_string(),
            });
        }

        let time_unit = get_param_string(condition, "time_unit")?;
        if !["SECONDS", "MINUTES", "HOURS"].contains(&time_unit.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "time unit",
                value: time_unit.clone(),
            });
        }

        // Validate time value ranges based on unit
        match time_unit.as_str() {
            "SECONDS" => {
                if time_value > 3600.0 {
                    return Err(Error::InvalidConditionValue {
                        field: "time value",
                        value: time_value.to_string(),
                    });
                }
            }
            "MINUTES" => {
                if time_value > 1440.0 {
                    return Err(Error::InvalidConditionValue {
                        field: "time value",
                        value: time_value.to_string(),
                    });
                }
            }
            "HOURS" => {
                if time_value > 720.0 {
                    return Err(Error::InvalidConditionValue {
                        field: "time value",
                        value: time_value.to_string(),
                    });
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
