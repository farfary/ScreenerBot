// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The rename engine every store runs on open: reconciles a store holding both names of a renamed object, renames legacy-only objects and resolves live column names.

use rusqlite::Connection;

use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};

use super::inspect::{table_columns, table_exists};
use super::types::{
    ColumnDeclaration, ColumnReconcile, ColumnRename, MergedColumn, StoreRenames, TableReconcile,
    TableRename,
};

/// One write the reconcile pass decided on, applied only after every renamed object of
/// the store was checked, so a refusal leaves the store untouched.
enum Repair<'a> {
    MergeRows {
        rename: &'a TableRename,
        insert: String,
    },
    FillDefaulted {
        rename: &'a ColumnRename,
        default: &'static str,
    },
}

/// Repair every renamed object the store holds under both names. Runs first in each
/// store's open transaction on every open: an object with a merge rule is merged into
/// its canonical name and the legacy object dropped in the same transaction; any other
/// object held under both names refuses the open with
/// [`DatabaseError::SchemaConflict`] before anything is written.
pub fn reconcile(conn: &Connection, renames: &StoreRenames) -> Result<(), DatabaseError> {
    let mut repairs = Vec::new();
    for rename in renames.tables {
        if !table_exists(conn, rename.legacy).map_err(inspect_failure(renames, rename.legacy))?
            || !table_exists(conn, rename.canonical)
                .map_err(inspect_failure(renames, rename.canonical))?
        {
            continue;
        }
        let TableReconcile::MergeRows { columns } = rename.reconcile else {
            return Err(DatabaseError::SchemaConflict {
                store: renames.store.to_owned(),
                object: rename.object(),
            });
        };
        let insert = merge_statement(conn, renames, rename, columns)?;
        repairs.push(Repair::MergeRows { rename, insert });
    }
    for rename in renames.columns {
        let columns =
            table_columns(conn, rename.table).map_err(inspect_failure(renames, rename.table))?;
        let (Some(legacy), Some(canonical)) = (
            find_column(&columns, rename.legacy),
            find_column(&columns, rename.canonical),
        ) else {
            continue;
        };
        let ColumnReconcile::FillDefaulted { default } = rename.reconcile else {
            return Err(DatabaseError::SchemaConflict {
                store: renames.store.to_owned(),
                object: rename.object(),
            });
        };
        if !legacy.declared_like(canonical) || canonical.default_sql() != default {
            return Err(DatabaseError::SchemaConflict {
                store: renames.store.to_owned(),
                object: rename.object(),
            });
        }
        repairs.push(Repair::FillDefaulted { rename, default });
    }

    for repair in repairs {
        match repair {
            Repair::MergeRows { rename, insert } => {
                let merged = conn
                    .execute(&insert, [])
                    .map_err(statement_failure(format!(
                        "merge {} into {}",
                        rename.legacy, rename.canonical
                    )))?;
                conn.execute(&format!("DROP TABLE {}", rename.legacy), [])
                    .map_err(statement_failure(format!("drop {}", rename.legacy)))?;
                logger::info(
                    LogTag::System,
                    &format!(
                        "{}: merged {merged} rows of {} into {} and dropped {}",
                        renames.store, rename.legacy, rename.canonical, rename.legacy
                    ),
                );
            }
            Repair::FillDefaulted { rename, default } => {
                let ColumnRename {
                    table,
                    legacy,
                    canonical,
                    ..
                } = rename;
                let filled = conn
                    .execute(
                        &format!(
                            "UPDATE {table} SET {canonical} = {legacy} \
                             WHERE {canonical} IS {default} AND {legacy} IS NOT {default}"
                        ),
                        [],
                    )
                    .map_err(statement_failure(format!(
                        "fill {table}.{canonical} from {legacy}"
                    )))?;
                conn.execute(&format!("ALTER TABLE {table} DROP COLUMN {legacy}"), [])
                    .map_err(statement_failure(format!("drop {table}.{legacy}")))?;
                logger::info(
                    LogTag::System,
                    &format!(
                        "{}: filled {filled} rows of {table}.{canonical} from {legacy} and dropped {legacy}",
                        renames.store
                    ),
                );
            }
        }
    }
    Ok(())
}

