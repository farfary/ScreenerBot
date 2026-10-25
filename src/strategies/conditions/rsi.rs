// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RSI condition — Wilder's relative strength index of the candle closes against a threshold.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_choice, get_param_f64_in_range, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// RSI of the candle closes is below or above a threshold
pub struct RsiCondition;

/// Validated parameters of an RSI condition.
struct Params {
    period: usize,
    below: bool,
    threshold: f64,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let period = get_param_f64_in_range(condition, "period", "period", 2.0, 100.0)?;
        let comparison =
            get_param_choice(condition, "comparison", "comparison", &["BELOW", "ABOVE"])?;
        let threshold = get_param_f64_in_range(condition, "threshold", "threshold", 1.0, 99.0)?;
        Ok(Self {
            period: period as usize,
            below: comparison == "BELOW",
            threshold,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

/// Wilder's RSI over `closes`: the first `period` changes seed the average gain and loss
/// with a simple mean, every later change is folded in with Wilder smoothing. Requires at
/// least `period + 1` closes.
fn wilder_rsi(closes: &[f64], period: usize) -> f64 {
    let period_f = period as f64;
    let mut changes = closes.windows(2).map(|pair| pair[1] - pair[0]);
    let (mut gain, mut loss) = changes
        .by_ref()
        .take(period)
        .fold((0.0, 0.0), |(gain, loss), change| {
            (gain + change.max(0.0), loss + (-change).max(0.0))
        });
    gain /= period_f;
    loss /= period_f;
    for change in changes {
        gain = (gain * (period_f - 1.0) + change.max(0.0)) / period_f;
        loss = (loss * (period_f - 1.0) + (-change).max(0.0)) / period_f;
    }
    if loss == 0.0 {
        return if gain == 0.0 { 50.0 } else { 100.0 };
    }
    100.0 - 100.0 / (1.0 + gain / loss)
}

#[async_trait]
impl ConditionEvaluator for RsiCondition {
    fn condition_type(&self) -> &'static str {
        "Rsi"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;

        if candles.len() < params.period + 1 {
            return Err(Error::InsufficientCandles {
                indicator: "rsi",
                available: candles.len(),
                required: params.period + 1,
            });
        }

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi = wilder_rsi(&closes, params.period);

        Ok(if params.below {
            rsi < params.threshold
        } else {
            rsi > params.threshold
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "Rsi",
            "category": "Technical Indicators",
            "icon": "icon-gauge",
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
                "period": {
                    "type": "number",
                    "unit": "candles",
                    "default": 14,
                    "min": 2,
                    "max": 100,
                    "step": 1
                },
                "comparison": {
                    "type": "enum",
                    "default": "BELOW",
                    "options": [
                        { "value": "BELOW" },
                        { "value": "ABOVE" }
                    ]
                },
                "threshold": {
                    "type": "number",
                    "default": 30,
                    "min": 1,
                    "max": 99,
                    "step": 1
                }
            }
        })
    }
}
