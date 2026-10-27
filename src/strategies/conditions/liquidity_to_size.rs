// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Liquidity-to-size condition — the pool's native reserves as a multiple of the size at stake.

use crate::strategies::conditions::{
    get_param_choice, get_param_f64_in_range, usable_basis, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The pool's native reserves are at least, or below, a multiple of the size at stake
///
/// The size is the trade an entry would spend in an entry strategy and the value the
/// position still holds in an exit strategy; both come with the context, so the condition
/// reads no configuration. A constant-product swap of size `s` into reserves `R` moves the
/// price by about `s / (R + s)`, so a multiple of `N` is about `1 / (N + 1)` price impact.
pub struct LiquidityToSizeCondition;

/// Validated parameters of a liquidity-to-size condition.
struct Params {
    at_least: bool,
    multiple: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let comparison = get_param_choice(
            condition,
            "comparison",
            "comparison",
            &["AT_LEAST", "BELOW"],
        )?;
        Ok(Self {
            at_least: comparison == "AT_LEAST",
            multiple: get_param_f64_in_range(condition, "multiple", "multiple", 1.0, 10_000.0)?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for LiquidityToSizeCondition {
    fn condition_type(&self) -> &'static str {
        "LiquidityToSize"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let market = context
            .market_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "market data",
            })?;
        let liquidity = market.liquidity_sol.ok_or(Error::MissingContextData {
            data: "liquidity data",
        })?;
        let liquidity = usable_basis("pool liquidity", liquidity)?;

        let size = match context.position_data.as_ref() {
            Some(position) => {
                let value = position
                    .remaining_value_native
                    .ok_or(Error::MissingContextData {
                        data: "remaining position value",
                    })?;
                usable_basis("remaining position value", value)?
            }
            None => {
                let size = market
                    .reference_size_native
                    .ok_or(Error::MissingContextData { data: "trade size" })?;
                usable_basis("trade size", size)?
            }
        };

        let multiple = liquidity / size;
        Ok(if params.at_least {
            multiple >= params.multiple
        } else {
            multiple < params.multiple
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Pool
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "LiquidityToSize",
            "category": "Market Context",
            "icon": "icon-ruler",
            "origin": "strategy",
            "parameters": {
                "comparison": {
                    "type": "enum",
                    "default": "AT_LEAST",
                    "options": [
                        { "value": "AT_LEAST" },
                        { "value": "BELOW" }
                    ]
                },
                "multiple": {
                    "type": "number",
                    "unit": "multiplier",
                    "hint": "price_impact",
                    "default": 50,
                    "min": 1,
                    "max": 10000,
                    "step": 1
                }
            }
        })
    }
}