/// Rename every renamed table and column the store still holds under its legacy name
/// only, tables first, and return how many were renamed. Each store calls it at its own
/// point of its open transaction; an object held under both names refuses the open with
/// [`DatabaseError::SchemaConflict`].
pub fn rename(conn: &Connection, renames: &StoreRenames) -> Result<usize, DatabaseError> {
    let mut renamed = 0;
    for rename in renames.tables {
        let has_legacy =
            table_exists(conn, rename.legacy).map_err(inspect_failure(renames, rename.legacy))?;
        let has_canonical = table_exists(conn, rename.canonical)
            .map_err(inspect_failure(renames, rename.canonical))?;
        match (has_legacy, has_canonical) {
            (true, true) => {
                return Err(DatabaseError::SchemaConflict {
                    store: renames.store.to_owned(),
                    object: rename.object(),
                })
            }
            (true, false) => {
                conn.execute(
                    &format!(
                        "ALTER TABLE {} RENAME TO {}",
                        rename.legacy, rename.canonical
                    ),
                    [],
                )
                .map_err(statement_failure(format!(
                    "rename {} to {}",
                    rename.legacy, rename.canonical
                )))?;
                renamed += 1;
            }
            (false, _) => {}
        }
    }
    for rename in renames.columns {
        let columns =
            table_columns(conn, rename.table).map_err(inspect_failure(renames, rename.table))?;
        let has_legacy = find_column(&columns, rename.legacy).is_some();
        let has_canonical = find_column(&columns, rename.canonical).is_some();
        match (has_legacy, has_canonical) {
            (true, true) => {
                return Err(DatabaseError::SchemaConflict {
                    store: renames.store.to_owned(),
                    object: rename.object(),
                })
            }
            (true, false) => {
                let ColumnRename {
                    table,
                    legacy,
                    canonical,
                    ..
                } = rename;
                conn.execute(
                    &format!("ALTER TABLE {table} RENAME COLUMN {legacy} TO {canonical}"),
                    [],
                )
                .map_err(statement_failure(format!(
                    "rename {table}.{legacy} to {canonical}"
                )))?;
                renamed += 1;
            }
            (false, _) => {}
        }
    }
    Ok(renamed)
}

/// `legacy_columns` (a `", "`-separated list) of `table` as the live table stores them.
/// A table created earlier in the same open already carries the canonical names, so each
/// renamed column is read under whichever of its two names is present.
pub fn live_column_list(
    conn: &Connection,
    renames: &StoreRenames,
    table: &str,
    legacy_columns: &str,
) -> Result<String, DatabaseError> {
    let live = table_columns(conn, table).map_err(inspect_failure(renames, table))?;
    let columns: Vec<&str> = legacy_columns
        .split(", ")
        .map(|column| {
            renames
                .columns
                .iter()
                .find(|rename| {
                    rename.table == table
                        && rename.legacy == column
                        && find_column(&live, rename.legacy).is_none()
                        && find_column(&live, rename.canonical).is_some()
                })
                .map_or(column, |rename| rename.canonical)
        })
        .collect();
    Ok(columns.join(", "))
}

