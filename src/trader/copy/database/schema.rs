// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Schema and migrations for the copy-trading database - copy_tasks, copy_spend and copy_paper_positions tables plus the mode-scoped spend rebuild.

use rusqlite::{params, OptionalExtension, Transaction};

use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::DatabaseError;
use crate::trader::copy::types::CopyOutcome;
use crate::trader::error::Error;

const SCHEMA_VERSION: i64 = 7;

/// The schema of `copy_trading.db`.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: SCHEMA_VERSION,
    min_reader_version: SCHEMA_VERSION,
    baseline: SCHEMA_VERSION,
    renames: Some(&super::column_names::RENAMES),
    step,
    legacy_stamp: None,
};

fn step(transaction: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    create_schema(transaction).map_err(|error| match error {
        Error::Database(error) => error,
        other => DatabaseError::Migration {
            store: "copy_trading.db".to_owned(),
            detail: other.to_string(),
        },
    })
}

/// Creates the tables, runs every migration and stamps the schema version.
fn create_schema(transaction: &Transaction<'_>) -> crate::trader::Result<()> {
    ddl::run_batch(transaction, "create copy-trading tables", SCHEMA_SQL)?;
    migrate(transaction)?;
    transaction
        .execute(
            "INSERT INTO copy_metadata (key, value) VALUES ('schema_version', ?1) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![SCHEMA_VERSION.to_string()],
        )
        .map_err(|e| DatabaseError::classify_sqlite_failure("stamp copy-trading schema", e))?;
    Ok(())
}

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS copy_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS copy_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    target_address TEXT NOT NULL,
    label TEXT,
    enabled INTEGER NOT NULL,
    mode_json TEXT NOT NULL,
    sizing_json TEXT NOT NULL,
    exit_mode_json TEXT NOT NULL,
    exit_policy_json TEXT NOT NULL DEFAULT '{}',
    max_native_per_trade REAL NOT NULL,
    max_native_per_token REAL NOT NULL,
    total_budget_native REAL NOT NULL,
    min_target_trade_native REAL,
    max_target_trade_native REAL,
    buy_once_per_token INTEGER NOT NULL,
    slippage_pct REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    require_filter_pass INTEGER,
    pause_reason_json TEXT,
    paused_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_copy_tasks_target_enabled
    ON copy_tasks(target_address, enabled);
