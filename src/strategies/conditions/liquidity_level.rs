use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Pool liquidity level condition - check if pool has sufficient/excessive liquidity
pub struct LiquidityLevelCondition;

#[async_trait]
impl ConditionEvaluator for LiquidityLevelCondition {
    fn condition_type(&self) -> &'static str {
        "LiquidityLevel"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let threshold = get_param_f64(condition, "threshold")?;
        let comparison = get_param_string(condition, "comparison")?;

        let market_data = context
            .market_data
            .as_ref()
            .ok_or_else(|| "Market data not available".to_string())?;

        let liquidity = market_data
            .liquidity_sol
            .ok_or_else(|| "Liquidity data not available".to_string())?;

        let result = match comparison.as_str() {
            "GREATER_THAN" => liquidity > threshold,
            "LESS_THAN" => liquidity < threshold,
            "GREATER_EQUAL" => liquidity >= threshold,
            "LESS_EQUAL" => liquidity <= threshold,
            _ => return Err(format!("Invalid comparison: {}", comparison)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let threshold = get_param_f64(condition, "threshold")?;
        if threshold < 0.0 {
            return Err("Threshold must be non-negative".to_string());
        }

        let comparison = get_param_string(condition, "comparison")?;
        let valid_comparisons = ["GREATER_THAN", "LESS_THAN", "GREATER_EQUAL", "LESS_EQUAL"];
        if !valid_comparisons.contains(&comparison.as_str()) {
            return Err(format!("Invalid comparison: {}", comparison));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "LiquidityLevel",
            "name": "Pool Liquidity Level",
            "category": "Market Context",
            "tags": ["liquidity", "safety", "sol", "risk"],
            "icon": "💧",
            "origin": "strategy",
            "description": "Check pool liquidity in SOL (Entry: ensure sufficient liquidity, Exit: detect liquidity drain)",
            "parameters": {
                "threshold": {
                    "type": "number",
                    "name": "Liquidity Threshold (SOL)",
                    "description": "Pool liquidity level in SOL",
                    "default": 50.0,
                    "min": 0.0,
                    "max": 100000.0,
                    "step": 10.0
                },
                "comparison": {
                    "type": "enum",
                    "name": "Comparison",
                    "description": "How to compare pool liquidity to threshold",
                    "default": "GREATER_THAN",
                    "options": [
                        { "value": "GREATER_THAN", "label": "Greater Than (>)" },
                        { "value": "GREATER_EQUAL", "label": "Greater or Equal (≥)" },
                        { "value": "LESS_THAN", "label": "Less Than (<)" },
                        { "value": "LESS_EQUAL", "label": "Less or Equal (≤)" }
                    ]
                }
            }
        })
    }
}
