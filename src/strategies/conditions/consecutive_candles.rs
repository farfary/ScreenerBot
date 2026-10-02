// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Consecutive candles condition — checks for N sequential bullish/bearish candles.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_f64, get_param_string, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Check for consecutive green (bullish) or red (bearish) candles
pub struct ConsecutiveCandlesCondition;

#[async_trait]
impl ConditionEvaluator for ConsecutiveCandlesCondition {
    fn condition_type(&self) -> &'static str {
        "ConsecutiveCandles"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let count = get_param_f64(condition, "count")? as usize;
        let direction = get_param_string(condition, "direction")?;
        let minimum_change = get_param_f64(condition, "minimum_change")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.len() < count {
            return Err(Error::InsufficientCandles {
                indicator: "consecutive candles",
                available: candles.len(),
                required: count,
            });
        }

        // Get the most recent candles
        let recent_candles = &candles[candles.len() - count..];

        // Check for consecutive pattern
        let mut consecutive_count = 0;
        for candle in recent_candles {
            // A candle with no usable open cannot be coloured: dividing by it yields
            // +/-inf or NaN, and a zero open used to score as green for ANY minimum
            // change, defeating the noise filter entirely. An unclassifiable candle
            // breaks the streak rather than extending it.
            if !candle.open.is_finite() || candle.open <= 0.0 || !candle.close.is_finite() {
                consecutive_count = 0;
                continue;
            }

            let price_change_pct = ((candle.close - candle.open) / candle.open) * 100.0;

            let is_match = match direction.as_str() {
                "GREEN" => price_change_pct >= minimum_change,
                "RED" => price_change_pct <= -minimum_change,
                _ => {
                    return Err(Error::InvalidConditionValue {
                        field: "direction",
                        value: direction.clone(),
                    })
                }
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

    fn validate(&self, condition: &Condition) -> Result<()> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let count = get_param_f64(condition, "count")?;
        if count < 2.0 || count > 20.0 {
            return Err(Error::InvalidConditionValue {
                field: "count",
                value: count.to_string(),
            });
        }

        let direction = get_param_string(condition, "direction")?;
        if !["GREEN", "RED"].contains(&direction.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "direction",
                value: direction,
            });
        }

        let minimum_change = get_param_f64(condition, "minimum_change")?;
        if minimum_change < 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "minimum change",
                value: minimum_change.to_string(),
            });
        }
        if minimum_change > 50.0 {
            return Err(Error::InvalidConditionValue {
                field: "minimum change",
                value: minimum_change.to_string(),
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "ConsecutiveCandles",
            "category": "Candle Patterns",
            "icon": "icon-chart-candlestick",
            "origin": "strategy",
            "parameters": {
                "timeframe": {
                    "type": "enum",
                    "default": null,
                    "optional": true,
                    "options": [
                        { "value": "1m" },
                        { "value": "5m" },
                        { "value": "15m" },
                        { "value": "1h" },
                        { "value": "4h" },
                        { "value": "12h" },
                        { "value": "1d" }
                    ]
                },
                "count": {
                    "type": "number",
                    "unit": "candles",
                    "default": 3,
                    "min": 2,
                    "max": 20,
                    "step": 1
                },
                "direction": {
                    "type": "enum",
                    "default": "GREEN",
                    "options": [
                        { "value": "GREEN" },
                        { "value": "RED" }
                    ]
                },
                "minimum_change": {
                    "type": "percent",
                    "default": 0.5,
                    "min": 0.1,
                    "max": 50.0,
                    "step": 0.1
                }
            }
        })
    }
}
