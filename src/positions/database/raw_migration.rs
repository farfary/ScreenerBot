// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Canonical shape of the position amount tables: every column and object is checked by name, released integer amounts become decimal TEXT, and a table whose layout differs is rebuilt from the canonical DDL.

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};

use crate::chains::RawAmount;
use crate::positions::{Error, Result};

use super::open_round::OPEN_ROUND_INDEX_NAME;
use super::types::{
    LEGACY_POSITIONS_COLUMNS, LEGACY_POSITIONS_INDEXES, PENDING_PARTIAL_EXIT_METADATA_KEY,
    POSITIONS_INDEXES, SCHEMA_POSITIONS, SCHEMA_POSITION_ENTRIES, SCHEMA_POSITION_EXITS,
};

/// A table holding raw token amounts, with what its canonical form allows.
struct AmountTable {
    name: &'static str,
    ddl: &'static str,
    /// Raw token amounts: TEXT now, INTEGER in released storage.
    amounts: &'static [&'static str],
    /// Columns of earlier releases that the rebuild drops.
    legacy_columns: &'static [&'static str],
}

/// Children before their root: the order tables are dropped in a rebuild.
const TABLES: [AmountTable; 3] = [
    AmountTable {
        name: "position_entries",
        ddl: SCHEMA_POSITION_ENTRIES,
        amounts: &["amount"],
        legacy_columns: &[],
    },
    AmountTable {
        name: "position_exits",
        ddl: SCHEMA_POSITION_EXITS,
        amounts: &["amount"],
        legacy_columns: &[],
    },
    AmountTable {
        name: "positions",
        ddl: SCHEMA_POSITIONS,
        amounts: &[
            "token_amount",
            "remaining_token_amount",
            "total_exited_amount",
        ],
        legacy_columns: LEGACY_POSITIONS_COLUMNS,
    },
];

/// What canonicalizing the amount tables changed.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Canonicalized {
    pub rebuilt: Vec<&'static str>,
    pub dropped_indexes: Vec<String>,
}

/// A column as `PRAGMA table_xinfo` reports it.
#[derive(Debug, Clone, PartialEq)]
struct Column {
    name: String,
    declared: String,
    not_null: bool,
    default: Option<String>,
    primary_key: i64,
    hidden: i64,
}

fn columns(conn: &Connection, table: &str) -> Result<Vec<Column>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_xinfo({table})"))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to inspect {table}: {e}"),
        })?;
    let result = stmt
        .query_map([], |row| {
            Ok(Column {
                name: row.get(1)?,
                declared: row.get(2)?,
                not_null: row.get(3)?,
                default: row.get(4)?,
                primary_key: row.get(5)?,
                hidden: row.get(6)?,
            })
        })
        .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to decode {table} columns: {e}"),
        })?;
    Ok(result)
}

/// The comma-separated items of a CREATE TABLE body, comments removed, whitespace
/// collapsed and lowercased, in their stored order.
fn definition_items(sql: &str) -> Result<Vec<String>> {
    let clean = sql
        .lines()
        .map(|line| line.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let start = clean.find('(').ok_or_else(|| Error::SchemaMigration {
        detail: "missing table definition".to_owned(),
    })?;
    let end = clean.rfind(')').ok_or_else(|| Error::SchemaMigration {
        detail: "missing table closing delimiter".to_owned(),
    })?;
    let normalize = |item: &str| {
        item.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
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
                items.push(normalize(&clean[from..i]));
                from = i + 1;
            }
            _ => {}
        }
    }
    items.push(normalize(&clean[from..end]));
    Ok(items)
}

/// The column an item defines, or `None` for a table constraint.
fn item_column<'a>(item: &str, names: &'a [String]) -> Option<&'a String> {
    let first = item
        .split_whitespace()
        .next()?
        .trim_matches(|c| matches!(c, '"' | '`' | '[' | ']'));
    names.iter().find(|name| name.eq_ignore_ascii_case(first))
}

