// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exact migration of the released position amounts to decimal TEXT: the amount columns and the persisted pending partial exits.

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::chains::RawAmount;
use crate::positions::{Error, Result};

use super::column_names::rename_unit_neutral_columns;
use super::types::{
    PENDING_PARTIAL_EXIT_METADATA_KEY, POSITIONS_INDEXES, SCHEMA_POSITIONS,
    SCHEMA_POSITION_ENTRIES, SCHEMA_POSITION_EXITS,
};

const TABLES: [(&str, &str, &[&str]); 3] = [
    (
        "positions",
        SCHEMA_POSITIONS,
        &[
            "token_amount",
            "remaining_token_amount",
            "total_exited_amount",
        ],
    ),
    ("position_entries", SCHEMA_POSITION_ENTRIES, &["amount"]),
    ("position_exits", SCHEMA_POSITION_EXITS, &["amount"]),
];

fn sql_items(sql: &str) -> Result<Vec<String>> {
    let clean = sql
        .lines()
        .map(|line| line.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let start = clean.find('(').ok_or_else(|| Error::SchemaMigration {
        detail: "missing table definition".into(),
    })?;
    let end = clean.rfind(')').ok_or_else(|| Error::SchemaMigration {
        detail: "missing table closing delimiter".into(),
    })?;
    let mut items = Vec::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut from = start + 1;
    let bytes = clean.as_bytes();
    for i in start + 1..end {
        match bytes[i] {
            b'\'' => quoted = !quoted,
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth -= 1,
            b',' if !quoted && depth == 0 => {
                items.push(
                    clean[from..i]
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                from = i + 1;
            }
            _ => {}
        }
    }
    items.push(
        clean[from..end]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    );
    items.sort();
    Ok(items)
}

fn columns(
    conn: &Connection,
    table: &str,
) -> Result<Vec<(String, String, i64, Option<String>, i64, i64)>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_xinfo({table})"))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("inspect {table}: {e}"),
        })?;
    let result = stmt
        .query_map([], |row| {
            Ok((
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .map_err(|e| Error::SchemaMigration {
            detail: format!("inspect {table}: {e}"),
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| Error::SchemaMigration {
            detail: format!("decode {table} columns: {e}"),
        })?;
    Ok(result)
}

fn validate_table(
    conn: &Connection,
    table: &str,
    canonical: &str,
    amounts: &[&str],
) -> Result<(String, Vec<String>, bool)> {
    let stored: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("inspect {table} definition: {e}"),
        })?;
    let actual = columns(conn, table)?;
    let canonical_db = Connection::open_in_memory().map_err(|e| Error::SchemaMigration {
        detail: e.to_string(),
    })?;
    canonical_db
        .execute(canonical, [])
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    let expected = columns(&canonical_db, table)?;
    let mut expected_items = sql_items(canonical)?;
    let actual_items = sql_items(&stored)?;
    for amount in amounts {
        if actual_items
            .iter()
            .any(|item| item.starts_with(&format!("{amount} INTEGER")))
        {
            for item in &mut expected_items {
                if item.starts_with(&format!("{amount} TEXT")) {
                    *item =
                        item.replacen(&format!("{amount} TEXT"), &format!("{amount} INTEGER"), 1);
                    if *amount == "total_exited_amount" {
                        *item = item.replace("DEFAULT '0'", "DEFAULT 0");
                    }
                }
            }
        }
    }
    // The prior chain migration appends a defaulted identity to older root tables.
    if table == "positions" {
        if let Some(item) = actual_items
            .iter()
            .find(|item| item.starts_with("chain_id "))
        {
            if item == "chain_id TEXT NOT NULL DEFAULT 'solana'" {
                expected_items.retain(|item| item != "chain_id TEXT NOT NULL");
                expected_items.push(item.clone());
            }
        }
    }
    expected_items.sort();
    if actual_items != expected_items {
        return Err(Error::SchemaMigration {
            detail: format!("unrecognized {table} definition would be lost by rebuild"),
        });
    }
    if actual.len() != expected.len()
        || actual.iter().zip(expected.iter()).any(|(a, b)| {
            a.0 != b.0
                || a.2 != b.2
                || a.4 != b.4
                || a.5 != 0
                || (a.1 != b.1
                    && !(amounts.contains(&a.0.as_str()) && a.1 == "INTEGER" && b.1 == "TEXT")
                    && !(table == "positions" && a.0 == "chain_id" && a.1 == "TEXT"))
                || (a.3 != b.3
                    && !(table == "positions"
                        && a.0 == "chain_id"
                        && a.3.as_deref() == Some("'solana'"))
                    && !(a.0 == "total_exited_amount" && a.3.as_deref() == Some("0")))
        })
    {
        return Err(Error::SchemaMigration {
            detail: format!("unrecognized {table} column metadata"),
        });
    }
    let amount_types = amounts
        .iter()
        .map(|name| {
            actual
                .iter()
                .find(|column| column.0 == *name)
                .map(|column| column.1.as_str())
        })
        .collect::<Vec<_>>();
    let legacy = amount_types.iter().all(|ty| *ty == Some("INTEGER"));
    let current = amount_types.iter().all(|ty| *ty == Some("TEXT"));
    if !legacy && !current {
        return Err(Error::SchemaMigration {
            detail: format!("mixed or unknown {table} amount column types"),
        });
    }
    Ok((
        stored,
        actual.into_iter().map(|column| column.0).collect(),
        legacy,
    ))
}

fn validate_objects(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT type, name, sql FROM sqlite_master WHERE tbl_name IN ('positions', 'position_entries', 'position_exits') AND type IN ('index', 'trigger') AND sql IS NOT NULL",
    ).map_err(|e| Error::SchemaMigration { detail: e.to_string() })?;
    let objects = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    for object in objects {
        let (kind, name, sql) = object.map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
        let accepted = POSITIONS_INDEXES.iter().any(|index| {
            let canonical = index.replace(" IF NOT EXISTS", "");
            (canonical.trim_end_matches(';') == sql || index.trim_end_matches(';') == sql)
                && canonical.contains(&format!("INDEX {name} ON "))
        });
        if kind != "index" || !accepted {
            return Err(Error::SchemaMigration {
                detail: format!("unrecognized {kind} {name} would be lost by rebuild"),
            });
        }
    }
    for (table, _, _) in TABLES {
        let mut stmt = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
        let indexes = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(3)?))
            })
            .map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
        for index in indexes {
            let (name, origin) = index.map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
            if origin != "c"
                || !POSITIONS_INDEXES
                    .iter()
                    .any(|sql| sql.contains(&format!("INDEX IF NOT EXISTS {name} ON ")))
            {
                return Err(Error::SchemaMigration {
                    detail: format!("unrecognized index {name} would be lost by rebuild"),
                });
            }
        }
    }
    Ok(())
}

