// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Paper store writes and reads: a paper round opened, and a paper fill booked onto it,
//! each with its leg evidence in one transaction, the paper exit attempts, plus the paper
//! account epochs.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::positions::types::{EntryRecord, Position, PositionOrigin};
use crate::positions::Result;

use super::booking::{
    insert_entry_record, log_saved_record, Booking, BookingReads, BookingRecord, Committed,
};
use super::operations::insert_position_row;
use super::tracking::insert_state_row;
use super::types::{
    PositionState, PositionsDatabase, POSITION_CREATED_REASON, POSITION_SELECT_COLUMNS,
};

/// What a paper leg booked onto its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaperLegKind {
    Entry,
    Dca,
    Partial,
    Exit,
}

impl PaperLegKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PaperLegKind::Entry => "entry",
            PaperLegKind::Dca => "dca",
            PaperLegKind::Partial => "partial",
            PaperLegKind::Exit => "exit",
        }
    }
}

/// The evidence row of one paper leg. `leg_ref` is also the `transaction_signature` of
/// the entry or exit record the leg booked.
pub(crate) struct PaperLegRow<'a> {
    pub kind: PaperLegKind,
    pub leg_ref: &'a str,
    pub strategy_id: Option<&'a str>,
    pub quote_json: &'a str,
    pub created_at: DateTime<Utc>,
}

/// One stored paper leg: what it booked, the reference its entry or exit record carries,
/// the strategy that decided it and the quotes it filled at.
#[derive(Debug, Clone, PartialEq)]
pub struct PaperLegRecord {
    pub kind: String,
    pub leg_ref: String,
    pub strategy_id: Option<String>,
    pub quote_json: String,
    pub created_at: String,
}

/// One paper run of a wallet on a chain.
#[derive(Debug, Clone, PartialEq)]
pub struct PaperEpoch {
    pub epoch: i64,
    pub starting_balance_native: f64,
    pub started_at: DateTime<Utc>,
}

/// What [`PositionsDatabase::insert_paper_round`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaperRoundInsert {
    /// The round was opened under this id.
    Inserted(i64),
    /// The leg was already booked; it opened the round with this id and nothing was
    /// written.
    AlreadyBooked(i64),
}

/// Whose paper exits a run of failed exits is counted for: the strategy that opened or
/// sold a round, or the copy task that opened it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperExitOwner {
    Strategy(String),
    CopyTask(i64),
}

impl PaperExitOwner {
    /// The owner of the rounds `origin` opens: its entry strategy or its copy task; none
    /// for a manual or external round, or an automatic one without a strategy.
    pub fn of_round(origin: &PositionOrigin) -> Option<Self> {
        match origin {
            PositionOrigin::Auto { strategy_id } => strategy_id.clone().map(Self::Strategy),
            PositionOrigin::Copy { task_id, .. } => Some(Self::CopyTask(*task_id)),
            PositionOrigin::Manual | PositionOrigin::External => None,
        }
    }
}

/// The condition on a `positions` row, its columns prefixed by `prefix`, that selects
/// the rounds copy task `?{param}` opened. The origin reference of a copy round is
/// `{task_id}:{source_wallet}` (`PositionOrigin::reference`).
fn copy_task_rounds_filter(prefix: &str, param: u8) -> String {
    format!("{prefix}origin_kind = 'copy' AND {prefix}origin_ref LIKE ?{param} || ':%'")
}

/// The round the paper leg `leg_ref` was booked onto, when it is stored.
fn leg_position(conn: &Connection, leg_ref: &str) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT position_id FROM paper_legs WHERE leg_ref = ?1",
        params![leg_ref],
        |row| row.get(0),
    )
    .optional()
}

fn insert_paper_leg(
    conn: &Connection,
    position_id: i64,
    leg: &PaperLegRow<'_>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "INSERT INTO paper_legs (position_id, leg_kind, leg_ref, strategy_id, quote_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            position_id,
            leg.kind.as_str(),
            leg.leg_ref,
            leg.strategy_id,
            leg.quote_json,
            leg.created_at.to_rfc3339(),
        ],
    )
}

