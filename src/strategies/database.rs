// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy database — persistence for custom strategy templates.

mod async_api;

pub use async_api::*;

use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::database::{self, WriteTransaction};
use crate::errors::{DatabaseError, Error};
use crate::logger::{self, LogTag};
use crate::strategies::types::{EvaluationResult, EvaluationTotals, Strategy, StrategyType};
use crate::trader::paper::TradingMode;
use chrono::{DateTime, SecondsFormat, Utc};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension, Transaction};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

/// The version the store's own `schema_version` table records once its tables exist.
const STRATEGIES_SCHEMA_VERSION: u32 = 1;

/// The store version this core writes. Version 2 drops `strategy_assignments`, which
/// nothing read or wrote (an automatic position records its strategy in its origin).
/// Version 3 adds `strategies.entry_strategy_ids_json`, the entry strategies an exit
/// strategy manages; its default `'[]'` (every position) is what a v2 core does.
/// Version 4 adds `strategies.mode`, paper or live; rows from before it trade live.
/// Version 5 adds `strategies.rules_changed_at` and `strategies.rules_fingerprint`, the
/// rules clock, and the `strategy_evaluation_totals` table; evaluations are counted in
/// the totals, and the per-evaluation rows of `strategy_performance` are folded into
/// them at every open.
const STORE_VERSION: i64 = 5;

/// The oldest core that reads the store. The v4 mode is breaking: a core that ignores
/// it would trade a paper strategy live, so an older core must refuse the store. Version
/// 5 changes the meaning of no existing column, so a v4 core still reads it: it trades
/// correctly, leaves the rules clock as it is and records evaluations per row, and the
/// next v5 open reconciles its edits and folds its rows.
const MIN_READER_VERSION: i64 = 4;

/// The version the store held when the schema ledger was adopted.
const LEDGER_BASELINE_VERSION: i64 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_STRATEGIES: &str = r#"
CREATE TABLE IF NOT EXISTS strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    type TEXT NOT NULL, -- 'ENTRY' or 'EXIT'
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 10,
    timeframe TEXT NOT NULL DEFAULT '5m', -- Which timeframe to use for OHLCV conditions
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    entry_strategy_ids_json TEXT NOT NULL DEFAULT '[]', -- exit strategies: the entry strategies managed, empty = all
    mode TEXT NOT NULL DEFAULT 'live' CHECK(mode IN ('paper', 'live')), -- the book the strategy trades
    rules_changed_at TEXT, -- when the rules fingerprint last changed
    rules_fingerprint TEXT -- the rules the strategy decides on, hashed
);

CREATE INDEX IF NOT EXISTS idx_strategies_type ON strategies(type);
CREATE INDEX IF NOT EXISTS idx_strategies_enabled ON strategies(enabled);
CREATE INDEX IF NOT EXISTS idx_strategies_priority ON strategies(priority);
"#;

const SCHEMA_STRATEGY_PERFORMANCE: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id TEXT NOT NULL,
    execution_time_ms INTEGER NOT NULL,
    result INTEGER NOT NULL, -- 0 or 1
    confidence REAL NOT NULL,
    details_json TEXT,
    token_mint TEXT,
    execution_timestamp TEXT NOT NULL,
    trade_id TEXT,
    chain_id TEXT NOT NULL DEFAULT 'solana',
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_performance_strategy ON strategy_performance(strategy_id);
CREATE INDEX IF NOT EXISTS idx_performance_timestamp ON strategy_performance(execution_timestamp);
CREATE INDEX IF NOT EXISTS idx_performance_token ON strategy_performance(token_mint);
"#;

/// One row per strategy: every evaluation recorded for it, summed. Its times are written
/// in [`totals_time`]'s fixed form, so text order is time order.
const SCHEMA_STRATEGY_EVALUATION_TOTALS: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_evaluation_totals (
    strategy_id TEXT PRIMARY KEY REFERENCES strategies(id) ON DELETE CASCADE,
    evaluations INTEGER NOT NULL DEFAULT 0,
    signals INTEGER NOT NULL DEFAULT 0,
    total_exec_ms INTEGER NOT NULL DEFAULT 0,
    last_evaluated_at TEXT,
    last_signal_at TEXT
);
"#;

