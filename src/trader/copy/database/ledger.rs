// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Mode-scoped paper spend reads, epoch state, and paper-position queries.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::trader::copy::types::{PaperPosition, SpendState};
use crate::trader::error::Error;
use crate::trader::paper::TradingMode;

use super::rows::parse_datetime;
use super::CopyDatabase;

pub(super) fn read_paper_epoch(
    connection: &Connection,
    task_id: i64,
) -> crate::trader::Result<u64> {
    let key = format!("paper_epoch:{task_id}");
    let value = connection
        .query_row(
            "SELECT value FROM copy_metadata WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(crate::errors::DatabaseError::from)?;
    match value {
        None => Ok(0),
        Some(value) => value.parse::<u64>().map_err(|_| {
            crate::errors::DatabaseError::Migration {
                store: "copy_trading".to_owned(),
                detail: format!("paper epoch for task {task_id} is malformed"),
            }
            .into()
        }),
    }
}

pub(super) fn mode_key(mode: TradingMode) -> &'static str {
    match mode {
        TradingMode::Paper => "paper",
        TradingMode::Live => "live",
    }
}

const PAPER_POSITION_COLUMNS: &str = "task_id, mint, token_amount, cost_basis_native, invested_native, \
     realized_proceeds_native, realized_cost_native, buys, sells, last_price_native, last_price_at, \
     opened_at, closed_at, peak_price_native, token_amount_raw, token_decimals, take_profit_level, \
     round_partial_exits";

pub(super) fn spend_state_on(
    connection: &Connection,
    task_id: i64,
    mode: TradingMode,
    mint: &str,
) -> crate::trader::Result<SpendState> {
    let total_spent_native: f64 = connection
        .query_row(
            "SELECT COALESCE(SUM(spent_native), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
            params![task_id, mode_key(mode)],
            |row| row.get(0),
        )
        .map_err(crate::errors::DatabaseError::from)?;
    let token = connection
        .query_row(
            "SELECT spent_native, buy_count FROM copy_spend WHERE task_id = ?1 AND mode = ?2 AND mint = ?3",
            params![task_id, mode_key(mode), mint],
            |row| Ok((row.get::<_, f64>(0)?, row.get::<_, u64>(1)?)),
        )
        .optional()
        .map_err(crate::errors::DatabaseError::from)?
        .unwrap_or_default();
    Ok(SpendState {
        total_spent_native,
        token_spent_native: token.0,
        token_buy_count: token.1,
    })
}

