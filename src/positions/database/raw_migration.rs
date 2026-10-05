// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Exact migration of the released position amount columns to decimal TEXT.

use rusqlite::{params_from_iter, types::Value, Connection};

use crate::positions::{Error, Result};

use super::types::{
    POSITIONS_INDEXES, SCHEMA_POSITIONS, SCHEMA_POSITION_ENTRIES, SCHEMA_POSITION_EXITS,
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

fn failure(detail: impl Into<String>) -> Error {
    Error::SchemaMigration {
        detail: detail.into(),
    }
}

fn sql_items(sql: &str) -> Result<Vec<String>> {
    let clean = sql
        .lines()
        .map(|line| line.split("--").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ");
    let start = clean
        .find('(')
        .ok_or_else(|| failure("missing table definition"))?;
    let end = clean
        .rfind(')')
        .ok_or_else(|| failure("missing table closing delimiter"))?;
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
        .map_err(|e| failure(format!("inspect {table}: {e}")))?;
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
        .map_err(|e| failure(format!("inspect {table}: {e}")))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| failure(format!("decode {table} columns: {e}")))?;
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
        .map_err(|e| failure(format!("inspect {table} definition: {e}")))?;
    let actual = columns(conn, table)?;
    let canonical_db = Connection::open_in_memory().map_err(|e| failure(e.to_string()))?;
    canonical_db
        .execute(canonical, [])
        .map_err(|e| failure(e.to_string()))?;
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
        return Err(failure(format!(
            "unrecognized {table} definition would be lost by rebuild"
        )));
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
        return Err(failure(format!("unrecognized {table} column metadata")));
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
        return Err(failure(format!(
            "mixed or unknown {table} amount column types"
        )));
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
    ).map_err(|e| failure(e.to_string()))?;
    let objects = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| failure(e.to_string()))?;
    for object in objects {
        let (kind, name, sql) = object.map_err(|e| failure(e.to_string()))?;
        let accepted = POSITIONS_INDEXES.iter().any(|index| {
            let canonical = index.replace(" IF NOT EXISTS", "");
            (canonical.trim_end_matches(';') == sql || index.trim_end_matches(';') == sql)
                && canonical.contains(&format!("INDEX {name} ON "))
        });
        if kind != "index" || !accepted {
            return Err(failure(format!(
                "unrecognized {kind} {name} would be lost by rebuild"
            )));
        }
    }
    for (table, _, _) in TABLES {
        let mut stmt = conn
            .prepare(&format!("PRAGMA index_list({table})"))
            .map_err(|e| failure(e.to_string()))?;
        let indexes = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(3)?))
            })
            .map_err(|e| failure(e.to_string()))?;
        for index in indexes {
            let (name, origin) = index.map_err(|e| failure(e.to_string()))?;
            if origin != "c"
                || !POSITIONS_INDEXES
                    .iter()
                    .any(|sql| sql.contains(&format!("INDEX IF NOT EXISTS {name} ON ")))
            {
                return Err(failure(format!(
                    "unrecognized index {name} would be lost by rebuild"
                )));
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
    let start = ddl
        .find('(')
        .ok_or_else(|| failure(format!("missing {table} definition")))?;
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
        .map_err(|e| failure(format!("create {table} replacement: {e}")))?;
    let names_sql = names.join(", ");
    let mut stmt = conn
        .prepare(&format!("SELECT {names_sql} FROM {table}"))
        .map_err(|e| failure(e.to_string()))?;
    let mut rows = stmt.query([]).map_err(|e| failure(e.to_string()))?;
    let insert = format!(
        "INSERT INTO {table}__raw_new ({names_sql}) VALUES ({})",
        vec!["?"; names.len()].join(", ")
    );
    while let Some(row) = rows.next().map_err(|e| failure(e.to_string()))? {
        let mut values = Vec::with_capacity(names.len());
        for (i, name) in names.iter().enumerate() {
            let value: Value = row.get(i).map_err(|e| failure(e.to_string()))?;
            values.push(if amounts.contains(&name.as_str()) {
                match value {
                    Value::Integer(bits) => {
                        Value::Text(u64::from_ne_bytes(bits.to_ne_bytes()).to_string())
                    }
                    Value::Null if name != "total_exited_amount" => Value::Null,
                    _ => return Err(failure(format!("invalid legacy {table}.{name} storage"))),
                }
            } else {
                value
            });
        }
        conn.execute(&insert, params_from_iter(values.iter()))
            .map_err(|e| failure(format!("copy {table}: {e}")))?;
    }
    let before: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .map_err(|e| failure(e.to_string()))?;
    let after: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {table}__raw_new"),
            [],
            |row| row.get(0),
        )
        .map_err(|e| failure(e.to_string()))?;
    if before != after {
        return Err(failure(format!("{table} row count changed during rebuild")));
    }
    Ok(())
}

pub(super) fn migrate_position_amounts(conn: &Connection) -> Result<()> {
    let inspected = TABLES
        .iter()
        .map(|(table, ddl, amounts)| validate_table(conn, table, ddl, amounts))
        .collect::<Result<Vec<_>>>()?;
    if inspected.iter().all(|(_, _, legacy)| !legacy) {
        return Ok(());
    }
    if inspected.iter().any(|(_, _, legacy)| !legacy) {
        return Err(failure("mixed legacy and canonical position amount tables"));
    }
    validate_objects(conn)?;
    let foreign_keys: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .map_err(|e| failure(e.to_string()))?;
    conn.pragma_update(None, "foreign_keys", 0)
        .map_err(|e| failure(e.to_string()))?;
    let result = (|| -> Result<()> {
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| failure(e.to_string()))?;
        for ((table, _, amounts), (ddl, names, _)) in TABLES.iter().zip(inspected.iter()) {
            rebuild(&tx, table, ddl, names, amounts)?;
        }
        // Child tables go first. Disabling FK enforcement before BEGIN prevents
        // ON DELETE CASCADE from clearing state, tracking, and snapshot rows.
        for table in ["position_entries", "position_exits", "positions"] {
            tx.execute(&format!("DROP TABLE {table}"), [])
                .map_err(|e| failure(e.to_string()))?;
        }
        for table in ["positions", "position_entries", "position_exits"] {
            tx.execute(
                &format!("ALTER TABLE {table}__raw_new RENAME TO {table}"),
                [],
            )
            .map_err(|e| failure(e.to_string()))?;
        }
        for index in POSITIONS_INDEXES {
            tx.execute(index, [])
                .map_err(|e| failure(format!("restore position indexes: {e}")))?;
        }
        let violations: i64 = tx
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| failure(e.to_string()))?;
        if violations != 0 {
            return Err(failure(format!(
                "{violations} position foreign-key violations"
            )));
        }
        tx.commit()
            .map_err(|e| failure(format!("commit position amount migration: {e}")))
    })();
    conn.pragma_update(None, "foreign_keys", foreign_keys)
        .map_err(|e| failure(format!("restore foreign keys: {e}")))?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_wide_amount_is_rejected_by_the_u64_domain_reader() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE amounts (amount TEXT NOT NULL); INSERT INTO amounts VALUES ('18446744073709551616')").unwrap();
        let error = conn
            .query_row("SELECT amount FROM amounts", [], |row| {
                super::super::operations::read_amount(row, "amount")
            })
            .unwrap_err();
        assert!(matches!(
            error,
            rusqlite::Error::FromSqlConversionFailure(..)
        ));
    }
}
