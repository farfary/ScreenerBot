use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use async_trait::async_trait;
use serde_json::json;

/// Position holding time condition - check how long position has been held
pub struct PositionHoldingTimeCondition;

#[async_trait]
impl ConditionEvaluator for PositionHoldingTimeCondition {
    fn condition_type(&self) -> &'static str {
        "PositionHoldingTime"
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
            "GREATER_EQUAL" => position_age_hours >= hours,
            "LESS_EQUAL" => position_age_hours <= hours,
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
        let valid_comparisons = ["GREATER_THAN", "LESS_THAN", "GREATER_EQUAL", "LESS_EQUAL"];
        if !valid_comparisons.contains(&comparison.as_str()) {
            return Err(format!("Invalid comparison: {}", comparison));
        }

        Ok(())
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PositionHoldingTime",
            "name": "Position Holding Time",
            "category": "Position & Performance",
            "tags": ["position", "time", "duration", "exit"],
            "icon": "⏱️",
            "origin": "strategy",
            "description": "Check how long a position has been held (for exit strategies - time-based exits)",
            "parameters": {
                "hours": {
                    "type": "number",
                    "name": "Time Threshold (Hours)",
                    "description": "Duration in hours since position opened",
                    "default": 1.0,
                    "min": 0.0,
                    "max": 720.0,
                    "step": 0.25
                },
                "comparison": {
                    "type": "enum",
                    "name": "Comparison",
                    "description": "How to compare position age to threshold",
                    "default": "GREATER_THAN",
                    "options": [
                        { "value": "GREATER_THAN", "label": "Older Than (>)" },
                        { "value": "GREATER_EQUAL", "label": "At Least (≥)" },
                        { "value": "LESS_THAN", "label": "Younger Than (<)" },
                        { "value": "LESS_EQUAL", "label": "At Most (≤)" }
                    ]
                }
            }
        })
    }
}
