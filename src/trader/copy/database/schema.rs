use rusqlite::{Connection, OptionalExtension};

use crate::trader::copy::types::CopyOutcome;
use crate::trader::error::Error;

pub(super) const SCHEMA_VERSION: i64 = 7;

pub(super) const SCHEMA: &str = r#"
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
    max_sol_per_trade REAL NOT NULL,
    max_sol_per_token REAL NOT NULL,
    total_budget_sol REAL NOT NULL,
    min_target_trade_sol REAL,
    max_target_trade_sol REAL,
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
    spent_sol REAL NOT NULL DEFAULT 0,
    buy_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, mode, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_paper_positions (
    task_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    token_amount REAL NOT NULL DEFAULT 0,
    cost_basis_sol REAL NOT NULL DEFAULT 0,
    invested_sol REAL NOT NULL DEFAULT 0,
    realized_proceeds_sol REAL NOT NULL DEFAULT 0,
    realized_cost_sol REAL NOT NULL DEFAULT 0,
    buys INTEGER NOT NULL DEFAULT 0,
    sells INTEGER NOT NULL DEFAULT 0,
    last_price_sol REAL,
    last_price_at TEXT,
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    updated_at TEXT NOT NULL,
    peak_price_sol REAL,
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

pub(super) fn migrate(connection: &Connection) -> crate::trader::Result<()> {
    let mut statement = connection
        .prepare("PRAGMA table_info(copy_tasks)")
        .map_err(crate::errors::DatabaseError::from)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(crate::errors::DatabaseError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::errors::DatabaseError::from)?;
    if !columns.iter().any(|column| column == "exit_policy_json") {
        connection
            .execute(
                "ALTER TABLE copy_tasks ADD COLUMN exit_policy_json TEXT NOT NULL DEFAULT '{}'",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    let mut statement = connection
        .prepare("PRAGMA table_info(copy_live_claims)")
        .map_err(crate::errors::DatabaseError::from)?;
    let claim_columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(crate::errors::DatabaseError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::errors::DatabaseError::from)?;
    if !claim_columns.iter().any(|column| column == "state") {
        connection
            .execute(
                "ALTER TABLE copy_live_claims ADD COLUMN state TEXT NOT NULL DEFAULT 'claimed'",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    if !claim_columns.iter().any(|column| column == "updated_at") {
        connection
            .execute(
                "ALTER TABLE copy_live_claims ADD COLUMN updated_at TEXT NOT NULL DEFAULT ''",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        connection
            .execute(
                "UPDATE copy_live_claims SET updated_at = claimed_at WHERE updated_at = ''",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    let mut statement = connection
        .prepare("PRAGMA table_info(copy_tasks)")
        .map_err(crate::errors::DatabaseError::from)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(crate::errors::DatabaseError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::errors::DatabaseError::from)?;
    drop(statement);
    if !columns.iter().any(|column| column == "chain_id") {
        let transaction = connection
            .unchecked_transaction()
            .map_err(crate::errors::DatabaseError::from)?;
        transaction
            .execute(
                "ALTER TABLE copy_tasks ADD COLUMN chain_id TEXT NOT NULL DEFAULT 'solana'",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
        let invalid = transaction
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(1_i64))
            .optional()
            .map_err(crate::errors::DatabaseError::from)?
            .unwrap_or(0);
        if invalid != 0 {
            return Err(Error::CopyValidation {
                detail: "chain migration failed foreign-key validation".to_owned(),
            });
        }
        transaction
            .commit()
            .map_err(crate::errors::DatabaseError::from)?;
    }
    // v6: the paper book tracks each round's peak for the trailing stop. Added
    // before the v5 rebuild, which books paper fills through `apply_paper_buy`.
    if !table_columns(connection, "copy_paper_positions")?
        .iter()
        .any(|column| column == "peak_price_sol")
    {
        connection
            .execute(
                "ALTER TABLE copy_paper_positions ADD COLUMN peak_price_sol REAL",
                [],
            )
            .map_err(crate::errors::DatabaseError::from)?;
    }
    // v7: the per-task filter override and the stored reason for a pause.
    let task_columns = table_columns(connection, "copy_tasks")?;
    for (column, definition) in [
        ("require_filter_pass", "INTEGER"),
        ("pause_reason_json", "TEXT"),
        ("paused_at", "TEXT"),
    ] {
        if !task_columns.iter().any(|existing| existing == column) {
            connection
                .execute(
                    &format!("ALTER TABLE copy_tasks ADD COLUMN {column} {definition}"),
                    [],
                )
                .map_err(crate::errors::DatabaseError::from)?;
        }
    }
    migrate_mode_scoped_spend(connection)?;
    Ok(())
}

fn table_columns(connection: &Connection, table: &str) -> crate::trader::Result<Vec<String>> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(crate::errors::DatabaseError::from)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(crate::errors::DatabaseError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(columns)
}

/// v5: spend was one ledger shared by paper and live, so paper fills consumed the
/// live budget and a task armed for live started half spent. Rebuild it keyed by
/// mode from the recorded decisions (the source of every spend increment); spend no
/// decision explains is kept under the task's current mode rather than dropped.
/// Every recorded paper fill is booked into the new paper position ledger.
fn migrate_mode_scoped_spend(connection: &Connection) -> crate::trader::Result<()> {
    if table_columns(connection, "copy_spend")?
        .iter()
        .any(|column| column == "mode")
    {
        return Ok(());
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(crate::errors::DatabaseError::from)?;
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
            super::ledger::apply_paper_buy(&transaction, &decision)?;
        }
    }
    transaction
        .commit()
        .map_err(crate::errors::DatabaseError::from)?;
    Ok(())
}