const SCHEMA_STRATEGY_TEMPLATES: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    risk_level TEXT NOT NULL, -- 'LOW', 'MEDIUM', 'HIGH'
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT
);

CREATE INDEX IF NOT EXISTS idx_templates_category ON strategy_templates(category);
CREATE INDEX IF NOT EXISTS idx_templates_risk ON strategy_templates(risk_level);
"#;

const SCHEMA_STRATEGY_BACKTESTS: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_backtests (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    win_trades INTEGER NOT NULL,
    loss_trades INTEGER NOT NULL,
    total_profit_sol REAL NOT NULL,
    results_json TEXT NOT NULL,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_backtests_strategy ON strategy_backtests(strategy_id);
CREATE INDEX IF NOT EXISTS idx_backtests_start ON strategy_backtests(start_time);
"#;

/// Drops the tables of removed features. A core from before v2 re-creates them
/// whenever it opens the store, so every open checks for them rather than only the
/// upgrade that retired them.
const DROP_RETIRED_TABLES: &str = r#"
DROP TABLE IF EXISTS strategy_assignments;
"#;

const SCHEMA_VERSION_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
"#;

// =============================================================================
// SCHEMA STEP AND CONNECTIONS
// =============================================================================

/// The schema of `strategies.db`.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: STORE_VERSION,
    min_reader_version: MIN_READER_VERSION,
    baseline: LEDGER_BASELINE_VERSION,
    renames: None,
    step: create_schema,
    legacy_stamp: None,
};

fn create_schema(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    ddl::run_batch(
        tx,
        "create strategies schema_version table",
        SCHEMA_VERSION_TABLE,
    )?;
    let current_version: Option<u32> = tx
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| DatabaseError::classify_sqlite_failure("read strategies schema version", e))?;

    if current_version.unwrap_or(0) < STRATEGIES_SCHEMA_VERSION {
        for (operation, sql) in [
            ("create strategies table", SCHEMA_STRATEGIES),
            (
                "create strategy_performance table",
                SCHEMA_STRATEGY_PERFORMANCE,
            ),
            ("create strategy_templates table", SCHEMA_STRATEGY_TEMPLATES),
            ("create strategy_backtests table", SCHEMA_STRATEGY_BACKTESTS),
        ] {
            ddl::run_batch(tx, operation, sql)?;
        }
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![STRATEGIES_SCHEMA_VERSION, Utc::now().to_rfc3339()],
        )
        .map_err(|e| {
            DatabaseError::classify_sqlite_failure("record strategies schema version", e)
        })?;
    }

    // Chain scope: `strategy_performance` gains a chain column, checked whatever the
    // version stamp says: a store from v0.2.13 already records version 1. Historical
    // rows are Solana's by construction, so the migration backfills 'solana'.
    ddl::add_column(
        tx,
        "strategy_performance",
        "chain_id",
        "TEXT NOT NULL DEFAULT 'solana'",
    )?;
    // Exit-strategy binding: a store from before v3 gains the column with the default
    // that manages every position, which is what every exit strategy did there.
    ddl::add_column(
        tx,
        "strategies",
        "entry_strategy_ids_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    // Paper or live: a store from before v4 gains the column with the mode every
    // strategy traded in there.
    ddl::add_column(
        tx,
        "strategies",
        "mode",
        "TEXT NOT NULL DEFAULT 'live' CHECK(mode IN ('paper', 'live'))",
    )?;
    // The rules clock: a store from before v5 starts every clock at its strategy's
    // creation, so no strategy's paper evidence restarts on the upgrade.
    let clock_added = ddl::add_column(tx, "strategies", "rules_changed_at", "TEXT")?;
    ddl::add_column(tx, "strategies", "rules_fingerprint", "TEXT")?;
    if clock_added {
        ddl::run(
            tx,
            "start the strategies rules clock",
            "UPDATE strategies SET rules_changed_at = created_at",
        )?;
    }
    reconcile_rules_clock(tx, clock_added)?;
    if !ddl::has_table(tx, "strategy_evaluation_totals")? {
        ddl::run_batch(
            tx,
            "create strategy_evaluation_totals table",
            SCHEMA_STRATEGY_EVALUATION_TOTALS,
        )?;
    }
    fold_evaluation_rows(tx)?;
    ddl::run_batch(tx, "drop retired strategies tables", DROP_RETIRED_TABLES)
}

