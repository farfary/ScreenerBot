// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Restates the average entry price, paid fees and realized P&L of bot positions from their swap records under the moving-average rule.

use crate::chains::RawAmount;
use crate::logger::{self, LogTag};

use super::apply::book_position;
use super::cost_basis::{recorded_fees, replay};
use super::db::{self, Booking, Committed};
use super::pnl::realized_pnl;
use super::round_state::is_closed;
use super::types::{EntryRecord, ExitRecord, Position};
use super::Result;

/// The tolerance, relative to the larger value, within which two money figures are the same.
const SAME_VALUE_RELATIVE: f64 = 1e-12;

/// The tolerance, in native units, within which the invested total equals what the buy
/// records spent.
const INVESTED_TOLERANCE_NATIVE: f64 = 1e-9;

/// Restates every bot position whose swap records show both a partial exit and a DCA, or
/// whose booked fee totals differ from the fees its records carry: the average entry price
/// is replayed from the records under the moving-average rule, the fee totals become the
/// records' sums, and a closed position's realized P&L is taken again from its booked totals.
///
/// A position whose records do not account for exactly what it acquired, sold and invested
/// is never written. Each write is a booking on the row read in its own transaction, so a
/// swap booked in between leaves that row to the next run. A second run changes nothing.
/// Returns the number of positions restated.
pub async fn restate_cost_basis() -> Result<usize> {
    let chain = db::get_store_chain().await?;
    let mut restated = 0;
    for position in db::load_all_positions().await? {
        let Some(position_id) = position.id else {
            continue;
        };
        if position.is_wallet_derived() {
            continue;
        }
        let entries = db::get_entry_history(position_id).await?;
        let exits = db::get_exit_history(position_id).await?;
        if !(covered(&position, &entries, &exits) && qualifies(&position, &entries, &exits)) {
            continue;
        }
        let Some(decimals) = crate::tokens::get_decimals(chain, &position.mint).await else {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Cost basis of position {position_id} is not restated: decimals of {} are unknown",
                    position.mint
                ),
            );
            continue;
        };
        let committed = book_position(position_id, |row, _| {
            let Some(restated) = restatement(row, &entries, &exits, decimals) else {
                return Ok(Booking::Skip(()));
            };
            *row = restated;
            Ok(Booking::Write {
                record: None,
                outcome: (),
            })
        })
        .await?;
        if let Committed::Written { row, .. } = committed {
            restated += 1;
            logger::info(
                LogTag::Positions,
                &format!(
                    "Cost basis of position {position_id} restated from its records: average entry price {:.12}, entry fees {:?}, exit fees {:?}, realized P&L {:?}",
                    row.average_entry_price, row.entry_fee_raw, row.exit_fee_raw, row.pnl
                ),
            );
        }
    }
    Ok(restated)
}

/// `row` restated from its records, or `None` when the records do not cover it, it does not
/// qualify, or nothing would change.
fn restatement(
    row: &Position,
    entries: &[EntryRecord],
    exits: &[ExitRecord],
    decimals: u8,
) -> Option<Position> {
    if !(covered(row, entries, exits) && qualifies(row, entries, exits)) {
        return None;
    }
    let replayed = replay(entries, exits, decimals)?;
    let mut restated = row.clone();
    if let Some(average) = replayed.average {
        restated.average_entry_price = average;
    }
    restated.entry_fee_raw = replayed.entry_fees_raw.or(row.entry_fee_raw);
    restated.exit_fee_raw = replayed.exit_fees_raw.or(row.exit_fee_raw);
    if is_closed(row) && row.pnl.is_some() {
        let (pnl, pnl_percent) = realized_pnl(&restated);
        restated.pnl = Some(pnl);
        restated.pnl_percent = Some(pnl_percent);
    }
    let unchanged = same_value(restated.average_entry_price, row.average_entry_price)
        && restated.entry_fee_raw == row.entry_fee_raw
        && restated.exit_fee_raw == row.exit_fee_raw
        && same_optional_value(restated.pnl, row.pnl)
        && same_optional_value(restated.pnl_percent, row.pnl_percent);
    (!unchanged).then_some(restated)
}

/// Whether the records account for the whole of `row`: one entry buy, every token acquired
/// and sold, what is held, and the invested total.
fn covered(row: &Position, entries: &[EntryRecord], exits: &[ExitRecord]) -> bool {
    let Some(held) = row.remaining_token_amount else {
        return false;
    };
    if entries.iter().filter(|entry| !entry.is_dca).count() != 1 {
        return false;
    }
    let Some(bought) = sum_amounts(entries.iter().map(|entry| entry.amount)) else {
        return false;
    };
    let Some(sold) = sum_amounts(exits.iter().map(|exit| exit.amount)) else {
        return false;
    };
    let spent: f64 = entries.iter().map(|entry| entry.native_spent).sum();
    bought.checked_sub(sold) == Some(held)
        && sold == row.total_exited_amount
        && (spent - row.total_size_native).abs() <= INVESTED_TOLERANCE_NATIVE
}

/// Whether `row` is one the restate applies to: its records show a partial exit and a DCA,
/// so its average may carry the cost of tokens already sold, or a fee total differs from
/// what its records carry.
fn qualifies(row: &Position, entries: &[EntryRecord], exits: &[ExitRecord]) -> bool {
    let partial_and_dca =
        entries.iter().any(|entry| entry.is_dca) && exits.iter().any(|exit| exit.is_partial);
    let (entry_fees, exit_fees) = recorded_fees(entries, exits);
    partial_and_dca
        || entry_fees.is_some_and(|fees| row.entry_fee_raw != Some(fees))
        || exit_fees.is_some_and(|fees| row.exit_fee_raw != Some(fees))
}

/// The sum of `amounts`, `None` on overflow.
fn sum_amounts(mut amounts: impl Iterator<Item = RawAmount>) -> Option<RawAmount> {
    amounts.try_fold(RawAmount::ZERO, RawAmount::checked_add)
}

/// Whether `a` and `b` are the same figure, up to the rounding of float arithmetic.
fn same_value(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= SAME_VALUE_RELATIVE * a.abs().max(b.abs())
}

fn same_optional_value(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => same_value(a, b),
        _ => false,
    }
}
