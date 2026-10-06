// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure bookings of verified fills onto a position: no locks, no I/O, no logging. A method
//! that returns an error leaves the position unchanged.

use chrono::{DateTime, Utc};

use crate::chains::RawAmount;

use super::types::Position;
use super::Result;

/// A verified entry swap.
pub(crate) struct EntryFill {
    pub effective_entry_price: f64,
    pub token_amount: RawAmount,
    pub fee_raw: u64,
    pub native_size: f64,
}

/// A verified full-close swap. `pnl` is the closed (P&L, percent) once computed.
pub(crate) struct CloseFill {
    pub effective_exit_price: f64,
    pub native_received: f64,
    pub fee_raw: u64,
    pub exit_time: DateTime<Utc>,
    pub pnl: Option<(f64, f64)>,
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

/// A verified DCA swap, with the decimals of the token bought.
pub(crate) struct DcaFill {
    pub tokens_bought: RawAmount,
    pub native_spent: f64,
    pub dca_time: DateTime<Utc>,
    pub decimals: u8,
}

/// How a DCA booking treated the average entry price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum DcaAverage {
    /// Recomputed from the new totals.
    Recomputed,
    /// Left unchanged: the held amount normalised to a non-positive or non-finite value.
    InvalidNormalization,
    /// Left unchanged: nothing is held or the invested total is not a positive finite value.
    InvalidState,
}

/// Result of a DCA booking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DcaBooking {
    pub remaining: RawAmount,
    pub average: DcaAverage,
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

    /// Books a verified full close: the remaining amount moves into the exited total and
    /// the proceeds accumulate onto those of earlier partial exits. Returns the amount this
    /// close sold.
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

        if let Some((pnl, pnl_percent)) = fill.pnl {
            self.pnl = Some(pnl);
            self.pnl_percent = Some(pnl_percent);
        }
        self.unrealized_pnl = None;
        self.unrealized_pnl_percent = None;
        Ok(closed)
    }

    /// Writes the position off: whatever is still held moves into the exited total with no
    /// proceeds. Returns the realized P&L, which still counts earlier partial-exit proceeds.
    pub(crate) fn book_synthetic_close(&mut self, exit_time: DateTime<Utc>) -> Result<f64> {
        self.book_remaining_as_exited()?;
        self.synthetic_exit = true;
        self.transaction_exit_verified = true;
        self.exit_time = Some(exit_time);
        self.closed_reason = Some("synthetic_exit_permanent_failure".to_owned());

        let realized_pnl = self.native_received.unwrap_or_default() - self.total_size_native;
        self.pnl = Some(realized_pnl);
        self.pnl_percent = Some(if self.total_size_native > 0.0 {
            (realized_pnl / self.total_size_native) * 100.0
        } else {
            0.0
        });
        self.unrealized_pnl = None;
        self.unrealized_pnl_percent = None;
        Ok(realized_pnl)
    }

    /// Writes the position off by operator action: whatever is still held moves into the
    /// exited total with no proceeds, while the proceeds of earlier partial exits stand.
    /// Returns the realized P&L.
    pub(crate) fn book_force_close(&mut self, fill: &ForceCloseFill) -> Result<f64> {
        self.book_remaining_as_exited()?;
        self.remaining_token_amount = Some(RawAmount::ZERO);
        self.synthetic_exit = true;
        self.transaction_exit_verified = true;
        self.exit_time = Some(fill.exit_time);
        self.exit_price = Some(fill.exit_price);
        self.effective_exit_price = Some(0.0);
        self.closed_reason = Some(fill.closed_reason.clone());

        let realized_native = self.native_received.unwrap_or_default();
        self.native_received = Some(realized_native);
        let realized_pnl = realized_native - self.total_size_native;
        self.pnl = Some(realized_pnl);
        self.pnl_percent = Some(if self.total_size_native > 0.0 {
            (realized_pnl / self.total_size_native) * 100.0
        } else {
            0.0
        });
        self.unrealized_pnl = None;
        self.unrealized_pnl_percent = None;
        Ok(realized_pnl)
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

    /// Books a verified DCA add: the tokens join the held amount, the SOL joins the
    /// invested total and the average entry price is recomputed from both when they are
    /// valid.
    pub(crate) fn book_dca(&mut self, fill: &DcaFill) -> Result<DcaBooking> {
        let remaining = self.book_acquisition(fill.tokens_bought)?;
        self.total_size_native += fill.native_spent;

        let average = if remaining > RawAmount::ZERO
            && self.total_size_native > 0.0
            && self.total_size_native.is_finite()
        {
            let normalized = remaining.to_whole_units(fill.decimals);
            if normalized > 0.0 && normalized.is_finite() {
                self.average_entry_price = self.total_size_native / normalized;
                DcaAverage::Recomputed
            } else {
                DcaAverage::InvalidNormalization
            }
        } else {
            DcaAverage::InvalidState
        };

        self.dca_count += 1;
        self.last_dca_time = Some(fill.dca_time);
        Ok(DcaBooking { remaining, average })
    }

    /// Clears a failed close so it can be retried: the exit signature, the verified flag
    /// and the exit prices it stamped. Returns the cleared exit signature.
    pub(crate) fn clear_failed_exit(&mut self) -> Option<String> {
        let old_signature = self.exit_transaction_signature.take();
        self.transaction_exit_verified = false;
        self.closed_reason = Some("exit_retry_pending".to_owned());
        // The close did not happen: a still-open position carrying exit prices reads as
        // closed to every check that looks at `exit_price`.
        self.exit_price = None;
        self.effective_exit_price = None;
        old_signature
    }
}
