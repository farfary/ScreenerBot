// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position database queries — read-only methods for fetching positions.

use std::collections::BTreeMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, params_from_iter, Connection, OpenFlags, OptionalExtension};

use crate::chains::RawAmount;
use crate::database::schema::ddl;
use crate::errors::DatabaseError;
use crate::positions::types::{EntryRecord, ExitRecord, Position};
use crate::positions::{Error, Result};

use super::types::*;

/// The bot's closed, exit-verified positions of wallet `?1` on chain `?2` that exited from
/// `?3`, before `?4` when it is not NULL, with the P&L the statistics count.
///
/// A position whose cost basis or history is incomplete booked its proceeds over a cost
/// that misses what it does not know, so its P&L is an upper bound of the real one: a loss
/// it shows is a loss at least that large and counts, a gain it shows may not exist and
/// counts as none, so the profit, net P&L and wins of a period never include it.
const COUNTED_CLOSED_POSITIONS: &str = r#"
      SELECT exit_time, pnl_percent, dca_count, partial_exit_count,
        CASE WHEN basis_complete = 1 AND history_complete = 1 THEN pnl ELSE MIN(pnl, 0) END AS pnl
      FROM positions
      WHERE wallet_address = ?1 AND chain_id = ?2
        AND transaction_exit_verified = 1
        AND origin_kind != 'external'
        AND exit_time IS NOT NULL
        AND datetime(exit_time) >= datetime(?3)
        AND (?4 IS NULL OR datetime(exit_time) < datetime(?4))
"#;

/// The one definition of an open position in the store: not archived, no exit recorded and
/// no verified exit transaction. Mirrors the in-memory filter in `state::get_open_positions()`.
const OPEN_POSITION_FILTER: &str =
    "archived = 0 AND exit_time IS NULL AND transaction_exit_verified = 0";

/// Count the open positions of `wallet` on every chain in the positions store at `path`,
/// opened read-only so the count never creates, migrates or pools the file. A missing file
/// or a store without the `positions` table holds no position and counts 0.
pub(crate) fn count_open_positions_in_file(
    path: &Path,
    wallet: &str,
) -> std::result::Result<usize, DatabaseError> {
    let Some(conn) = open_positions_file_read_only(path)? else {
        return Ok(0);
    };
    let query = format!(
        "SELECT COUNT(*) FROM positions WHERE wallet_address = ?1 AND {OPEN_POSITION_FILTER}"
    );
    conn.query_row(&query, params![wallet], |row| row.get::<_, usize>(0))
        .map_err(DatabaseError::from)
}

/// The open positions of every wallet holding one, on every chain, in the positions store
/// at `path`, opened read-only as [`count_open_positions_in_file`] opens it.
pub(crate) fn open_positions_by_wallet_in_file(
    path: &Path,
) -> std::result::Result<BTreeMap<String, usize>, DatabaseError> {
    let Some(conn) = open_positions_file_read_only(path)? else {
        return Ok(BTreeMap::new());
    };
    let query = format!(
        "SELECT wallet_address, COUNT(*) FROM positions WHERE {OPEN_POSITION_FILTER} GROUP BY wallet_address"
    );
    let mut statement = conn.prepare(&query).map_err(DatabaseError::from)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })
        .map_err(DatabaseError::from)?;
    rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .map_err(DatabaseError::from)
}