/// The `INSERT OR IGNORE` that copies the legacy table's rows into the canonical table.
/// A canonical table missing a merged column, or a legacy table missing a column with no
/// fallback value, is a shape no merge rule was written for and refuses the open.
fn merge_statement(
    conn: &Connection,
    renames: &StoreRenames,
    rename: &TableRename,
    columns: &[MergedColumn],
) -> Result<String, DatabaseError> {
    let canonical = table_columns(conn, rename.canonical)
        .map_err(inspect_failure(renames, rename.canonical))?;
    let legacy =
        table_columns(conn, rename.legacy).map_err(inspect_failure(renames, rename.legacy))?;
    let mut targets = Vec::with_capacity(columns.len());
    let mut sources = Vec::with_capacity(columns.len());
    for column in columns {
        if find_column(&canonical, column.canonical).is_none() {
            return Err(DatabaseError::SchemaConflict {
                store: renames.store.to_owned(),
                object: rename.object(),
            });
        }
        let source = match (find_column(&legacy, column.legacy), column.absent) {
            (Some(_), _) => column.legacy,
            (None, Some(absent)) => absent,
            (None, None) => {
                return Err(DatabaseError::SchemaConflict {
                    store: renames.store.to_owned(),
                    object: rename.object(),
                })
            }
        };
        targets.push(column.canonical);
        sources.push(source);
    }
    Ok(format!(
        "INSERT OR IGNORE INTO {} ({}) SELECT {} FROM {}",
        rename.canonical,
        targets.join(", "),
        sources.join(", "),
        rename.legacy
    ))
}

fn find_column<'a>(columns: &'a [ColumnDeclaration], name: &str) -> Option<&'a ColumnDeclaration> {
    columns.iter().find(|column| column.name == name)
}

fn inspect_failure<'a>(
    renames: &'a StoreRenames,
    table: &'a str,
) -> impl FnOnce(rusqlite::Error) -> DatabaseError + 'a {
    move |error| {
        DatabaseError::classify_sqlite_failure(
            &format!("inspect {}.{table} schema", renames.store),
            error,
        )
    }
}

