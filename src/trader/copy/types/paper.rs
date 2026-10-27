// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Quote-backed paper fills, provenance, and exact-balance positions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::CopyTelemetry;
use crate::chains::RawAmount;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperFill {
    pub input_sol: f64,
    pub market_price_sol: f64,
    /// True for historical fills whose market price came from a pool; quote fills may still have a market price.
    #[serde(default)]
    pub priced_from_pool: bool,
    pub fill_price_sol: f64,
    pub token_amount: f64,
    pub referral_fee_sol: f64,
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
    pub total_cost_sol: f64,
    /// Absent historical fills are estimates from the earlier pool-price model.
    #[serde(default)]
    pub model: PaperModel,
    #[serde(default)]
    pub quote: Option<PaperQuoteMetadata>,
}

impl PaperFill {
    pub fn has_quote_provenance(&self) -> bool {
        self.model == PaperModel::Quote && self.quote.is_some()
    }
}

/// A simulated sell leg with quote-derived proceeds after referral fees and estimated network costs.
/// Historical fills retain their earlier pool-price estimates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperSellFill {
    pub token_amount: f64,
    pub market_price_sol: f64,
    /// True for historical fills whose market price came from a pool; quote fills may still have a market price.
    #[serde(default)]
    pub priced_from_pool: bool,
    pub fill_price_sol: f64,
    pub gross_sol: f64,
    pub referral_fee_sol: f64,
    pub network_fee_sol: f64,
    pub priority_fee_sol: f64,
    pub net_proceeds_sol: f64,
    /// Absent historical fills are estimates from the earlier pool-price model.
    #[serde(default)]
    pub model: PaperModel,
    #[serde(default)]
    pub quote: Option<PaperQuoteMetadata>,
}

impl PaperSellFill {
    pub fn has_quote_provenance(&self) -> bool {
        self.model == PaperModel::Quote && self.quote.is_some()
    }
}

/// The execution model attached to a persisted paper leg. Missing values in
/// pre-quote JSON remain explicitly classified as legacy estimates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperModel {
    #[default]
    Legacy,
    Quote,
}

/// Provenance for the two size-specific router samples that define a paper fill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaperQuoteMetadata {
    pub router_id: String,
    /// Receipt time of the first size-specific quote.
    pub initial_at: DateTime<Utc>,
    /// Receipt time of the fresh quote after the configured wait.
    pub landing_at: DateTime<Utc>,
    pub requested_delay_ms: u64,
    /// Time from enqueue until the first quote request started.
    pub queue_duration_ms: u64,
    pub minimum_output_amount: RawAmount,
    pub final_output_amount: RawAmount,
    /// Exact quote input. On sells this is the token quantity booked out of the paper holding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_amount: Option<RawAmount>,
    /// Token precision used to derive the displayed whole-unit amount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_decimals: Option<u8>,
    pub epoch: u64,
    pub network_fee_estimated: bool,
    pub platform_fee_included_in_output: bool,
}

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
    TakeProfit,
    TimeOverride,
    /// Closed by hand from the dashboard or an agent tool.
    Manual,
}
