// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Buy pressure condition — the share of buys among a window's transactions.

use crate::strategies::conditions::{
    get_param_choice, get_param_f64_in_range, market_window, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext, WindowCounts};
use crate::strategies::Result;
use async_trait::async_trait;
use serde_json::json;

/// Buys make up at least a share of a window's transactions, with a minimum of activity
pub struct BuyPressureCondition;

/// Validated parameters of a buy pressure condition.
struct Params {
    window: String,
    min_buy_share_pct: f64,
    min_transactions: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        Ok(Self {
            window: get_param_choice(condition, "window", "window", &["5m", "1h", "6h", "24h"])?,
            min_buy_share_pct: get_param_f64_in_range(
                condition,
                "min_buy_share_pct",
                "minimum buy share",
                50.0,
                100.0,
            )?,
            min_transactions: get_param_f64_in_range(
                condition,
                "min_transactions",
                "minimum transactions",
                1.0,
                100_000.0,
            )?,
        })
    }
}

#[async_trait]
impl ConditionEvaluator for BuyPressureCondition {
    fn condition_type(&self) -> &'static str {
        "BuyPressure"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        let WindowCounts { buys, sells } = market_window(context, &params.window)?;

        let total = buys.saturating_add(sells);
        if (total as f64) < params.min_transactions {
            return Ok(false);
        }

        Ok(buys as f64 / total as f64 * 100.0 >= params.min_buy_share_pct)
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Market
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "BuyPressure",
            "category": "Market Context",
            "icon": "icon-scale",
            "origin": "strategy",
            "parameters": {
                "window": {
                    "type": "enum",
                    "default": "1h",
                    "options": [
                        { "value": "5m" },
                        { "value": "1h" },
                        { "value": "6h" },
                        { "value": "24h" }
                    ]
                },
                "min_buy_share_pct": {
                    "type": "percent",
                    "default": 60,
                    "min": 50,
                    "max": 100,
                    "step": 1
                },
                "min_transactions": {
                    "type": "number",
                    "unit": "trades",
                    "default": 50,
                    "min": 1,
                    "max": 100000,
                    "step": 10
                }
            }
        })
    }
}
