// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Transaction velocity condition — the trade rate of a short window against a longer one.

use crate::strategies::conditions::{
    get_param_choice, get_param_f64_in_range, market_window, ConditionEvaluator,
};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use serde_json::json;

/// The trade rate of a short window, per second, against the rate of the longer window
/// that contains it is above or below a ratio
pub struct TransactionVelocityCondition;

/// The window pairs offered: `(id, short window, its seconds, long window, its seconds)`.
const WINDOW_PAIRS: [(&str, &str, f64, &str, f64); 3] = [
    ("5m_vs_1h", "5m", 300.0, "1h", 3_600.0),
    ("1h_vs_6h", "1h", 3_600.0, "6h", 21_600.0),
    ("6h_vs_24h", "6h", 21_600.0, "24h", 86_400.0),
];

/// Validated parameters of a transaction velocity condition.
struct Params {
    short: &'static str,
    short_seconds: f64,
    long: &'static str,
    long_seconds: f64,
    above: bool,
    ratio: f64,
    min_transactions: f64,
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let ids: Vec<&str> = WINDOW_PAIRS.iter().map(|pair| pair.0).collect();
        let window = get_param_choice(condition, "window", "window", &ids)?;
        let (_, short, short_seconds, long, long_seconds) = WINDOW_PAIRS
            .into_iter()
            .find(|pair| pair.0 == window)
            .ok_or(Error::InvalidConditionValue {
                field: "window",
                value: window.clone(),
            })?;
        let comparison =
            get_param_choice(condition, "comparison", "comparison", &["ABOVE", "BELOW"])?;
        Ok(Self {
            short,
            short_seconds,
            long,
            long_seconds,
            above: comparison == "ABOVE",
            ratio: get_param_f64_in_range(condition, "ratio", "ratio", 0.1, 50.0)?,
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
impl ConditionEvaluator for TransactionVelocityCondition {
    fn condition_type(&self) -> &'static str {
        "TransactionVelocity"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;

        // The short window first: a snapshot older than it answers neither window.
        let short = market_window(context, params.short)?.total() as f64;
        let long = market_window(context, params.long)?.total() as f64;

        if short < params.min_transactions {
            return Ok(false);
        }
        if long <= 0.0 {
            return Err(Error::UnusableData {
                field: "long window transactions",
                value: long.to_string(),
            });
        }

        let ratio = (short / params.short_seconds) / (long / params.long_seconds);
        Ok(if params.above {
            ratio >= params.ratio
        } else {
            ratio <= params.ratio
        })
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Market
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "TransactionVelocity",
            "category": "Market Context",
            "icon": "icon-activity",
            "origin": "strategy",
            "parameters": {
                "window": {
                    "type": "enum",
                    "default": "5m_vs_1h",
                    "options": [
                        { "value": "5m_vs_1h" },
                        { "value": "1h_vs_6h" },
                        { "value": "6h_vs_24h" }
                    ]
                },
                "comparison": {
                    "type": "enum",
                    "default": "ABOVE",
                    "options": [
                        { "value": "ABOVE" },
                        { "value": "BELOW" }
                    ]
                },
                "ratio": {
                    "type": "number",
                    "unit": "multiplier",
                    "default": 2.0,
                    "min": 0.1,
                    "max": 50,
                    "step": 0.1
                },
                "min_transactions": {
                    "type": "number",
                    "unit": "trades",
                    "default": 30,
                    "min": 1,
                    "max": 100000,
                    "step": 10
                }
            }
        })
    }
}
