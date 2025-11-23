use crate::strategies::conditions::{get_candles_for_timeframe, get_param_f64, get_param_string, get_param_string_optional, validate_timeframe_param, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Detect specific candle size and body patterns
pub struct CandleSizeCondition;

#[async_trait]
impl ConditionEvaluator for CandleSizeCondition {
    fn condition_type(&self) -> &'static str {
        "CandleSize"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let pattern = get_param_string(condition, "pattern")?;
        let threshold = get_param_f64(condition, "threshold")?;
        let timeframe = get_param_string_optional(condition, "timeframe");

        let candles = get_candles_for_timeframe(context, timeframe.as_deref())?;

        if candles.is_empty() {
            return Err("No candles available".to_string());
        }

        // Get the most recent candle
        let candle = candles.last().unwrap();

        // Calculate candle metrics
        let body_size = (candle.close - candle.open).abs();
        let total_range = candle.high - candle.low;
        let upper_wick = candle.high - candle.close.max(candle.open);
        let lower_wick = candle.close.min(candle.open) - candle.low;

        // Calculate percentages
        let body_pct = if total_range > 0.0 {
            (body_size / total_range) * 100.0
        } else {
            0.0
        };

        let price_change_pct = ((candle.close - candle.open) / candle.open).abs() * 100.0;

        let result = match pattern.as_str() {
            "LARGE_BODY" => {
                // Large body: body is >= threshold% of total range AND price change >= threshold%
                body_pct >= threshold && price_change_pct >= threshold
            }
            "SMALL_BODY" => {
                // Small body (doji-like): body is <= threshold% of total range
                body_pct <= threshold
            }
            "LONG_UPPER_WICK" => {
                // Long upper wick: upper wick >= threshold% of total range
                if total_range > 0.0 {
                    (upper_wick / total_range) * 100.0 >= threshold
                } else {
                    false
                }
            }
            "LONG_LOWER_WICK" => {
                // Long lower wick: lower wick >= threshold% of total range
                if total_range > 0.0 {
                    (lower_wick / total_range) * 100.0 >= threshold
                } else {
                    false
                }
            }
            _ => return Err(format!("Invalid pattern: {}", pattern)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        // Validate timeframe if provided
        validate_timeframe_param(condition)?;

        let pattern = get_param_string(condition, "pattern")?;;
        if ![
            "LARGE_BODY",
            "SMALL_BODY",
            "LONG_UPPER_WICK",
            "LONG_LOWER_WICK",
        ]
        .contains(&pattern.as_str())
        {
            return Err(format!("Invalid pattern: {}", pattern));
        }

        let threshold = get_param_f64(condition, "threshold")?;
        if threshold < 0.0 {
            return Err("Threshold must be non-negative".to_string());
        }
        if threshold > 100.0 {
            return Err("Threshold must be 100% or less".to_string());
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "CandleSize",
            "name": "Candle Size Pattern",
            "category": "Candle Patterns",
            "tags": ["candles", "pattern", "doji", "wick"],
            "icon": "icon-expand",
            "origin": "strategy",
            "description": "Detect specific candle patterns: large body, small body (doji), long wicks",
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
                "pattern": {
                    "type": "enum",
                    "name": "Pattern Type",
                    "description": "Candle pattern to detect",
                    "default": "LARGE_BODY",
                    "options": [
                        { "value": "LARGE_BODY", "label": "Large Body (Strong Move)" },
                        { "value": "SMALL_BODY", "label": "Small Body (Doji/Indecision)" },
                        { "value": "LONG_UPPER_WICK", "label": "Long Upper Wick (Rejection)" },
                        { "value": "LONG_LOWER_WICK", "label": "Long Lower Wick (Support)" }
                    ]
                },
                "threshold": {
                    "type": "percent",
                    "name": "Size Threshold %",
                    "description": "Percentage threshold for pattern detection",
                    "default": 50.0,
                    "min": 10.0,
                    "max": 100.0,
                    "step": 5.0
                }
            }
        })
    }
}