/// The stored definitions released builds wrote for a canonical column: the canonical
/// one, an amount column still declared INTEGER (with `DEFAULT 0` for the cumulative
/// exit amount), and the chain identity appended with its default.
fn accepted_definitions(table: &AmountTable, column: &str, canonical: &str) -> Vec<String> {
    let mut accepted = vec![canonical.to_owned()];
    if table.amounts.contains(&column) {
        let integer =
            canonical.replacen(&format!("{column} text"), &format!("{column} integer"), 1);
        accepted.push(integer.replace("default '0'", "default 0"));
        accepted.push(integer);
    }
    if table.name == "positions" && column == "chain_id" {
        accepted.push(format!("{canonical} default 'solana'"));
    }
    accepted
}

fn accepted_column(table: &AmountTable, stored: &Column, canonical: &Column) -> bool {
    if stored == canonical {
        return true;
    }
    let same_shape = stored.not_null == canonical.not_null
        && stored.primary_key == canonical.primary_key
        && stored.hidden == 0;
    let name = stored.name.as_str();
    if table.amounts.contains(&name) && stored.declared == "INTEGER" && canonical.declared == "TEXT"
    {
        return same_shape
            && (stored.default == canonical.default
                || (name == "total_exited_amount"
                    && stored.default.as_deref() == Some("0")
                    && canonical.default.as_deref() == Some("'0'")));
    }
    table.name == "positions"
        && name == "chain_id"
        && same_shape
        && stored.declared == canonical.declared
        && stored.default.as_deref() == Some("'solana'")
        && canonical.default.is_none()
}

/// What an open has to do with one amount table.
struct Inspection {
    table: &'static AmountTable,
    canonical: Vec<Column>,
    /// Amount columns still declared INTEGER: their values are released bit patterns.
    integer_amounts: Vec<&'static str>,
    rebuild: bool,
}

/// Compare a stored table with its canonical definition by column name. Accepted
/// differences are the released ones; anything else refuses and is named.
fn inspect_table(conn: &Connection, table: &'static AmountTable) -> Result<Inspection> {
    let stored_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table.name],
            |row| row.get(0),
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to inspect {} definition: {e}", table.name),
        })?;
    let canonical_db = Connection::open_in_memory().map_err(|e| Error::SchemaMigration {
        detail: format!("failed to open canonical schema: {e}"),
    })?;
    canonical_db
        .execute(table.ddl, [])
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to create canonical {}: {e}", table.name),
        })?;
    let canonical = columns(&canonical_db, table.name)?;
    let stored = columns(conn, table.name)?;
    let mut legacy = false;
    for column in &stored {
        if column.hidden != 0 {
            return Err(Error::SchemaMigration {
                detail: format!(
                    "unrecognized generated column {}.{}",
                    table.name, column.name
                ),
            });
        }
        match canonical.iter().find(|c| c.name == column.name) {
            Some(expected) if accepted_column(table, column, expected) => {}
            Some(_) => {
                return Err(Error::SchemaMigration {
                    detail: format!(
                        "unrecognized definition of column {}.{}",
                        table.name, column.name
                    ),
                })
            }
            None if table.legacy_columns.contains(&column.name.as_str()) => legacy = true,
            None => {
                return Err(Error::SchemaMigration {
                    detail: format!("unrecognized column {}.{}", table.name, column.name),
                })
            }
        }
    }
    if let Some(missing) = canonical
        .iter()
        .find(|c| !stored.iter().any(|s| s.name == c.name))
    {
        return Err(Error::SchemaMigration {
            detail: format!("missing column {}.{}", table.name, missing.name),
        });
    }

    // Clauses the column metadata does not show: CHECK, REFERENCES, COLLATE, UNIQUE
    // and table constraints are compared on the definition text.
    let stored_names = stored.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    let canonical_names = canonical.iter().map(|c| c.name.clone()).collect::<Vec<_>>();
    let canonical_items = definition_items(table.ddl)?;
    let mut stored_constraints = Vec::new();
    for item in definition_items(&stored_sql)? {
        let Some(name) = item_column(&item, &stored_names) else {
            stored_constraints.push(item);
            continue;
        };
        if table.legacy_columns.contains(&name.as_str()) {
            continue;
        }
        let expected = canonical_items
            .iter()
            .find(|candidate| item_column(candidate, &canonical_names) == Some(name))
            .ok_or_else(|| Error::SchemaMigration {
                detail: format!("unrecognized column {}.{name}", table.name),
            })?;
        if !accepted_definitions(table, name, expected).contains(&item) {
            return Err(Error::SchemaMigration {
                detail: format!("unrecognized definition of column {}.{name}", table.name),
            });
        }
    }
    let mut canonical_constraints = canonical_items
        .into_iter()
        .filter(|item| item_column(item, &canonical_names).is_none())
        .collect::<Vec<_>>();
    stored_constraints.sort();
    canonical_constraints.sort();
    if stored_constraints != canonical_constraints {
        return Err(Error::SchemaMigration {
            detail: format!("unrecognized table constraint on {}", table.name),
        });
    }

    let integer_amounts = table
        .amounts
        .iter()
        .copied()
        .filter(|amount| {
            stored
                .iter()
                .any(|c| c.name == *amount && c.declared == "INTEGER")
        })
        .collect::<Vec<_>>();
    let order_differs = stored
        .iter()
        .filter(|c| !table.legacy_columns.contains(&c.name.as_str()))
        .map(|c| &c.name)
        .ne(canonical.iter().map(|c| &c.name));
    Ok(Inspection {
        table,
        rebuild: legacy || order_differs || !integer_amounts.is_empty(),
        canonical,
        integer_amounts,
    })
}

