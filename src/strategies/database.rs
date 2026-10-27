// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Strategy database — persistence for custom strategy templates.

use crate::database;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::{DatabaseError, Error};
use crate::logger::{self, LogTag};
use crate::strategies::types::{EvaluationResult, Strategy, StrategyPerformance, StrategyType};
use chrono::{DateTime, Utc};
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension, Transaction};

/// The version the store's own `schema_version` table records once its tables exist.
const STRATEGIES_SCHEMA_VERSION: u32 = 1;

/// The store version this core writes. Version 2 drops `strategy_assignments`, which
/// nothing read or wrote (an automatic position records its strategy in its origin).
/// Version 3 adds `strategies.entry_strategy_ids_json`, the entry strategies an exit
/// strategy manages; its default `'[]'` (every position) is what a v2 core does.
const STORE_VERSION: i64 = 3;

/// The oldest core that reads the store: the v2 drop is breaking, the v3 column is not
/// (an older core neither reads it nor writes it, and its rows keep the default).
const MIN_READER_VERSION: i64 = 2;

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
    entry_strategy_ids_json TEXT NOT NULL DEFAULT '[]' -- exit strategies: the entry strategies managed, empty = all
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
    ddl::run_batch(tx, "drop retired strategies tables", DROP_RETIRED_TABLES)
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
     rules_json, parameters_json, created_at, updated_at, author, version, entry_strategy_ids_json";

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
        let created_at = parse_time(&self.created_at, "created_at")?;
        let updated_at = parse_time(&self.updated_at, "updated_at")?;
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

/// Insert a new strategy
pub fn insert_strategy(strategy: &Strategy) -> crate::Result<()> {
    let conn = get_connection()?;

    let rules_json = serde_json::to_string(&strategy.rules)?;
    let parameters_json = serde_json::to_string(&strategy.parameters)?;
    let entry_strategy_ids_json = serde_json::to_string(&strategy.entry_strategy_ids)?;

    conn.execute(
        "INSERT INTO strategies (id, name, description, type, enabled, priority, timeframe, rules_json, parameters_json, created_at, updated_at, author, version, entry_strategy_ids_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
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
        ],
    )?;

    logger::info(
        LogTag::System,
        &format!(
            "Inserted strategy: id={}, name={}, type={}",
            strategy.id, strategy.name, strategy.strategy_type
        ),
    );

    Ok(())
}

/// Update an existing strategy
pub fn update_strategy(strategy: &Strategy) -> crate::Result<()> {
    let conn = get_connection()?;

    let rules_json = serde_json::to_string(&strategy.rules)?;
    let parameters_json = serde_json::to_string(&strategy.parameters)?;
    let entry_strategy_ids_json = serde_json::to_string(&strategy.entry_strategy_ids)?;

    let rows_affected = conn.execute(
        "UPDATE strategies 
             SET name = ?2, description = ?3, type = ?4, enabled = ?5, priority = ?6, 
                 timeframe = ?7, rules_json = ?8, parameters_json = ?9, updated_at = ?10, author = ?11, version = ?12,
                 entry_strategy_ids_json = ?13
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
        ],
    )?;

    if rows_affected == 0 {
        return Err(Error::Database(DatabaseError::Query {
            operation: "update_strategy".to_owned(),
            message: format!("Strategy not found: {}", strategy.id),
        }));
    }

    logger::info(
        LogTag::System,
        &format!(
            "Updated strategy: id={}, name={}",
            strategy.id, strategy.name
        ),
    );

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
    read_strategies(
        &conn,
        &format!("SELECT {STRATEGY_COLUMNS} FROM strategies ORDER BY priority ASC, name ASC"),
        [],
    )
}

/// Check if any enabled strategies exist for a given type (lightweight check)
pub fn has_enabled_strategies(strategy_type: StrategyType) -> crate::Result<bool> {
    let conn = get_connection()?;

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM strategies WHERE type = ?1 AND enabled = 1",
        params![strategy_type.to_string()],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Get enabled strategies by type
pub fn get_enabled_strategies(strategy_type: StrategyType) -> crate::Result<Vec<Strategy>> {
    let conn = get_connection()?;
    read_strategies(
        &conn,
        &format!(
            "SELECT {STRATEGY_COLUMNS} FROM strategies WHERE type = ?1 AND enabled = 1 \
             ORDER BY priority ASC, name ASC"
        ),
        params![strategy_type.to_string()],
    )
}

// =============================================================================
// PERFORMANCE TRACKING
// =============================================================================

/// Record strategy evaluation result
pub fn record_evaluation(result: &EvaluationResult, token_mint: &str) -> crate::Result<()> {
    let conn = get_connection()?;

    let details_json = serde_json::to_string(&result.details)?;

    conn.execute(
        "INSERT INTO strategy_performance (strategy_id, execution_time_ms, result, confidence, details_json, token_mint, execution_timestamp, chain_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            result.strategy_id,
            result.execution_time_ms,
            result.result,
            result.confidence,
            details_json,
            token_mint,
            Utc::now().to_rfc3339(),
            crate::chains::legacy_row_chain().as_str(),
        ],
    )?;

    Ok(())
}

/// Get performance statistics for a strategy
pub fn get_strategy_performance(strategy_id: &str) -> crate::Result<Option<StrategyPerformance>> {
    let conn = get_connection()?;

    let result = conn
        .query_row(
            "SELECT 
                COUNT(*) as total_evaluations,
                SUM(CASE WHEN result = 1 THEN 1 ELSE 0 END) as successful_signals,
                AVG(execution_time_ms) as avg_execution_time_ms,
                MAX(execution_timestamp) as last_evaluation
             FROM strategy_performance
             WHERE strategy_id = ?1",
            params![strategy_id],
            |row| {
                let total: u64 = row.get(0)?;
                let successful: u64 = row.get(1)?;
                let avg_time: f64 = row.get(2)?;
                let last_eval_str: String = row.get(3)?;
                Ok((total, successful, avg_time, last_eval_str))
            },
        )
        .optional()?;

    match result {
        Some((total_evaluations, successful_signals, avg_execution_time_ms, last_eval_str)) => {
            if total_evaluations == 0 {
                return Ok(None);
            }

            let last_evaluation = DateTime::parse_from_rfc3339(&last_eval_str)
                .map_err(|e| Error::parse_error(format!("Failed to parse timestamp: {e}")))?
                .with_timezone(&Utc);

            Ok(Some(StrategyPerformance {
                strategy_id: strategy_id.to_string(),
                total_evaluations,
                successful_signals,
                avg_execution_time_ms,
                last_evaluation,
            }))
        }
        None => Ok(None),
    }
}