fn statement_failure(operation: String) -> impl FnOnce(rusqlite::Error) -> DatabaseError {
    move |error| DatabaseError::classify_sqlite_failure(&operation, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    const FILL_RENAMES: StoreRenames = StoreRenames {
        store: "test.db",
        tables: &[],
        columns: &[
            ColumnRename {
                table: "zero",
                legacy: "old_value",
                canonical: "new_value",
                reconcile: ColumnReconcile::FillDefaulted { default: "0" },
            },
            ColumnRename {
                table: "one",
                legacy: "old_value",
                canonical: "new_value",
                reconcile: ColumnReconcile::FillDefaulted { default: "1" },
            },
            ColumnRename {
                table: "nullable",
                legacy: "old_value",
                canonical: "new_value",
                reconcile: ColumnReconcile::FillDefaulted { default: "NULL" },
            },
        ],
    };

    const FLOW_COLUMNS: &[MergedColumn] = &[
        MergedColumn {
            canonical: "chain_id",
            legacy: "chain_id",
            absent: Some("'solana'"),
        },
        MergedColumn {
            canonical: "wallet_address",
            legacy: "wallet_address",
            absent: Some("COALESCE((SELECT value FROM metadata WHERE key = 'wallet'), '')"),
        },
        MergedColumn {
            canonical: "signature",
            legacy: "signature",
            absent: None,
        },
        MergedColumn {
            canonical: "delta",
            legacy: "old_delta",
            absent: None,
        },
    ];

    const MERGE_RENAMES: StoreRenames = StoreRenames {
        store: "test.db",
        tables: &[TableRename {
            legacy: "old_flows",
            canonical: "flows",
            reconcile: TableReconcile::MergeRows {
                columns: FLOW_COLUMNS,
            },
        }],
        columns: &[],
    };

    const REFUSE_RENAMES: StoreRenames = StoreRenames {
        store: "test.db",
        tables: &[TableRename {
            legacy: "old_flows",
            canonical: "flows",
            reconcile: TableReconcile::Refuse,
        }],
        columns: &[ColumnRename {
            table: "zero",
            legacy: "old_value",
            canonical: "new_value",
            reconcile: ColumnReconcile::Refuse,
        }],
    };

    fn open() -> Connection {
        Connection::open_in_memory().expect("open in-memory database")
    }

    /// The whole schema and every row, so a pass that writes anything is visible.
    fn snapshot(conn: &Connection) -> Vec<String> {
        let mut statement = conn
            .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
            .expect("prepare schema read");
        let objects: Vec<(String, String, Option<String>)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("read schema")
            .collect::<rusqlite::Result<_>>()
            .expect("decode schema");
        let mut snapshot = Vec::new();
        for (kind, name, sql) in objects {
            snapshot.push(format!("{kind} {name} {sql:?}"));
            if kind == "table" {
                let mut rows = conn
                    .prepare(&format!("SELECT * FROM {name}"))
                    .expect("prepare rows");
                let width = rows.column_count();
                let mut rendered: Vec<String> = rows
                    .query_map([], |row| {
                        (0..width)
                            .map(|index| {
                                row.get::<_, rusqlite::types::Value>(index)
                                    .map(|value| format!("{value:?}"))
                            })
                            .collect::<rusqlite::Result<Vec<_>>>()
                            .map(|values| values.join("|"))
                    })
                    .expect("read rows")
                    .collect::<rusqlite::Result<_>>()
                    .expect("decode rows");
                rendered.sort();
                snapshot.extend(rendered.into_iter().map(|row| format!("{name}: {row}")));
            }
        }
        snapshot
    }

    fn values(conn: &Connection, table: &str) -> Vec<(i64, Option<i64>)> {
        conn.prepare(&format!("SELECT id, new_value FROM {table} ORDER BY id"))
            .expect("prepare values")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("read values")
            .collect::<rusqlite::Result<_>>()
            .expect("decode values")
    }

    fn seed_fill_tables(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE zero (id INTEGER PRIMARY KEY, new_value INTEGER NOT NULL DEFAULT 0, old_value INTEGER NOT NULL DEFAULT 0);
             INSERT INTO zero VALUES (1, 0, 0), (2, 0, 7), (3, 5, 0), (4, 5, 7);
             CREATE TABLE one (id INTEGER PRIMARY KEY, new_value INTEGER NOT NULL DEFAULT 1, old_value INTEGER NOT NULL DEFAULT 1);
             INSERT INTO one VALUES (1, 1, 1), (2, 1, 0), (3, 0, 1), (4, 0, 0);
             CREATE TABLE nullable (id INTEGER PRIMARY KEY, new_value INTEGER, old_value INTEGER);
             INSERT INTO nullable VALUES (1, NULL, NULL), (2, NULL, 7), (3, 5, NULL), (4, 5, 7);",
        )
        .expect("seed tables holding both names");
    }

    #[test]
    fn fill_defaulted_takes_the_legacy_value_only_where_the_canonical_one_is_the_default() {
        let conn = open();
        seed_fill_tables(&conn);

        reconcile(&conn, &FILL_RENAMES).expect("reconcile");

        // Rows: both default, legacy-only write, canonical-only write, both written.
        assert_eq!(
            values(&conn, "zero"),
            vec![(1, Some(0)), (2, Some(7)), (3, Some(5)), (4, Some(5))]
        );
        assert_eq!(
            values(&conn, "one"),
            vec![(1, Some(1)), (2, Some(0)), (3, Some(0)), (4, Some(0))]
        );
        assert_eq!(
            values(&conn, "nullable"),
            vec![(1, None), (2, Some(7)), (3, Some(5)), (4, Some(5))]
        );
        for table in ["zero", "one", "nullable"] {
            let names: Vec<String> = table_columns(&conn, table)
                .expect("inspect")
                .into_iter()
                .map(|column| column.name)
                .collect();
            assert_eq!(
                names,
                vec!["id", "new_value"],
                "{table} keeps only canonical"
            );
        }
    }

    #[test]
    fn a_second_pass_writes_nothing() {
        let conn = open();
        seed_fill_tables(&conn);
        conn.execute_batch(
            "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE flows (chain_id TEXT NOT NULL, wallet_address TEXT NOT NULL, signature TEXT NOT NULL, delta REAL, PRIMARY KEY (chain_id, wallet_address, signature));
             CREATE TABLE old_flows (chain_id TEXT NOT NULL, wallet_address TEXT NOT NULL, signature TEXT NOT NULL, old_delta REAL, PRIMARY KEY (chain_id, wallet_address, signature));
             INSERT INTO old_flows VALUES ('solana', 'w', 's1', 1.5);",
        )
        .expect("seed flows");
        reconcile(&conn, &FILL_RENAMES).expect("first reconcile");
        reconcile(&conn, &MERGE_RENAMES).expect("first merge");
        let before = snapshot(&conn);
        let changes = conn.total_changes();

        reconcile(&conn, &FILL_RENAMES).expect("second reconcile");
        reconcile(&conn, &MERGE_RENAMES).expect("second merge");
        assert_eq!(
            rename(&conn, &FILL_RENAMES).expect("rename after reconcile"),
            0
        );

        assert_eq!(snapshot(&conn), before);
        assert_eq!(conn.total_changes(), changes);
    }

    #[test]
    fn fill_defaulted_refuses_a_declaration_it_was_not_written_for() {
        let conn = open();
        conn.execute_batch(
            "CREATE TABLE zero (id INTEGER PRIMARY KEY, new_value INTEGER NOT NULL DEFAULT 0, old_value INTEGER);
             INSERT INTO zero VALUES (1, 0, 7);",
        )
        .expect("seed mismatched declarations");
        let before = snapshot(&conn);

        let error = reconcile(&conn, &FILL_RENAMES).expect_err("mismatch refuses");

        assert!(matches!(error, DatabaseError::SchemaConflict { .. }));
        assert_eq!(snapshot(&conn), before);
    }

    fn seed_flows(conn: &Connection, legacy_ddl: &str, legacy_rows: &str) {
        conn.execute_batch(&format!(
            "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO metadata VALUES ('wallet', 'w1');
             CREATE TABLE flows (chain_id TEXT NOT NULL, wallet_address TEXT NOT NULL, signature TEXT NOT NULL, delta REAL, PRIMARY KEY (chain_id, wallet_address, signature));
             CREATE INDEX idx_flows_wallet ON flows (chain_id, wallet_address);
             INSERT INTO flows VALUES ('solana', 'w1', 'shared', 2.0), ('solana', 'w1', 'canonical-only', 3.0);
             {legacy_ddl};
             {legacy_rows};"
        ))
        .expect("seed flow tables");
    }

    fn flows(conn: &Connection) -> Vec<(String, String, String, f64)> {
        conn.prepare(
            "SELECT chain_id, wallet_address, signature, delta FROM flows ORDER BY signature",
        )
        .expect("prepare flows")
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .expect("read flows")
        .collect::<rusqlite::Result<_>>()
        .expect("decode flows")
    }

    fn assert_merged(conn: &Connection) {
        let row = |signature: &str, delta: f64| {
            (
                "solana".to_owned(),
                "w1".to_owned(),
                signature.to_owned(),
                delta,
            )
        };
        assert_eq!(
            flows(conn),
            vec![
                row("canonical-only", 3.0),
                row("legacy-only", 9.0),
                row("shared", 2.0),
            ]
        );
        assert!(!table_exists(conn, "old_flows").expect("inspect"));
        let index: Option<String> = conn
            .query_row(
                "SELECT tbl_name FROM sqlite_master WHERE type = 'index' AND name = 'idx_flows_wallet'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("inspect index");
        assert_eq!(index.as_deref(), Some("flows"), "canonical index survives");
    }

    #[test]
    fn merge_rows_inserts_missing_keys_and_drops_the_legacy_table() {
        let conn = open();
        seed_flows(
            &conn,
            "CREATE TABLE old_flows (chain_id TEXT NOT NULL, wallet_address TEXT NOT NULL, signature TEXT NOT NULL, old_delta REAL, PRIMARY KEY (chain_id, wallet_address, signature))",
            "INSERT INTO old_flows VALUES ('solana', 'w1', 'shared', 8.0), ('solana', 'w1', 'legacy-only', 9.0)",
        );

        reconcile(&conn, &MERGE_RENAMES).expect("merge");

        assert_merged(&conn);
    }

    #[test]
    fn merge_rows_fills_columns_a_older_legacy_shape_lacks() {
        let conn = open();
        seed_flows(
            &conn,
            "CREATE TABLE old_flows (signature TEXT PRIMARY KEY, old_delta REAL)",
            "INSERT INTO old_flows VALUES ('shared', 8.0), ('legacy-only', 9.0)",
        );

        reconcile(&conn, &MERGE_RENAMES).expect("merge");

        assert_merged(&conn);
    }

    #[test]
    fn merge_rows_refuses_a_legacy_shape_missing_a_required_column() {
        let conn = open();
        seed_flows(
            &conn,
            "CREATE TABLE old_flows (signature TEXT PRIMARY KEY)",
            "INSERT INTO old_flows VALUES ('legacy-only')",
        );
        let before = snapshot(&conn);

        let error = reconcile(&conn, &MERGE_RENAMES).expect_err("unknown shape refuses");

        assert!(matches!(error, DatabaseError::SchemaConflict { .. }));
        assert_eq!(snapshot(&conn), before);
    }

    #[test]
    fn refuse_returns_schema_conflict_and_changes_nothing() {
        let conn = open();
        seed_fill_tables(&conn);
        seed_flows(
            &conn,
            "CREATE TABLE old_flows (signature TEXT PRIMARY KEY, old_delta REAL)",
            "INSERT INTO old_flows VALUES ('legacy-only', 9.0)",
        );
        let before = snapshot(&conn);

        let error = reconcile(&conn, &REFUSE_RENAMES).expect_err("table refuses");
        assert_eq!(error.to_string(), "test.db holds both old_flows and flows");
        assert_eq!(snapshot(&conn), before);

        conn.execute_batch("DROP TABLE old_flows")
            .expect("leave only the column conflict");
        let before = snapshot(&conn);
        let error = reconcile(&conn, &REFUSE_RENAMES).expect_err("column refuses");
        assert_eq!(
            error.to_string(),
            "test.db holds both zero.old_value and zero.new_value"
        );
        assert_eq!(snapshot(&conn), before);

        let error = rename(&conn, &REFUSE_RENAMES).expect_err("rename refuses both names");
        assert!(matches!(error, DatabaseError::SchemaConflict { .. }));
        assert_eq!(snapshot(&conn), before);
    }

    #[test]
    fn a_refusal_anywhere_in_the_store_leaves_every_mergeable_object_unwritten() {
        let conn = open();
        seed_fill_tables(&conn);
        const MIXED: StoreRenames = StoreRenames {
            store: "test.db",
            tables: &[],
            columns: &[
                FILL_RENAMES.columns[0],
                ColumnRename {
                    table: "one",
                    legacy: "old_value",
                    canonical: "new_value",
                    reconcile: ColumnReconcile::Refuse,
                },
            ],
        };
        let before = snapshot(&conn);

        reconcile(&conn, &MIXED).expect_err("refusal wins");

        assert_eq!(snapshot(&conn), before);
    }

    #[test]
    fn rename_moves_legacy_only_objects_and_live_columns_follow() {
        let conn = open();
        conn.execute_batch(
            "CREATE TABLE zero (id INTEGER PRIMARY KEY, old_value INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE one (id INTEGER PRIMARY KEY, new_value INTEGER NOT NULL DEFAULT 1);
             CREATE TABLE old_flows (signature TEXT PRIMARY KEY, old_delta REAL);",
        )
        .expect("seed legacy-only store");
        assert_eq!(
            live_column_list(&conn, &FILL_RENAMES, "one", "id, old_value").expect("live"),
            "id, new_value"
        );
        assert_eq!(
            live_column_list(&conn, &FILL_RENAMES, "zero", "id, old_value").expect("live"),
            "id, old_value"
        );

        assert_eq!(rename(&conn, &FILL_RENAMES).expect("rename columns"), 1);
        assert_eq!(rename(&conn, &MERGE_RENAMES).expect("rename tables"), 1);

        assert!(table_exists(&conn, "flows").expect("inspect"));
        assert!(!table_exists(&conn, "old_flows").expect("inspect"));
        let names: Vec<String> = table_columns(&conn, "zero")
            .expect("inspect")
            .into_iter()
            .map(|column| column.name)
            .collect();
        assert_eq!(names, vec!["id", "new_value"]);
        assert_eq!(
            live_column_list(&conn, &FILL_RENAMES, "zero", "id, old_value").expect("live"),
            "id, new_value"
        );
        assert_eq!(rename(&conn, &FILL_RENAMES).expect("rename again"), 0);
    }

    /// Every store's rename declarations. A store that gains renames is added here.
    const STORES: [&StoreRenames; 7] = [
        &crate::ohlcvs::column_names::RENAMES,
        &crate::pools::database::column_names::RENAMES,
        &crate::tokens::column_names::RENAMES,
        &crate::positions::database::column_names::RENAMES,
        &crate::trader::copy::column_names::RENAMES,
        &crate::transactions::database::column_names::RENAMES,
        &crate::wallets::balance_monitor::column_names::RENAMES,
    ];

    /// Only an object a released build re-creates on a migrated store may carry a merge
    /// rule; every other renamed object held under both names has unknown provenance
    /// and must refuse. A new merge rule, or a refusal relaxed, fails here first.
    #[test]
    fn only_objects_released_builds_recreate_carry_a_merge_rule() {
        let mut merged = Vec::new();
        let mut refused = 0;
        for store in STORES {
            for rename in store.tables {
                match rename.reconcile {
                    TableReconcile::MergeRows { .. } => {
                        merged.push(format!("{} {}", store.store, rename.legacy))
                    }
                    TableReconcile::Refuse => refused += 1,
                }
            }
            for rename in store.columns {
                match rename.reconcile {
                    ColumnReconcile::FillDefaulted { default } => merged.push(format!(
                        "{} {}.{} default {default}",
                        store.store, rename.table, rename.legacy
                    )),
                    ColumnReconcile::Refuse => refused += 1,
                }
            }
        }
        merged.sort();
        assert_eq!(
            merged,
            vec![
                "copy_trading.db copy_paper_positions.peak_price_sol default NULL",
                "ohlcvs.db ohlcv_pools.is_sol_pair default 1",
                "transactions.db processed_transactions.fee_sol default 0",
                "transactions.db processed_transactions.sol_delta default NULL",
                "wallet.db sol_flow_cache",
                "wallet.db wallet_snapshots.total_equity_sol default NULL",
            ]
        );
        assert_eq!(refused, 29);
    }

    /// Each store's declarations name every object once, and no legacy name is also a
    /// canonical name in the same table, so rename order cannot matter.
    #[test]
    fn declarations_name_each_object_once() {
        for store in STORES {
            let mut seen = std::collections::HashSet::new();
            for rename in store.tables {
                assert!(
                    seen.insert(format!("table {}", rename.legacy)),
                    "{}",
                    store.store
                );
                assert!(
                    seen.insert(format!("table {}", rename.canonical)),
                    "{}",
                    store.store
                );
            }
            for rename in store.columns {
                assert!(
                    seen.insert(format!("{}.{}", rename.table, rename.legacy)),
                    "{} {}.{}",
                    store.store,
                    rename.table,
                    rename.legacy
                );
                assert!(
                    seen.insert(format!("{}.{}", rename.table, rename.canonical)),
                    "{} {}.{}",
                    store.store,
                    rename.table,
                    rename.canonical
                );
            }
        }
    }
}