/// The name a canonical index definition creates, with its normalized SQL as SQLite
/// stores it.
fn canonical_indexes() -> Vec<(String, String)> {
    POSITIONS_INDEXES
        .iter()
        .filter_map(|sql| {
            let stored = sql.replace(" IF NOT EXISTS", "");
            let stored = stored.trim_end_matches(';');
            let name = stored
                .split_whitespace()
                .skip_while(|w| *w != "INDEX")
                .nth(1)?;
            Some((name.to_owned(), stored.to_owned()))
        })
        .collect()
}

/// Drop the legacy indexes and canonical indexes whose definition drifted (both are
/// recreated by the open), and refuse every other index or trigger on an amount table.
fn classify_objects(conn: &Connection) -> Result<Vec<String>> {
    let canonical = canonical_indexes();
    let canonical_db = Connection::open_in_memory().map_err(|e| Error::SchemaMigration {
        detail: format!("failed to open canonical schema: {e}"),
    })?;
    for table in &TABLES {
        canonical_db
            .execute(table.ddl, [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to create canonical {}: {e}", table.name),
            })?;
    }
    let automatic: Vec<String> = canonical_db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'index' AND sql IS NULL")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to inspect canonical automatic indexes: {e}"),
        })?;
    let objects: Vec<(String, String, String, Option<String>)> = conn
        .prepare(
            "SELECT type, name, tbl_name, sql FROM sqlite_master WHERE tbl_name IN ('positions', 'position_entries', 'position_exits') AND type IN ('index', 'trigger') ORDER BY name",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()
        })
        .map_err(|e| Error::SchemaMigration {
 detail: format!("failed to inspect position indexes and triggers: {e}"),
 })?;
    let mut dropped = Vec::new();
    for (kind, name, table, sql) in objects {
        if kind != "index" {
            return Err(Error::SchemaMigration {
                detail: format!("unrecognized {kind} {name} on {table}"),
            });
        }
        let Some(sql) = sql else {
            if automatic.contains(&name) {
                continue;
            }
            return Err(Error::SchemaMigration {
                detail: format!("unrecognized index {name} on {table}"),
            });
        };
        if name == OPEN_ROUND_INDEX_NAME {
            continue;
        }
        let known = canonical
            .iter()
            .find(|(canonical_name, _)| *canonical_name == name);
        let drop = match known {
            Some((_, canonical_sql)) => !canonical_sql.eq_ignore_ascii_case(&sql),
            None if LEGACY_POSITIONS_INDEXES.contains(&name.as_str()) => true,
            None => {
                return Err(Error::SchemaMigration {
                    detail: format!("unrecognized index {name} on {table}"),
                })
            }
        };
        if drop {
            conn.execute(&format!("DROP INDEX {name}"), [])
                .map_err(|e| Error::SchemaMigration {
                    detail: format!("failed to drop index {name}: {e}"),
                })?;
            dropped.push(name);
        }
    }
    Ok(dropped)
}

