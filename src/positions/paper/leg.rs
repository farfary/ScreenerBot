// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! One paper leg: the reference that stands in for a transaction signature, the
//! strategy behind it and the quote evidence of its simulated fill.

use chrono::{DateTime, Utc};

use crate::positions::db::{PaperLegKind, PaperLegRow};
use crate::positions::{Error, Result};
use crate::trader::paper::{PaperFill, PaperSellFill};

/// Prefix of every paper leg reference; no transaction signature carries it.
pub const PAPER_LEG_PREFIX: &str = "paper-";

/// One simulated fill, with the reference its entry or exit record is booked under. A
/// retried booking passes the same leg, so the reference makes it idempotent; a leg built
/// from a caller's key carries the same reference on every call, so a decision replayed
/// under that key books once.
#[derive(Debug, Clone)]
pub struct PaperLeg {
    pub leg_ref: String,
    pub strategy_id: Option<String>,
    pub at: DateTime<Utc>,
    quote_json: String,
}

impl PaperLeg {
    /// The reference of the leg booked under the caller's `key`.
    pub fn reference(key: &str) -> String {
        format!("{PAPER_LEG_PREFIX}{key}")
    }

    /// The leg of a simulated buy, referenced by `key` when given.
    pub fn buy(
        fill: &PaperFill,
        strategy_id: Option<String>,
        at: DateTime<Utc>,
        key: Option<&str>,
    ) -> Result<Self> {
        Self::new(serde_json::to_string(fill), strategy_id, at, key)
    }

    /// The leg of a simulated sale, referenced by `key` when given.
    pub fn sell(
        fill: &PaperSellFill,
        strategy_id: Option<String>,
        at: DateTime<Utc>,
        key: Option<&str>,
    ) -> Result<Self> {
        Self::new(serde_json::to_string(fill), strategy_id, at, key)
    }

    fn new(
        quote_json: serde_json::Result<String>,
        strategy_id: Option<String>,
        at: DateTime<Utc>,
        key: Option<&str>,
    ) -> Result<Self> {
        let quote_json = quote_json.map_err(|e| Error::PaperLegEncode {
            detail: e.to_string(),
        })?;
        Ok(Self {
            leg_ref: match key {
                Some(key) => Self::reference(key),
                None => Self::reference(&uuid::Uuid::new_v4().to_string()),
            },
            strategy_id,
            at,
            quote_json,
        })
    }

    pub(super) fn row(&self, kind: PaperLegKind) -> PaperLegRow<'_> {
        PaperLegRow {
            kind,
            leg_ref: &self.leg_ref,
            strategy_id: self.strategy_id.as_deref(),
            quote_json: &self.quote_json,
            created_at: self.at,
        }
    }
}
