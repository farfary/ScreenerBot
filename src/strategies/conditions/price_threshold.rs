use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Price threshold condition - check if price is above/below a target value
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
        let value = get_param_f64(condition, "value")?;
        let comparison = get_param_string(condition, "comparison")?;

        let current_price = context
            .current_price
            .ok_or_else(|| "Current price not available".to_string())?;

        let result = match comparison.as_str() {
            "ABOVE" | "GREATER_THAN" => current_price > value,
            "BELOW" | "LESS_THAN" => current_price < value,
            "EQUAL" => (current_price - value).abs() < f64::EPSILON,
            "GREATER_THAN_OR_EQUAL" => current_price >= value,
            "LESS_THAN_OR_EQUAL" => current_price <= value,
            _ => return Err(format!("Invalid comparison operator: {}", comparison)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let value = get_param_f64(condition, "value")?;
        if value < 0.0 {
            return Err("Value must be non-negative".to_string());
        }

        let comparison = get_param_string(condition, "comparison")?;
        let valid_comparisons = [
            "ABOVE",
            "BELOW",
            "EQUAL",
            "GREATER_THAN",
            "LESS_THAN",
            "GREATER_THAN_OR_EQUAL",
            "LESS_THAN_OR_EQUAL",
        ];
        if !valid_comparisons.contains(&comparison.as_str()) {
            return Err(format!("Invalid comparison: {}", comparison));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PriceThreshold",
            "name": "Price Threshold",
            "category": "Price Patterns",
            "tags": ["price", "threshold"],
            "icon": "🎯",
            "origin": "strategy",
            "description": "Check if current price is above/below a target value",
            "parameters": {
                "value": {
                    "type": "number",
                    "description": "Target price value in SOL",
                    "default": 0.0,
                    "min": 0.0
                },
                "comparison": {
                    "type": "string",
                    "description": "Comparison operator",
                    "default": "ABOVE",
                    "options": ["ABOVE", "BELOW", "EQUAL", "GREATER_THAN", "LESS_THAN", "GREATER_THAN_OR_EQUAL", "LESS_THAN_OR_EQUAL"]
                }
            }
        })
    }
}
