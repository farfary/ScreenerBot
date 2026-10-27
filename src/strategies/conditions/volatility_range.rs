// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Volatility range condition — the average true range as a percentage of the live price.

use crate::strategies::conditions::indicators::atr;
use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_choice, get_param_f64_in_range,
    get_param_string_optional, validate_timeframe_param, ConditionEvaluator, CANDLE_WINDOW_MAX,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The widest period: the average true range over `period` candles reads `period + 1`
/// (each true range needs the close before it), and the bundle holds
/// [`CANDLE_WINDOW_MAX`] candles.
const PERIOD_MAX: usize = CANDLE_WINDOW_MAX - 1;

/// Wilder's average true range over `period` candles, as a percentage of the live price, is
/// above or below a threshold; a range is two of these under one group
pub struct VolatilityRangeCondition;

/// Validated parameters of a volatility range condition.
struct Params {
    period: usize,
    above: bool,
    threshold_pct: f64,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let period = get_param_f64_in_range(condition, "period", "period", 2.0, PERIOD_MAX as f64)?;
        let comparison =
            get_param_choice(condition, "comparison", "comparison", &["ABOVE", "BELOW"])?;
        Ok(Self {
            period: period as usize,
            above: comparison == "ABOVE",
            threshold_pct: get_param_f64_in_range(
                condition,
                "threshold_pct",
                "threshold",
                0.1,
                500.0,
            )?,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

#[async_trait]
impl ConditionEvaluator for VolatilityRangeCondition {
    fn condition_type(&self) -> &'static str {
        "VolatilityRange"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;

        let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
        let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let range =
            atr(&highs, &lows, &closes, params.period).ok_or(Error::InsufficientCandles {
                indicator: "volatility range",
                available: candles.len(),
                required: params.period + 1,
            })?;
        let price = get_current_price(context)?;

        let range_pct = range / price * 100.0;
        Ok(if params.above {
            range_pct >= params.threshold_pct
        } else {
            range_pct <= params.threshold_pct
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
            "type": "VolatilityRange",
            "category": "Technical Indicators",
            "icon": "icon-move-vertical",
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
                    "max": PERIOD_MAX,
                    "step": 1
                },
                "comparison": {
                    "type": "enum",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE" },
                        { "value": "BELOW" }
                    ]
                },
                "threshold_pct": {
                    "type": "percent",
                    "default": 5,
                    "min": 0.1,
                    "max": 500,
                    "step": 0.1
                }
            }
        })
    }
}
