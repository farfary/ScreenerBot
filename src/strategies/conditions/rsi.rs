// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! RSI condition — Wilder's relative strength index of the candle closes against a threshold.

use crate::strategies::conditions::indicators::rsi;
use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_choice, get_param_f64_in_range, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
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

#[async_trait]
impl ConditionEvaluator for RsiCondition {
    fn condition_type(&self) -> &'static str {
        "Rsi"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi = rsi(&closes, params.period).ok_or(Error::InsufficientCandles {
            indicator: "rsi",
            available: candles.len(),
            required: params.period + 1,
        })?;

        Ok(if params.below {
            rsi < params.threshold
        } else {
            rsi > params.threshold
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Candles
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