fn rebuild(
    conn: &Connection,
    table: &str,
    ddl: &str,
    names: &[String],
    amounts: &[&str],
) -> Result<()> {
    let start = ddl.find('(').ok_or_else(|| Error::SchemaMigration {
        detail: format!("missing {table} definition"),
    })?;
    let mut create = format!("CREATE TABLE {table}__raw_new {}", &ddl[start..]);
    for amount in amounts {
        create = create.replacen(&format!("{amount} INTEGER"), &format!("{amount} TEXT"), 1);
    }
    if table == "positions" {
        create = create.replacen(
            "total_exited_amount TEXT NOT NULL DEFAULT 0",
            "total_exited_amount TEXT NOT NULL DEFAULT '0'",
            1,
        );
    }
    conn.execute(&create, [])
        .map_err(|e| Error::SchemaMigration {
            detail: format!("create {table} replacement: {e}"),
        })?;
    let names_sql = names.join(", ");
    let mut stmt = conn
        .prepare(&format!("SELECT {names_sql} FROM {table}"))
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    let mut rows = stmt.query([]).map_err(|e| Error::SchemaMigration {
        detail: e.to_string(),
    })?;
    let insert = format!(
        "INSERT INTO {table}__raw_new ({names_sql}) VALUES ({})",
        vec!["?"; names.len()].join(", ")
    );
    while let Some(row) = rows.next().map_err(|e| Error::SchemaMigration {
        detail: e.to_string(),
    })? {
        let mut values = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let value: Value = row.get(i).map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
            values.push(if amounts.contains(&name.as_str()) {
                match value {
                    Value::Integer(bits) => {
                        Value::Text(u64::from_ne_bytes(bits.to_ne_bytes()).to_string())
                    }
                    Value::Null if name != "total_exited_amount" => Value::Null,
                    _ => {
                        return Err(Error::SchemaMigration {
                            detail: format!("invalid legacy {table}.{name} storage"),
                        })
                    }
                }
            } else {
                value
            });
        }
        conn.execute(&insert, params_from_iter(values.iter()))
            .map_err(|e| Error::SchemaMigration {
                detail: format!("copy {table}: {e}"),
            })?;
    }
    let before: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    let after: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {table}__raw_new"),
            [],
            |row| row.get(0),
        )
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    if before != after {
        return Err(Error::SchemaMigration {
            detail: format!("{table} row count changed during rebuild"),
        });
    }
    Ok(())
}

