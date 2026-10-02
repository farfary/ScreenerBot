// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position holding time condition — triggers based on how long a position has been held.

use crate::strategies::conditions::{get_param_f64, get_param_string, ConditionEvaluator};
use crate::strategies::types::{Condition, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Position holding time condition - check how long position has been held
pub struct PositionHoldingTimeCondition;

#[async_trait]
impl ConditionEvaluator for PositionHoldingTimeCondition {
    fn condition_type(&self) -> &'static str {
        "PositionHoldingTime"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let hours = get_param_f64(condition, "hours")?;
        let comparison = get_param_string(condition, "comparison")?;

        let position_data =
            context
                .position_data
                .as_ref()
                .ok_or_else(|| Error::MissingContextData {
                    data: "position data",
                })?;

        let position_age_hours = position_data.position_age_hours;

        let result = match comparison.as_str() {
            "GREATER_THAN" => position_age_hours > hours,
            "LESS_THAN" => position_age_hours < hours,
            "GREATER_EQUAL" => position_age_hours >= hours,
            "LESS_EQUAL" => position_age_hours <= hours,
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
        let hours = get_param_f64(condition, "hours")?;
        if hours < 0.0 {
            return Err(Error::InvalidConditionValue {
                field: "hours",
                value: hours.to_string(),
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
            "type": "PositionHoldingTime",
            "category": "Position & Performance",
            "icon": "icon-hourglass",
            "origin": "strategy",
            "parameters": {
                "hours": {
                    "type": "number",
                    "unit": "hours",
                    "default": 1.0,
                    "min": 0.0,
                    "max": 720.0,
                    "step": 0.25
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
