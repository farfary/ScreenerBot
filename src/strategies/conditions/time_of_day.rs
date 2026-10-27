// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Time-of-day condition — the UTC hour of the evaluation inside a window of hours.

use crate::strategies::conditions::{get_param_f64_in_range, ConditionEvaluator};
use crate::strategies::types::{Condition, ConditionData, EvaluationContext};
use crate::strategies::{Error, Result};
use async_trait::async_trait;
use chrono::Timelike;
use serde_json::json;

/// The UTC hour of the evaluation instant is inside `[start_hour, end_hour)`; a start after
/// the end wraps past midnight
///
/// The hours are UTC so a strategy means the same thing on every machine; the editor shows
/// the local equivalent beside them.
pub struct TimeOfDayCondition;

/// Validated parameters of a time-of-day condition.
struct Params {
    start_hour: u32,
    end_hour: u32,
}

/// A whole hour in range, refusing a fraction rather than truncating it.
fn whole_hour(
    condition: &Condition,
    param: &'static str,
    field: &'static str,
    min: f64,
    max: f64,
) -> Result<u32> {
    let hour = get_param_f64_in_range(condition, param, field, min, max)?;
    if hour.fract() != 0.0 {
        return Err(Error::InvalidConditionValue {
            field,
            value: hour.to_string(),
        });
    }
    Ok(hour as u32)
}

impl Params {
    fn read(condition: &Condition) -> Result<Self> {
        let start_hour = whole_hour(condition, "start_hour", "start hour", 0.0, 23.0)?;
        let end_hour = whole_hour(condition, "end_hour", "end hour", 1.0, 24.0)?;
        if start_hour == end_hour {
            return Err(Error::InvalidConditionValue {
                field: "end hour",
                value: end_hour.to_string(),
            });
        }
        Ok(Self {
            start_hour,
            end_hour,
        })
    }

    fn contains(&self, hour: u32) -> bool {
        if self.start_hour < self.end_hour {
            (self.start_hour..self.end_hour).contains(&hour)
        } else {
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}

#[async_trait]
impl ConditionEvaluator for TimeOfDayCondition {
    fn condition_type(&self) -> &'static str {
        "TimeOfDay"
    }

    async fn evaluate(&self, condition: &Condition, context: &EvaluationContext) -> Result<bool> {
        let params = Params::read(condition)?;
        Ok(params.contains(context.evaluated_at.hour()))
    }

    fn validate(&self, condition: &Condition) -> Result<()> {
        Params::read(condition).map(|_| ())
    }

    fn reads(&self) -> ConditionData {
        ConditionData::Clock
    }

    fn parameter_schema(&self) -> serde_json::Value {
        json!({
            "type": "TimeOfDay",
            "category": "Market Context",
            "icon": "icon-calendar-clock",
            "origin": "strategy",
            "parameters": {
                "start_hour": {
                    "type": "number",
                    "unit": "utc_hour",
                    "hint": "local_time",
                    "default": 13,
                    "min": 0,
                    "max": 23,
                    "step": 1
                },
                "end_hour": {
                    "type": "number",
                    "unit": "utc_hour",
                    "hint": "local_time",
                    "default": 22,
                    "min": 1,
                    "max": 24,
                    "step": 1
                }
            }
        })
    }
}
