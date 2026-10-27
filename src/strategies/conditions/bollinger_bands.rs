// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Bollinger bands condition — the live price against bands of standard deviations around the average close.

use crate::strategies::conditions::indicators::{sma, stddev_population};
use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_choice, get_param_f64_in_range,
    get_param_string_optional, usable_basis, validate_timeframe_param, ConditionEvaluator,
    CANDLE_WINDOW_MAX,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The live price is below the lower band or above the upper band, or the bands are
/// squeezed to at most a width
///
/// The bands are the simple average of the last `period` closes plus and minus `std_dev`
/// population standard deviations of the same closes (divided by `period`, not
/// `period − 1`: the window is the whole series described). The squeeze width is the
/// distance between the bands as a percentage of the average.
pub struct BollingerBandsCondition;

/// What the bands must show.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Signal {
    BelowLower,
    AboveUpper,
    Squeeze,
}

/// Validated parameters of a Bollinger bands condition.
struct Params {
    period: usize,
    std_dev: f64,
    signal: Signal,
    squeeze_width_pct: f64,
    timeframe: Option<String>,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        validate_timeframe_param(condition)?;
        let period =
            get_param_f64_in_range(condition, "period", "period", 5.0, CANDLE_WINDOW_MAX as f64)?;
        let signal = match get_param_choice(
            condition,
            "signal",
            "signal",
            &["BELOW_LOWER", "ABOVE_UPPER", "SQUEEZE"],
        )?
        .as_str()
        {
            "BELOW_LOWER" => Signal::BelowLower,
            "ABOVE_UPPER" => Signal::AboveUpper,
            _ => Signal::Squeeze,
        };
        Ok(Self {
            period: period as usize,
            std_dev: get_param_f64_in_range(condition, "std_dev", "standard deviations", 0.5, 5.0)?,
            signal,
            squeeze_width_pct: get_param_f64_in_range(
                condition,
                "squeeze_width_pct",
                "squeeze width",
                0.5,
                100.0,
            )?,
            timeframe: get_param_string_optional(condition, "timeframe"),
        })
    }
}

#[async_trait]
impl ConditionEvaluator for BollingerBandsCondition {
    fn condition_type(&self) -> &'static str {
        "BollingerBands"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        let candles = get_candles_for_timeframe(context, params.timeframe.as_deref())?;
        if candles.len() < params.period {
            return Err(Error::InsufficientCandles {
                indicator: "bollinger bands",
                available: candles.len(),
                required: params.period,
            });
        }

        let closes: Vec<f64> = candles[candles.len() - params.period..]
            .iter()
            .map(|c| c.close)
            .collect();
        let insufficient = || Error::InsufficientCandles {
            indicator: "bollinger bands",
            available: closes.len(),
            required: params.period,
        };
        let average = *sma(&closes, params.period)
            .last()
            .ok_or_else(insufficient)?;
        let average = usable_basis("moving average", average)?;
        let deviation = stddev_population(&closes).ok_or_else(insufficient)?;
        let upper = average + params.std_dev * deviation;
        let lower = average - params.std_dev * deviation;

        Ok(match params.signal {
            Signal::BelowLower => get_current_price(context)? <= lower,
            Signal::AboveUpper => get_current_price(context)? >= upper,
            Signal::Squeeze => (upper - lower) / average * 100.0 <= params.squeeze_width_pct,
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
            "type": "BollingerBands",
            "category": "Technical Indicators",
            "icon": "icon-unfold-vertical",
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
                    "default": 20,
                    "min": 5,
                    "max": CANDLE_WINDOW_MAX,
                    "step": 1
                },
                "std_dev": {
                    "type": "number",
                    "default": 2,
                    "min": 0.5,
                    "max": 5,
                    "step": 0.1
                },
                "signal": {
                    "type": "enum",
                    "default": "BELOW_LOWER",
                    "options": [
                        { "value": "BELOW_LOWER" },
                        { "value": "ABOVE_UPPER" },
                        { "value": "SQUEEZE" }
                    ]
                },
                "squeeze_width_pct": {
                    "type": "percent",
                    "default": 10,
                    "min": 0.5,
                    "max": 100,
                    "step": 0.5
                }
            }
        })
    }
}
