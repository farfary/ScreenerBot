// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tokens schema upgrade mechanics — ordered steps run by
//! `schema::initialize_schema` separately from fresh `CREATE TABLE IF NOT EXISTS`.
//!
//! Three independent, structurally gated steps:
//! 1. `migrate_legacy_schema` — one-time rebuild of every token table onto the
//!    chain-scoped primary key (pre-chain-identity databases only).
//! 2. `apply_additive_migrations` — ordered additive columns for databases that
//!    already exist. `CREATE TABLE IF NOT EXISTS` never widens a live table, so a
//!    later nullable/defaulted column must be listed here as well as in
//!    `CREATE_TABLES`. Presence of the column, not `user_version`, decides whether
//!    the step runs.
//! 3. `drop_retired_indexes` — drops indexes no query uses any more, on every open.
//!
//! A NOT NULL column with no DEFAULT cannot go through `ALTER TABLE ADD COLUMN`
//! on a non-empty table; SQLite rejects that shape. Additive steps must be
//! nullable or carry a `DEFAULT`.

use crate::database::schema::ddl;
use crate::errors::DatabaseError;
use crate::tokens::Error;
use rusqlite::{Connection, OptionalExtension};

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
    "token_media",
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
const ADDITIVE_COLUMNS: &[AdditiveColumn] = &[
    AdditiveColumn {
        table: "token_favorites",
        column: "notes",
        definition: "notes TEXT",
    },
    AdditiveColumn {
        table: "update_tracking",
        column: "filter_evaluated_at",
        definition: "filter_evaluated_at INTEGER",
    },
];

/// Indexes no query uses any more. An older core still creates them on open, so every
/// open of this core drops them again rather than only the open that upgraded the store.
const RETIRED_INDEXES: &[&str] = &["idx_tracking_active_priority"];

pub(super) fn table_needs_chain_rebuild(conn: &Connection) -> Result<bool, Error> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'tokens'",
            [],
            |_| Ok(()),
        )
        .optional()
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to inspect tokens schema"),
            message: error.to_string(),
        })?
        .is_some();
    if !exists {
        return Ok(false);
    }
    Ok(!ddl::column_names(conn, "tokens")?
        .iter()
        .any(|column| column == "chain_id"))
}