impl CopyDatabase {
    pub async fn paper_epoch(&self, task_id: i64) -> crate::trader::Result<u64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            read_paper_epoch(&connection, task_id)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Spend in one execution mode. Paper and live budgets are separate ledgers:
    /// simulated fills never consume the budget a live task may spend.
    pub async fn spend_state(
        &self,
        task_id: i64,
        mode: TradingMode,
        mint: &str,
    ) -> crate::trader::Result<SpendState> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || db.spend_state_sync(task_id, mode, &mint))
            .await
            .map_err(|e| Error::CopyDatabaseUnavailable {
                detail: e.to_string(),
            })?
    }

    pub async fn task_total_spent(
        &self,
        task_id: i64,
        mode: TradingMode,
    ) -> crate::trader::Result<f64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT COALESCE(SUM(spent_native), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
                    params![task_id, mode_key(mode)],
                    |row| row.get(0),
                )
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    fn spend_state_sync(
        &self,
        task_id: i64,
        mode: TradingMode,
        mint: &str,
    ) -> crate::trader::Result<SpendState> {
        let connection = self.connection()?;
        spend_state_on(&connection, task_id, mode, mint)
    }

    /// Every paper position a task has held, open ones first.
    pub async fn paper_positions(&self, task_id: i64) -> crate::trader::Result<Vec<PaperPosition>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            let mut statement = connection
                .prepare(&format!(
                    "SELECT {PAPER_POSITION_COLUMNS} FROM copy_paper_positions \
                     WHERE task_id = ?1 ORDER BY closed_at IS NOT NULL, opened_at DESC"
                ))
                .map_err(crate::errors::DatabaseError::from)?;
            let rows = statement
                .query_map(params![task_id], row_to_active_paper_position)
                .map_err(crate::errors::DatabaseError::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(crate::errors::DatabaseError::from)?;
            Ok(rows)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Frozen pre-v8 estimates retained for read-only workspace disclosure.
    pub async fn legacy_paper_positions(
        &self,
        task_id: i64,
    ) -> crate::trader::Result<Vec<PaperPosition>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            let mut statement = connection
                .prepare(&format!(
                    "SELECT task_id, mint, token_amount, cost_basis_native, invested_native, \
                     realized_proceeds_native, realized_cost_native, buys, sells, last_price_native, \
                     last_price_at, opened_at, closed_at, peak_price_native, NULL, NULL, 0, 0 \
                     FROM copy_paper_legacy_positions \
                     WHERE task_id = ?1 ORDER BY closed_at IS NOT NULL, opened_at DESC"
                ))
                .map_err(crate::errors::DatabaseError::from)?;
            let rows = statement
                .query_map(params![task_id], row_to_paper_position)
                .map_err(crate::errors::DatabaseError::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(crate::errors::DatabaseError::from)?;
            Ok(rows)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    pub async fn legacy_task_total_spent(&self, task_id: i64) -> crate::trader::Result<f64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT COALESCE(SUM(spent_native), 0) FROM copy_spend \
                     WHERE task_id = ?1 AND mode = 'paper_legacy'",
                    params![task_id],
                    |row| row.get(0),
                )
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Raise an open paper holding's running peak to `price_native` when it is higher,
    /// while the holding is still the one read: opened at `opened_at` and holding the `buys`
    /// it was read with. A buy booked since then restarted the peak at its fill, and a
    /// holding opened since then (after a close or a paper reset) has its own peak; a peak
    /// read before either must not land on it.
    pub async fn raise_paper_peak(
        &self,
        task_id: i64,
        mint: &str,
        opened_at: DateTime<Utc>,
        buys: u64,
        price_native: f64,
    ) -> crate::trader::Result<()> {
        let db = self.clone();
        let mint = mint.to_owned();
        let opened_at = opened_at.to_rfc3339();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .execute(
                    "UPDATE copy_paper_positions \
                     SET peak_price_native = MAX(COALESCE(peak_price_native, ?3), ?3) \
                     WHERE task_id = ?1 AND mint = ?2 AND closed_at IS NULL AND buys = ?4 \
                     AND opened_at = ?5",
                    params![task_id, mint, price_native, buys, opened_at],
                )
                .map(|_| ())
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    pub async fn paper_position(
        &self,
        task_id: i64,
        mint: &str,
    ) -> crate::trader::Result<Option<PaperPosition>> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    &format!(
                        "SELECT {PAPER_POSITION_COLUMNS} FROM copy_paper_positions \
                         WHERE task_id = ?1 AND mint = ?2"
                    ),
                    params![task_id, mint],
                    row_to_active_paper_position,
                )
                .optional()
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }
}

fn row_to_paper_position(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaperPosition> {
    let optional_time = |index: usize| -> rusqlite::Result<Option<DateTime<Utc>>> {
        row.get::<_, Option<String>>(index)?
            .map(|value| parse_datetime(&value, index))
            .transpose()
    };
    Ok(PaperPosition {
        task_id: row.get(0)?,
        mint: row.get(1)?,
        token_amount: row.get(2)?,
        raw_token_amount: row.get(14)?,
        token_decimals: row.get(15)?,
        cost_basis_native: row.get(3)?,
        invested_native: row.get(4)?,
        realized_proceeds_native: row.get(5)?,
        realized_cost_native: row.get(6)?,
        buys: row.get(7)?,
        sells: row.get(8)?,
        last_price_native: row.get(9)?,
        last_price_at: optional_time(10)?,
        opened_at: parse_datetime(&row.get::<_, String>(11)?, 11)?,
        closed_at: optional_time(12)?,
        peak_price_native: row.get(13)?,
        take_profit_level: row.get(16)?,
        round_partial_exits: row.get(17)?,
    })
}

fn row_to_active_paper_position(row: &rusqlite::Row<'_>) -> rusqlite::Result<PaperPosition> {
    let position = row_to_paper_position(row)?;
    if position.raw_token_amount.is_none() || position.token_decimals.is_none() {
        return Err(rusqlite::Error::InvalidColumnType(
            14,
            "token_amount_raw".to_owned(),
            rusqlite::types::Type::Null,
        ));
    }
    Ok(position)
}
