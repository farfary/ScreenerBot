// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Atomic booking: a position row read, decided on and written back with its history record in one transaction.

use rusqlite::{params, Connection, OptionalExtension};

use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position};
use crate::positions::{Error, Result};

use super::operations::write_position_row;
use super::queries::{query_trader_swap_legs, TraderSwapLeg};
use super::types::{PositionsDatabase, POSITION_SELECT_COLUMNS};

/// The history record written in the same transaction as the row.
pub(crate) enum BookingRecord {
    Exit(ExitRecord),
    Entry(EntryRecord),
}

/// What a booking decided for the row it was handed.
pub(crate) enum Booking<T> {
    /// Write the row as the booking left it, with its record when there is one.
    Write {
        record: Option<BookingRecord>,
        outcome: T,
    },
    /// Delete the row; its history records go with it through the foreign-key cascade.
    Delete { outcome: T },
    /// Write nothing: the booking is already on the row, or does not apply to it.
    Skip(T),
}

/// Outcome of a booking transaction.
pub(crate) enum Committed<T> {
    /// The row, as written, and its record were committed.
    Written { row: Position, outcome: T },
    /// The row, as it was read, was deleted.
    Deleted { row: Position, outcome: T },
    /// Nothing was written.
    Skipped(T),
}

/// The reads a booking may make inside its transaction, consistent with the row it was
/// handed.
pub(crate) struct BookingReads<'a> {
    conn: &'a Connection,
    position_id: i64,
    wallet_address: &'a Result<&'a str>,
}

impl BookingReads<'_> {
    /// True when this position already has an entry record for `signature`.
    pub(crate) fn entry_record_exists(&self, signature: &str) -> Result<bool> {
        self.record_exists(
            "SELECT 1 FROM position_entries WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
            signature,
        )
    }

    /// True when this position already has an exit record for `signature`.
    pub(crate) fn exit_record_exists(&self, signature: &str) -> Result<bool> {
        self.record_exists(
            "SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
            signature,
        )
    }

    /// The swap legs the trader booked for this position.
    pub(crate) fn trader_swap_legs(&self) -> Result<Vec<TraderSwapLeg>> {
        let wallet_address = self.wallet_address.clone()?;
        query_trader_swap_legs(self.conn, wallet_address, Some(self.position_id))
            .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e).into())
    }

    fn record_exists(&self, query: &str, signature: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row(query, params![self.position_id, signature], |_| Ok(()))
            .optional()
            .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e))?
            .is_some())
    }
}

impl PositionsDatabase {
    /// Books onto the stored row in one IMMEDIATE transaction: reads the row, hands it and
    /// the in-transaction reads to `book`, then writes the row it left and its record.
    /// Every failure, including one returned by `book`, rolls the transaction back and
    /// leaves the row and the records as they were.
    pub(crate) fn commit_booking<T>(
        &self,
        position_id: i64,
        wallet_address: Result<&str>,
        book: impl FnOnce(&mut Position, &BookingReads<'_>) -> Result<Booking<T>>,
    ) -> Result<Committed<T>> {
        let mut conn = self.get_connection()?;
        let committed = self.run_booking(&mut conn, position_id, wallet_address, book)?;

        if let Committed::Written { .. } | Committed::Deleted { .. } = committed {
            if let Ok(mut stmt) = conn.prepare("PRAGMA wal_checkpoint(PASSIVE);") {
                let _ = stmt.query([]);
            }
        }
        Ok(committed)
    }

    fn run_booking<T>(
        &self,
        conn: &mut Connection,
        position_id: i64,
        wallet_address: Result<&str>,
        book: impl FnOnce(&mut Position, &BookingReads<'_>) -> Result<Booking<T>>,
    ) -> Result<Committed<T>> {
        let chain = self.chain.as_str();
        let sqlite = |e| DatabaseError::classify_sqlite_failure("commit_booking", e);
        let tx = conn.write_tx().map_err(sqlite)?;

        let mut row = tx
            .query_row(
                &format!(
                    "SELECT {POSITION_SELECT_COLUMNS} FROM positions WHERE id = ?1 AND chain_id = ?2"
                ),
                params![position_id, chain],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(sqlite)?
            .ok_or(Error::NotFoundById { position_id })?;

        let reads = BookingReads {
            conn: &tx,
            position_id,
            wallet_address: &wallet_address,
        };
        let (record, outcome) = match book(&mut row, &reads)? {
            Booking::Skip(outcome) => return Ok(Committed::Skipped(outcome)),
            Booking::Delete { outcome } => {
                tx.execute(
                    "DELETE FROM positions WHERE id = ?1 AND chain_id = ?2",
                    params![position_id, chain],
                )
                .map_err(sqlite)?;
                tx.commit().map_err(sqlite)?;
                return Ok(Committed::Deleted { row, outcome });
            }
            Booking::Write { record, outcome } => (record, outcome),
        };

        write_position_row(&tx, chain, position_id, &row).map_err(sqlite)?;
        if let Some(record) = &record {
            let wallet_address = wallet_address.clone()?;
            match record {
                BookingRecord::Exit(record) => insert_exit_record(&tx, wallet_address, record),
                BookingRecord::Entry(record) => insert_entry_record(&tx, wallet_address, record),
            }
            .map_err(sqlite)?;
        }
        tx.commit().map_err(sqlite)?;

        if let Some(record) = &record {
            log_saved_record(record);
        }
        Ok(Committed::Written { row, outcome })
    }
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
