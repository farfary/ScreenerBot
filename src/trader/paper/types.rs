// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Quote-backed paper fills and their router-sample provenance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::PaperFillError;
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

/// What the paper executor did with a decision, carried on its `TradeResult` so a
/// caller reads the outcome from its type, never from the error text.
#[derive(Debug, Clone, PartialEq)]
pub enum PaperExecution {
    /// A leg was booked, now or by an earlier call under the same leg key.
    Filled(PaperLegEvidence),
    /// Nothing was booked.
    Refused(PaperRefusal),
}

impl PaperExecution {
    /// The booked leg, when the decision filled.
    pub fn filled(&self) -> Option<&PaperLegEvidence> {
        match self {
            PaperExecution::Filled(evidence) => Some(evidence),
            PaperExecution::Refused(_) => None,
        }
    }

    /// The refusal, when nothing was booked.
    pub fn refusal(&self) -> Option<PaperRefusal> {
        match self {
            PaperExecution::Filled(_) => None,
            PaperExecution::Refused(refusal) => Some(*refusal),
        }
    }
}

/// The paper leg a decision booked.
#[derive(Debug, Clone, PartialEq)]
pub struct PaperLegEvidence {
    /// The leg reference, also the signature of the round's entry or exit record.
    pub leg_ref: String,
    pub position_id: i64,
    pub fill: PaperLegFill,
    /// True when the leg was booked by an earlier call under the same key and
    /// this call booked nothing.
    pub replayed: bool,
}

/// The fill of a booked paper leg.
#[derive(Debug, Clone, PartialEq)]
pub enum PaperLegFill {
    Buy(PaperFill),
    Sell(PaperSellFill),
}

/// Why the paper executor booked nothing for a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperRefusal {
    /// The decision's owner no longer wants it (trader, strategy or task changed,
    /// force stop), checked before the fill and again after the landing wait.
    NotWanted,
    /// The paper balance does not cover the trade.
    BalanceExhausted,
    /// Every paper slot is taken.
    PositionLimit,
    /// The latest paper open was less than the open cooldown ago.
    OpenCooldown { wait_secs: u64 },
    /// The wallet already holds an open paper round of the mint.
    AlreadyOpen,
    /// The token's metadata is not available.
    TokenUnavailable,
    /// The token's decimals are not known.
    DecimalsUnknown,
    /// The decision carries no positive size.
    InvalidSize,
    /// The quote-based fill did not settle.
    Fill(PaperFillError),
    /// The decision names no open paper round.
    NoOpenRound,
    /// The round closed before the leg was booked.
    AlreadyClosed,
    /// The take-profit step was already taken.
    StepTaken,
    /// The paper store refused the booking.
    BookingFailed,
}

#[cfg(test)]
mod tests;
