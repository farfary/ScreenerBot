use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Liquidity depth condition - check pool liquidity level
pub struct LiquidityDepthCondition;

#[async_trait]
impl ConditionEvaluator for LiquidityDepthCondition {
    fn condition_type(&self) -> &'static str {
        "LiquidityDepth"
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
            "GREATER_THAN_OR_EQUAL" => liquidity >= threshold,
            "LESS_THAN_OR_EQUAL" => liquidity <= threshold,
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
        let valid_comparisons = [
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
            "type": "LiquidityDepth",
            "name": "Pool Liquidity Level",
            "category": "Market Context",
            "tags": ["liquidity", "safety", "sol"],
            "icon": "💧",
            "origin": "strategy",
            "description": "Check pool liquidity level in SOL. Used for both entry (ensure sufficient liquidity) and exit (check if liquidity dried up).",
            "parameters": {
                "threshold": {
                    "type": "number",
                    "name": "Liquidity Threshold (SOL)",
                    "description": "Minimum/maximum liquidity in SOL",
                    "default": 50.0,
                    "min": 0.0,
                    "max": 10000.0,
                    "step": 1.0
                },
                "comparison": {
                    "type": "enum",
                    "name": "Comparison",
                    "description": "How to compare pool liquidity to threshold",
                    "default": "GREATER_THAN",
                    "options": [
                        { "value": "GREATER_THAN", "label": "Greater Than (>)" },
                        { "value": "GREATER_THAN_OR_EQUAL", "label": "Greater or Equal (≥)" },
                        { "value": "LESS_THAN", "label": "Less Than (<)" },
                        { "value": "LESS_THAN_OR_EQUAL", "label": "Less or Equal (≤)" }
                    ]
                }
            }
        })
    }
}