/// Bring every strategy's rules clock in line with its rules. With the clock `just_added`
/// each row only takes its fingerprint. Otherwise a row whose stored fingerprint is
/// missing (inserted by an older core) or differs (its rules edited by an older core)
/// moves its clock to the later of its clock and its last update, which never moves it
/// earlier, and takes the fingerprint. A row that does not parse is left as it is.
/// Idempotent: a second open writes nothing.
fn reconcile_rules_clock(tx: &Transaction<'_>, just_added: bool) -> Result<(), DatabaseError> {
    let operation = "reconcile strategies rules clock";
    let classify = |e| DatabaseError::classify_sqlite_failure(operation, e);
    let rows = {
        let mut stmt = tx
            .prepare(&format!(
                "SELECT {STRATEGY_COLUMNS}, rules_fingerprint FROM strategies"
            ))
            .map_err(classify)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    read_strategy_row(row),
                    row.get::<_, Option<String>>(STRATEGY_COLUMN_COUNT)?,
                ))
            })
            .map_err(classify)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(classify)?;
        rows
    };
    let mut reconciled = 0;
    for (row, stored) in rows {
        let strategy = match row
            .map_err(crate::Error::from)
            .and_then(StrategyRow::into_strategy)
        {
            Ok(Some(strategy)) => strategy,
            Ok(None) => continue,
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("Strategies rules clock: a stored strategy does not parse: {e}"),
                );
                continue;
            }
        };
        let fingerprint = strategy.rules_fingerprint();
        if stored.as_deref() == Some(fingerprint.as_str()) {
            continue;
        }
        if just_added {
            tx.execute(
                "UPDATE strategies SET rules_fingerprint = ?2 WHERE id = ?1",
                params![strategy.id, fingerprint],
            )
            .map_err(classify)?;
        } else {
            let changed_at = strategy.rules_changed_at.max(strategy.updated_at);
            tx.execute(
                "UPDATE strategies SET rules_fingerprint = ?2, rules_changed_at = ?3 WHERE id = ?1",
                params![strategy.id, fingerprint, changed_at.to_rfc3339()],
            )
            .map_err(classify)?;
            reconciled += 1;
        }
    }
    if reconciled > 0 {
        logger::info(
            LogTag::System,
            &format!("Strategies rules clock: {reconciled} strategies written by an older version reconciled"),
        );
    }
    Ok(())
}

/// Fold the per-evaluation rows into the evaluation totals and delete them. A v5 core
/// writes no such row; those present were written before the upgrade or by an older
/// core since, so the totals count every evaluation either version recorded.
fn fold_evaluation_rows(tx: &Transaction<'_>) -> Result<(), DatabaseError> {
    let operation = "fold strategy evaluations into the totals";
    let classify = |e| DatabaseError::classify_sqlite_failure(operation, e);
    let started = Instant::now();
    let rows = {
        let mut stmt = tx
            .prepare(
                "SELECT strategy_id, COUNT(*), SUM(result), SUM(execution_time_ms),
                        MAX(execution_timestamp),
                        MAX(CASE WHEN result = 1 THEN execution_timestamp END)
                 FROM strategy_performance
                 GROUP BY strategy_id",
            )
            .map_err(classify)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(classify)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(classify)?;
        rows
    };
    if rows.is_empty() {
        return Ok(());
    }
    let mut folded = 0;
    for (strategy_id, evaluations, signals, total_exec_ms, last_evaluated, last_signal) in rows {
        let totals = EvaluationTotals {
            evaluations: count_of(evaluations),
            signals: count_of(signals),
            total_exec_ms: count_of(total_exec_ms),
            last_evaluated_at: last_evaluated.as_deref().and_then(stored_evaluation_time),
            last_signal_at: last_signal.as_deref().and_then(stored_evaluation_time),
        };
        folded += totals.evaluations;
        add_to_totals(tx, &strategy_id, &totals).map_err(classify)?;
    }
    ddl::run(
        tx,
        "delete folded strategy evaluations",
        "DELETE FROM strategy_performance",
    )?;
    logger::info(
        LogTag::System,
        &format!(
            "Strategy evaluations folded into the totals: rows={folded}, elapsed_ms={}",
            started.elapsed().as_millis()
        ),
    );
    Ok(())
}