pub(super) fn migrate_position_amounts(conn: &Connection) -> Result<()> {
    migrate_amount_columns(conn)?;
    migrate_pending_partial_exit_amounts(conn)
}

/// Pending partial exits persisted before amounts were raw store `expected_exit_amount` as a
/// JSON number; the canonical form is the decimal string. Unreadable payloads are left for
/// rehydration to report.
fn migrate_pending_partial_exit_amounts(conn: &Connection) -> Result<()> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT value FROM position_metadata WHERE key = ?1",
            [PENDING_PARTIAL_EXIT_METADATA_KEY],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| Error::SchemaMigration {
            detail: format!("read pending partial exits: {e}"),
        })?;
    let Some(stored) = stored else {
        return Ok(());
    };
    let Ok(mut pending) = serde_json::from_str::<serde_json::Value>(&stored) else {
        return Ok(());
    };
    let Some(entries) = pending.as_array_mut() else {
        return Ok(());
    };
    let mut changed = false;
    for entry in entries {
        let Some(amount) = entry.get_mut("expected_exit_amount") else {
            continue;
        };
        if let Some(raw) = amount.as_u64() {
            *amount = serde_json::Value::String(RawAmount::from(raw).to_string());
            changed = true;
        }
    }
    if !changed {
        return Ok(());
    }
    conn.execute(
        "UPDATE position_metadata SET value = ?1, updated_at = datetime('now') WHERE key = ?2",
        params![pending.to_string(), PENDING_PARTIAL_EXIT_METADATA_KEY],
    )
    .map_err(|e| Error::SchemaMigration {
        detail: format!("rewrite pending partial exits: {e}"),
    })?;
    Ok(())
}

