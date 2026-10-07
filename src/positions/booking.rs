// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure bookings of verified fills onto a position: no locks, no I/O, no logging. A method
//! that returns an error leaves the position unchanged.

use chrono::{DateTime, Utc};

use crate::chains::RawAmount;

use super::pnl::realized_pnl;
use super::types::Position;
use super::Result;

/// A verified entry swap.
pub(crate) struct EntryFill {
    pub effective_entry_price: f64,
    pub token_amount: RawAmount,
    pub fee_raw: u64,
    pub native_size: f64,
}

/// A verified full-close swap.
pub(crate) struct CloseFill {
    pub effective_exit_price: f64,
    pub native_received: f64,
    pub fee_raw: u64,
    pub exit_time: DateTime<Utc>,
}

/// An operator write-off of a position, with no swap behind it.
pub(crate) struct ForceCloseFill {
    pub exit_time: DateTime<Utc>,
    pub exit_price: f64,
    pub closed_reason: String,
}

/// A verified partial-exit swap. `unrealized_pnl` is the (P&L, percent) of what is still
/// held after it, when a current price was known.
pub(crate) struct PartialExitFill {
    pub exit_amount: RawAmount,
    pub native_received: f64,
    pub effective_exit_price: f64,
    pub unrealized_pnl: Option<(f64, f64)>,
}

/// A verified DCA swap.
pub(crate) struct DcaFill {
    pub tokens_bought: RawAmount,
    pub native_spent: f64,
    pub dca_time: DateTime<Utc>,
}

/// How a recompute of the average entry price went.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DcaAverage {
    /// Recomputed from the new totals.
    Recomputed,
    /// Left unchanged: the held amount normalised to a non-positive or non-finite value.
    InvalidNormalization,
    /// Left unchanged: nothing is held or the invested total is not a positive finite value.
    InvalidState,
}

impl Position {
    /// Records a verified entry fill.
    pub(crate) fn apply_entry_fill(&mut self, fill: &EntryFill) {
        self.transaction_entry_verified = true;
        self.effective_entry_price = Some(fill.effective_entry_price);
        self.total_size_native = fill.native_size;
        self.token_amount = Some(fill.token_amount);
        self.entry_fee_raw = Some(fill.fee_raw);
        self.entry_size_native = fill.native_size;
        self.remaining_token_amount = Some(fill.token_amount);
        self.average_entry_price = fill.effective_entry_price;
    }

    /// Books a verified full close: the remaining amount moves into the exited total, the
    /// proceeds accumulate onto those of earlier partial exits, and the realized P&L is
    /// taken from the booked totals. Returns the amount this close sold.
    pub(crate) fn book_close(&mut self, fill: &CloseFill) -> Result<RawAmount> {
        let closed = self.book_remaining_as_exited()?;
        self.transaction_exit_verified = true;
        self.effective_exit_price = Some(fill.effective_exit_price);
        // Accumulate: partial exits have already added their proceeds, and closed P&L is
        // computed from this total.
        self.native_received =
            Some(self.native_received.unwrap_or_default() + fill.native_received);
        self.exit_fee_raw = Some(fill.fee_raw);
        self.exit_time = Some(fill.exit_time);

        if let Some(reason) = &self.closed_reason {
            if reason.ends_with(super::PENDING_VERIFICATION_SUFFIX) {
                self.closed_reason = Some(
                    reason
                        .trim_end_matches(super::PENDING_VERIFICATION_SUFFIX)
                        .to_string(),
                );
            }
        }

        let (pnl, pnl_percent) = realized_pnl(self);
        self.pnl = Some(pnl);
        self.pnl_percent = Some(pnl_percent);
        self.unrealized_pnl = None;
        self.unrealized_pnl_percent = None;
        Ok(closed)
    }

    /// Writes the position off by operator action: whatever is still held moves into the
    /// exited total with no proceeds, while the proceeds of earlier partial exits stand. The
    /// realized P&L is taken from the booked legs, fees included.
    pub(crate) fn book_force_close(&mut self, fill: &ForceCloseFill) -> Result<()> {
        self.book_remaining_as_exited()?;
        self.remaining_token_amount = Some(RawAmount::ZERO);
        self.synthetic_exit = true;
        self.transaction_exit_verified = true;
        self.exit_time = Some(fill.exit_time);
        self.exit_price = Some(fill.exit_price);
        self.effective_exit_price = Some(0.0);
        self.closed_reason = Some(fill.closed_reason.clone());

        self.native_received = Some(self.native_received.unwrap_or_default());
        let (pnl, pnl_percent) = realized_pnl(self);
        self.pnl = Some(pnl);
        self.pnl_percent = Some(pnl_percent);
        self.unrealized_pnl = None;
        self.unrealized_pnl_percent = None;
        Ok(())
    }

