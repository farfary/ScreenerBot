// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pullback-from-high condition — the live price inside a band below the recent candle high.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_f64_in_range,
    get_param_string_optional, usable_basis, validate_timeframe_param, ConditionEvaluator,
    CANDLE_WINDOW_MAX,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The live price sits between `min_drop_pct` and `max_drop_pct` below the highest high of
/// the last `lookback` candles
pub struct PullbackFromHighCondition;

/// Validated parameters of a pullback-from-high condition.
struct Params {
    lookback: usize,
    min_drop_pct: f64,
    max_drop_pct: f64,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let lookback = get_param_f64_in_range(
            condition,
            "lookback",
            "lookback",
            3.0,
            CANDLE_WINDOW_MAX as f64,
        )?;
        let min_drop_pct =
            get_param_f64_in_range(condition, "min_drop_pct", "minimum drop", 1.0, 90.0)?;
        let max_drop_pct =
            get_param_f64_in_range(condition, "max_drop_pct", "maximum drop", 2.0, 95.0)?;
        if min_drop_pct >= max_drop_pct {
            return Err(Error::InvalidConditionValue {
                field: "maximum drop",
                value: max_drop_pct.to_string(),
            });
        }
        Ok(Self {
            lookback: lookback as usize,
            min_drop_pct,
            max_drop_pct,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

#[async_trait]
impl ConditionEvaluator for PullbackFromHighCondition {
    fn condition_type(&self) -> &'static str {
        "PullbackFromHigh"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;
        if candles.len() < params.lookback {
            return Err(Error::InsufficientCandles {
                indicator: "pullback from high",
                available: candles.len(),
                required: params.lookback,
            });
        }

        let highest = candles[candles.len() - params.lookback..]
            .iter()
            .map(|candle| candle.high)
            .fold(f64::NEG_INFINITY, f64::max);
        let highest = usable_basis("highest high", highest)?;
        let price = get_current_price(context)?;

        let drop_pct = (highest - price) / highest * 100.0;
        Ok(drop_pct >= params.min_drop_pct && drop_pct <= params.max_drop_pct)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Candles
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PullbackFromHigh",
            "category": "Price Analysis",
            "icon": "icon-undo-2",
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
                    "min": 3,
                    "max": CANDLE_WINDOW_MAX,
                    "step": 1
                },
                "min_drop_pct": {
                    "type": "percent",
                    "default": 10,
                    "min": 1,
                    "max": 90,
                    "step": 1
                },
                "max_drop_pct": {
                    "type": "percent",
                    "default": 30,
                    "min": 2,
                    "max": 95,
                    "step": 1
                }
            }
        })
    }
}
