use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Price change percentage condition - check if price moved by % from reference price
pub struct PriceChangePercentCondition;

#[async_trait]
impl ConditionEvaluator for PriceChangePercentCondition {
    fn condition_type(&self) -> &'static str {
        "PriceChangePercent"
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
        if percentage > 1000.0 {
            return Err("Percentage must be 1000 or less".to_string());
        }

        let direction = get_param_string(condition, "direction")?;
        if !["ABOVE", "BELOW", "WITHIN"].contains(&direction.as_str()) {
            return Err(format!("Invalid direction: {}", direction));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceChangePercent",
            "name": "Price Change %",
            "category": "Price Analysis",
            "tags": ["price", "percentage", "change"],
            "icon": "📈",
            "origin": "strategy",
            "description": "Check if price changed by a percentage threshold (Entry: vs current price, Exit: vs entry price)",
            "parameters": {
                "percentage": {
                    "type": "percent",
                    "name": "Change Threshold %",
                    "description": "Percentage price change to trigger (1-1000%)",
                    "default": 10.0,
                    "min": 0.1,
                    "max": 1000.0,
                    "step": 0.5
                },
                "direction": {
                    "type": "enum",
                    "name": "Direction",
                    "description": "Price movement direction",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE", "label": "Gain (+%)" },
                        { "value": "BELOW", "label": "Loss (-%)"},
                        { "value": "WITHIN", "label": "Within Range (±%)" }
                    ]
                }
            }
        })
    }
}
