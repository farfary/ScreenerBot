// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Drawdown-from-peak condition — the pool price falling a percentage below the position's peak.

use crate::strategies::conditions::{
    get_current_price, get_param_f64_in_range, usable_basis, ConditionEvaluator,
};
use crate::strategies::types::{Condition, EvaluationContext, StrategyType};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The pool price fell `drop_pct` from the highest price seen since the average entry price
/// last changed, once that peak stood at least `arm_after_profit_pct` above the entry price
pub struct DrawdownFromPeakCondition;

/// Validated parameters of a drawdown-from-peak condition.
struct Params {
    drop_pct: f64,
    arm_after_profit_pct: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        Ok(Self {
            drop_pct: get_param_f64_in_range(condition, "drop_pct", "drop percentage", 0.1, 99.0)?,
            arm_after_profit_pct: get_param_f64_in_range(
                condition,
                "arm_after_profit_pct",
                "arm after profit",
                0.0,
                10000.0,
            )?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for DrawdownFromPeakCondition {
    fn condition_type(&self) -> &'static str {
        "DrawdownFromPeak"
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

        let drop_pct = (peak - current) / peak * 100.0;
        let peak_profit_pct = (peak - entry) / entry * 100.0;

        Ok(drop_pct >= params.drop_pct && peak_profit_pct >= params.arm_after_profit_pct)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn applies_to(&self) -> &'static [StrategyType] {
        &[StrategyType::Exit]
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "DrawdownFromPeak",
            "category": "Position & Performance",
            "icon": "icon-trending-down",
            "origin": "strategy",
            "parameters": {
                "drop_pct": {
                    "type": "percent",
                    "default": 10,
                    "min": 0.1,
                    "max": 99,
                    "step": 0.5
                },
                "arm_after_profit_pct": {
                    "type": "percent",
                    "default": 0,
                    "min": 0,
                    "max": 10000,
                    "step": 1
                }
            }
        })
    }
}
