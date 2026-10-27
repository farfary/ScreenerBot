// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Profit giveback condition — the share of the position's best unrealized gain given back.

use crate::strategies::conditions::{
    get_current_price, get_param_f64_in_range, usable_basis, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// At least `giveback_pct` of the best unrealized gain is given back, once that gain
/// reached `arm_after_profit_pct`
///
/// The best gain is the basis peak (the highest pool price since the average entry price
/// last changed) against the average entry price, so a later buy restarts it. A giveback
/// of 100% means the price is back at the entry price.
pub struct ProfitGivebackCondition;

/// Percentage points within which a derived share reaches its threshold. Both shares are
/// computed in floating point from prices, so a price exactly at a threshold can compute
/// just under it (an average of 0.8 and a peak of 1.0 is a 24.999999999999993% gain).
const THRESHOLD_TOLERANCE_PCT: f64 = 1e-9;

/// Validated parameters of a profit giveback condition.
struct Params {
    arm_after_profit_pct: f64,
    giveback_pct: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        Ok(Self {
            arm_after_profit_pct: get_param_f64_in_range(
                condition,
                "arm_after_profit_pct",
                "arm after profit",
                1.0,
                10_000.0,
            )?,
            giveback_pct: get_param_f64_in_range(
                condition,
                "giveback_pct",
                "giveback",
                5.0,
                100.0,
            )?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for ProfitGivebackCondition {
    fn condition_type(&self) -> &'static str {
        "ProfitGiveback"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let position = context
            .position_data
            .as_ref()
            .ok_or(Error::MissingContextData {
                data: "position data",
            })?;
        let peak = position
            .peak_price
            .ok_or(Error::MissingContextData { data: "peak price" })?;
        let peak = usable_basis("peak price", peak)?;
        let entry = usable_basis("entry price", position.entry_price)?;
        let current = get_current_price(context)?;

        let peak_profit_pct = (peak - entry) / entry * 100.0;
        if peak_profit_pct <= 0.0
            || peak_profit_pct + THRESHOLD_TOLERANCE_PCT < params.arm_after_profit_pct
        {
            return Ok(false);
        }
        let current_profit_pct = (current - entry) / entry * 100.0;
        let giveback_pct = (peak_profit_pct - current_profit_pct) / peak_profit_pct * 100.0;
        Ok(giveback_pct + THRESHOLD_TOLERANCE_PCT >= params.giveback_pct)
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
            "type": "ProfitGiveback",
            "category": "Position & Performance",
            "icon": "icon-coins",
            "origin": "strategy",
            "parameters": {
                "arm_after_profit_pct": {
                    "type": "percent",
                    "default": 30,
                    "min": 1,
                    "max": 10000,
                    "step": 1
                },
                "giveback_pct": {
                    "type": "percent",
                    "default": 50,
                    "min": 5,
                    "max": 100,
                    "step": 1
                }
            }
        })
    }
}