CREATE TABLE IF NOT EXISTS copy_spend (
    task_id INTEGER NOT NULL,
    mode TEXT NOT NULL,
    mint TEXT NOT NULL,
    spent_native REAL NOT NULL DEFAULT 0,
    buy_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, mode, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_paper_positions (
    task_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    token_amount REAL NOT NULL DEFAULT 0,
    cost_basis_native REAL NOT NULL DEFAULT 0,
    invested_native REAL NOT NULL DEFAULT 0,
    realized_proceeds_native REAL NOT NULL DEFAULT 0,
    realized_cost_native REAL NOT NULL DEFAULT 0,
    buys INTEGER NOT NULL DEFAULT 0,
    sells INTEGER NOT NULL DEFAULT 0,
    last_price_native REAL,
    last_price_at TEXT,
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    updated_at TEXT NOT NULL,
    peak_price_native REAL,
    PRIMARY KEY (task_id, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    signature TEXT NOT NULL,
    mint TEXT,
    outcome_json TEXT NOT NULL,
    decided_at TEXT NOT NULL,
    UNIQUE (task_id, signature),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_live_claims (
    task_id INTEGER NOT NULL,
    signature TEXT NOT NULL,
    claimed_at TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'claimed',
    updated_at TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (task_id, signature),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_target_holdings (
    task_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    token_amount REAL NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_target_events (
    task_id INTEGER NOT NULL,
    signature TEXT NOT NULL,
    mint TEXT NOT NULL,
    token_delta REAL NOT NULL,
    observed_at TEXT NOT NULL,
    PRIMARY KEY (task_id, signature),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_copy_decisions_task_time
    ON copy_decisions(task_id, decided_at DESC);
CREATE TABLE IF NOT EXISTS copy_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    details_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_copy_activity_task_time
    ON copy_activity(task_id, created_at DESC);
CREATE TABLE IF NOT EXISTS copy_position_links (
    task_id INTEGER NOT NULL,
    position_id TEXT NOT NULL UNIQUE,
    mint TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, position_id),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
"#;

/// Every schema step of one open. Runs inside the opener's transaction, so a
/// step refused here leaves the stored schema exactly as it was.
fn migrate(connection: &Transaction<'_>) -> crate::trader::Result<()> {
    ddl::add_column(
        connection,
        "copy_tasks",
        "exit_policy_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    ddl::add_column(
        connection,
        "copy_live_claims",
        "state",
        "TEXT NOT NULL DEFAULT 'claimed'",
    )?;
    if ddl::add_column(
        connection,
        "copy_live_claims",
        "updated_at",
        "TEXT NOT NULL DEFAULT ''",
    )? {
        connection
            .execute(
                "UPDATE copy_live_claims SET updated_at = claimed_at WHERE updated_at = ''",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    if ddl::add_column(
        connection,
        "copy_tasks",
        "chain_id",
        "TEXT NOT NULL DEFAULT 'solana'",
    )? {
        let invalid = connection
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(1_i64))
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .unwrap_or(0);
        if invalid != 0 {
            return Err(Error::CopyValidation {
                detail: "chain migration failed foreign-key validation".to_owned(),
            });
        }
    }
    // v6: the paper book tracks each round's peak for the trailing stop. Added
    // before the v5 rebuild, which books paper fills through `apply_paper_buy`.
    // A legacy book may still carry the pre-rename column, which the unit
    // rename below moves to the canonical name.
    if !ddl::column_names(connection, "copy_paper_positions")?
        .iter()
        .any(|column| column == "peak_price_native" || column == "peak_price_sol")
    {
        connection
            .execute(
                "ALTER TABLE copy_paper_positions ADD COLUMN peak_price_native REAL",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    // v7: the per-task filter override and the stored reason for a pause.
    for (column, definition) in [
        ("require_filter_pass", "INTEGER"),
        ("pause_reason_json", "TEXT"),
        ("paused_at", "TEXT"),
    ] {
        ddl::add_column(connection, "copy_tasks", column, definition)?;
    }
    migrate_mode_scoped_spend(connection)?;
    // Unit-neutral column names come last, so every step above reads the
    // historical shape it was written for.
    crate::database::schema::renames::rename(connection, &super::column_names::RENAMES)?;
    Ok(())
}

/// v5: spend was one ledger shared by paper and live, so paper fills consumed the
/// live budget and a task armed for live started half spent. Rebuild it keyed by
/// mode from the recorded decisions (the source of every spend increment); spend no
/// decision explains is kept under the task's current mode rather than dropped.
/// Every recorded paper fill is booked into the new paper position ledger.
fn migrate_mode_scoped_spend(transaction: &Transaction<'_>) -> crate::trader::Result<()> {
    if ddl::column_names(transaction, "copy_spend")?
        .iter()
        .any(|column| column == "mode")
    {
        return Ok(());
    }
    transaction
        .execute_batch(
            "CREATE TABLE copy_spend_v5 (
                task_id INTEGER NOT NULL,
                mode TEXT NOT NULL,
                mint TEXT NOT NULL,
                spent_sol REAL NOT NULL DEFAULT 0,
                buy_count INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (task_id, mode, mint),
                FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
            );
            INSERT INTO copy_spend_v5 (task_id, mode, mint, spent_sol, buy_count, updated_at)
                SELECT task_id,
                       CASE json_extract(outcome_json, '$.outcome') WHEN 'paper_filled' THEN 'paper' ELSE 'live' END,
                       mint,
                       SUM(json_extract(outcome_json, '$.sized_sol')),
                       COUNT(*),
                       MAX(decided_at)
                FROM copy_decisions
                WHERE mint IS NOT NULL
                  AND json_extract(outcome_json, '$.outcome') IN ('paper_filled', 'live_submitted', 'live_confirmed')
                GROUP BY 1, 2, 3;
            INSERT INTO copy_spend_v5 (task_id, mode, mint, spent_sol, buy_count, updated_at)
                SELECT s.task_id,
                       CASE json_extract(t.mode_json, '$') WHEN 'live' THEN 'live' ELSE 'paper' END,
                       s.mint, s.spent_sol, s.buy_count, s.updated_at
                FROM copy_spend s JOIN copy_tasks t ON t.id = s.task_id
                WHERE NOT EXISTS (
                    SELECT 1 FROM copy_spend_v5 v WHERE v.task_id = s.task_id AND v.mint = s.mint
                );
            DROP TABLE copy_spend;
            ALTER TABLE copy_spend_v5 RENAME TO copy_spend;",
        )
        .map_err(crate::errors::DatabaseError::from)?;
    let fills = {
        let mut statement = transaction
            .prepare(
                "SELECT outcome_json FROM copy_decisions \
                 WHERE json_extract(outcome_json, '$.outcome') = 'paper_filled' ORDER BY id",
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(crate::errors::DatabaseError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::errors::DatabaseError::from)?;
        rows
    };
    for json in fills {
        if let Ok(CopyOutcome::PaperFilled(decision)) = serde_json::from_str(&json) {
            super::ledger::apply_paper_buy(transaction, &decision)?;
        }
    }
    Ok(())
}
