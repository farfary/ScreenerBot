// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token age condition — the time since the token's on-chain creation against a duration.

use crate::strategies::conditions::{get_param_choice, get_param_f64_in_range, ConditionEvaluator};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The token is older, or younger, than a duration since its on-chain creation
///
/// Only the on-chain creation time counts: a pool or discovery time is a different fact,
/// and a token whose creation time is unknown leaves the condition unknown.
pub struct TokenAgeCondition;

/// Validated parameters of a token age condition.
struct Params {
    older: bool,
    seconds: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let comparison = get_param_choice(
            condition,
            "comparison",
            "comparison",
            &["OLDER_THAN", "YOUNGER_THAN"],
        )?;
        let value = get_param_f64_in_range(condition, "time_value", "time value", 1.0, 10_000.0)?;
        let unit = get_param_choice(
            condition,
            "time_unit",
            "time unit",
            &["MINUTES", "HOURS", "DAYS"],
        )?;
        let unit_seconds = match unit.as_str() {
            "MINUTES" => 60.0,
            "HOURS" => 3_600.0,
            _ => 86_400.0,
        };
        Ok(Self {
            older: comparison == "OLDER_THAN",
            seconds: value * unit_seconds,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for TokenAgeCondition {
    fn condition_type(&self) -> &'static str {
        "TokenAge"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let market = context
            .market_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "market data",
            })?;
        let created = market.token_created_at.ok_or(Error::MissingContextData {
            data: "token creation time",
        })?;
        let age_seconds = (context.evaluated_at - created).num_seconds() as f64;
        if age_seconds < 0.0 {
            return Err(Error::UnusableData {
                field: "token creation time",
                value: created.to_rfc3339(),
            });
        }

        Ok(if params.older {
            age_seconds >= params.seconds
        } else {
            age_seconds < params.seconds
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn applies_to(&self) -> &'static [StrategyType] {
        &[StrategyType::Entry]
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Market
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "TokenAge",
            "category": "Market Context",
            "icon": "icon-sprout",
            "origin": "strategy",
            "parameters": {
                "comparison": {
                    "type": "enum",
                    "default": "OLDER_THAN",
                    "options": [
                        { "value": "OLDER_THAN" },
                        { "value": "YOUNGER_THAN" }
                    ]
                },
                "time_value": {
                    "type": "number",
                    "default": 30,
                    "min": 1,
                    "max": 10000,
                    "step": 1
                },
                "time_unit": {
                    "type": "enum",
                    "default": "MINUTES",
                    "options": [
                        { "value": "MINUTES" },
                        { "value": "HOURS" },
                        { "value": "DAYS" }
                    ]
                }
            }
        })
    }
}
