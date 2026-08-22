//! Tokens schema upgrade mechanics — ordered steps run by
//! `schema::initialize_schema` separately from fresh `CREATE TABLE IF NOT EXISTS`.
//!
//! Two independent, structurally gated steps:
//! 1. `migrate_legacy_schema` — one-time rebuild of every token table onto the
//!    chain-scoped primary key (pre-chain-identity databases only).
//! 2. `apply_additive_migrations` — ordered additive columns for databases that
//!    already exist. `CREATE TABLE IF NOT EXISTS` never widens a live table, so a
//!    later nullable/defaulted column must be listed here as well as in
//!    `CREATE_TABLES`. Presence of the column, not `user_version`, decides whether
//!    the step runs.
//!
//! A NOT NULL column with no DEFAULT cannot go through `ALTER TABLE ADD COLUMN`
//! on a non-empty table; SQLite rejects that shape. Additive steps must be
//! nullable or carry a `DEFAULT`.

use rusqlite::{Connection, OptionalExtension, Transaction};

pub(super) const TOKEN_TABLES: &[&str] = &[
    "tokens",
    "market_dexscreener",
    "market_geckoterminal",
    "token_pools",
    "security_rugcheck",
    "blacklist",
    "update_tracking",
    "token_favorites",
    "rejection_history",
    "rejection_stats",
    "authority_reputation",
];

/// One additive column. Applied in array order when the table exists and the
/// named column is absent.
struct AdditiveColumn {
    table: &'static str,
    column: &'static str,
    definition: &'static str,
}

/// Ordered additive repairs. Append a step here (and the matching column in
/// `CREATE_TABLES`) when widening an already-versioned tokens database.
const ADDITIVE_COLUMNS: &[AdditiveColumn] = &[AdditiveColumn {
    table: "token_favorites",
    column: "notes",
    definition: "notes TEXT",
}];

pub(super) fn table_needs_chain_rebuild(conn: &Connection) -> Result<bool, String> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tokens'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| format!("Failed to inspect tokens schema: {error}"))?
        .is_some();
    if !exists {
        return Ok(false);
    }
    Ok(!table_columns(conn, "tokens")?
        .iter()
        .any(|column| column == "chain_id"))
}