/// Convert one stored amount to its canonical decimal TEXT. A column still declared
/// INTEGER holds the released `u64` bit pattern; anything else must already be a
/// canonical decimal.
fn canonical_amount(value: Value, integer_declared: bool) -> Option<Value> {
    match value {
        Value::Integer(bits) if integer_declared => Some(Value::Text(
            u64::from_ne_bytes(bits.to_ne_bytes()).to_string(),
        )),
        Value::Text(text) if text.parse::<RawAmount>().is_ok() => Some(Value::Text(text)),
        Value::Null => Some(Value::Null),
        _ => None,
    }
}

/// Copy a table into `{table}__raw_new`, created from the canonical DDL, by column
/// name.
fn copy_into_canonical(conn: &Connection, inspection: &Inspection) -> Result<()> {
    let table = inspection.table.name;
    let create = inspection.table.ddl.replacen(
        &format!("CREATE TABLE IF NOT EXISTS {table} ("),
        &format!("CREATE TABLE {table}__raw_new ("),
        1,
    );
    conn.execute(&create, [])
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to create {table} replacement: {e}"),
        })?;
    let names = inspection
        .canonical
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>();
    let names_sql = names.join(", ");
    let mut select = conn
        .prepare(&format!("SELECT {names_sql} FROM {table} ORDER BY rowid"))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to read {table}: {e}"),
        })?;
    let mut insert = conn
        .prepare(&format!(
            "INSERT INTO {table}__raw_new ({names_sql}) VALUES ({})",
            vec!["?"; names.len()].join(", ")
        ))
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to prepare {table} copy: {e}"),
        })?;
    let mut rows = select.query([]).map_err(|e| Error::SchemaMigration {
        detail: format!("failed to read {table}: {e}"),
    })?;
    while let Some(row) = rows.next().map_err(|e| Error::SchemaMigration {
        detail: format!("failed to read {table}: {e}"),
    })? {
        let id: Value = row.get(0).map_err(|e| Error::SchemaMigration {
            detail: format!("failed to read {table}.id: {e}"),
        })?;
        let mut values = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let value: Value = row.get(i).map_err(|e| Error::SchemaMigration {
                detail: format!("failed to read {table}.{name}: {e}"),
            })?;
            if !inspection.table.amounts.contains(name) {
                values.push(value);
                continue;
            }
            let integer_declared = inspection.integer_amounts.contains(name);
            let converted = canonical_amount(value, integer_declared).ok_or_else(|| {
                Error::SchemaMigration {
                    detail: format!(
                        "invalid legacy {table}.{name} storage at id {}",
                        sql_value_text(&id)
                    ),
                }
            })?;
            values.push(converted);
        }
        insert
            .execute(params_from_iter(values.iter()))
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to copy {table} row {}: {e}", sql_value_text(&id)),
            })?;
    }
    let count = |name: &str| -> Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {name}"), [], |row| {
            row.get(0)
        })
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to count {name}: {e}"),
        })
    };
    if count(table)? != count(&format!("{table}__raw_new"))? {
        return Err(Error::SchemaMigration {
            detail: format!("{table} row count changed during rebuild"),
        });
    }
    Ok(())
}

fn sql_value_text(value: &Value) -> String {
    match value {
        Value::Integer(v) => v.to_string(),
        Value::Text(v) => v.clone(),
        Value::Real(v) => v.to_string(),
        Value::Null => "NULL".to_owned(),
        Value::Blob(_) => "<blob>".to_owned(),
    }
}

