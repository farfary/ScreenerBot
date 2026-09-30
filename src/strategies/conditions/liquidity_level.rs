//! Liquidity level condition — checks pool liquidity against thresholds.

use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Pool liquidity level condition - check if pool has sufficient/excessive liquidity
pub struct LiquidityLevelCondition;

#[async_trait]
impl ConditionEvaluator for LiquidityLevelCondition {
    fn condition_type(&self) -> &'static str {
        "LiquidityLevel"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let threshold = get_param_f64(condition, "threshold")?;
        let comparison = get_param_string(condition, "comparison")?;

        let market_data =
            context
                .market_data
                .as_ref()
                .ok_or_else(|| Error::MissingContextData {
                    data: "market data",
                })?;

        let liquidity = market_data
            .liquidity_sol
            .ok_or_else(|| Error::MissingContextData {
                data: "liquidity data",
            })?;

        let result = match comparison.as_str() {
            "GREATER_THAN" => liquidity > threshold,
            "LESS_THAN" => liquidity < threshold,
            "GREATER_EQUAL" => liquidity >= threshold,
            "LESS_EQUAL" => liquidity <= threshold,
            _ => {
                return Err(Error::InvalidConditionValue {
                    field: "comparison",
                    value: comparison,
                })
            }
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        let threshold = get_param_f64(condition, "threshold")?;
        if threshold < 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "threshold",
                value: threshold.to_string(),
            });
        }

        let comparison = get_param_string(condition, "comparison")?;
        let valid_comparisons = ["GREATER_THAN", "LESS_THAN", "GREATER_EQUAL", "LESS_EQUAL"];
        if !valid_comparisons.contains(&comparison.as_str()) {
            return Err(Error::InvalidConditionValue {
                field: "comparison",
                value: comparison,
            });
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "LiquidityLevel",
            "category": "Market Context",
            "icon": "icon-droplet",
            "origin": "strategy",
            "parameters": {
                "threshold": {
                    "type": "number",
                    "default": 50.0,
                    "min": 0.0,
                    "max": 100000.0,
                    "step": 10.0
                },
                "comparison": {
                    "type": "enum",
                    "default": "GREATER_THAN",
                    "options": [
                        { "value": "GREATER_THAN" },
                        { "value": "GREATER_EQUAL" },
                        { "value": "LESS_THAN" },
                        { "value": "LESS_EQUAL" }
                    ]
                }
            }
        })
    }
}