/// The positions store at `path` opened read-only, `None` when the file or its
/// `positions` table does not exist.
fn open_positions_file_read_only(
    path: &Path,
) -> std::result::Result<Option<Connection>, DatabaseError> {
    if !path.exists() {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(DatabaseError::from)?;
    if !ddl::has_table(&conn, "positions")? {
        return Ok(None);
    }
    Ok(Some(conn))
}

impl PositionsDatabase {
    /// Get position by ID
    pub async fn get_position_by_id(&self, id: i64) -> Result<Option<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
            "SELECT {} FROM positions WHERE id = ?1 AND wallet_address = ?2 AND chain_id = ?3",
            POSITION_SELECT_COLUMNS
        );
        let result = conn
            .query_row(
                &query,
                params![id, wallet_address, self.chain.as_str()],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| DatabaseError::Query {
                operation: "get position by ID".to_owned(),
                message: e.to_string(),
            })?;

        Ok(result)
    }

    /// The most recent position for a mint, OPEN OR CLOSED (ORDER BY entry_time DESC).
    ///
    /// This is the history lookup — callers that want the tradeable position must use the
    /// in-memory `state::get_position_by_mint`, which returns only an open one.
    pub async fn get_latest_position_by_mint(&self, mint: &str) -> Result<Option<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      "SELECT {} FROM positions WHERE mint = ?1 AND wallet_address = ?2 AND chain_id = ?3 ORDER BY entry_time DESC LIMIT 1",
      POSITION_SELECT_COLUMNS
    );
        let result = conn
            .query_row(
                &query,
                params![mint, wallet_address, self.chain.as_str()],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| DatabaseError::Query {
                operation: "get position by mint".to_owned(),
                message: e.to_string(),
            })?;

        Ok(result)
    }

    /// EVERY position ever opened on a mint — open, closed and archived — oldest first.
    ///
    /// A token can be traded, exited and re-entered any number of times, and each round is
    /// its own position row with its own entry/exit records. This is what the token's
    /// all-time activity view is built from; the single-position lookups
    /// (`get_position_by_mint`, `get_latest_position_by_mint`) each answer a narrower
    /// question and would silently drop every earlier round.
    pub async fn get_all_positions_for_mint(&self, mint: &str) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
            "SELECT {POSITION_SELECT_COLUMNS} FROM positions \
             WHERE mint = ?1 AND wallet_address = ?2 AND chain_id = ?3 ORDER BY entry_time ASC"
        );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare positions-for-mint query".to_owned(),
            message: e.to_string(),
        })?;

        let rows = stmt
            .query_map(params![mint, wallet_address, self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute positions-for-mint query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for row in rows.flatten() {
            positions.push(row);
        }
        Ok(positions)
    }

    /// Get position by entry transaction signature
    pub async fn get_position_by_entry_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
            "SELECT {POSITION_SELECT_COLUMNS} FROM positions \
             WHERE entry_transaction_signature = ?1 AND wallet_address = ?2 AND chain_id = ?3"
        );

        let result = conn
            .query_row(
                &query,
                params![signature, wallet_address, self.chain.as_str()],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| DatabaseError::Query {
                operation: "get position by entry signature".to_owned(),
                message: e.to_string(),
            })?;

        Ok(result)
    }

    /// Get position by exit transaction signature
    pub async fn get_position_by_exit_signature(
        &self,
        signature: &str,
    ) -> Result<Option<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      "SELECT {} FROM positions WHERE exit_transaction_signature = ?1 AND wallet_address = ?2 AND chain_id = ?3",
      POSITION_SELECT_COLUMNS
    );

        let result = conn
            .query_row(
                &query,
                params![signature, wallet_address, self.chain.as_str()],
                |row| self.row_to_position(row),
            )
            .optional()
            .map_err(|e| DatabaseError::Query {
                operation: "get position by exit signature".to_owned(),
                message: e.to_string(),
            })?;

        Ok(result)
    }

    /// Get all positions with optional filtering
    pub async fn get_positions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;

        let mut query = format!(
            "SELECT {} FROM positions WHERE chain_id = ?1 ORDER BY entry_time DESC",
            POSITION_SELECT_COLUMNS
        );

        if let Some(limit) = limit {
            query.push_str(&format!("LIMIT {limit}"));
            if let Some(offset) = offset {
                query.push_str(&format!("OFFSET {offset}"));
            }
        }

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare positions query".to_owned(),
            message: e.to_string(),
        })?;

        let position_iter = stmt
            .query_map(params![self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute positions query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }

    /// Get the open positions of the current wallet (`OPEN_POSITION_FILTER`).
    pub async fn get_open_positions(&self) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      "SELECT {POSITION_SELECT_COLUMNS} FROM positions WHERE wallet_address = ?1 AND chain_id = ?2 AND {OPEN_POSITION_FILTER} ORDER BY entry_time DESC"
    );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare open positions query".to_owned(),
            message: e.to_string(),
        })?;

        let position_iter = stmt
            .query_map(params![wallet_address, self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute open positions query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }

    /// Get closed positions (exit verified, not archived).
    /// Mirrors the in-memory filter in state::get_closed_positions().
    pub async fn get_closed_positions(&self) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND chain_id = ?2 AND archived = 0 AND transaction_exit_verified = 1 ORDER BY exit_time DESC",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare closed positions query".to_owned(),
            message: e.to_string(),
        })?;

        let position_iter = stmt
            .query_map(params![wallet_address, self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute closed positions query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }

    /// Get closed positions since a specific date (have exit_time >= since)
    pub async fn get_closed_positions_since(&self, since: DateTime<Utc>) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      // `exit_time` is compared as stored text, never wrapped in datetime(): every
      // write goes through DateTime<Utc>::to_rfc3339(), so the column is a single
      // fixed-offset format that orders lexicographically, and a bare comparison is
      // the only form that can use idx_positions_exit_time. Wrapping the column in a
      // function forced a full scan of `positions` on every stats poll.
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND chain_id = ?2 AND archived = 0 AND transaction_exit_verified = 1 AND origin_kind != 'external' AND exit_time >= ?3 ORDER BY exit_time DESC",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare closed positions since query".to_owned(),
            message: e.to_string(),
        })?;

        let since_str = since.to_rfc3339();
        let position_iter = stmt
            .query_map(
                params![wallet_address, self.chain.as_str(), since_str],
                |row| self.row_to_position(row),
            )
            .map_err(|e| DatabaseError::Query {
                operation: "execute closed positions since query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }

    /// Count closed positions since the provided timestamp
    pub async fn count_closed_positions_since(&self, since: DateTime<Utc>) -> Result<i64> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT COUNT(1)
      FROM positions
      WHERE wallet_address = ?1 AND chain_id = ?2
       AND archived = 0
       AND transaction_exit_verified = 1
       AND origin_kind != 'external'
       AND exit_time IS NOT NULL
       AND datetime(exit_time) >= datetime(?3)
      "#,
            )
            .map_err(|e| DatabaseError::Query {
                operation: "prepare closed position count query".to_owned(),
                message: e.to_string(),
            })?;

        let since_str = since.to_rfc3339();
        let count: i64 = stmt
            .query_row(
                params![wallet_address, self.chain.as_str(), since_str],
                |row| row.get(0),
            )
            .map_err(|e| DatabaseError::Query {
                operation: "execute closed position count query".to_owned(),
                message: e.to_string(),
            })?;

        Ok(count)
    }

    /// Aggregated trading statistics of the bot's positions closed from `period_start`, up
    /// to `period_end` when one is given, in one SQL aggregation.
    ///
    /// Wallet-derived rounds (`origin_kind = 'external'`) are excluded here and in every
    /// other performance query: they are the user's own pre-existing history, not trades
    /// the bot placed. Counting them would misreport win rate and — because
    /// `initialize_from_history` seeds the loss limiter from this — could trip the loss
    /// limit at startup on losses the bot never took. P&L is counted as
    /// [`COUNTED_CLOSED_POSITIONS`] defines it.
    pub async fn get_period_trading_stats(
        &self,
        period_start: DateTime<Utc>,
        period_end: Option<DateTime<Utc>>,
    ) -> Result<PeriodTradingStats> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
            r#"
      WITH closed AS ({COUNTED_CLOSED_POSITIONS})
      SELECT
        COUNT(*) as trade_count,
        SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
        COALESCE(SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END), 0) as profit,
        COALESCE(SUM(CASE WHEN pnl < 0 THEN ABS(pnl) ELSE 0 END), 0) as loss,
        COALESCE(SUM(pnl), 0) as total_pnl,
        COALESCE(SUM(1 + dca_count), 0) as total_buys,
        COALESCE(SUM(CASE
          WHEN partial_exit_count > 0
          THEN partial_exit_count + 1
          ELSE 1
        END), 0) as total_sells,
        COALESCE(MAX(CASE WHEN pnl_percent < 0 THEN ABS(pnl_percent) ELSE 0 END), 0) as max_dd
      FROM closed
      "#
        );

        conn.query_row(
            &query,
            params![
                wallet_address,
                self.chain.as_str(),
                period_start.to_rfc3339(),
                period_end.map(|end| end.to_rfc3339())
            ],
            |row| {
                let trade_count: i64 = row.get(0)?;
                let wins: i64 = row.get::<_, Option<i64>>(1)?.unwrap_or_default();
                let win_rate = if trade_count > 0 {
                    (wins as f64 / trade_count as f64) * 100.0
                } else {
                    0.0
                };

                Ok(PeriodTradingStats {
                    buys: row.get(5)?,
                    sells: row.get(6)?,
                    profit_native: row.get(2)?,
                    loss_native: row.get(3)?,
                    net_pnl_native: row.get(4)?,
                    drawdown_percent: row.get(7)?,
                    win_rate,
                    closed_positions: trade_count,
                    wins,
                })
            },
        )
        .map_err(|e| {
            DatabaseError::Query {
                operation: "execute period stats query".to_owned(),
                message: e.to_string(),
            }
            .into()
        })
    }

    /// Get realized trading statistics grouped by calendar day (UTC) for a period.
    /// Used by the home portfolio calendar. Only exit-verified, closed positions count,
    /// keyed by the day their exit was recorded (`exit_time`), with P&L counted as
    /// [`COUNTED_CLOSED_POSITIONS`] defines it.
    pub async fn get_daily_trading_stats(
        &self,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<DailyTradingStats>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let mut stmt = conn
            .prepare(&format!(
                r#"
        WITH closed AS ({COUNTED_CLOSED_POSITIONS})
        SELECT
          strftime('%Y-%m-%d', exit_time) as day,
          COUNT(*) as trades,
          SUM(CASE WHEN pnl > 0 THEN 1 ELSE 0 END) as wins,
          COALESCE(SUM(CASE WHEN pnl > 0 THEN pnl ELSE 0 END), 0) as profit,
          COALESCE(SUM(CASE WHEN pnl < 0 THEN ABS(pnl) ELSE 0 END), 0) as loss,
          COALESCE(SUM(pnl), 0) as total_pnl
        FROM closed
        GROUP BY day
        ORDER BY day ASC
        "#
            ))
            .map_err(|e| DatabaseError::Query {
                operation: "prepare daily stats query".to_owned(),
                message: e.to_string(),
            })?;

        let rows = stmt
            .query_map(
                params![
                    wallet_address,
                    self.chain.as_str(),
                    period_start.to_rfc3339(),
                    Some(period_end.to_rfc3339())
                ],
                |row| {
                    Ok(DailyTradingStats {
                        date: row.get(0)?,
                        trades: row.get(1)?,
                        wins: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                        profit_native: row.get(3)?,
                        loss_native: row.get(4)?,
                        net_pnl_native: row.get(5)?,
                    })
                },
            )
            .map_err(|e| DatabaseError::Query {
                operation: "execute daily stats query".to_owned(),
                message: e.to_string(),
            })?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| DatabaseError::Query {
                operation: "read daily stats row".to_owned(),
                message: e.to_string(),
            })?);
        }
        Ok(result)
    }

    /// Get recent closed & verified positions for a specific mint (exit verified)
    /// Ordered by most recent exit_time DESC. Used for adaptive re-entry profit capping.
    pub async fn get_recent_closed_positions_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let query = format!(
      "SELECT {} FROM positions WHERE wallet_address = ?1 AND chain_id = ?2 AND mint = ?3 AND transaction_exit_verified = 1 AND exit_price IS NOT NULL AND exit_time IS NOT NULL ORDER BY exit_time DESC LIMIT ?4",
      POSITION_SELECT_COLUMNS
    );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare recent closed positions query".to_owned(),
            message: e.to_string(),
        })?;

        let rows = stmt
            .query_map(
                params![wallet_address, self.chain.as_str(), mint, limit as i64],
                |row| self.row_to_position(row),
            )
            .map_err(|e| DatabaseError::Query {
                operation: "execute recent closed positions query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for row in rows {
            if let Ok(p) = row {
                positions.push(p);
            }
        }
        Ok(positions)
    }

    /// Lightweight variant: only fetch (exit_price, effective_exit_price) for recent verified exits
    /// to reduce row size & parsing overhead for re-entry heuristics.
    pub async fn get_recent_closed_exit_prices_for_mint(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<(Option<f64>, Option<f64>)>> {
        let conn = self.get_connection()?;
        let wallet_address =
            crate::utils::get_wallet_address().map_err(|e| Error::WalletUnavailable {
                detail: e.to_string(),
            })?;

        let mut stmt = conn
            .prepare(
                r#"
      SELECT exit_price, effective_exit_price
      FROM positions
      WHERE wallet_address = ?1 AND chain_id = ?2 AND mint = ?3 AND transaction_exit_verified = 1
       AND exit_price IS NOT NULL AND exit_time IS NOT NULL
      ORDER BY datetime(exit_time) DESC
      LIMIT ?4
      "#,
            )
            .map_err(|e| DatabaseError::Query {
                operation: "prepare recent closed exit prices query".to_owned(),
                message: e.to_string(),
            })?;

        let mut out: Vec<(Option<f64>, Option<f64>)> = Vec::new();
        let rows = stmt
            .query_map(
                params![wallet_address, self.chain.as_str(), mint, limit as i64],
                |row| {
                    let exit_p: Option<f64> = row.get(0).ok();
                    let eff_exit_p: Option<f64> = row.get(1).ok();
                    Ok((exit_p, eff_exit_p))
                },
            )
            .map_err(|e| DatabaseError::Query {
                operation: "execute recent closed exit prices query".to_owned(),
                message: e.to_string(),
            })?;
        for r in rows {
            if let Ok(v) = r {
                out.push(v);
            }
        }
        Ok(out)
    }

    /// Get positions by state
    pub async fn get_positions_by_state(&self, state: &PositionState) -> Result<Vec<Position>> {
        // This requires joining with position_states to get current state
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {POSITION_SELECT_COLUMNS} FROM positions p \
             WHERE p.chain_id = ?2 AND EXISTS ( \
               SELECT 1 FROM position_states ps \
               WHERE ps.position_id = p.id \
                 AND ps.state = ?1 \
                 AND ps.changed_at = ( \
                   SELECT MAX(ps2.changed_at) FROM position_states ps2 \
                   WHERE ps2.position_id = p.id \
                 ) \
             ) \
             ORDER BY p.entry_time DESC"
        );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare positions by state query".to_owned(),
            message: e.to_string(),
        })?;

        let position_iter = stmt
            .query_map(params![state.to_string(), self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute positions by state query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }

    /// Get positions with unverified transactions
    pub async fn get_unverified_positions(&self) -> Result<Vec<Position>> {
        let conn = self.get_connection()?;

        let query = format!(
            "SELECT {POSITION_SELECT_COLUMNS} FROM positions \
             WHERE chain_id = ?1 AND (transaction_entry_verified = false \
                OR (exit_transaction_signature IS NOT NULL AND transaction_exit_verified = false) \
             ) ORDER BY entry_time DESC"
        );

        let mut stmt = conn.prepare(&query).map_err(|e| DatabaseError::Query {
            operation: "prepare unverified positions query".to_owned(),
            message: e.to_string(),
        })?;

        let position_iter = stmt
            .query_map(params![self.chain.as_str()], |row| {
                self.row_to_position(row)
            })
            .map_err(|e| DatabaseError::Query {
                operation: "execute unverified positions query".to_owned(),
                message: e.to_string(),
            })?;

        let mut positions = Vec::new();
        for position_result in position_iter {
            positions.push(position_result.map_err(|e| Error::RowDecode {
                column: "<row>",
                detail: e.to_string(),
            })?);
        }

        Ok(positions)
    }
}

