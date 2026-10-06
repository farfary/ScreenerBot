// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token-holding bookkeeping on a position: what it holds now, and checked moves between the
//! held and exited amounts. A method that returns an error leaves the position unchanged.

use crate::chains::RawAmount;

use super::types::Position;
use super::{Error, Result};

impl Position {
    /// Raw amount held now: the remaining amount once recorded, otherwise the entry fill.
    pub fn held_amount(&self) -> Option<RawAmount> {
        self.remaining_token_amount.or(self.token_amount)
    }

    /// Raw amount ever acquired: what is held plus what has exited. `None` only when the sum
    /// overflows.
    pub fn acquired_amount(&self) -> Option<RawAmount> {
        self.remaining_token_amount
            .unwrap_or_default()
            .checked_add(self.total_exited_amount)
    }

    /// Books `sold` as exited and lowers the remaining amount, floored at zero. Returns the
    /// new exited total.
    pub fn book_exit(&mut self, sold: RawAmount) -> Result<RawAmount> {
        let Some(exited) = self.total_exited_amount.checked_add(sold) else {
            return Err(Error::AmountOverflow {
                mint: self.mint.clone(),
                operation: "booking an exit",
            });
        };
        self.remaining_token_amount = self
            .remaining_token_amount
            .map(|remaining| remaining.checked_sub(sold).unwrap_or(RawAmount::ZERO));
        self.total_exited_amount = exited;
        Ok(exited)
    }

    /// Adds `bought` to the remaining amount. Returns the new remaining amount.
    pub fn book_acquisition(&mut self, bought: RawAmount) -> Result<RawAmount> {
        let held = match self.remaining_token_amount {
            Some(remaining) => remaining.checked_add(bought),
            None => Some(bought),
        };
        let Some(held) = held else {
            return Err(Error::AmountOverflow {
                mint: self.mint.clone(),
                operation: "booking an acquisition",
            });
        };
        self.remaining_token_amount = Some(held);
        Ok(held)
    }

    /// Moves the whole remaining amount into the exited total, leaving zero held. Returns the
    /// amount moved; a position with no remaining amount recorded is left unchanged.
    pub fn book_remaining_as_exited(&mut self) -> Result<RawAmount> {
        let Some(remaining) = self.remaining_token_amount else {
            return Ok(RawAmount::ZERO);
        };
        let Some(exited) = self.total_exited_amount.checked_add(remaining) else {
            return Err(Error::AmountOverflow {
                mint: self.mint.clone(),
                operation: "booking the remaining amount as exited",
            });
        };
        self.total_exited_amount = exited;
        self.remaining_token_amount = Some(RawAmount::ZERO);
        Ok(remaining)
    }
}
