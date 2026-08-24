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

use crate::errors::DatabaseError;
use crate::tokens::Error;
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
    Ok(!table_columns(conn, "tokens")?
        .iter()
        .any(|column| column == "chain_id"))
}

pub(super) fn migrate_legacy_schema(conn: &Connection) -> Result<(), Error> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to begin tokens chain migration"),
            message: error.to_string(),
        })?;
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
    transaction.commit().map_err(|error| DatabaseError::Query {
        operation: format!("Failed to commit tokens chain migration"),
        message: error.to_string(),
    })?;
    if conn
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
/// requires. No-ops commit cleanly so repeated initialization is idempotent.
pub(super) fn apply_additive_migrations(conn: &Connection) -> Result<(), Error> {
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to begin tokens additive migration"),
            message: error.to_string(),
        })?;

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
            .map_err(|error| DatabaseError::Query {
                operation: format!("add column {} to {}", step.column, step.table),
                message: error.to_string(),
            })?;
    }

    transaction.commit().map_err(|error| {
        Error::from(DatabaseError::Query {
            operation: "commit tokens additive migration".to_owned(),
            message: error.to_string(),
        })
    })
}

fn copy_legacy_rows(transaction: &Transaction<'_>, table: &str) -> Result<(), Error> {
    let legacy = format!("{table}_legacy_chain");
    let columns = |name: &str| -> Result<Vec<String>, Error> {
        let mut statement = transaction
            .prepare(&format!("PRAGMA table_info({name})"))
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to inspect {name}"),
                message: error.to_string(),
            })?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| DatabaseError::Query {
                operation: format!("Failed to read {name} columns"),
                message: error.to_string(),
            })?;
        rows.collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            .map_err(|error| {
                Error::from(DatabaseError::Query {
                    operation: format!("decode {name} columns"),
                    message: error.to_string(),
                })
            })
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

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, Error> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to inspect {table} columns"),
            message: error.to_string(),
        })?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to read {table} columns"),
            message: error.to_string(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| DatabaseError::Query {
            operation: format!("Failed to decode {table} columns"),
            message: error.to_string(),
        })?;
    Ok(columns)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, Error> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |_| Ok(()),
    )
    .optional()
    .map_err(|error| {
        Error::from(DatabaseError::Query {
            operation: format!("inspect table {table}"),
            message: error.to_string(),
        })
    })
    .map(|row| row.is_some())
}
