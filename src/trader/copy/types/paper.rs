// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy paper context, decisions and exact-balance positions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::CopyTelemetry;
use crate::chains::RawAmount;
use crate::trader::paper::{PaperFill, PaperModel};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperContext {
    pub model: PaperModel,
    pub epoch: u64,
}

/// One task's paper holding in one token, accumulated across its buys and sells.
/// `cost_basis_native` is the cost of the tokens still held; realized figures only
/// ever grow.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PaperPosition {
    pub task_id: i64,
    pub mint: String,
    pub token_amount: f64,
    /// Exact active holding. Archived pre-v8 estimates deliberately have no raw balance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_token_amount: Option<RawAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_decimals: Option<u8>,
    pub cost_basis_native: f64,
    pub invested_native: f64,
    pub realized_proceeds_native: f64,
    pub realized_cost_native: f64,
    pub buys: u64,
    pub sells: u64,
    pub last_price_native: Option<f64>,
    pub last_price_at: Option<DateTime<Utc>>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// Highest pool price seen while this round was open; arms the trailing stop.
    pub peak_price_native: Option<f64>,
    /// Take-profit steps this round has taken.
    pub take_profit_level: u32,
    /// Partial sales this round has booked; `sells` counts every round's sales.
    pub round_partial_exits: u32,
}

impl PaperPosition {
    pub fn is_open(&self) -> bool {
        if self.closed_at.is_some() {
            return false;
        }
        self.raw_token_amount
            .map_or(self.token_amount > 0.0, |raw| raw > RawAmount::ZERO)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperDecision {
    pub task_id: i64,
    pub target_address: String,
    pub signature: String,
    pub mint: String,
    pub target_size_sol: f64,
    #[serde(default)]
    pub target_token_amount: f64,
    pub sized_sol: f64,
    pub fill: PaperFill,
    pub telemetry: CopyTelemetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperExitRule {
    StopLoss,
    TrailingStop,
    BreakEven,
    TakeProfit,
    TimeOverride,
    TimeStagnation,
    /// Closed by hand from the dashboard or an agent tool.
    Manual,
}