/// A count read from SQLite, which stores it signed; never negative in this store.
fn count_of(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

/// A per-evaluation row's timestamp, or `None` with a warning when it does not parse.
fn stored_evaluation_time(value: &str) -> Option<DateTime<Utc>> {
    match DateTime::parse_from_rfc3339(value) {
        Ok(time) => Some(time.with_timezone(&Utc)),
        Err(e) => {
            logger::warning(
                LogTag::System,
                &format!("Strategy evaluation time '{value}' does not parse: {e}"),
            );
            None
        }
    }
}

/// Get a connection from the strategies store's pool.
fn get_connection() -> crate::Result<PooledConnection<SqliteConnectionManager>> {
    database::open::store(&database::catalog::STRATEGIES)?
        .get()
        .map_err(Into::into)
}

/// Open the strategies store.
pub fn init_strategies_db() -> crate::Result<()> {
    database::open::store(&database::catalog::STRATEGIES)?;
    Ok(())
}

// =============================================================================
// STRATEGY CRUD OPERATIONS
// =============================================================================

/// The `strategies` columns every reader selects, in the order [`read_strategy_row`]
/// reads them.
const STRATEGY_COLUMNS: &str = "id, name, description, type, enabled, priority, timeframe, \
     rules_json, parameters_json, created_at, updated_at, author, version, entry_strategy_ids_json, \
     mode, rules_changed_at";

/// How many columns [`STRATEGY_COLUMNS`] names.
const STRATEGY_COLUMN_COUNT: usize = 16;

/// The order strategies are listed in: lowest priority first, the name breaking a tie. It
/// is the entry evaluation order of each book, so the trader and the strategies overview
/// read the one order this constant defines.
const STRATEGY_ORDER: &str = "ORDER BY priority ASC, name ASC";

/// One `strategies` row as stored, before its JSON and timestamps are parsed.
struct StrategyRow {
    id: String,
    name: String,
    description: Option<String>,
    type_str: String,
    enabled: bool,
    priority: i32,
    timeframe: String,
    rules_json: String,
    parameters_json: String,
    created_at: String,
    updated_at: String,
    author: Option<String>,
    version: i32,
    entry_strategy_ids_json: String,
    mode: String,
    rules_changed_at: Option<String>,
}

fn read_strategy_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StrategyRow> {
    Ok(StrategyRow {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        type_str: row.get(3)?,
        enabled: row.get(4)?,
        priority: row.get(5)?,
        timeframe: row.get(6)?,
        rules_json: row.get(7)?,
        parameters_json: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        author: row.get(11)?,
        version: row.get(12)?,
        entry_strategy_ids_json: row.get(13)?,
        mode: row.get(14)?,
        rules_changed_at: row.get(15)?,
    })
}

impl StrategyRow {
    /// The strategy this row stores, or `None` when its type is neither entry nor exit.
    fn into_strategy(self) -> crate::Result<Option<Strategy>> {
        let strategy_type = match self.type_str.as_str() {
            "ENTRY" => StrategyType::Entry,
            "EXIT" => StrategyType::Exit,
            _ => return Ok(None),
        };
        let id = self.id;
        let parse_time = |value: &str, field: &str| {
            DateTime::parse_from_rfc3339(value)
                .map(|time| time.with_timezone(&Utc))
                .map_err(|e| Error::parse_error(format!("Failed to parse {field} for {id}: {e}")))
        };
        let mode = TradingMode::parse(&self.mode)
            .ok_or_else(|| Error::parse_error(format!("Unknown mode '{}' for {id}", self.mode)))?;
        let created_at = parse_time(&self.created_at, "created_at")?;
        let updated_at = parse_time(&self.updated_at, "updated_at")?;
        // A clock an older core left unset reads as the last update: never earlier than
        // the rules it could have changed.
        let rules_changed_at = match self.rules_changed_at.as_deref() {
            Some(value) => parse_time(value, "rules_changed_at")?,
            None => updated_at,
        };
        Ok(Some(Strategy {
            name: self.name,
            description: self.description,
            strategy_type,
            enabled: self.enabled,
            priority: self.priority,
            timeframe: self.timeframe,
            rules: serde_json::from_str(&self.rules_json)?,
            parameters: serde_json::from_str(&self.parameters_json)?,
            created_at,
            updated_at,
            author: self.author,
            version: self.version,
            entry_strategy_ids: serde_json::from_str(&self.entry_strategy_ids_json)?,
            mode,
            rules_changed_at,
            id,
        }))
    }
}

/// Every strategy a query returns, skipping rows whose type is neither entry nor exit.
fn read_strategies(
    conn: &rusqlite::Connection,
    sql: &str,
    params: impl rusqlite::Params,
) -> crate::Result<Vec<Strategy>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params, read_strategy_row)?
        .collect::<Result<Vec<_>, _>>()?;
    let mut strategies = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(strategy) = row.into_strategy()? {
            strategies.push(strategy);
        }
    }
    Ok(strategies)
}