/// A swap leg the trader booked, read from an entry or exit record.
#[derive(Debug, Clone, PartialEq)]
pub struct TraderSwapLeg {
    pub position_id: i64,
    pub signature: String,
    pub is_exit: bool,
    /// SOL spent on an entry, or received from an exit, as the record booked it.
    pub native: f64,
    /// Tokens the leg bought or sold, raw units.
    pub amount: RawAmount,
}

/// Which positions a swap-leg read covers.
#[derive(Debug, Clone, Copy)]
pub(super) enum SwapLegScope<'a> {
    /// Every position of the wallet.
    Wallet,
    /// Every position of `mint` in the store of `chain`.
    Mint { chain: &'a str, mint: &'a str },
}

/// Every swap leg the trader booked for `wallet_address` in `scope`, from the entry and exit
/// records. A record whose amount does not decode fails the read.
pub(super) fn query_trader_swap_legs(
    conn: &Connection,
    wallet_address: &str,
    scope: SwapLegScope<'_>,
) -> rusqlite::Result<Vec<TraderSwapLeg>> {
    let position_filter = match scope {
        SwapLegScope::Wallet => "",
        SwapLegScope::Mint { .. } => {
            " AND position_id IN (SELECT id FROM positions WHERE chain_id = ?2 AND mint = ?3)"
        }
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT position_id, transaction_signature, 0 AS is_exit, native_spent AS native, amount
           FROM position_entries WHERE wallet_address = ?1{position_filter}
         UNION ALL
         SELECT position_id, transaction_signature, 1 AS is_exit, native_received AS native, amount
           FROM position_exits WHERE wallet_address = ?1{position_filter}"
    ))?;
    let leg = |row: &rusqlite::Row<'_>| {
        Ok(TraderSwapLeg {
            position_id: row.get(0)?,
            signature: row.get(1)?,
            is_exit: row.get::<_, i64>(2)? != 0,
            native: row.get(3)?,
            amount: row.get(4)?,
        })
    };
    match scope {
        SwapLegScope::Wallet => stmt.query_map(params![wallet_address], leg)?.collect(),
        SwapLegScope::Mint { chain, mint } => stmt
            .query_map(params![wallet_address, chain, mint], leg)?
            .collect(),
    }
}