pub(super) fn migrate_legacy_schema(conn: &Connection) -> Result<(), String> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| format!("Failed to begin tokens chain migration: {error}"))?;
    transaction
        .execute_batch("PRAGMA defer_foreign_keys = ON")
        .map_err(|error| format!("Failed to defer tokens foreign keys for migration: {error}"))?;

    let legacy_indexes = TOKEN_TABLES
        .iter()
        .map(|table| -> Result<Vec<String>, String> {
            let mut statement = transaction
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'index' AND tbl_name = ?1 AND sql IS NOT NULL",
                )
                .map_err(|error| {
                    format!("Failed to inspect legacy tokens indexes for {table}: {error}")
                })?;
            let indexes = statement
                .query_map([table], |row| row.get::<_, String>(0))
                .map_err(|error| {
                    format!("Failed to read legacy tokens indexes for {table}: {error}")
                })?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    format!("Failed to decode legacy tokens indexes for {table}: {error}")
                })?;
            Ok(indexes)
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    for index in legacy_indexes {
        // SQLite does not bind identifiers. Metadata supplies the name, and quoting it prevents a
        // malformed legacy index name from changing the migration statement.
        let quoted_index = format!("\"{}\"", index.replace('"', "\"\""));
        transaction
            .execute(&format!("DROP INDEX {quoted_index}"), [])
            .map_err(|error| format!("Failed to drop legacy tokens index {index}: {error}"))?;
    }
    for table in TOKEN_TABLES {
        transaction
            .execute(
                &format!("ALTER TABLE {table} RENAME TO {table}_legacy_chain"),
                [],
            )
            .map_err(|error| format!("Failed to stage legacy tokens table {table}: {error}"))?;
    }
    for statement in super::schema::CREATE_TABLES {
        transaction
            .execute(statement, [])
            .map_err(|error| format!("Failed to create chain-scoped tokens table: {error}"))?;
    }
    for table in TOKEN_TABLES {
        copy_legacy_rows(&transaction, table)?;
    }
    for table in TOKEN_TABLES {
        let legacy = format!("{table}_legacy_chain");
        let old_count: i64 = transaction
            .query_row(&format!("SELECT COUNT(*) FROM {legacy}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Failed to count legacy {table} rows: {error}"))?;
        let new_count: i64 = transaction
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Failed to count migrated {table} rows: {error}"))?;
        if old_count != new_count {
            return Err(format!(
                "Tokens chain migration changed {table} row count: {old_count} -> {new_count}"
            ));
        }
        transaction
            .execute(&format!("DROP TABLE {legacy}"), [])
            .map_err(|error| format!("Failed to remove staged legacy table {legacy}: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("Failed to commit tokens chain migration: {error}"))?;
    if conn
        .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()
        .map_err(|error| format!("Failed to validate tokens foreign keys: {error}"))?
        .is_some()
    {
        return Err("Tokens chain migration failed foreign-key validation".to_owned());
    }
    Ok(())
}

/// Apply every missing additive column. Structural presence is the gate: a database
/// already stamped at `SCHEMA_VERSION` still receives a column the live schema
/// requires. No-ops commit cleanly so repeated initialization is idempotent.
pub(super) fn apply_additive_migrations(conn: &Connection) -> Result<(), String> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| format!("Failed to begin tokens additive migration: {error}"))?;

    for step in ADDITIVE_COLUMNS {
        if !table_exists(&transaction, step.table)? {
            continue;
        }
        if table_columns(&transaction, step.table)?
            .iter()
            .any(|column| column == step.column)
        {
            continue;
        }
        transaction
            .execute(
                &format!("ALTER TABLE {} ADD COLUMN {}", step.table, step.definition),
                [],
            )
            .map_err(|error| {
                format!(
                    "Failed to add column {} to {}: {error}",
                    step.column, step.table
                )
            })?;
    }

    transaction
        .commit()
        .map_err(|error| format!("Failed to commit tokens additive migration: {error}"))
}

fn copy_legacy_rows(transaction: &Transaction<'_>, table: &str) -> Result<(), String> {
    let legacy = format!("{table}_legacy_chain");
    let columns = |name: &str| -> Result<Vec<String>, String> {
        let mut statement = transaction
            .prepare(&format!("PRAGMA table_info({name})"))
            .map_err(|error| format!("Failed to inspect {name}: {error}"))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| format!("Failed to read {name} columns: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Failed to decode {name} columns: {error}"))
    };
    let old = columns(&legacy)?;
    let new = columns(table)?;
    let mut shared = new
        .into_iter()
        .filter(|column| column != "chain_id" && old.contains(column))
        .collect::<Vec<_>>();
    if table == "market_dexscreener" {
        shared.retain(|column| column != "provider_chain_id");
    }
    let mut target = vec!["chain_id".to_owned()];
    let mut source = vec!["'solana'".to_owned()];
    if table == "market_dexscreener" {
        target.push("provider_chain_id".to_owned());
        source.push(if old.contains(&"chain_id".to_owned()) {
            "chain_id".to_owned()
        } else {
            "NULL".to_owned()
        });
    }
    target.extend(shared.iter().cloned());
    source.extend(shared);
    transaction
        .execute(
            &format!(
                "INSERT INTO {table} ({}) SELECT {} FROM {legacy}",
                target.join(", "),
                source.join(", ")
            ),
            [],
        )
        .map_err(|error| format!("Failed to copy legacy {table} rows: {error}"))?;
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| format!("Failed to inspect {table} columns: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("Failed to read {table} columns: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to decode {table} columns: {error}"))?;
    Ok(columns)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map_err(|error| format!("Failed to inspect table {table}: {error}"))
    .map(|row| row.is_some())
}
