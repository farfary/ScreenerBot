// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position profit condition — the open position's unrealized profit against a percentage.

use crate::strategies::conditions::{get_param_choice, get_param_f64_in_range, ConditionEvaluator};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// Unrealized profit of the position (pool price against entry) is above or below a percentage
pub struct PositionProfitCondition;

/// Validated parameters of a position profit condition.
struct Params {
    above: bool,
    threshold_pct: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let comparison =
            get_param_choice(condition, "comparison", "comparison", &["ABOVE", "BELOW"])?;
        let threshold_pct =
            get_param_f64_in_range(condition, "threshold_pct", "threshold", -99.0, 10000.0)?;
        Ok(Self {
            above: comparison == "ABOVE",
            threshold_pct,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for PositionProfitCondition {
    fn condition_type(&self) -> &'static str {
        "PositionProfit"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let profit_pct = context
            .position_data
            .as_ref()
            .and_then(|position| position.unrealized_profit_pct)
            .filter(|pct| pct.is_finite())
            .ok_or(Error::MissingContextData {
                data: "position data",
            })?;

        Ok(if params.above {
            profit_pct >= params.threshold_pct
        } else {
            profit_pct <= params.threshold_pct
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
            "type": "PositionProfit",
            "category": "Position & Performance",
            "icon": "icon-trending-up",
            "origin": "strategy",
            "parameters": {
                "comparison": {
                    "type": "enum",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE" },
                        { "value": "BELOW" }
                    ]
                },
                "threshold_pct": {
                    "type": "percent",
                    "default": 20,
                    "min": -99,
                    "max": 10000,
                    "step": 1
                }
            }
        })
    }
}
