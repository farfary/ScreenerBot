// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Volume spike condition — detects unusual volume relative to recent average.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_param_f64, get_param_string_optional, validate_timeframe_param,
    ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Detect volume spikes compared to average volume
pub struct VolumeSpikeCondition;

#[async_trait]
impl ConditionEvaluator for VolumeSpikeCondition {
    fn condition_type(&self) -> &'static str {
        "VolumeSpike"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let lookback = get_param_f64(condition, "lookback")? as usize;
        let multiplier = get_param_f64(condition, "multiplier")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.len() < lookback + 1 {
            return Err(Error::InsufficientCandles {
                indicator: "volume spike",
                available: candles.len(),
                required: lookback + 1,
            });
        }

        // Get current candle volume (safe - checked above)
        let current_candle = candles.last().ok_or_else(|| Error::NoCandleData {
            timeframe: timeframe
                .as_deref()
                .unwrap_or(&context.strategy_timeframe)
                .to_owned(),
        })?;
        let current_volume = current_candle.volume;

        // Calculate average volume over lookback period (excluding current)
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

        let avg_volume: f64 =
            lookback_candles.iter().map(|c| c.volume).sum::<f64>() / lookback_candles.len() as f64;

        if avg_volume <= 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "average volume",
                value: avg_volume.to_string(),
            });
        }

        // Check if current volume is multiplier times the average
        let volume_ratio = current_volume / avg_volume;
        let result = volume_ratio >= multiplier;

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

        let multiplier = get_param_f64(condition, "multiplier")?;
        if multiplier < 1.0 || multiplier > 50.0 {
            return Err(Error::InvalidConditionValue {
                field: "multiplier",
                value: multiplier.to_string(),
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "VolumeSpike",
            "category": "Volume Analysis",
            "icon": "icon-chart-bar",
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
                "multiplier": {
                    "type": "number",
                    "unit": "multiplier",
                    "default": 2.0,
                    "min": 1.0,
                    "max": 50.0,
                    "step": 0.5
                }
            }
        })
    }
}
