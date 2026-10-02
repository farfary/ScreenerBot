// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Price breakout condition — detects price breaking above/below recent highs/lows.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_f64, get_param_string,
    get_param_string_optional, validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Detect price breakouts from recent high/low levels
pub struct PriceBreakoutCondition;

#[async_trait]
impl ConditionEvaluator for PriceBreakoutCondition {
    fn condition_type(&self) -> &'static str {
        "PriceBreakout"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let lookback = get_param_f64(condition, "lookback")? as usize;
        let direction = get_param_string(condition, "direction")?;
        let confirmation = get_param_f64(condition, "confirmation")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        // `lookback` candles form the level, and the current candle is excluded from it,
        // so `lookback + 1` are needed — the same arithmetic VolumeSpike demands. Asking
        // for only `lookback` used to leave the window one candle short of what the user
        // configured, measuring the level over a narrower range and making the breakout
        // easier to clear than the strategy said.
        if candles.len() < lookback + 1 {
            return Err(Error::InsufficientCandles {
                indicator: "price breakout",
                available: candles.len(),
                required: lookback + 1,
            });
        }

        let current_price = get_current_price(context)?;

        // Get lookback candles (excluding current)
        let end_idx = candles.len().saturating_sub(1);
        let start_idx = end_idx.saturating_sub(lookback);
        let lookback_candles = &candles[start_idx..end_idx];

        if lookback_candles.is_empty() {
            return Err(Error::NoCandleData {
                timeframe: timeframe
                    .as_deref()
                    .unwrap_or(&context.strategy_timeframe)
                    .to_owned(),
            });
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

        let lookback = get_param_f64(condition, "lookback")?;
        if lookback < 2.0 || lookback > 100.0 {
            return Err(Error::InvalidConditionValue {
                field: "lookback",
                value: lookback.to_string(),
            });
        }

        let direction = get_param_string(condition, "direction")?;
        if !["UPWARD", "DOWNWARD"].contains(&direction.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "direction",
                value: direction,
            });
        }

        let confirmation = get_param_f64(condition, "confirmation")?;
        if confirmation < 0.0 || confirmation > 20.0 {
            return Err(Error::InvalidConditionValue {
                field: "confirmation",
                value: confirmation.to_string(),
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceBreakout",
            "category": "Price Analysis",
            "icon": "icon-rocket",
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
                "lookback": {
                    "type": "number",
                    "unit": "candles",
                    "default": 20,
                    "min": 2,
                    "max": 100,
                    "step": 1
                },
                "direction": {
                    "type": "enum",
                    "default": "UPWARD",
                    "options": [
                        { "value": "UPWARD" },
                        { "value": "DOWNWARD" }
                    ]
                },
                "confirmation": {
                    "type": "percent",
                    "default": 1.0,
                    "min": 0.0,
                    "max": 20.0,
                    "step": 0.5
                }
            }
        })
    }
}
