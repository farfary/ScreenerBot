// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! ATA failed cache database operations.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, OptionalExtension};

use super::schema::get_connection;
use crate::errors::DatabaseError;
use crate::wallets::Error;

/// Failed ATA database row
#[derive(Debug, Clone)]
pub struct FailedAtaRow {
    pub ata_address: String,
    pub token_mint: Option<String>,
    pub wallet_address: String,
    pub failure_count: i32,
    pub last_error: Option<String>,
    pub first_failed_at: String,
    pub last_failed_at: String,
    pub next_retry_at: Option<String>,
    pub is_permanent_failure: bool,
}

impl FailedAtaRow {
    pub(crate) fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, Error> {
        let is_permanent_int: i32 = row.get(8).map_err(DatabaseError::from)?;
        Ok(Self {
            ata_address: row.get(0).map_err(DatabaseError::from)?,
            token_mint: row.get(1).map_err(DatabaseError::from)?,
            wallet_address: row.get(2).map_err(DatabaseError::from)?,
            failure_count: row.get(3).map_err(DatabaseError::from)?,
            last_error: row.get(4).map_err(DatabaseError::from)?,
            first_failed_at: row.get(5).map_err(DatabaseError::from)?,
            last_failed_at: row.get(6).map_err(DatabaseError::from)?,
            next_retry_at: row.get(7).map_err(DatabaseError::from)?,
            is_permanent_failure: is_permanent_int != 0,
        })
    }
}

// =============================================================================
// ATA FAILED CACHE OPERATIONS
// =============================================================================

/// Delay before the first retry of a transient close failure.
const RETRY_BASE_DELAY_MINUTES: i64 = 10;

/// Longest delay between retries of a transient close failure.
const RETRY_MAX_DELAY_HOURS: i64 = 24;

/// Delay before the next close attempt after `failure_count` consecutive
/// failures: the base delay doubled per failure, capped at the maximum.
fn retry_delay(failure_count: u32) -> Duration {
    let max = Duration::hours(RETRY_MAX_DELAY_HOURS);
    let doublings = failure_count.saturating_sub(1).min(16);
    Duration::minutes(RETRY_BASE_DELAY_MINUTES)
        .checked_mul(1 << doublings)
        .map_or(max, |delay| delay.min(max))
}

/// Add or update failed ATA entry
///
/// A transient failure records `next_retry_at` with an exponential backoff on the
/// failure count; a permanent failure leaves it empty and is never retried.
pub fn upsert_failed_ata(
    ata_address: &str,
    token_mint: Option<&str>,
    wallet_address: &str,
    error: &str,
    is_permanent: bool,
) -> Result<(), Error> {
    let conn = get_connection()?;
    let now = Utc::now();

    let previous_failures: u32 = conn
        .query_row(
            "SELECT failure_count FROM ata_failed_cache WHERE ata_address = ?1",
            params![ata_address],
            |row| row.get(0),
        )
        .optional()
        .map_err(DatabaseError::from)?
        .unwrap_or(0);
    let next_retry_at = (!is_permanent)
        .then(|| (now + retry_delay(previous_failures.saturating_add(1))).to_rfc3339());

    conn.execute(
        r#"
        INSERT INTO ata_failed_cache (ata_address, token_mint, wallet_address, last_error, is_permanent_failure, last_failed_at, next_retry_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(ata_address) DO UPDATE SET
            failure_count = failure_count + 1,
            last_error = ?4,
            is_permanent_failure = ?5,
            last_failed_at = ?6,
            next_retry_at = ?7
        "#,
        params![
            ata_address,
            token_mint,
            wallet_address,
            error,
            is_permanent as i32,
            now.to_rfc3339(),
            next_retry_at
        ],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}

/// ATA addresses of a wallet that the cleanup must not attempt at `now`: permanent
/// failures, and transient failures whose retry time has not arrived.
pub fn get_excluded_atas_for_wallet(
    wallet_address: &str,
    now: DateTime<Utc>,
) -> Result<HashSet<String>, Error> {
    Ok(get_failed_atas_for_wallet(wallet_address)?
        .into_iter()
        .filter(|row| {
            row.is_permanent_failure
                || row
                    .next_retry_at
                    .as_deref()
                    .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
                    .is_some_and(|at| at > now)
        })
        .map(|row| row.ata_address)
        .collect())
}

/// Get all failed ATAs for a wallet
pub fn get_failed_atas_for_wallet(wallet_address: &str) -> Result<Vec<FailedAtaRow>, Error> {
    let conn = get_connection()?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT ata_address, token_mint, wallet_address,
                   failure_count, last_error, first_failed_at, last_failed_at,
                   next_retry_at, is_permanent_failure
            FROM ata_failed_cache 
            WHERE wallet_address = ?1
            ORDER BY last_failed_at DESC
            "#,
        )
        .map_err(DatabaseError::from)?;

    let rows = stmt
        .query_map(params![wallet_address], |row| {
            Ok(FailedAtaRow::from_row(row))
        })
        .map_err(DatabaseError::from)?;

    let mut atas = Vec::new();
    for row in rows {
        match row {
            Ok(Ok(ata)) => atas.push(ata),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(DatabaseError::from(e).into()),
        }
    }

    Ok(atas)
}

/// Remove ATA from failed cache
pub fn remove_failed_ata(ata_address: &str) -> Result<(), Error> {
    let conn = get_connection()?;

    conn.execute(
        "DELETE FROM ata_failed_cache WHERE ata_address = ?1",
        params![ata_address],
    )
    .map_err(DatabaseError::from)?;

    Ok(())
}
