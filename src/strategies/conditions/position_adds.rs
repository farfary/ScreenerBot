// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position adds condition — the count of buys added to, or partial sells taken from, a position.

use crate::strategies::conditions::{get_param_choice, get_param_f64_in_range, ConditionEvaluator};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The position's adds after its first entry, or its partial sells, are at least or at
/// most a count
pub struct PositionAddsCondition;

/// Validated parameters of a position adds condition.
struct Params {
    partial_sells: bool,
    at_least: bool,
    count: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let subject =
            get_param_choice(condition, "subject", "subject", &["ADDS", "PARTIAL_SELLS"])?;
        let comparison = get_param_choice(
            condition,
            "comparison",
            "comparison",
            &["AT_LEAST", "AT_MOST"],
        )?;
        Ok(Self {
            partial_sells: subject == "PARTIAL_SELLS",
            at_least: comparison == "AT_LEAST",
            count: get_param_f64_in_range(condition, "count", "count", 0.0, 20.0)?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for PositionAddsCondition {
    fn condition_type(&self) -> &'static str {
        "PositionAdds"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let position = context
            .position_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "position data",
            })?;
        let booked = f64::from(if params.partial_sells {
            position.partial_exit_count
        } else {
            position.dca_count
        });

        Ok(if params.at_least {
            booked >= params.count
        } else {
            booked <= params.count
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn applies_to(&self) -> &'static [StrategyType] {
        &[StrategyType::Exit]
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Position
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "PositionAdds",
            "category": "Position & Performance",
            "icon": "icon-layers",
            "origin": "strategy",
            "parameters": {
                "subject": {
                    "type": "enum",
                    "default": "ADDS",
                    "options": [
                        { "value": "ADDS" },
                        { "value": "PARTIAL_SELLS" }
                    ]
                },
                "comparison": {
                    "type": "enum",
                    "default": "AT_LEAST",
                    "options": [
                        { "value": "AT_LEAST" },
                        { "value": "AT_MOST" }
                    ]
                },
                "count": {
                    "type": "number",
                    "default": 1,
                    "min": 0,
                    "max": 20,
                    "step": 1
                }
            }
        })
    }
}
