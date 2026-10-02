// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Candle size condition — checks for large-body candles exceeding a threshold.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_f64, get_param_string, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Detect specific candle size and body patterns
pub struct CandleSizeCondition;

#[async_trait]
impl ConditionEvaluator for CandleSizeCondition {
    fn condition_type(&self) -> &'static str {
        "CandleSize"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let pattern = get_param_string(condition, "pattern")?;
        let threshold = get_param_f64(condition, "threshold")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.is_empty() {
            return Err(Error::NoCandleData {
                timeframe: timeframe
                    .as_deref()
                    .unwrap_or(&context.strategy_timeframe)
                    .to_owned(),
            });
        }

        // Get the most recent candle (safe - checked above)
        let candle = candles.last().ok_or_else(|| Error::NoCandleData {
            timeframe: timeframe
                .as_deref()
                .unwrap_or(&context.strategy_timeframe)
                .to_owned(),
        })?;

        // Calculate candle metrics
        let body_size = (candle.close - candle.open).abs();
        let total_range = candle.high - candle.low;
        let upper_wick = candle.high - candle.close.max(candle.open);
        let lower_wick = candle.close.min(candle.open) - candle.low;

        // Calculate percentages
        let body_pct = if total_range > 0.0 {
            (body_size / total_range) * 100.0
        } else {
            0.0
        };

        // Guard the denominator: a zero or non-finite open produces inf/NaN, and inf
        // clears every LARGE_BODY threshold while NaN silently fails all of them.
        let price_change_pct = if candle.open.is_finite() && candle.open > 0.0 {
            ((candle.close - candle.open) / candle.open).abs() * 100.0
        } else {
            return Err(Error::InvalidConditionValue {
                field: "candle open",
                value: candle.open.to_string(),
            });
        };

        let result = match pattern.as_str() {
            "LARGE_BODY" => {
                // Large body: body is >= threshold% of total range AND price change >= threshold%
                body_pct >= threshold && price_change_pct >= threshold
            }
            "SMALL_BODY" => {
                // Small body (doji-like): body is <= threshold% of total range
                body_pct <= threshold
            }
            "LONG_UPPER_WICK" => {
                // Long upper wick: upper wick >= threshold% of total range
                if total_range > 0.0 {
                    (upper_wick / total_range) * 100.0 >= threshold
                } else {
                    false
                }
            }
            "LONG_LOWER_WICK" => {
                // Long lower wick: lower wick >= threshold% of total range
                if total_range > 0.0 {
                    (lower_wick / total_range) * 100.0 >= threshold
                } else {
                    false
                }
            }
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "pattern",
                    value: pattern,
                })
            }
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let pattern = get_param_string(condition, "pattern")?;
        if ![
            "LARGE_BODY",
            "SMALL_BODY",
            "LONG_UPPER_WICK",
            "LONG_LOWER_WICK",
        ]
        .contains(&pattern.as_str())
        {
            return Err(Error::InvalidConditionValue {
                field: "pattern",
                value: pattern,
            });
        }

        let threshold = get_param_f64(condition, "threshold")?;
        if threshold < 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "threshold",
                value: threshold.to_string(),
            });
        }
        if threshold > 100.0 {
            return Err(Error::InvalidConditionValue {
                field: "threshold",
                value: threshold.to_string(),
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "CandleSize",
            "category": "Candle Patterns",
            "icon": "icon-expand",
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
                "pattern": {
                    "type": "enum",
                    "default": "LARGE_BODY",
                    "options": [
                        { "value": "LARGE_BODY" },
                        { "value": "SMALL_BODY" },
                        { "value": "LONG_UPPER_WICK" },
                        { "value": "LONG_LOWER_WICK" }
                    ]
                },
                "threshold": {
                    "type": "percent",
                    "default": 50.0,
                    "min": 10.0,
                    "max": 100.0,
                    "step": 5.0
                }
            }
        })
    }
}
