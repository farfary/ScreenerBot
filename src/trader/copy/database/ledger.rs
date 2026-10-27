// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The live spend ledger and the read-only reads of the archived copy paper books. The
//! spend of a paper task is derived from its rounds in the shared paper book
//! (`copy::paper_book`), never stored here.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::trader::copy::types::{ArchivedPaperPosition, SpendState};
use crate::trader::error::Error;

use super::rows::parse_datetime;
use super::CopyDatabase;

/// The `copy_spend.mode` of live spend, the only spend the ledger still books.
pub(super) const LIVE_SPEND: &str = "live";

/// Columns `row_to_archived_position` decodes, in order.
const ARCHIVED_POSITION_COLUMNS: &str = "task_id, mint, token_amount, cost_basis_native, \
     realized_proceeds_native, realized_cost_native, last_price_native, last_price_at, \
     opened_at, closed_at";

fn live_spend_state_on(
    connection: &Connection,
    task_id: i64,
    mint: &str,
) -> crate::trader::Result<SpendState> {
    let total_spent_native: f64 = connection
        .query_row(
            "SELECT COALESCE(SUM(spent_native), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
            params![task_id, LIVE_SPEND],
            |row| row.get(0),
        )
        .map_err(crate::errors::DatabaseError::from)?;
    let token = connection
        .query_row(
            "SELECT spent_native, buy_count FROM copy_spend WHERE task_id = ?1 AND mode = ?2 AND mint = ?3",
            params![task_id, LIVE_SPEND, mint],
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
    /// What a task spent live, in total and on `mint`. Simulated fills never consume
    /// the budget a live task may spend.
    pub async fn live_spend_state(
        &self,
        task_id: i64,
        mint: &str,
    ) -> crate::trader::Result<SpendState> {
        let db = self.clone();
        let mint = mint.to_owned();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            live_spend_state_on(&connection, task_id, &mint)
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// Everything a task spent live.
    pub async fn live_total_spent(&self, task_id: i64) -> crate::trader::Result<f64> {
        self.spent_in(task_id, LIVE_SPEND).await
    }

    /// What the archived quote-model copy paper book spent.
    pub async fn archived_paper_spent(&self, task_id: i64) -> crate::trader::Result<f64> {
        self.spent_in(task_id, "paper_archive").await
    }

    /// What the pre-quote estimate paper book spent.
    pub async fn legacy_task_total_spent(&self, task_id: i64) -> crate::trader::Result<f64> {
        self.spent_in(task_id, "paper_legacy").await
    }

    async fn spent_in(&self, task_id: i64, mode: &'static str) -> crate::trader::Result<f64> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            db.connection()?
                .query_row(
                    "SELECT COALESCE(SUM(spent_native), 0) FROM copy_spend WHERE task_id = ?1 AND mode = ?2",
                    params![task_id, mode],
                    |row| row.get(0),
                )
                .map_err(|e| Error::from(crate::errors::DatabaseError::from(e)))
        })
        .await
        .map_err(|e| Error::CopyDatabaseUnavailable {
            detail: e.to_string(),
        })?
    }

    /// The holdings of the archived quote-model copy paper book, open ones first.
    pub async fn archived_paper_positions(
        &self,
        task_id: i64,
    ) -> crate::trader::Result<Vec<ArchivedPaperPosition>> {
        self.archived_positions_in(task_id, "copy_paper_positions")
            .await
    }

    /// Frozen pre-v8 estimates retained for read-only workspace disclosure.
    pub async fn legacy_paper_positions(
        &self,
        task_id: i64,
    ) -> crate::trader::Result<Vec<ArchivedPaperPosition>> {
        self.archived_positions_in(task_id, "copy_paper_legacy_positions")
            .await
    }

    async fn archived_positions_in(
        &self,
        task_id: i64,
        table: &'static str,
    ) -> crate::trader::Result<Vec<ArchivedPaperPosition>> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || {
            let connection = db.connection()?;
            let mut statement = connection
                .prepare(&format!(
                    "SELECT {ARCHIVED_POSITION_COLUMNS} FROM {table} \
                     WHERE task_id = ?1 ORDER BY closed_at IS NOT NULL, opened_at DESC"
                ))
                .map_err(crate::errors::DatabaseError::from)?;
            let rows = statement
                .query_map(params![task_id], row_to_archived_position)
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
}

fn row_to_archived_position(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArchivedPaperPosition> {
    let optional_time = |index: usize| -> rusqlite::Result<Option<DateTime<Utc>>> {
        row.get::<_, Option<String>>(index)?
            .map(|value| parse_datetime(&value, index))
            .transpose()
    };
    Ok(ArchivedPaperPosition {
        task_id: row.get(0)?,
        mint: row.get(1)?,
        token_amount: row.get(2)?,
        cost_basis_native: row.get(3)?,
        realized_proceeds_native: row.get(4)?,
        realized_cost_native: row.get(5)?,
        last_price_native: row.get(6)?,
        last_price_at: optional_time(7)?,
        opened_at: parse_datetime(&row.get::<_, String>(8)?, 8)?,
        closed_at: optional_time(9)?,
    })
}