fn migrate_amount_columns(conn: &Connection) -> Result<()> {
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    conn.pragma_update(None, "foreign_keys", 0)
        .map_err(|e| Error::SchemaMigration {
            detail: e.to_string(),
        })?;
    let result = (|| -> Result<()> {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
        // Unit column names are canonicalized first, in this transaction, so the
        // validation below sees canonical names and a refused rebuild leaves the
        // stored schema exactly as it was.
        rename_unit_neutral_columns(&tx)?;
        let inspected = TABLES
            .iter()
            .map(|(table, ddl, amounts)| validate_table(&tx, table, ddl, amounts))
            .collect::<Result<Vec<_>>>()?;
        if inspected.iter().all(|(_, _, legacy)| !legacy) {
            return tx.commit().map_err(|e| Error::SchemaMigration {
                detail: format!("commit position column names: {e}"),
            });
        }
        if inspected.iter().any(|(_, _, legacy)| !legacy) {
            return Err(Error::SchemaMigration {
                detail: "mixed legacy and canonical position amount tables".into(),
            });
        }
        validate_objects(&tx)?;
        for ((table, _, amounts), (ddl, names, _)) in TABLES.iter().zip(inspected.iter()) {
            rebuild(&tx, table, ddl, names, amounts)?;
        }
        // Child tables go first. Disabling FK enforcement before BEGIN prevents
        // ON DELETE CASCADE from clearing state, tracking, and snapshot rows.
        for table in ["position_entries", "position_exits", "positions"] {
            tx.execute(&format!("DROP TABLE {table}"), [])
                .map_err(|e| Error::SchemaMigration {
                    detail: e.to_string(),
                })?;
        }
        for table in ["positions", "position_entries", "position_exits"] {
            tx.execute(
                &format!("ALTER TABLE {table}__raw_new RENAME TO {table}"),
                [],
            )
            .map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
        }
        for index in POSITIONS_INDEXES {
            tx.execute(index, []).map_err(|e| Error::SchemaMigration {
                detail: format!("restore position indexes: {e}"),
            })?;
        }
        let violations: i64 = tx
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::SchemaMigration {
                detail: e.to_string(),
            })?;
        if violations != 0 {
            return Err(Error::SchemaMigration {
                detail: format!("{violations} position foreign-key violations"),
            });
        }
        tx.commit().map_err(|e| Error::SchemaMigration {
            detail: format!("commit position amount migration: {e}"),
        })
    })();
    conn.pragma_update(None, "foreign_keys", foreign_keys)
        .map_err(|e| Error::SchemaMigration {
            detail: format!("restore foreign keys: {e}"),
        })?;
    result
}

#[cfg(test)]
mod tests {
    use super::super::types::SCHEMA_POSITION_METADATA;
    use super::*;

    fn metadata_connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(SCHEMA_POSITION_METADATA, []).unwrap();
        conn
    }

    fn store_pending(conn: &Connection, value: &str) {
        conn.execute(
            "INSERT INTO position_metadata (key, value) VALUES (?1, ?2)",
            params![PENDING_PARTIAL_EXIT_METADATA_KEY, value],
        )
        .unwrap();
    }

    fn stored_pending(conn: &Connection) -> String {
        conn.query_row(
            "SELECT value FROM position_metadata WHERE key = ?1",
            [PENDING_PARTIAL_EXIT_METADATA_KEY],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn pending_amounts_become_decimal_strings() {
        let conn = metadata_connection();
        let input = r#"[{"signature":"s","mint":"m","position_id":1,"expected_exit_amount":18446744073709551615,"requested_exit_percentage":25.0,"expiry_height":null,"created_at":"2025-01-02T00:00:00Z"}]"#;
        store_pending(&conn, input);

        migrate_pending_partial_exit_amounts(&conn).unwrap();
        let first = stored_pending(&conn);
        let migrated: serde_json::Value = serde_json::from_str(&first).unwrap();
        let original: serde_json::Value = serde_json::from_str(input).unwrap();
        let migrated_entry = migrated[0].as_object().unwrap();
        let original_entry = original[0].as_object().unwrap();
        assert_eq!(
            migrated_entry["expected_exit_amount"],
            serde_json::Value::String("18446744073709551615".to_owned())
        );
        assert_eq!(migrated_entry.len(), original_entry.len());
        for (key, value) in original_entry {
            if key != "expected_exit_amount" {
                assert_eq!(&migrated_entry[key], value, "field {key}");
            }
        }

        migrate_pending_partial_exit_amounts(&conn).unwrap();
        assert_eq!(stored_pending(&conn), first);
    }

    #[test]
    fn unreadable_pending_payloads_are_left_unchanged() {
        for payload in [
            "not json",
            r#"{"a":1}"#,
            r#"[{"expected_exit_amount":1.5}]"#,
            r#"[{"expected_exit_amount":-3}]"#,
            "[]",
        ] {
            let conn = metadata_connection();
            store_pending(&conn, payload);
            migrate_pending_partial_exit_amounts(&conn).unwrap();
            assert_eq!(stored_pending(&conn), payload);
        }
    }

    #[test]
    fn a_missing_pending_entry_is_a_no_op() {
        let conn = metadata_connection();
        migrate_pending_partial_exit_amounts(&conn).unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM position_metadata", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 0);
    }
}
