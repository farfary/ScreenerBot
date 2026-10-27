// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! VWAP condition — the live price against the volume-weighted average price of recent candles.

use crate::strategies::conditions::indicators::vwap;
use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_choice, get_param_f64_in_range,
    get_param_string_optional, usable_basis, validate_timeframe_param, ConditionEvaluator,
    CANDLE_WINDOW_MAX,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The live price is at least `distance_pct` above, or below, the volume-weighted average
/// of the typical price `(high + low + close) / 3` over the last `lookback` candles
pub struct VwapCondition;

/// Validated parameters of a VWAP condition.
struct Params {
    lookback: usize,
    above: bool,
    distance_pct: f64,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let lookback = get_param_f64_in_range(
            condition,
            "lookback",
            "lookback",
            5.0,
            CANDLE_WINDOW_MAX as f64,
        )?;
        let position = get_param_choice(condition, "position", "position", &["ABOVE", "BELOW"])?;
        Ok(Self {
            lookback: lookback as usize,
            above: position == "ABOVE",
            distance_pct: get_param_f64_in_range(
                condition,
                "distance_pct",
                "distance",
                0.0,
                100.0,
            )?,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

#[async_trait]
impl ConditionEvaluator for VwapCondition {
    fn condition_type(&self) -> &'static str {
        "Vwap"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;
        if candles.len() < params.lookback {
            return Err(Error::InsufficientCandles {
                indicator: "vwap",
                available: candles.len(),
                required: params.lookback,
            });
        }

        let window = &candles[candles.len() - params.lookback..];
        let typical: Vec<f64> = window
            .iter()
            .map(|candle| (candle.high + candle.low + candle.close) / 3.0)
            .collect();
        let volumes: Vec<f64> = window.iter().map(|candle| candle.volume).collect();
        let average = vwap(&typical, &volumes).ok_or_else(|| Error::UnusableData {
            field: "total volume",
            value: volumes.iter().sum::<f64>().to_string(),
        })?;
        let average = usable_basis("vwap", average)?;
        let price = get_current_price(context)?;

        let distance = params.distance_pct / 100.0;
        Ok(if params.above {
            price >= average * (1.0 + distance)
        } else {
            price <= average * (1.0 - distance)
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
            "type": "Vwap",
            "category": "Technical Indicators",
            "icon": "icon-chart-spline",
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
                    "default": 30,
                    "min": 5,
                    "max": CANDLE_WINDOW_MAX,
                    "step": 1
                },
                "position": {
                    "type": "enum",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE" },
                        { "value": "BELOW" }
                    ]
                },
                "distance_pct": {
                    "type": "percent",
                    "default": 0,
                    "min": 0,
                    "max": 100,
                    "step": 0.5
                }
            }
        })
    }
}
