use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;

/// Position age condition - check how long position has been open
pub struct PositionAgeCondition;

#[async_trait]
impl ConditionEvaluator for PositionAgeCondition {
    fn condition_type(&self) -> &'static str {
        "PositionAge"
    }

    async fn evaluate(
        &self,
        condition: &Condition,
        context: &EvaluationContext,
    ) -> Result<bool, String> {
        let hours = get_param_f64(condition, "hours")?;
        let comparison = get_param_string(condition, "comparison")?;

        let position_data = context
            .position_data
            .as_ref()
            .ok_or_else(|| "Position data not available".to_string())?;

        let position_age_hours = position_data.position_age_hours;

        let result = match comparison.as_str() {
            "GREATER_THAN" => position_age_hours > hours,
            "LESS_THAN" => position_age_hours < hours,
            "GREATER_THAN_OR_EQUAL" => position_age_hours >= hours,
            "LESS_THAN_OR_EQUAL" => position_age_hours <= hours,
            _ => return Err(format!("Invalid comparison: {}", comparison)),
        };

        Ok(result)
    }

    fn validate(&self, condition: &Condition) -> Result<(), String> {
        let hours = get_param_f64(condition, "hours")?;
        if hours < 0.0 {
            return Err("Hours must be non-negative".to_string());
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
            "type": "PositionAge",
            "name": "Position Age",
            "category": "Position & Performance",
            "tags": ["position", "risk"],
            "icon": "⏱️",
            "origin": "strategy",
            "description": "Check how long position has been open",
            "parameters": {
                "hours": {
                    "type": "number",
                    "description": "Age threshold in hours",
                    "default": 1.0,
                    "min": 0.0
                },
                "comparison": {
                    "type": "string",
                    "description": "Comparison operator",
                    "default": "GREATER_THAN",
                    "options": ["GREATER_THAN", "LESS_THAN", "GREATER_THAN_OR_EQUAL", "LESS_THAN_OR_EQUAL"]
                }
            }
        })
    }
}
