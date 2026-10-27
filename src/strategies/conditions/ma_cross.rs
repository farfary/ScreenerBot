// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Moving-average cross condition — a fast average crossing a slow one within the last candles.

use crate::strategies::conditions::indicators::{ema, sma};
use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_choice, get_param_f64_in_range, get_param_string_optional,
    validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The fast moving average crossed the slow one within the last `within_candles` closes
pub struct MaCrossCondition;

/// Validated parameters of a moving-average cross condition.
struct Params {
    exponential: bool,
    fast_period: usize,
    slow_period: usize,
    bullish: bool,
    within_candles: usize,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let ma_type =
            get_param_choice(condition, "ma_type", "moving average type", &["EMA", "SMA"])?;
        let fast_period =
            get_param_f64_in_range(condition, "fast_period", "fast period", 2.0, 100.0)?;
        let slow_period =
            get_param_f64_in_range(condition, "slow_period", "slow period", 3.0, 200.0)?;
        if slow_period <= fast_period {
            return Err(Error::InvalidConditionValue {
                field: "slow period",
                value: slow_period.to_string(),
            });
        }
        let direction =
            get_param_choice(condition, "direction", "direction", &["BULLISH", "BEARISH"])?;
        let within_candles =
            get_param_f64_in_range(condition, "within_candles", "within candles", 1.0, 20.0)?;
        Ok(Self {
            exponential: ma_type == "EMA",
            fast_period: fast_period as usize,
            slow_period: slow_period as usize,
            bullish: direction == "BULLISH",
            within_candles: within_candles as usize,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

#[async_trait]
impl ConditionEvaluator for MaCrossCondition {
    fn condition_type(&self) -> &'static str {
        "MaCross"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;

        let required = params.slow_period + params.within_candles;
        if candles.len() < required {
            return Err(Error::InsufficientCandles {
                indicator: "moving average cross",
                available: candles.len(),
                required,
            });
        }

        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let moving_average = if params.exponential { ema } else { sma };
        let fast = moving_average(&closes, params.fast_period);
        let slow = moving_average(&closes, params.slow_period);
        let fast_at = |index: usize| fast[index + 1 - params.fast_period];
        let slow_at = |index: usize| slow[index + 1 - params.slow_period];

        let crossed = (closes.len() - params.within_candles..closes.len()).any(|index| {
            let (fast_before, slow_before) = (fast_at(index - 1), slow_at(index - 1));
            let (fast_now, slow_now) = (fast_at(index), slow_at(index));
            if params.bullish {
                fast_before <= slow_before && fast_now > slow_now
            } else {
                fast_before >= slow_before && fast_now < slow_now
            }
        });

        Ok(crossed)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Candles
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "MaCross",
            "category": "Technical Indicators",
            "icon": "icon-git-compare",
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
                "ma_type": {
                    "type": "enum",
                    "default": "EMA",
                    "options": [
                        { "value": "EMA" },
                        { "value": "SMA" }
                    ]
                },
                "fast_period": {
                    "type": "number",
                    "unit": "candles",
                    "default": 9,
                    "min": 2,
                    "max": 100,
                    "step": 1
                },
                "slow_period": {
                    "type": "number",
                    "unit": "candles",
                    "default": 21,
                    "min": 3,
                    "max": 200,
                    "step": 1
                },
                "direction": {
                    "type": "enum",
                    "default": "BULLISH",
                    "options": [
                        { "value": "BULLISH" },
                        { "value": "BEARISH" }
                    ]
                },
                "within_candles": {
                    "type": "number",
                    "unit": "candles",
                    "default": 1,
                    "min": 1,
                    "max": 20,
                    "step": 1
                }
            }
        })
    }
}
