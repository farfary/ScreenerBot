// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper or live mode of a copy task or a strategy, and the gate that arms live.

use serde::{Deserialize, Serialize};

use super::error::PaperModeError;
use crate::positions::Book;

/// Execution mode. Entering `Live` requires the confirmation-gated transition
/// before the runtime can submit a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradingMode {
    Paper,
    Live,
}

impl TradingMode {
    /// The persisted and serialized id.
    pub fn as_str(self) -> &'static str {
        match self {
            TradingMode::Paper => "paper",
            TradingMode::Live => "live",
        }
    }

    /// Parse a persisted id; `None` for anything else.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "paper" => Some(TradingMode::Paper),
            "live" => Some(TradingMode::Live),
            _ => None,
        }
    }

    /// The book a trade decided in this mode is booked on.
    pub fn book(self) -> Book {
        match self {
            TradingMode::Paper => Book::Paper,
            TradingMode::Live => Book::Live,
        }
    }
}

/// Explicit acknowledgement that arms a copy task live.
pub const LIVE_ARM_CONFIRMATION: &str = "ARM LIVE COPY TRADING";

/// Explicit acknowledgement that arms a strategy live.
pub const STRATEGY_LIVE_CONFIRMATION: &str = "ARM LIVE STRATEGY";

/// Gate a mode transition. Staying in a mode or returning to paper is always
/// allowed; entering live needs `confirmation` to equal `phrase` exactly.
pub fn confirm_live(
    current: TradingMode,
    requested: TradingMode,
    confirmation: Option<&str>,
    phrase: &str,
) -> Result<TradingMode, PaperModeError> {
    if current == requested || requested == TradingMode::Paper {
        return Ok(requested);
    }
    if confirmation != Some(phrase) {
        return Err(PaperModeError::LiveConfirmationRequired);
    }
    Ok(TradingMode::Live)
}

#[cfg(test)]
mod tests;