    /// Books the leg of a verified partial exit onto a closed position: its proceeds join
    /// the books. The exited amount stays, since the close already counted every token it
    /// held as exited; the round decides what is held afterwards.
    pub(crate) fn book_late_partial_exit(&mut self, native_received: f64) {
        self.native_received = Some(self.native_received.unwrap_or_default() + native_received);
        self.partial_exit_count += 1;
    }

    /// Books the leg of a verified full-close swap onto a closed position: its proceeds,
    /// price and fee join the books, and the exited amount stays as the close counted it. A
    /// write-off is from then on closed by this sale, at the sale's time.
    pub(crate) fn book_late_close(&mut self, fill: &CloseFill) {
        self.native_received =
            Some(self.native_received.unwrap_or_default() + fill.native_received);
        self.effective_exit_price = Some(fill.effective_exit_price);
        self.exit_fee_raw = Some(fill.fee_raw);
        if self.synthetic_exit {
            self.exit_time = Some(fill.exit_time);
            self.synthetic_exit = false;
        }
    }

    /// Books a verified partial exit. The position stays open: no exit time or exit
    /// signature is set. Returns the new exited total.
    pub(crate) fn book_partial_exit(&mut self, fill: &PartialExitFill) -> Result<RawAmount> {
        let total_exited = self.book_exit(fill.exit_amount)?;

        // Weighted average exit price. The exited total includes this exit, so the
        // subtraction cannot underflow.
        if total_exited > RawAmount::ZERO {
            if let Some(prev_avg) = self.average_exit_price {
                let prev_weight = (total_exited.raw() - fill.exit_amount.raw()) as f64
                    / total_exited.raw() as f64;
                let new_weight = fill.exit_amount.raw() as f64 / total_exited.raw() as f64;
                self.average_exit_price =
                    Some((prev_avg * prev_weight) + (fill.effective_exit_price * new_weight));
            } else {
                self.average_exit_price = Some(fill.effective_exit_price);
            }
        }

        self.partial_exit_count += 1;
        self.native_received =
            Some(self.native_received.unwrap_or_default() + fill.native_received);

        if let Some((pnl, pnl_percent)) = fill.unrealized_pnl {
            self.unrealized_pnl = Some(pnl);
            self.unrealized_pnl_percent = Some(pnl_percent);
        }
        Ok(total_exited)
    }

    /// Books a verified DCA add: the tokens join the held amount and the SOL joins the
    /// invested total. The average entry price is the caller's to recompute, and only
    /// for a position that holds the round once the add is booked: on a closed row the
    /// held amount is not yet the round's.
    pub(crate) fn book_dca(&mut self, fill: &DcaFill) -> Result<()> {
        self.book_acquisition(fill.tokens_bought)?;
        self.total_size_native += fill.native_spent;
        self.dca_count += 1;
        self.last_dca_time = Some(fill.dca_time);
        Ok(())
    }

    /// Recomputes the average entry price from the invested total and the held amount, in a
    /// token of `decimals`, when both are valid.
    pub(crate) fn recompute_average_entry_price(&mut self, decimals: u8) -> DcaAverage {
        let remaining = self.remaining_token_amount.unwrap_or_default();
        if !(remaining > RawAmount::ZERO
            && self.total_size_native > 0.0
            && self.total_size_native.is_finite())
        {
            return DcaAverage::InvalidState;
        }
        let normalized = remaining.to_whole_units(decimals);
        if normalized > 0.0 && normalized.is_finite() {
            self.average_entry_price = self.total_size_native / normalized;
            DcaAverage::Recomputed
        } else {
            DcaAverage::InvalidNormalization
        }
    }

    /// Drops the sale that was in flight when the position was written off, once the chain
    /// proved that sale failed or never landed: the write-off stands as the close, and no
    /// signature is left that would be verified again.
    pub(crate) fn drop_failed_written_off_sale(&mut self) {
        self.exit_transaction_signature = None;
    }

    /// Clears a failed close so it can be retried: the exit signature, the verified flag
    /// and the exit prices it stamped.
    pub(crate) fn clear_failed_exit(&mut self) {
        self.exit_transaction_signature = None;
        self.transaction_exit_verified = false;
        self.closed_reason = Some(super::EXIT_RETRY_PENDING.to_owned());
        // The close did not happen: a still-open position carrying exit prices reads as
        // closed to every check that looks at `exit_price`.
        self.exit_price = None;
        self.effective_exit_price = None;
    }
}
