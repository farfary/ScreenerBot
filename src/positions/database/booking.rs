// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Atomic booking: a position row read, decided on and written back with its history record in one transaction.

use rusqlite::{params, Connection, OptionalExtension};

use crate::chains::RawAmount;
use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::positions::types::{EntryRecord, ExitRecord, Position};
use crate::positions::{Error, Result};

use super::open_round::{query_active_open_round_ids, query_open_round_id};
use super::operations::write_position_row;
use super::queries::{
    query_mint_rows, query_trader_swap_legs, MintRow, SwapLegScope, TraderSwapLeg,
};
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
/// handed, and the one write it may make beyond that row: booking onto the open position of
/// its mint ([`BookingReads::book_open_round`]), so a closed row hands its holding over in
/// the same transaction.
pub(crate) struct BookingReads<'a> {
    db: &'a PositionsDatabase,
    conn: &'a Connection,
    chain: &'a str,
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

    /// True when any position of `mint` in this store's chain already has an entry record
    /// for `signature`: a buy whose tokens were handed to the open position of its mint is
    /// recorded there, not on the position it was bought for.
    pub(crate) fn entry_recorded_for_mint(&self, mint: &str, signature: &str) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM position_entries
                 WHERE transaction_signature = ?1
                   AND position_id IN (SELECT id FROM positions WHERE chain_id = ?2 AND mint = ?3)
                 LIMIT 1",
                params![signature, self.chain, mint],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e))?
            .is_some())
    }

    /// The active (not archived) open positions of `mint` in this wallet other than
    /// `position_id`; empty unless the store held several open positions of the mint before
    /// the open-round index.
    pub(crate) fn active_open_rounds_beside(
        &self,
        mint: &str,
        position_id: i64,
    ) -> Result<Vec<i64>> {
        let wallet_address = self.wallet_address.clone()?;
        query_active_open_round_ids(self.conn, self.chain, wallet_address, mint, position_id)
    }

    /// True when this position already has an exit record for `signature`.
    pub(crate) fn exit_record_exists(&self, signature: &str) -> Result<bool> {
        self.record_exists(
            "SELECT 1 FROM position_exits WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
            signature,
        )
    }

    /// True when this position has any entry or exit record, of whatever signature.
    pub(crate) fn has_any_record(&self) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM position_entries WHERE position_id = ?1
                 UNION ALL
                 SELECT 1 FROM position_exits WHERE position_id = ?1
                 LIMIT 1",
                params![self.position_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e))?
            .is_some())
    }

    /// What the other open positions of `mint` in this wallet hold, failing retryably while
    /// it is unattributable; see [`query_other_open_held`].
    pub(crate) fn other_open_held(&self, mint: &str) -> Result<RawAmount> {
        let wallet_address = self.wallet_address.clone()?;
        query_other_open_held(
            self.conn,
            self.chain,
            wallet_address,
            mint,
            Some(self.position_id),
        )?
        .booked(mint)
    }

    /// The open position of `mint` in this wallet other than this one, archived or not.
    pub(crate) fn open_round_beside(&self, mint: &str) -> Result<Option<i64>> {
        let wallet_address = self.wallet_address.clone()?;
        query_open_round_id(
            self.conn,
            self.chain,
            wallet_address,
            mint,
            Some(self.position_id),
        )
    }

    /// Books onto the open position `open_round_id` inside this transaction: reads its row,
    /// hands it to `book`, then writes the row and the entry record `book` returns under
    /// `signature`. An archived row that `book` unarchives is unarchived with the booking.
    /// Returns the row as written, which the caller publishes once the transaction commits,
    /// or `None` without writing anything when the open position already has an entry
    /// record for `signature`: the booking is already on it.
    pub(crate) fn book_open_round(
        &self,
        open_round_id: i64,
        signature: &str,
        book: impl FnOnce(&mut Position) -> Result<EntryRecord>,
    ) -> Result<Option<Position>> {
        let wallet_address = self.wallet_address.clone()?;
        let sqlite = |e| DatabaseError::classify_sqlite_failure("commit_booking", e);
        let already_booked = self
            .conn
            .query_row(
                "SELECT 1 FROM position_entries WHERE position_id = ?1 AND transaction_signature = ?2 LIMIT 1",
                params![open_round_id, signature],
                |_| Ok(()),
            )
            .optional()
            .map_err(sqlite)?
            .is_some();
        if already_booked {
            return Ok(None);
        }
        let mut row = read_row(self.db, self.conn, self.chain, open_round_id)?;
        let was_archived = row.archived;
        let record = book(&mut row)?;
        write_position_row(self.conn, self.chain, open_round_id, &row).map_err(sqlite)?;
        if was_archived && !row.archived {
            write_unarchived(self.conn, self.chain, open_round_id).map_err(sqlite)?;
        }
        insert_entry_record(self.conn, wallet_address, &record).map_err(sqlite)?;
        log_saved_record(&BookingRecord::Entry(record));
        Ok(Some(row))
    }

    /// The swap legs the trader booked for every position of `mint` in this wallet.
    pub(crate) fn mint_swap_legs(&self, mint: &str) -> Result<Vec<TraderSwapLeg>> {
        let wallet_address = self.wallet_address.clone()?;
        let scope = SwapLegScope::Mint {
            chain: self.chain,
            mint,
        };
        query_trader_swap_legs(self.conn, wallet_address, scope)
            .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e).into())
    }

    /// Every position of `mint` in this wallet, with the signatures it is booked under.
    pub(crate) fn mint_rows(&self, mint: &str) -> Result<Vec<MintRow>> {
        let wallet_address = self.wallet_address.clone()?;
        query_mint_rows(self.conn, self.chain, wallet_address, mint)
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

/// What the other open positions of a mint in the wallet hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtherOpenHeld {
    /// Every one of them has a verified entry: together they hold this.
    Booked(RawAmount),
    /// The entry of `position_id` is not verified, so its holding is not booked and the
    /// wallet's holding cannot be split between the positions. A reader that would act on
    /// the split treats any part of the holding as possibly theirs.
    Unattributable { position_id: i64 },
}

