//! Price-to-MA condition — compares current price against moving averages.

use crate::strategies::conditions::{
    get_candles_for_timeframe, get_current_price, get_param_f64, get_param_string,
    get_param_string_optional, usable_basis, validate_timeframe_param, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Price position relative to moving average
pub struct PriceToMaCondition;

#[async_trait]
impl ConditionEvaluator for PriceToMaCondition {
    fn condition_type(&self) -> &'static str {
        "PriceToMA"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let period = get_param_f64(condition, "period")? as usize;
        let position = get_param_string(condition, "position")?;
        let distance = get_param_f64(condition, "distance")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.len() < period {
            return Err(Error::InsufficientCandles {
                indicator: "moving average",
                available: candles.len(),
                required: period,
            });
        }

        // Calculate simple moving average
        let recent_candles = &candles[candles.len() - period..];
        let ma: f64 = recent_candles.iter().map(|c| c.close).sum::<f64>() / period as f64;
        // A dead series averages to zero, which would put any price infinitely above its
        // own moving average and satisfy every ABOVE rule ever configured.
        let ma = usable_basis("moving average", ma)?;

        let current_price = get_current_price(context)?;

        // Calculate percentage distance from MA
        let distance_pct = ((current_price - ma) / ma) * 100.0;

        let result = match position.as_str() {
            "ABOVE" => distance_pct >= distance,
            "BELOW" => distance_pct <= -distance,
            "WITHIN" => distance_pct.abs() <= distance,
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "position",
                    value: position,
                })
            }
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let period = get_param_f64(condition, "period")?;
        if period < 2.0 {
            return Err(Error::InvalidConditionValue {
                field: "period",
                value: period.to_string(),
            });
        }
        if period > 200.0 {
            return Err(Error::InvalidConditionValue {
                field: "period",
                value: period.to_string(),
            });
        }

        let distance = get_param_f64(condition, "distance")?;
        if distance < 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "distance",
                value: distance.to_string(),
            });
        }
        if distance > 100.0 {
            return Err(Error::InvalidConditionValue {
                field: "distance",
                value: distance.to_string(),
            });
        }

        let position = get_param_string(condition, "position")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&position.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "position",
                value: position,
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceToMA",
            "name": "Price vs Moving Average",
            "category": "Technical Indicators",
            "tags": ["ma", "sma", "trend", "technical"],
            "icon": "icon-chart-line",
            "origin": "strategy",
            "description": "Check if price is above, below, or within range of its Simple Moving Average",
            "parameters": {
                "timeframe": {
                    "type": "enum",
                    "name": "Timeframe",
                    "description": "Candle timeframe to analyze (defaults to strategy timeframe if not set)",
                    "default": null,
                    "optional": true,
                    "options": [
                        { "value": "1m", "label": "1 Minute" },
                        { "value": "5m", "label": "5 Minutes" },
                        { "value": "15m", "label": "15 Minutes" },
                        { "value": "1h", "label": "1 Hour" },
                        { "value": "4h", "label": "4 Hours" },
                        { "value": "12h", "label": "12 Hours" },
                        { "value": "1d", "label": "1 Day" }
                    ]
                },
                "period": {
                    "type": "number",
                    "name": "MA Period",
                    "description": "Number of candles for moving average calculation",
                    "default": 20,
                    "min": 2,
                    "max": 200,
                    "step": 1
                },
                "position": {
                    "type": "enum",
                    "name": "Position",
                    "description": "Price position relative to MA",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE", "label": "Above MA" },
                        { "value": "BELOW", "label": "Below MA" },
                        { "value": "WITHIN", "label": "Within Range" }
                    ]
                },
                "distance": {
                    "type": "percent",
                    "name": "Distance %",
                    "description": "Minimum distance from MA (for ABOVE/BELOW) or maximum range (for WITHIN)",
                    "default": 2.0,
                    "min": 0.1,
                    "max": 100.0,
                    "step": 0.5
                }
            }
        })
    }
}
