// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Eviction of the chart history of tokens no longer monitored: their candles, gaps and pools and then their monitor entry, deleted in bounded transactions that stop as soon as the chart is monitored or viewed again.

use std::collections::HashSet;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};

use crate::chains::ChainId;
use crate::database::WriteTransaction;
use crate::errors::DatabaseError;

/// Rows one eviction transaction deletes at most before it commits.
pub const MAX_ROWS_PER_TRANSACTION: usize = 2_000;
/// Pause between two transactions, so writers waiting on the store get their turn.
const TRANSACTION_PAUSE: Duration = Duration::from_millis(25);
/// The tables holding a chart's history, deleted before its monitor entry.
const CHART_TABLES: [&str; 3] = ["ohlcv_candles", "ohlcv_gaps", "ohlcv_pools"];

/// A token whose chart is no longer monitored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InactiveChart {
    pub mint: String,
    /// When the chart was last monitored or viewed, as stored (RFC 3339).
    pub last_activity: String,
    pub candles: u64,
}

/// What one chart eviction did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChartEviction {
    pub rows: u64,
    /// Whether the monitor entry was removed, so nothing of the chart is left.
    pub completed: bool,
    /// Whether the chart was monitored or viewed again, or shutdown was requested,
    /// before it was removed.
    pub interrupted: bool,
}

fn failed(operation: &'static str) -> impl Fn(rusqlite::Error) -> DatabaseError {
    move |error| DatabaseError::classify_sqlite_failure(operation, error)
}

/// The tokens of `chain` whose charts are monitored now.
pub fn active_chart_mints(
    conn: &Connection,
    chain: ChainId,
) -> Result<HashSet<String>, DatabaseError> {
    let mut statement = conn
        .prepare_cached(
            "SELECT mint FROM ohlcv_monitor_config WHERE chain_id = ?1 AND is_active = 1",
        )
        .map_err(failed("list monitored charts"))?;
    let mints = statement
        .query_map(params![chain.as_str()], |row| row.get(0))
        .map_err(failed("list monitored charts"))?
        .collect::<rusqlite::Result<_>>()
        .map_err(failed("read monitored charts"))?;
    Ok(mints)
}

/// Every candle `chain` holds, and each token whose chart is no longer monitored
/// with its own candle count. Counted on the candle lookup index.
pub fn inactive_charts(
    conn: &Connection,
    chain: ChainId,
) -> Result<(u64, Vec<InactiveChart>), DatabaseError> {
    let as_count = |count: i64| u64::try_from(count).unwrap_or(0);
    let total = conn
        .query_row(
            "SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = ?1",
            params![chain.as_str()],
            |row| row.get(0),
        )
        .map(as_count)
        .map_err(failed("count candles"))?;
    let charts: Vec<(String, String)> = conn
        .prepare_cached(
            "SELECT mint, last_activity FROM ohlcv_monitor_config \
             WHERE chain_id = ?1 AND is_active = 0",
        )
        .map_err(failed("list inactive charts"))?
        .query_map(params![chain.as_str()], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(failed("list inactive charts"))?
        .collect::<rusqlite::Result<_>>()
        .map_err(failed("read inactive charts"))?;
    let mut count = conn
        .prepare_cached("SELECT COUNT(*) FROM ohlcv_candles WHERE chain_id = ?1 AND mint = ?2")
        .map_err(failed("count chart candles"))?;
    let mut inactive = Vec::with_capacity(charts.len());
    for (mint, last_activity) in charts {
        let candles = count
            .query_row(params![chain.as_str(), mint], |row| row.get(0))
            .map(as_count)
            .map_err(failed("count chart candles"))?;
        inactive.push(InactiveChart {
            mint,
            last_activity,
            candles,
        });
    }
    Ok((total, inactive))
}

/// Delete the chart history of `mint` on `chain`, then its monitor entry, as long as
/// the chart is still unmonitored and was last active at `last_activity`. Each
/// transaction deletes at most `MAX_ROWS_PER_TRANSACTION` rows and first checks the
/// chart again, so a chart monitored or viewed meanwhile keeps what is left of it;
/// `stop` is checked between transactions.
pub fn evict_chart_history(
    conn: &mut Connection,
    chain: ChainId,
    mint: &str,
    last_activity: &str,
    stop: &dyn Fn() -> bool,
) -> Result<ChartEviction, DatabaseError> {
    let mut eviction = ChartEviction::default();
    loop {
        if stop() {
            eviction.interrupted = true;
            return Ok(eviction);
        }
        let transaction = conn.write_tx().map_err(failed("begin chart eviction"))?;
        let unchanged = transaction
            .query_row(
                "SELECT 1 FROM ohlcv_monitor_config \
                 WHERE chain_id = ?1 AND mint = ?2 AND is_active = 0 AND last_activity = ?3",
                params![chain.as_str(), mint, last_activity],
                |_| Ok(()),
            )
            .optional()
            .map_err(failed("check chart before eviction"))?
            .is_some();
        if !unchanged {
            eviction.interrupted = true;
            return Ok(eviction);
        }
        let mut budget = MAX_ROWS_PER_TRANSACTION;
        for table in CHART_TABLES {
            let deleted = transaction
                .prepare_cached(&format!(
                    "DELETE FROM {table} WHERE id IN \
                     (SELECT id FROM {table} WHERE chain_id = ?1 AND mint = ?2 LIMIT ?3)"
                ))
                .map_err(failed("prepare chart delete"))?
                .execute(params![chain.as_str(), mint, budget])
                .map_err(failed("delete chart rows"))?;
            budget -= deleted;
            if budget == 0 {
                break;
            }
        }
        if budget > 0 {
            budget -= transaction
                .execute(
                    "DELETE FROM ohlcv_monitor_config WHERE chain_id = ?1 AND mint = ?2",
                    params![chain.as_str(), mint],
                )
                .map_err(failed("delete chart monitor entry"))?;
            eviction.completed = true;
        }
        transaction
            .commit()
            .map_err(failed("commit chart eviction"))?;
        eviction.rows += (MAX_ROWS_PER_TRANSACTION - budget) as u64;
        if eviction.completed {
            return Ok(eviction);
        }
        std::thread::sleep(TRANSACTION_PAUSE);
    }
}
