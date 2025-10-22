use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Relative to moving average condition - check price position relative to MA
pub struct RelativeToMaCondition;

#[async_trait]
impl ConditionEvaluator for RelativeToMaCondition {
    fn condition_type(&self) -> &'static str {
        "RelativeToMA"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let period = get_param_f64(condition, "period")? as usize;
        let comparison = get_param_string(condition, "comparison")?;
        let percentage = get_param_f64(condition, "percentage")?;

        let ohlcv_data = context
            .ohlcv_data
            .as_ref()
            .ok_or_else(|| "OHLCV data not available".to_string())?;

        if ohlcv_data.candles.len() < period {
            return Err(format!(
                "Not enough candles for MA calculation: {} < {}",
                ohlcv_data.candles.len(),
                period
            ));
        }

        // Calculate simple moving average
        let recent_candles = &ohlcv_data.candles[ohlcv_data.candles.len() - period..];
        let ma: f64 = recent_candles.iter().map(|c| c.close).sum::<f64>() / period as f64;

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        // Calculate percentage difference from MA
        let diff_pct = ((current_price - ma) / ma) * 100.0;

        let result = match comparison.as_str() {
            "ABOVE" => diff_pct >= percentage,
            "BELOW" => diff_pct <= -percentage,
            "WITHIN" => diff_pct.abs() <= percentage,
            _ => return Err(format!("Invalid comparison: {}", comparison)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let period = get_param_f64(condition, "period")?;
        if period < 2.0 {
            return Err("Period must be at least 2".to_string());
        }

        let percentage = get_param_f64(condition, "percentage")?;
        if percentage < 0.0 {
            return Err("Percentage must be non-negative".to_string());
        }

        let comparison = get_param_string(condition, "comparison")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&comparison.as_str()) {
            return Err(format!("Invalid comparison: {}", comparison));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "RelativeToMA",
            "name": "Price vs Moving Average",
            "category": "Technical Indicators",
            "tags": ["ma", "trend", "technical"],
            "icon": "📉",
            "origin": "strategy",
            "description": "Check if price is above/below/within range of its moving average",
            "parameters": {
                "period": {
                    "type": "number",
                    "name": "MA Period",
                    "description": "Number of candles for moving average calculation",
                    "default": 20,
                    "min": 2,
                    "max": 200,
                    "step": 1
                },
                "comparison": {
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
                "percentage": {
                    "type": "percent",
                    "name": "Distance %",
                    "description": "Percentage distance from MA (for ABOVE/BELOW: minimum, WITHIN: maximum)",
                    "default": 1.0,
                    "min": 0.1,
                    "max": 50.0,
                    "step": 0.5
                }
            }
        })
    }
}
