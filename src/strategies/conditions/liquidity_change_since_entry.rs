// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Liquidity change since entry condition — the pool's native reserves now against those at entry.

use crate::strategies::conditions::{
    get_param_choice, get_param_f64_in_range, usable_basis, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The pool's native reserves dropped, or rose, by at least `change_pct` since the
/// position opened
///
/// The entry reading is the reserves of the pool price the entry was made at. A position
/// opened before that reading was recorded, imported or adopted has none, and leaves the
/// condition unknown rather than measured against a guess.
///
/// Only two readings of one pool's real reserves compare. When the live price comes from
/// another pool than the entry did (a graduation, a deeper pool found later), or from a
/// pool whose decoder reports virtual reserves (a bonding curve, whose fixed virtual base
/// dilutes every change), or from a pool whose program is not known, the condition is
/// unknown.
pub struct LiquidityChangeSinceEntryCondition;

/// Validated parameters of a liquidity change since entry condition.
struct Params {
    dropped: bool,
    change_pct: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let comparison = get_param_choice(
            condition,
            "comparison",
            "comparison",
            &["DROPPED_BY", "ROSE_BY"],
        )?;
        Ok(Self {
            dropped: comparison == "DROPPED_BY",
            change_pct: get_param_f64_in_range(condition, "change_pct", "change", 1.0, 100.0)?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for LiquidityChangeSinceEntryCondition {
    fn condition_type(&self) -> &'static str {
        "LiquidityChangeSinceEntry"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let position = context
            .position_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "position data",
            })?;
        let at_entry = position
            .entry_liquidity_native
            .ok_or(Error::MissingContextData {
                data: "entry liquidity",
            })?;
        let at_entry = usable_basis("entry liquidity", at_entry)?;
        let market = context
            .market_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "liquidity data",
            })?;
        let now = market.liquidity_sol.ok_or(Error::MissingContextData {
            data: "liquidity data",
        })?;
        let same_pool = market
            .liquidity_pool
            .as_ref()
            .zip(position.entry_liquidity_pool.as_deref())
            .is_some_and(|(live, entry)| live.address == entry);
        if !same_pool {
            return Err(Error::MissingContextData {
                data: "entry pool liquidity",
            });
        }
        if market
            .liquidity_pool
            .as_ref()
            .and_then(|pool| pool.virtual_reserves)
            != Some(false)
        {
            return Err(Error::MissingContextData {
                data: "real pool reserves",
            });
        }
        if !now.is_finite() || now < 0.0 {
            return Err(Error::UnusableData {
                field: "pool liquidity",
                value: now.to_string(),
            });
        }

        let change_pct = (now - at_entry) / at_entry * 100.0;
        Ok(if params.dropped {
            -change_pct >= params.change_pct
        } else {
            change_pct >= params.change_pct
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn applies_to(&self) -> &'static [StrategyType] {
        &[StrategyType::Exit]
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Pool
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "LiquidityChangeSinceEntry",
            "category": "Position & Performance",
            "icon": "icon-waves",
            "origin": "strategy",
            "parameters": {
                "comparison": {
                    "type": "enum",
                    "default": "DROPPED_BY",
                    "options": [
                        { "value": "DROPPED_BY" },
                        { "value": "ROSE_BY" }
                    ]
                },
                "change_pct": {
                    "type": "percent",
                    "default": 30,
                    "min": 1,
                    "max": 100,
                    "step": 1
                }
            }
        })
    }
}