/// The signatures a position of a mint is booked under, beside its records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintRow {
    pub id: i64,
    pub entry_signature: Option<String>,
    pub exit_signature: Option<String>,
}

impl MintRow {
    /// The row's identity and signatures, `None` for a row not stored yet.
    pub fn of(position: &Position) -> Option<Self> {
        Some(Self {
            id: position.id?,
            entry_signature: position.entry_transaction_signature.clone(),
            exit_signature: position.exit_transaction_signature.clone(),
        })
    }
}

/// Every position of `mint` for `wallet_address` in the store of `chain`.
pub(super) fn query_mint_rows(
    conn: &Connection,
    chain: &str,
    wallet_address: &str,
    mint: &str,
) -> rusqlite::Result<Vec<MintRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, entry_transaction_signature, exit_transaction_signature
         FROM positions
         WHERE chain_id = ?1 AND wallet_address = ?2 AND mint = ?3",
    )?;
    let rows = stmt.query_map(params![chain, wallet_address, mint], |row| {
        Ok(MintRow {
            id: row.get(0)?,
            entry_signature: row.get(1)?,
            exit_signature: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// Columns of an entry record, in the order [`entry_record_from_row`] reads them.
pub(super) const ENTRY_RECORD_COLUMNS: &str =
    "id, position_id, timestamp, amount, price, native_spent, transaction_signature, is_dca, fees_raw";

/// Columns of an exit record, in the order [`exit_record_from_row`] reads them.
pub(super) const EXIT_RECORD_COLUMNS: &str = "id, position_id, timestamp, amount, price, native_received, transaction_signature, is_partial, percentage, fees_raw";

/// The RFC 3339 record timestamp in column `index`.
fn record_timestamp(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&row.get::<_, String>(index)?)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
}

/// An entry record from a row selected with [`ENTRY_RECORD_COLUMNS`].
pub(super) fn entry_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EntryRecord> {
    Ok(EntryRecord {
        id: row.get(0)?,
        position_id: row.get(1)?,
        timestamp: record_timestamp(row, 2)?,
        amount: row.get(3)?,
        price: row.get(4)?,
        native_spent: row.get(5)?,
        transaction_signature: row.get(6)?,
        is_dca: row.get(7)?,
        fees_raw: row.get::<_, Option<i64>>(8)?.map(|f| f as u64),
    })
}

/// An exit record from a row selected with [`EXIT_RECORD_COLUMNS`].
pub(super) fn exit_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExitRecord> {
    Ok(ExitRecord {
        id: row.get(0)?,
        position_id: row.get(1)?,
        timestamp: record_timestamp(row, 2)?,
        amount: row.get(3)?,
        price: row.get(4)?,
        native_received: row.get(5)?,
        transaction_signature: row.get(6)?,
        is_partial: row.get(7)?,
        percentage: row.get(8)?,
        fees_raw: row.get::<_, Option<i64>>(9)?.map(|f| f as u64),
    })
}

/// The entry and exit records `wallet_address` booked under any of `signatures`. A swap
/// is booked at most once per position, so each signature yields at most one record per
/// position.
pub(super) fn query_records_by_signatures(
    conn: &Connection,
    wallet_address: &str,
    signatures: &[String],
) -> rusqlite::Result<(Vec<EntryRecord>, Vec<ExitRecord>)> {
    if signatures.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let placeholders = (2..signatures.len() + 2)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let values = || std::iter::once(wallet_address.to_owned()).chain(signatures.iter().cloned());

    let entries = conn
        .prepare(&format!(
            "SELECT {ENTRY_RECORD_COLUMNS} FROM position_entries
             WHERE wallet_address = ?1 AND transaction_signature IN ({placeholders})"
        ))?
        .query_map(params_from_iter(values()), entry_record_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let exits = conn
        .prepare(&format!(
            "SELECT {EXIT_RECORD_COLUMNS} FROM position_exits
             WHERE wallet_address = ?1 AND transaction_signature IN ({placeholders})"
        ))?
        .query_map(params_from_iter(values()), exit_record_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok((entries, exits))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{count_open_positions_in_file, open_positions_by_wallet_in_file};

    #[test]
    fn a_missing_store_or_table_counts_no_open_position() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        assert_eq!(count_open_positions_in_file(&path, "wallet-a").unwrap(), 0);
        assert!(open_positions_by_wallet_in_file(&path).unwrap().is_empty());

        Connection::open(&path).unwrap();
        assert_eq!(count_open_positions_in_file(&path, "wallet-a").unwrap(), 0);
        assert!(open_positions_by_wallet_in_file(&path).unwrap().is_empty());
    }

    #[test]
    fn open_positions_are_counted_for_the_wallet_on_every_chain() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("positions.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE positions (
                    wallet_address TEXT NOT NULL,
                    chain_id TEXT NOT NULL,
                    archived INTEGER NOT NULL DEFAULT 0,
                    exit_time TEXT,
                    transaction_exit_verified INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT INTO positions VALUES ('wallet-a', 'solana', 0, NULL, 0);
                 INSERT INTO positions VALUES ('wallet-b', 'solana', 0, NULL, 0);
                 INSERT INTO positions VALUES ('wallet-a', 'solana', 1, NULL, 0);
                 INSERT INTO positions VALUES ('wallet-a', 'solana', 0, '2026-01-01T00:00:00Z', 0);
                 INSERT INTO positions VALUES ('wallet-a', 'solana', 0, NULL, 1);
                 INSERT INTO positions VALUES ('wallet-a', 'other-chain', 0, NULL, 0);
                 INSERT INTO positions VALUES ('wallet-c', 'solana', 1, NULL, 0);",
            )
            .unwrap();
        drop(connection);

        assert_eq!(count_open_positions_in_file(&path, "wallet-a").unwrap(), 2);
        assert_eq!(count_open_positions_in_file(&path, "wallet-b").unwrap(), 1);
        assert_eq!(count_open_positions_in_file(&path, "wallet-c").unwrap(), 0);
        assert_eq!(
            open_positions_by_wallet_in_file(&path).unwrap(),
            [("wallet-a".to_owned(), 2), ("wallet-b".to_owned(), 1)]
                .into_iter()
                .collect()
        );
    }
}
