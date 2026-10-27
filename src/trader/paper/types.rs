// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Quote-backed paper fills and their router-sample provenance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    /// Closed at the round's pool mark with no quote, when a paper run is reset.
    Mark,
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

#[cfg(test)]
mod tests;