impl PositionsDatabase {
    /// Opens a paper round in one transaction: the row of `position`, its entry record,
    /// the entry leg and the opening state. Refused with `AlreadyOpen` while the mint has
    /// an open round in this wallet. A leg reference already stored writes nothing and
    /// returns the round that leg opened, so a keyed buy opens its round once however
    /// often it is booked.
    pub(crate) fn insert_paper_round(
        &self,
        wallet_address: &str,
        position: &Position,
        mut entry: EntryRecord,
        leg: &PaperLegRow<'_>,
    ) -> Result<PaperRoundInsert> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("insert_paper_round", e);
        let mut conn = self.get_connection()?;
        let tx = conn.write_tx().map_err(sqlite)?;
        if let Some(position_id) = leg_position(&tx, leg.leg_ref).map_err(sqlite)? {
            return Ok(PaperRoundInsert::AlreadyBooked(position_id));
        }
        let position_id = insert_position_row(&tx, self.chain, wallet_address, position)?;
        entry.position_id = position_id;
        insert_entry_record(&tx, wallet_address, &entry).map_err(sqlite)?;
        insert_paper_leg(&tx, position_id, leg).map_err(sqlite)?;
        insert_state_row(
            &tx,
            position_id,
            &PositionState::Open,
            &leg.created_at.to_rfc3339(),
            Some(POSITION_CREATED_REASON),
        )
        .map_err(sqlite)?;
        tx.commit().map_err(sqlite)?;
        log_saved_record(&BookingRecord::Entry(entry));
        Ok(PaperRoundInsert::Inserted(position_id))
    }

    /// Books a paper fill onto `position_id` as [`Self::commit_booking`] does, and writes
    /// `leg` and, for a booking that closed the round, its closing state in the same
    /// transaction.
    pub(crate) fn commit_paper_booking<T>(
        &self,
        position_id: i64,
        wallet_address: &str,
        leg: &PaperLegRow<'_>,
        book: impl FnOnce(&mut Position, &BookingReads<'_>) -> Result<Booking<T>>,
    ) -> Result<Committed<T>> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("commit_paper_booking", e);
        let mut conn = self.get_connection()?;
        self.run_booking(
            &mut conn,
            position_id,
            Ok(wallet_address),
            book,
            |conn, row| {
                insert_paper_leg(conn, position_id, leg).map_err(sqlite)?;
                if row.exit_time.is_some() {
                    insert_state_row(
                        conn,
                        position_id,
                        &PositionState::Closed,
                        &leg.created_at.to_rfc3339(),
                        row.closed_reason.as_deref(),
                    )
                    .map_err(sqlite)?;
                }
                Ok(())
            },
        )
    }

    /// The positions of `wallet_address` entered at or after `since`, open or closed.
    pub(crate) fn positions_entered_since(
        &self,
        wallet_address: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<Position>> {
        self.select_positions(
            "positions_entered_since",
            "",
            since,
            params![wallet_address, self.chain.as_str(), since.to_rfc3339()],
        )
    }

    /// The positions of `wallet_address` entered at or after `since` that strategy
    /// `strategy_id` took part in: opened by it as an entry strategy, or closed by a
    /// paper exit leg it decided.
    pub(crate) fn strategy_positions_entered_since(
        &self,
        wallet_address: &str,
        strategy_id: &str,
        since: DateTime<Utc>,
    ) -> Result<Vec<Position>> {
        self.select_positions(
            "strategy_positions_entered_since",
            " AND ((origin_kind = 'auto' AND origin_ref = ?4)
                   OR EXISTS (SELECT 1 FROM paper_legs
                              WHERE paper_legs.position_id = positions.id
                                AND paper_legs.leg_kind = 'exit'
                                AND paper_legs.strategy_id = ?4))",
            since,
            params![
                wallet_address,
                self.chain.as_str(),
                since.to_rfc3339(),
                strategy_id
            ],
        )
    }

    /// The positions of `wallet_address` entered at or after `since` that copy task
    /// `task_id` opened, open or closed.
    pub(crate) fn copy_task_positions_entered_since(
        &self,
        wallet_address: &str,
        task_id: i64,
        since: DateTime<Utc>,
    ) -> Result<Vec<Position>> {
        self.select_positions(
            "copy_task_positions_entered_since",
            &format!(" AND {}", copy_task_rounds_filter("", 4)),
            since,
            params![
                wallet_address,
                self.chain.as_str(),
                since.to_rfc3339(),
                task_id.to_string()
            ],
        )
    }

    /// The positions of wallet `?1` on chain `?2` entered at or after `since` (`?3`),
    /// narrowed by `filter`, in entry order. SQLite's `datetime()` compares whole seconds,
    /// so the query narrows to the second and the decoded entry time decides: a round
    /// entered in the second a run starts, before it started, is not counted in that run.
    fn select_positions(
        &self,
        operation: &'static str,
        filter: &str,
        since: DateTime<Utc>,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<Position>> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure(operation, e);
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {POSITION_SELECT_COLUMNS} FROM positions
                 WHERE wallet_address = ?1 AND chain_id = ?2
                   AND datetime(entry_time) >= datetime(?3){filter}
                 ORDER BY entry_time"
            ))
            .map_err(sqlite)?;
        let rows = stmt
            .query_map(params, |row| self.row_to_position(row))
            .map_err(sqlite)?;
        let mut positions = Vec::new();
        for row in rows {
            let position = row.map_err(sqlite)?;
            if position.entry_time >= since {
                positions.push(position);
            }
        }
        Ok(positions)
    }

    /// The legs booked onto paper round `position_id`, in booking order.
    pub(crate) fn paper_legs(&self, position_id: i64) -> Result<Vec<PaperLegRecord>> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("paper_legs", e);
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT leg_kind, leg_ref, strategy_id, quote_json, created_at FROM paper_legs
                 WHERE position_id = ?1 ORDER BY id",
            )
            .map_err(sqlite)?;
        let rows = stmt
            .query_map(params![position_id], |row| {
                Ok(PaperLegRecord {
                    kind: row.get(0)?,
                    leg_ref: row.get(1)?,
                    strategy_id: row.get(2)?,
                    quote_json: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(sqlite)?;
        let mut legs = Vec::new();
        for row in rows {
            legs.push(row.map_err(sqlite)?);
        }
        Ok(legs)
    }

    /// The stored paper leg `leg_ref` and the round it was booked onto.
    pub(crate) fn paper_leg(&self, leg_ref: &str) -> Result<Option<(i64, PaperLegRecord)>> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("paper_leg", e);
        let conn = self.get_connection()?;
        Ok(conn
            .query_row(
                "SELECT position_id, leg_kind, leg_ref, strategy_id, quote_json, created_at
             FROM paper_legs WHERE leg_ref = ?1",
                params![leg_ref],
                |row| {
                    Ok((
                        row.get(0)?,
                        PaperLegRecord {
                            kind: row.get(1)?,
                            leg_ref: row.get(2)?,
                            strategy_id: row.get(3)?,
                            quote_json: row.get(4)?,
                            created_at: row.get(5)?,
                        },
                    ))
                },
            )
            .optional()
            .map_err(sqlite)?)
    }

    /// Records a paper exit attempt of `position_id` decided by `strategy_id`, and
    /// whether it filled.
    pub(crate) fn insert_paper_exit_attempt(
        &self,
        position_id: i64,
        strategy_id: Option<&str>,
        filled: bool,
        at: DateTime<Utc>,
    ) -> Result<()> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("insert_paper_exit_attempt", e);
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT INTO paper_exit_attempts (position_id, strategy_id, filled, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![position_id, strategy_id, filled, at.to_rfc3339()],
        )
        .map_err(sqlite)?;
        Ok(())
    }

    /// The latest paper exit attempts of `wallet_address`'s rounds that `owner` took part
    /// in that found no fill since the last one that filled. A strategy takes part as the
    /// entry strategy of the round or the exit strategy that decided the sale; a copy task
    /// as the task that opened the round.
    pub(crate) fn failed_paper_exit_streak(
        &self,
        wallet_address: &str,
        owner: &PaperExitOwner,
    ) -> Result<u32> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("failed_paper_exit_streak", e);
        let (filter, key) = match owner {
            PaperExitOwner::Strategy(strategy_id) => (
                "(p.origin_kind = 'auto' AND p.origin_ref = ?3) OR a.strategy_id = ?3".to_owned(),
                strategy_id.clone(),
            ),
            PaperExitOwner::CopyTask(task_id) => {
                (copy_task_rounds_filter("p.", 3), task_id.to_string())
            }
        };
        let conn = self.get_connection()?;
        let streak: i64 = conn
            .query_row(
                &format!(
                    "WITH attempts AS (
                         SELECT a.id, a.filled FROM paper_exit_attempts a
                         JOIN positions p ON p.id = a.position_id
                         WHERE p.wallet_address = ?1 AND p.chain_id = ?2 AND ({filter})
                     )
                     SELECT COUNT(*) FROM attempts
                     WHERE filled = 0
                       AND id > COALESCE((SELECT MAX(id) FROM attempts WHERE filled = 1), 0)"
                ),
                params![wallet_address, self.chain.as_str(), key],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        Ok(u32::try_from(streak).unwrap_or(u32::MAX))
    }

    /// The current paper epoch of `wallet_address`: the latest one started.
    pub(crate) fn current_paper_epoch(&self, wallet_address: &str) -> Result<Option<PaperEpoch>> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("current_paper_epoch", e);
        let conn = self.get_connection()?;
        let row = conn
            .query_row(
                "SELECT epoch, starting_balance_native, started_at FROM paper_account
                 WHERE wallet_address = ?1 AND chain_id = ?2
                 ORDER BY epoch DESC LIMIT 1",
                params![wallet_address, self.chain.as_str()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, f64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(sqlite)?;
        row.map(|(epoch, starting_balance_native, started_at)| {
            let started_at = DateTime::parse_from_rfc3339(&started_at)
                .map(|at| at.with_timezone(&Utc))
                .map_err(|e| crate::positions::Error::RowDecode {
                    column: "started_at",
                    detail: e.to_string(),
                })?;
            Ok(PaperEpoch {
                epoch,
                starting_balance_native,
                started_at,
            })
        })
        .transpose()
    }

    /// Starts the next paper epoch of `wallet_address` with `starting_balance_native`,
    /// numbered one above the latest, and returns it.
    pub(crate) fn start_paper_epoch(
        &self,
        wallet_address: &str,
        starting_balance_native: f64,
        started_at: DateTime<Utc>,
    ) -> Result<PaperEpoch> {
        let sqlite = |e| DatabaseError::classify_sqlite_failure("start_paper_epoch", e);
        let mut conn = self.get_connection()?;
        let tx = conn.write_tx().map_err(sqlite)?;
        let epoch: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(epoch), 0) + 1 FROM paper_account
                 WHERE wallet_address = ?1 AND chain_id = ?2",
                params![wallet_address, self.chain.as_str()],
                |row| row.get(0),
            )
            .map_err(sqlite)?;
        tx.execute(
            "INSERT INTO paper_account (wallet_address, chain_id, epoch, starting_balance_native, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                wallet_address,
                self.chain.as_str(),
                epoch,
                starting_balance_native,
                started_at.to_rfc3339(),
            ],
        )
        .map_err(sqlite)?;
        tx.commit().map_err(sqlite)?;
        Ok(PaperEpoch {
            epoch,
            starting_balance_native,
            started_at,
        })
    }
}
