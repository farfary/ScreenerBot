// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Copy paper context, decisions, exit rules and the archived copy paper book.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::CopyTelemetry;
use crate::trader::paper::{PaperFill, PaperModel};

/// The paper model a paper decision was made under. Rows written before the shared
/// paper book also carry the retired per-task run counter, which decoding ignores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperContext {
    pub model: PaperModel,
}

/// One task's holding of one token in the archived copy paper book, accumulated
/// across its buys and sells. Read-only history: paper copies are rounds of the
/// shared paper book. `cost_basis_native` is the cost of the tokens still held.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArchivedPaperPosition {
    pub task_id: i64,
    pub mint: String,
    pub token_amount: f64,
    pub cost_basis_native: f64,
    pub realized_proceeds_native: f64,
    pub realized_cost_native: f64,
    pub last_price_native: Option<f64>,
    pub last_price_at: Option<DateTime<Utc>>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
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