/// Insert a new strategy. Its rules clock starts at its creation time, whatever
/// `rules_changed_at` it carries.
pub fn insert_strategy(strategy: &Strategy) -> crate::Result<()> {
    let conn = get_connection()?;
    insert_strategy_in(&conn, strategy)?;

    logger::info(
        LogTag::System,
        &format!(
            "Inserted strategy: id={}, name={}, type={}",
            strategy.id, strategy.name, strategy.strategy_type
        ),
    );

    Ok(())
}

fn insert_strategy_in(conn: &rusqlite::Connection, strategy: &Strategy) -> crate::Result<()> {
    let rules_json = serde_json::to_string(&strategy.rules)?;
    let parameters_json = serde_json::to_string(&strategy.parameters)?;
    let entry_strategy_ids_json = serde_json::to_string(&strategy.entry_strategy_ids)?;

    conn.execute(
        "INSERT INTO strategies (id, name, description, type, enabled, priority, timeframe, rules_json, parameters_json, created_at, updated_at, author, version, entry_strategy_ids_json, mode, rules_changed_at, rules_fingerprint)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?10, ?16)",
        params![
            strategy.id,
            strategy.name,
            strategy.description,
            strategy.strategy_type.to_string(),
            strategy.enabled,
            strategy.priority,
            strategy.timeframe,
            rules_json,
            parameters_json,
            strategy.created_at.to_rfc3339(),
            strategy.updated_at.to_rfc3339(),
            strategy.author,
            strategy.version,
            entry_strategy_ids_json,
            strategy.mode.as_str(),
            strategy.rules_fingerprint(),
        ],
    )?;
    Ok(())
}

/// Update an existing strategy. Its mode is not written: it changes only through
/// [`set_strategy_mode`]. Its rules clock moves to now, taken inside the write
/// transaction, only when its rules fingerprint changes; the `rules_changed_at` it
/// carries is ignored. Returns whether the rules changed.
pub fn update_strategy(strategy: &Strategy) -> crate::Result<bool> {
    let mut conn = get_connection()?;
    let rules_changed = update_strategy_in(&mut conn, strategy)?;

    logger::info(
        LogTag::System,
        &format!(
            "Updated strategy: id={}, name={}, rules_changed={rules_changed}",
            strategy.id, strategy.name
        ),
    );

    Ok(rules_changed)
}