/// Runs inside the opener's transaction, so a refused or failed rebuild leaves
/// the stored schema as it was.
pub(super) fn migrate_legacy_schema(transaction: &Connection) -> Result<(), Error> {
    transaction
        .execute_batch("PRAGMA defer_foreign_keys = ON")
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to defer tokens foreign keys for migration"),
            message: error.to_string(),
        })?;

    let legacy_indexes = TOKEN_TABLES
        .iter()
        .map(|table| -> Result<Vec<String>, Error> {
            let mut statement = transaction
                .prepare(
                    "SELECT name FROM sqlite_master \
                     WHERE type = 'index' AND tbl_name = ?1 AND sql IS NOT NULL",
                )
                .map_err(|error| DatabaseError::Query {
                    operation: format!("inspect legacy tokens indexes for {table}"),
                    message: error.to_string(),
                })?;
            let indexes = statement
                .query_map([table], |row| row.get::<_, String>(0))
                .map_err(|error| DatabaseError::Query {
                    operation: format!("read legacy tokens indexes for {table}"),
                    message: error.to_string(),
                })?
                .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
                .map_err(|error| DatabaseError::Query {
                    operation: format!("decode legacy tokens indexes for {table}"),
                    message: error.to_string(),
                })?;
            Ok(indexes)
        })
        .collect::<Result<Vec<_>, Error>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    for index in legacy_indexes {
        // SQLite does not bind identifiers. Metadata supplies the name, and quoting it prevents a
        // malformed legacy index name from changing the migration statement.
        let quoted_index = format!("\"{}\"", index.replace('"', "\"\""));
        transaction
            .execute(&format!("DROP INDEX {quoted_index}"), [])
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to drop legacy tokens index {index}"),
                message: error.to_string(),
            })?;
    }
    for table in TOKEN_TABLES {
        transaction
            .execute(
                &format!("ALTER TABLE {table} RENAME TO {table}_legacy_chain"),
                [],
            )
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to stage legacy tokens table {table}"),
                message: error.to_string(),
            })?;
    }
    for statement in super::schema::CREATE_TABLES {
        transaction
            .execute(statement, [])
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to create chain-scoped tokens table"),
                message: error.to_string(),
            })?;
    }
    for table in TOKEN_TABLES {
        copy_legacy_rows(&transaction, table)?;
    }
    // Verification must complete before ANY staged table is dropped, and the
    // teardown must run children-first. `ALTER TABLE ... RENAME` rewrites a
    // child's `REFERENCES tokens(...)` onto `tokens_legacy_chain`, and `DROP
    // TABLE` fires that foreign key's `ON DELETE CASCADE` action (which
    // `defer_foreign_keys` does not suppress — it defers constraint checking,
    // not referential actions). Verifying and dropping in one forward pass
    // therefore emptied `token_pools_legacy_chain` the moment the parent went,
    // and the later count read `0 -> 14412` and aborted the whole migration.
    for table in TOKEN_TABLES {
        let legacy = format!("{table}_legacy_chain");
        let old_count: i64 = transaction
            .query_row(&format!("SELECT COUNT(*) FROM {legacy}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to count legacy {table} rows"),
                message: error.to_string(),
            })?;
        let new_count: i64 = transaction
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to count migrated {table} rows"),
                message: error.to_string(),
            })?;
        if old_count != new_count {
            return Err(Error::MigrationIntegrity {
                table: table.to_string(),
                detail: format!("row count changed: {old_count} -> {new_count}"),
            });
        }
    }
    // `tokens` is the first entry and the only foreign-key parent, so reverse
    // order drops every child before it and no cascade can reach a staged table
    // that is still needed.
    for table in TOKEN_TABLES.iter().rev() {
        let legacy = format!("{table}_legacy_chain");
        transaction
            .execute(&format!("DROP TABLE {legacy}"), [])
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to remove staged legacy table {legacy}"),
                message: error.to_string(),
            })?;
    }
    if transaction
        .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to validate tokens foreign keys"),
            message: error.to_string(),
        })?
        .is_some()
    {
        return Err(Error::MigrationIntegrity {
            table: "tokens".to_owned(),
            detail: "foreign-key validation failed".to_owned(),
        });
    }
    Ok(())
}

/// Apply every missing additive column. Structural presence is the gate: a database
/// already stamped at `SCHEMA_VERSION` still receives a column the live schema
/// requires. Runs inside the opener's transaction; a no-op leaves the schema
/// unchanged, so repeated initialization is idempotent.
pub(super) fn apply_additive_migrations(transaction: &Connection) -> Result<(), Error> {
    for step in ADDITIVE_COLUMNS {
        if !ddl::has_table(&transaction, step.table)? {
            continue;
        }
        if ddl::column_names(&transaction, step.table)?
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
            .map_err(|error| DatabaseError::Query {
                operation: format!("add column {} to {}", step.column, step.table),
                message: error.to_string(),
            })?;
    }
    Ok(())
}

/// Drop every retired index that exists. Runs inside the opener's transaction and is
/// idempotent.
pub(super) fn drop_retired_indexes(transaction: &Connection) -> Result<(), Error> {
    for index in RETIRED_INDEXES {
        // SQLite does not bind identifiers; the names are constants and are quoted like
        // the legacy index names above.
        let quoted_index = format!("\"{}\"", index.replace('"', "\"\""));
        transaction
            .execute(&format!("DROP INDEX IF EXISTS {quoted_index}"), [])
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to drop retired tokens index {index}"),
                message: error.to_string(),
            })?;
    }
    Ok(())
}

fn copy_legacy_rows(transaction: &Connection, table: &str) -> Result<(), Error> {
    let legacy = format!("{table}_legacy_chain");
    let columns = |name: &str| -> Result<Vec<String>, Error> {
        ddl::column_names(transaction, name).map_err(Error::from)
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
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to copy legacy {table} rows"),
            message: error.to_string(),
        })?;
    Ok(())
}
