use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Price change percentage condition - check if price moved by percentage from entry/reference
pub struct PriceThresholdCondition;

#[async_trait]
impl ConditionEvaluator for PriceThresholdCondition {
    fn condition_type(&self) -> &'static str {
        "PriceThreshold"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let percentage = get_param_f64(condition, "percentage")?;
        let direction = get_param_string(condition, "direction")?;

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        // Get reference price (entry price for exit strategies, or current for entry)
        let reference_price = context
            .position_data
            .as_ref()
            .map(|p| p.entry_price)
            .unwrap_or(current_price);

        // Calculate price change percentage
        let price_change_pct = ((current_price - reference_price) / reference_price) * 100.0;

        let result = match direction.as_str() {
            "ABOVE" => price_change_pct >= percentage,
            "BELOW" => price_change_pct <= -percentage,
            "WITHIN" => price_change_pct.abs() <= percentage,
            _ => return Err(format!("Invalid direction: {}", direction)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let percentage = get_param_f64(condition, "percentage")?;
        if percentage < 0.0 {
            return Err("Percentage must be non-negative".to_string());
        }
        if percentage > 100.0 {
            return Err("Percentage must be 100 or less".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceThreshold",
            "name": "Price Change %",
            "category": "Price Patterns",
            "tags": ["price", "percentage", "threshold"],
            "icon": "🎯",
            "origin": "strategy",
            "description": "Check if price changed by a percentage (Entry: vs current, Exit: vs entry price)",
            "parameters": {
                "percentage": {
                    "type": "percent",
                    "name": "Price Change %",
                    "description": "Percentage price change to trigger (1-100%)",
                    "default": 5.0,
                    "min": 1.0,
                    "max": 100.0,
                    "step": 0.5
                },
                "direction": {
                    "type": "enum",
                    "name": "Direction",
                    "description": "Price movement direction",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE", "label": "Above (+%)" },
                        { "value": "BELOW", "label": "Below (-%)" },
                        { "value": "WITHIN", "label": "Within (±%)" }
                    ]
                }
            }
        })
    }
}