fn update_strategy_in(conn: &mut rusqlite::Connection, strategy: &Strategy) -> crate::Result<bool> {
    let rules_json = serde_json::to_string(&strategy.rules)?;
    let parameters_json = serde_json::to_string(&strategy.parameters)?;
    let entry_strategy_ids_json = serde_json::to_string(&strategy.entry_strategy_ids)?;
    let fingerprint = strategy.rules_fingerprint();

    let tx = conn.write_tx()?;
    let Some(stored) = tx
        .query_row(
            "SELECT rules_fingerprint FROM strategies WHERE id = ?1",
            params![strategy.id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
    else {
        return Err(Error::Database(DatabaseError::Query {
            operation: "update_strategy".to_owned(),
            message: format!("Strategy not found: {}", strategy.id),
        }));
    };
    let rules_changed = stored.as_deref() != Some(fingerprint.as_str());
    let now = Utc::now();
    tx.execute(
        "UPDATE strategies
             SET name = ?2, description = ?3, type = ?4, enabled = ?5, priority = ?6,
                 timeframe = ?7, rules_json = ?8, parameters_json = ?9, updated_at = ?10, author = ?11, version = ?12,
                 entry_strategy_ids_json = ?13, rules_fingerprint = ?14,
                 rules_changed_at = CASE WHEN ?15 THEN ?16 ELSE rules_changed_at END
             WHERE id = ?1",
        params![
            strategy.id,
            strategy.name,
            strategy.description,
            strategy.strategy_type.to_string(),
            strategy.enabled,
            strategy.priority,
            strategy.timeframe,
            rules_json,
            parameters_json,
            strategy.updated_at.to_rfc3339(),
            strategy.author,
            strategy.version,
            entry_strategy_ids_json,
            fingerprint,
            rules_changed,
            now.to_rfc3339(),
        ],
    )?;
    tx.commit()?;
    Ok(rules_changed)
}

/// Set the paper or live mode of a strategy. The rules clock stays where it is: the mode
/// is not part of the rules.
pub fn set_strategy_mode(strategy_id: &str, mode: TradingMode) -> crate::Result<()> {
    let conn = get_connection()?;
    set_strategy_mode_in(&conn, strategy_id, mode)?;
    logger::info(
        LogTag::System,
        &format!(
            "Strategy mode set: id={strategy_id}, mode={}",
            mode.as_str()
        ),
    );
    Ok(())
}

fn set_strategy_mode_in(
    conn: &rusqlite::Connection,
    strategy_id: &str,
    mode: TradingMode,
) -> crate::Result<()> {
    let rows_affected = conn.execute(
        "UPDATE strategies SET mode = ?2, updated_at = ?3 WHERE id = ?1",
        params![strategy_id, mode.as_str(), Utc::now().to_rfc3339()],
    )?;
    if rows_affected == 0 {
        return Err(Error::Database(DatabaseError::Query {
            operation: "set_strategy_mode".to_owned(),
            message: format!("Strategy not found: {strategy_id}"),
        }));
    }
    Ok(())
}

/// Delete a strategy
pub fn delete_strategy(strategy_id: &str) -> crate::Result<()> {
    let conn = get_connection()?;

    let rows_affected =
        conn.execute("DELETE FROM strategies WHERE id = ?1", params![strategy_id])?;

    if rows_affected == 0 {
        return Err(Error::Database(DatabaseError::Query {
            operation: "delete_strategy".to_owned(),
            message: format!("Strategy not found: {strategy_id}"),
        }));
    }

    logger::info(
        LogTag::System,
        &format!("Deleted strategy: id={strategy_id}"),
    );

    Ok(())
}

/// Get a strategy by ID
pub fn get_strategy(strategy_id: &str) -> crate::Result<Option<Strategy>> {
    let conn = get_connection()?;

    let row = conn
        .query_row(
            &format!("SELECT {STRATEGY_COLUMNS} FROM strategies WHERE id = ?1"),
            params![strategy_id],
            read_strategy_row,
        )
        .optional()?;

    let Some(row) = row else {
        return Ok(None);
    };
    let type_str = row.type_str.clone();
    match row.into_strategy()? {
        Some(strategy) => Ok(Some(strategy)),
        None => Err(Error::Database(DatabaseError::Query {
            operation: "get_strategy".to_owned(),
            message: format!("Invalid strategy type: {type_str}"),
        })),
    }
}

/// Get all strategies
pub fn get_all_strategies() -> crate::Result<Vec<Strategy>> {
    let conn = get_connection()?;
    all_strategies(&conn)
}

/// Every strategy in `conn`, in [`STRATEGY_ORDER`].
fn all_strategies(conn: &rusqlite::Connection) -> crate::Result<Vec<Strategy>> {
    read_strategies(
        conn,
        &format!("SELECT {STRATEGY_COLUMNS} FROM strategies {STRATEGY_ORDER}"),
        [],
    )
}

/// The enabled strategies of each trading mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnabledStrategyCounts {
    pub live: usize,
    pub paper: usize,
}

/// Count the enabled strategies of each trading mode.
pub fn enabled_strategy_counts() -> crate::Result<EnabledStrategyCounts> {
    let conn = get_connection()?;
    let mut stmt =
        conn.prepare("SELECT mode, COUNT(*) FROM strategies WHERE enabled = 1 GROUP BY mode")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut counts = EnabledStrategyCounts::default();
    for row in rows {
        let (mode, count) = row?;
        let count = usize::try_from(count).unwrap_or_default();
        match TradingMode::parse(&mode) {
            Some(TradingMode::Live) => counts.live = count,
            Some(TradingMode::Paper) => counts.paper = count,
            None => return Err(Error::parse_error(format!("Unknown strategy mode: {mode}"))),
        }
    }
    Ok(counts)
}

/// Get enabled strategies by type
pub fn get_enabled_strategies(strategy_type: StrategyType) -> crate::Result<Vec<Strategy>> {
    let conn = get_connection()?;
    enabled_strategies(&conn, strategy_type)
}

/// The enabled strategies of `strategy_type` in `conn`, in [`STRATEGY_ORDER`].
fn enabled_strategies(
    conn: &rusqlite::Connection,
    strategy_type: StrategyType,
) -> crate::Result<Vec<Strategy>> {
    read_strategies(
        conn,
        &format!(
            "SELECT {STRATEGY_COLUMNS} FROM strategies WHERE type = ?1 AND enabled = 1 \
             {STRATEGY_ORDER}"
        ),
        params![strategy_type.to_string()],
    )
}

// =============================================================================
// EVALUATION TOTALS
// =============================================================================

/// One strategy evaluation as the store counts it, stamped when it was evaluated.
#[derive(Debug, Clone)]
pub struct EvaluationRecord {
    pub result: EvaluationResult,
    pub evaluated_at: DateTime<Utc>,
}

/// Add a batch of strategy evaluations to the evaluation totals in one transaction.
/// Synchronous: callers on the async runtime queue evaluations through
/// [`crate::strategies::recorder`], which runs this on the blocking pool.
pub fn record_evaluations(records: &[EvaluationRecord]) -> crate::Result<()> {
    let mut conn = get_connection()?;
    insert_evaluations(&mut conn, records)
}

/// Add `records` to the totals of their strategies, summed per strategy first, in one
/// transaction. An evaluation of a strategy deleted since is dropped alone; the rest of
/// the batch is counted.
fn insert_evaluations(
    conn: &mut rusqlite::Connection,
    records: &[EvaluationRecord],
) -> crate::Result<()> {
    let tx = conn.write_tx()?;
    for (strategy_id, totals) in batch_totals(records) {
        add_to_totals(&tx, strategy_id, &totals)?;
    }
    tx.commit()?;
    Ok(())
}

/// The totals of each strategy over `records`.
fn batch_totals(records: &[EvaluationRecord]) -> BTreeMap<&str, EvaluationTotals> {
    let mut totals: BTreeMap<&str, EvaluationTotals> = BTreeMap::new();
    for record in records {
        let strategy = totals
            .entry(record.result.strategy_id.as_str())
            .or_default();
        strategy.evaluations += 1;
        strategy.total_exec_ms += record.result.execution_time_ms;
        strategy.last_evaluated_at = strategy.last_evaluated_at.max(Some(record.evaluated_at));
        if record.result.result {
            strategy.signals += 1;
            strategy.last_signal_at = strategy.last_signal_at.max(Some(record.evaluated_at));
        }
    }
    totals
}

/// Add `totals` to the stored totals of `strategy_id`, or start them; nothing when the
/// strategy no longer exists. A last time only ever moves later.
fn add_to_totals(
    conn: &rusqlite::Connection,
    strategy_id: &str,
    totals: &EvaluationTotals,
) -> rusqlite::Result<()> {
    let signed = |count: u64| i64::try_from(count).unwrap_or(i64::MAX);
    conn.prepare_cached(
        "INSERT INTO strategy_evaluation_totals
             (strategy_id, evaluations, signals, total_exec_ms, last_evaluated_at, last_signal_at)
         SELECT ?1, ?2, ?3, ?4, ?5, ?6 WHERE EXISTS (SELECT 1 FROM strategies WHERE id = ?1)
         ON CONFLICT(strategy_id) DO UPDATE SET
             evaluations = evaluations + excluded.evaluations,
             signals = signals + excluded.signals,
             total_exec_ms = total_exec_ms + excluded.total_exec_ms,
             last_evaluated_at = CASE WHEN excluded.last_evaluated_at IS NULL THEN last_evaluated_at
                 ELSE max(coalesce(last_evaluated_at, ''), excluded.last_evaluated_at) END,
             last_signal_at = CASE WHEN excluded.last_signal_at IS NULL THEN last_signal_at
                 ELSE max(coalesce(last_signal_at, ''), excluded.last_signal_at) END",
    )?
    .execute(params![
        strategy_id,
        signed(totals.evaluations),
        signed(totals.signals),
        signed(totals.total_exec_ms),
        totals.last_evaluated_at.map(totals_time),
        totals.last_signal_at.map(totals_time),
    ])?;
    Ok(())
}

/// A time as the totals store it: UTC with milliseconds and a fixed width, so the text
/// order of two stored times is their time order.
fn totals_time(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// The evaluation totals of every strategy that was evaluated, by strategy id.
fn evaluation_totals(
    conn: &rusqlite::Connection,
) -> crate::Result<HashMap<String, EvaluationTotals>> {
    let mut stmt = conn.prepare(
        "SELECT strategy_id, evaluations, signals, total_exec_ms, last_evaluated_at, last_signal_at
         FROM strategy_evaluation_totals",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let parse_time = |value: Option<String>, strategy_id: &str| {
        value
            .map(|value| {
                DateTime::parse_from_rfc3339(&value)
                    .map(|time| time.with_timezone(&Utc))
                    .map_err(|e| {
                        Error::parse_error(format!(
                            "Failed to parse an evaluation time for {strategy_id}: {e}"
                        ))
                    })
            })
            .transpose()
    };
    let mut totals = HashMap::with_capacity(rows.len());
    for (strategy_id, evaluations, signals, total_exec_ms, last_evaluated, last_signal) in rows {
        let strategy_totals = EvaluationTotals {
            evaluations: count_of(evaluations),
            signals: count_of(signals),
            total_exec_ms: count_of(total_exec_ms),
            last_evaluated_at: parse_time(last_evaluated, &strategy_id)?,
            last_signal_at: parse_time(last_signal, &strategy_id)?,
        };
        totals.insert(strategy_id, strategy_totals);
    }
    Ok(totals)
}

/// Every strategy, in [`STRATEGY_ORDER`], with the evaluation totals of each strategy that
/// was evaluated: two statements on one connection, whatever the number of strategies.
pub fn get_strategies_with_totals() -> crate::Result<StrategiesWithTotals> {
    let conn = get_connection()?;
    Ok(StrategiesWithTotals {
        strategies: all_strategies(&conn)?,
        totals: evaluation_totals(&conn)?,
    })
}

/// Every strategy and the evaluation totals of each, read together.
#[derive(Debug, Clone, Default)]
pub struct StrategiesWithTotals {
    /// In [`STRATEGY_ORDER`].
    pub strategies: Vec<Strategy>,
    /// By strategy id; a strategy never evaluated has none.
    pub totals: HashMap<String, EvaluationTotals>,
}

#[cfg(test)]
#[path = "database_tests.rs"]
mod tests;