impl OtherOpenHeld {
    /// The booked holding, failing retryably while it is unattributable: a late fill waits
    /// for the other entry instead of booking a guess.
    pub fn booked(self, mint: &str) -> Result<RawAmount> {
        match self {
            OtherOpenHeld::Booked(held) => Ok(held),
            OtherOpenHeld::Unattributable { position_id } => Err(Error::HoldingUnattributable {
                mint: mint.to_owned(),
                detail: format!("the entry of position {position_id} is not verified"),
            }),
        }
    }

    /// The booked holding, or `None` while it is unattributable.
    pub fn known(self) -> Option<RawAmount> {
        match self {
            OtherOpenHeld::Booked(held) => Some(held),
            OtherOpenHeld::Unattributable { .. } => None,
        }
    }
}

/// What the open positions of `mint` in this wallet other than `excluded` hold, archived
/// and wallet-derived ones included: the remaining amount once recorded, otherwise the entry
/// fill, and nothing for a row that recorded neither. Saturates at the largest raw amount.
/// [`OtherOpenHeld::Unattributable`] while one of them has an entry that is not verified.
/// The one owner of this reading, inside a booking transaction and outside one.
pub(super) fn query_other_open_held(
    conn: &Connection,
    chain: &str,
    wallet_address: &str,
    mint: &str,
    excluded: Option<i64>,
) -> Result<OtherOpenHeld> {
    let sqlite = |e| DatabaseError::classify_sqlite_failure("other_open_held", e);
    let mut stmt = conn
        .prepare(
            "SELECT id, transaction_entry_verified, COALESCE(remaining_token_amount, token_amount)
             FROM positions
             WHERE chain_id = ?1 AND wallet_address = ?2 AND mint = ?3
               AND (?4 IS NULL OR id != ?4)
               AND exit_time IS NULL",
        )
        .map_err(sqlite)?;
    let rows = stmt
        .query_map(params![chain, wallet_address, mint, excluded], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, Option<RawAmount>>(2)?,
            ))
        })
        .map_err(sqlite)?;
    let mut held = RawAmount::ZERO;
    for row in rows {
        let (id, entry_verified, amount) = row.map_err(sqlite)?;
        if !entry_verified {
            return Ok(OtherOpenHeld::Unattributable { position_id: id });
        }
        held = held
            .checked_add(amount.unwrap_or(RawAmount::ZERO))
            .unwrap_or(RawAmount::new(u128::MAX));
    }
    Ok(OtherOpenHeld::Booked(held))
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

        let mut row = read_row(self, &tx, chain, position_id)?;
        let was_archived = row.archived;

        let reads = BookingReads {
            db: self,
            conn: &tx,
            chain,
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
        if was_archived && !row.archived {
            write_unarchived(&tx, chain, position_id).map_err(sqlite)?;
        }
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

/// The stored row of `position_id` on `chain`, read inside the booking transaction.
fn read_row(
    db: &PositionsDatabase,
    conn: &Connection,
    chain: &str,
    position_id: i64,
) -> Result<Position> {
    conn.query_row(
        &format!("SELECT {POSITION_SELECT_COLUMNS} FROM positions WHERE id = ?1 AND chain_id = ?2"),
        params![position_id, chain],
        |row| db.row_to_position(row),
    )
    .optional()
    .map_err(|e| DatabaseError::classify_sqlite_failure("commit_booking", e))?
    .ok_or(Error::NotFoundById { position_id })
}

/// Clears the archive flag of a row a booking brought back to the open positions. The flag
/// is not among the columns [`write_position_row`] writes, so the booking writes it here, in
/// its own transaction; memory mirrors it once the booking commits.
fn write_unarchived(conn: &Connection, chain: &str, position_id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE positions SET archived = 0, archived_at = NULL, updated_at = datetime('now')
         WHERE id = ?1 AND chain_id = ?2",
        params![position_id, chain],
    )
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
