// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure bookings of verified fills onto a position: no locks, no I/O, no logging. A method
//! that returns an error leaves the position unchanged.

use chrono::{DateTime, Utc};

use crate::chains::RawAmount;

use super::cost_basis::average_after_acquisition;
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
    pub fee_raw: u64,
}

/// A verified DCA swap.
pub(crate) struct DcaFill {
    pub tokens_bought: RawAmount,
    pub native_spent: f64,
    pub fee_raw: u64,
    pub dca_time: DateTime<Utc>,
}

/// How a blend of a buy into the average entry price went.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DcaAverage {
    /// Moved to the moving average of what is held after the buy.
    Recomputed,
    /// Left unchanged: the amounts normalised to values the moving average cannot use, or
    /// earlier tokens are held with no usable average before the buy.
    InvalidNormalization,
    /// Left unchanged: nothing is held, nothing was bought, or the buy's cost is not a
    /// positive finite value.
    InvalidState,
}

/// Adds `fee` to the fees `slot` already holds.
pub(super) fn add_fee(slot: &mut Option<u64>, fee: u64) {
    *slot = Some(slot.unwrap_or(0).saturating_add(fee));
}

impl Position {
    /// Records a verified entry fill; its fee joins the paid buy fees.
    pub(crate) fn apply_entry_fill(&mut self, fill: &EntryFill) {
        self.transaction_entry_verified = true;
        self.effective_entry_price = Some(fill.effective_entry_price);
        self.total_size_native = fill.native_size;
        self.token_amount = Some(fill.token_amount);
        add_fee(&mut self.entry_fee_raw, fill.fee_raw);
        self.entry_size_native = fill.native_size;
        self.remaining_token_amount = Some(fill.token_amount);
        self.average_entry_price = fill.effective_entry_price;
    }

    /// Books a verified full close: the remaining amount moves into the exited total, the
    /// proceeds accumulate onto those of earlier partial exits, the fee joins the paid sale
    /// fees, and the realized P&L is taken from the booked totals. Returns the amount this
    /// close sold.
    pub(crate) fn book_close(&mut self, fill: &CloseFill) -> Result<RawAmount> {
        let closed = self.book_remaining_as_exited()?;
        self.transaction_exit_verified = true;
        self.effective_exit_price = Some(fill.effective_exit_price);
        // Accumulate: partial exits have already added their proceeds, and closed P&L is
        // computed from this total.
        self.native_received =
            Some(self.native_received.unwrap_or_default() + fill.native_received);
        add_fee(&mut self.exit_fee_raw, fill.fee_raw);
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
    /// the books and its fee joins the paid sale fees. The exited amount stays, since the
    /// close already counted every token it held as exited; the round decides what is held
    /// afterwards.
    pub(crate) fn book_late_partial_exit(&mut self, native_received: f64, fee_raw: u64) {
        self.native_received = Some(self.native_received.unwrap_or_default() + native_received);
        add_fee(&mut self.exit_fee_raw, fee_raw);
        self.partial_exit_count += 1;
    }

    /// Books the leg of a verified full-close swap onto a closed position: its proceeds and
    /// price join the books, its fee joins the paid sale fees, and the exited amount stays as
    /// the close counted it. A write-off is from then on closed by this sale, at the sale's
    /// time.
    pub(crate) fn book_late_close(&mut self, fill: &CloseFill) {
        self.native_received =
            Some(self.native_received.unwrap_or_default() + fill.native_received);
        self.effective_exit_price = Some(fill.effective_exit_price);
        add_fee(&mut self.exit_fee_raw, fill.fee_raw);
        if self.synthetic_exit {
            self.exit_time = Some(fill.exit_time);
            self.synthetic_exit = false;
        }
    }

    /// Books a verified partial exit; its fee joins the paid sale fees. The position stays
    /// open: no exit time or exit signature is set. Returns the new exited total.
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
        add_fee(&mut self.exit_fee_raw, fill.fee_raw);

        if let Some((pnl, pnl_percent)) = fill.unrealized_pnl {
            self.unrealized_pnl = Some(pnl);
            self.unrealized_pnl_percent = Some(pnl_percent);
        }
        Ok(total_exited)
    }

    /// Books a verified DCA add: the tokens join the held amount, the SOL joins the
    /// invested total and the fee joins the paid buy fees. The average entry price is the
    /// caller's to blend, and only for a position that holds the round once the add is
    /// booked: on a closed row the held amount is not yet the round's.
    pub(crate) fn book_dca(&mut self, fill: &DcaFill) -> Result<()> {
        self.book_acquisition(fill.tokens_bought)?;
        self.total_size_native += fill.native_spent;
        add_fee(&mut self.entry_fee_raw, fill.fee_raw);
        self.dca_count += 1;
        self.last_dca_time = Some(fill.dca_time);
        Ok(())
    }

    /// Blends a booked buy of `bought` for `spent` into the average entry price under the
    /// moving-average rule (see [`average_after_acquisition`]), with the held amount after
    /// the buy in a token of `decimals`. The invested total keeps the cost of tokens already
    /// sold and never enters the average.
    pub(crate) fn blend_average_entry_price(
        &mut self,
        bought: RawAmount,
        spent: f64,
        decimals: u8,
    ) -> DcaAverage {
        let held_after = self.remaining_token_amount.unwrap_or_default();
        if !(held_after > RawAmount::ZERO
            && bought > RawAmount::ZERO
            && spent > 0.0
            && spent.is_finite())
        {
            return DcaAverage::InvalidState;
        }
        match average_after_acquisition(
            self.average_entry_price,
            bought.to_whole_units(decimals),
            spent,
            held_after.to_whole_units(decimals),
        ) {
            Some(average) => {
                self.average_entry_price = average;
                DcaAverage::Recomputed
            }
            None => DcaAverage::InvalidNormalization,
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