/// Swap every copied table in for its original, children first, keeping each
/// AUTOINCREMENT sequence so ids are never reused.
fn swap_rebuilt(conn: &Connection, rebuilt: &[&'static str]) -> Result<()> {
    let mut sequences = Vec::new();
    for table in rebuilt {
        let sequence: Option<i64> = conn
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = ?1",
                [table],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to read {table} sequence: {e}"),
            })?;
        sequences.push((*table, sequence));
    }
    for table in rebuilt {
        conn.execute(&format!("DROP TABLE {table}"), [])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to drop {table}: {e}"),
            })?;
    }
    for (table, sequence) in &sequences {
        conn.execute(
            &format!("ALTER TABLE {table}__raw_new RENAME TO {table}"),
            [],
        )
        .map_err(|e| Error::SchemaMigration {
            detail: format!("failed to rename {table} replacement: {e}"),
        })?;
        let Some(sequence) = sequence else {
            continue;
        };
        conn.execute("DELETE FROM sqlite_sequence WHERE name = ?1", [table])
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to reset {table} sequence: {e}"),
            })?;
        conn.execute(
            &format!(
                "INSERT INTO sqlite_sequence (name, seq) SELECT ?1, MAX(?2, COALESCE((SELECT MAX(id) FROM {table}), 0))"
            ),
            params![table, sequence],
        )
        .map_err(|e| Error::SchemaMigration {
 detail: format!("failed to restore {table} sequence: {e}"),
 })?;
    }
    Ok(())
}

/// Bring the amount tables to their canonical shape inside the caller's transaction,
/// which must run with foreign keys off: a dropped root would otherwise cascade into
/// its children. Unknown columns, definitions and objects refuse before anything is
/// rebuilt, and the result is checked before the caller commits.
pub(super) fn canonicalize_amount_tables(conn: &Connection) -> Result<Canonicalized> {
    let inspections = TABLES
        .iter()
        .map(|table| inspect_table(conn, table))
        .collect::<Result<Vec<_>>>()?;
    let dropped_indexes = classify_objects(conn)?;
    let mut rebuilt = Vec::new();
    for inspection in inspections.iter().filter(|i| i.rebuild) {
        copy_into_canonical(conn, inspection)?;
        rebuilt.push(inspection.table.name);
    }
    if !rebuilt.is_empty() {
        swap_rebuilt(conn, &rebuilt)?;
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to check position foreign keys: {e}"),
            })?;
        if violations != 0 {
            return Err(Error::SchemaMigration {
                detail: format!("{violations} position foreign-key violations after rebuild"),
            });
        }
        let quick_check: String = conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|e| Error::SchemaMigration {
                detail: format!("failed to check positions storage: {e}"),
            })?;
        if quick_check != "ok" {
            return Err(Error::SchemaMigration {
                detail: format!("positions storage check failed after rebuild: {quick_check}"),
            });
        }
    }
    Ok(Canonicalized {
        rebuilt,
        dropped_indexes,
    })
}

/// Pending partial exits persisted before amounts were raw store `expected_exit_amount` as a
/// JSON number; the canonical form is the decimal string. Unreadable payloads are left for
/// rehydration to report. Returns whether the stored list was rewritten.
pub(super) fn migrate_pending_partial_exit_amounts(conn: &Connection) -> Result<bool> {
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
        return Ok(false);
    };
    let Ok(mut pending) = serde_json::from_str::<serde_json::Value>(&stored) else {
        return Ok(false);
    };
    let Some(entries) = pending.as_array_mut() else {
        return Ok(false);
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
        return Ok(false);
    }
    conn.execute(
        "UPDATE position_metadata SET value = ?1, updated_at = datetime('now') WHERE key = ?2",
        params![pending.to_string(), PENDING_PARTIAL_EXIT_METADATA_KEY],
    )
    .map_err(|e| Error::SchemaMigration {
        detail: format!("rewrite pending partial exits: {e}"),
    })?;
    Ok(true)
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
