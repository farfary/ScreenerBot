// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Atomic booking: a position row, its idempotence guard and its history record in one transaction.

use rusqlite::{params, Connection, OptionalExtension};

use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position};
use crate::positions::{Error, Result};

use super::operations::write_position_row;
use super::types::PositionsDatabase;

/// The condition, read inside the booking transaction, under which a booking has not
/// happened yet.
pub(crate) enum BookingGuard<'a> {
    /// Always book; the transition overwrites rather than accumulates.
    Unconditional,
    /// Book only while the stored row is not exit-verified.
    ExitNotVerified,
    /// Book only while no exit record exists for this position and signature.
    ExitRecordAbsent(&'a str),
    /// Book only while no entry record exists for this position and signature.
    EntryRecordAbsent(&'a str),
}

/// The history record written in the same transaction as the row.
pub(crate) enum BookingRecord {
    Exit(ExitRecord),
    Entry(EntryRecord),
}

/// Outcome of a booking transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BookingCommit {
    /// The row and the record were written and committed.
    Committed,
    /// The guard found the booking already present; nothing was written.
    AlreadyBooked,
}

impl PositionsDatabase {
    /// Writes `position`'s row and `record` in one IMMEDIATE transaction when `guard`
    /// holds. Every failure rolls the transaction back, leaving the row and the records
    /// as they were.
    pub(crate) fn commit_booking(
        &self,
        position: &Position,
        guard: BookingGuard<'_>,
        record: Option<&BookingRecord>,
        wallet_address: Option<&str>,
    ) -> Result<BookingCommit> {
        let position_id = position.id.ok_or_else(|| Error::TransitionFailed {
            transition: "update",
            mint: position.mint.clone(),
            detail: "position has no id".to_owned(),
        })?;
        let record = match (record, wallet_address) {
            (Some(record), Some(wallet_address)) => Some((record, wallet_address)),
            (None, _) => None,
            (Some(_), None) => {
                return Err(Error::WalletUnavailable {
                    detail: "no wallet address supplied for the booking record".to_owned(),
                })
            }
        };

        let mut conn = self.get_connection()?;
        let written = run_booking(
            &mut conn,
            self.chain.as_str(),
            position_id,
            position,
            &guard,
            record,
        )
        .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e))?;

        match written {
            None => Ok(BookingCommit::AlreadyBooked),
            Some(0) => Err(Error::NotFoundById { position_id }),
            Some(_) => {
                if let Some((record, _)) = record {
                    log_saved_record(record);
                }
                if let Ok(mut stmt) = conn.prepare("PRAGMA wal_checkpoint(PASSIVE);") {
                    let _ = stmt.query([]);
                }
                Ok(BookingCommit::Committed)
            }
        }
    }
}

/// Runs the booking transaction. `None` means the guard found the booking already
/// present; `Some(0)` means no row matched, and the transaction was rolled back.
fn run_booking(
    conn: &mut Connection,
    chain: &str,
    position_id: i64,
    position: &Position,
    guard: &BookingGuard<'_>,
    record: Option<(&BookingRecord, &str)>,
) -> rusqlite::Result<Option<usize>> {
    let tx = conn.write_tx()?;

    let already_booked = match guard {
        BookingGuard::Unconditional => false,
        BookingGuard::ExitNotVerified => tx
            .query_row(
                "SELECT transaction_exit_verified FROM positions WHERE id = ?1 AND chain_id = ?2",
                params![position_id, chain],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .unwrap_or(false),
        BookingGuard::ExitRecordAbsent(signature) => tx
            .query_row(
                "SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
                params![position_id, signature],
                |_| Ok(()),
            )
            .optional()?
            .is_some(),
        BookingGuard::EntryRecordAbsent(signature) => tx
            .query_row(
                "SELECT 1 FROM position_entries WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
                params![position_id, signature],
                |_| Ok(()),
            )
            .optional()?
            .is_some(),
    };
    if already_booked {
        return Ok(None);
    }

    let written = write_position_row(&tx, chain, position_id, position)?;
    if written == 0 {
        return Ok(Some(0));
    }

    match record {
        Some((BookingRecord::Exit(record), wallet_address)) => {
            insert_exit_record(&tx, wallet_address, record)?;
        }
        Some((BookingRecord::Entry(record), wallet_address)) => {
            insert_entry_record(&tx, wallet_address, record)?;
        }
        None => {}
    }

    tx.commit()?;
    Ok(Some(written))
}

/// Inserts an exit record unless one already exists for its position and signature: one
/// on-chain swap is one exit record.
fn insert_exit_record(
    conn: &Connection,
    wallet_address: &str,
    record: &ExitRecord,
) -> rusqlite::Result<usize> {
    conn.execute(
    "INSERT INTO position_exits (position_id, wallet_address, timestamp, amount, price, native_received,
     transaction_signature, is_partial, percentage, fees_raw)
     SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10
     WHERE NOT EXISTS (
       SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?7
     )",
    params![
      record.position_id,
      wallet_address,
      record.timestamp.to_rfc3339(),
      record.amount,
      record.price,
      record.native_received,
      record.transaction_signature,
      record.is_partial,
      record.percentage,
      record.fees_raw.map(|f| f as i64),
    ],
  )
}

/// Inserts an entry record unless one already exists for its position and signature: one
/// on-chain swap is one entry record.
fn insert_entry_record(
    conn: &Connection,
    wallet_address: &str,
    record: &EntryRecord,
) -> rusqlite::Result<usize> {
    conn.execute(
    "INSERT INTO position_entries (position_id, wallet_address, timestamp, amount, price, native_spent,
     transaction_signature, is_dca, fees_raw)
     SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9
     WHERE NOT EXISTS (
       SELECT 1 FROM position_entries WHERE position_id = ?1 AND transaction_signature = ?7
     )",
    params![
      record.position_id,
      wallet_address,
      record.timestamp.to_rfc3339(),
      record.amount,
      record.price,
      record.native_spent,
      record.transaction_signature,
      record.is_dca,
      record.fees_raw.map(|f| f as i64),
    ],
  )
}

fn log_saved_record(record: &BookingRecord) {
    let message = match record {
        BookingRecord::Exit(record) => format!(
            "Saved exit record: position={} amount={} partial={} tx={}",
            record.position_id, record.amount, record.is_partial, record.transaction_signature
        ),
        BookingRecord::Entry(record) => format!(
            "Saved entry record: position={} amount={} dca={} tx={}",
            record.position_id, record.amount, record.is_dca, record.transaction_signature
        ),
    };
    logger::info(LogTag::Positions, &message);
}
