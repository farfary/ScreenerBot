// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The legacy schema objects the last released build re-creates when it reopens a store a newer build renamed, recorded as the statements it runs.
//!
//! A released build reopening a renamed store runs its own additive steps: it adds each
//! column it checks for and does not find, and creates each table and index it declares
//! with `IF NOT EXISTS`. Its writes then land under the legacy names. The next open by the
//! current build must fold those objects into the canonical names. Each entry also names
//! the values that fold must produce.

use rusqlite::Connection;

/// One statement a released build runs when it reopens a renamed store.
pub enum ReopenStep {
    /// `ALTER TABLE {table} ADD COLUMN {column} {declaration}`, run only when the table
    /// lacks the column.
    AddColumnIfMissing {
        table: &'static str,
        column: &'static str,
        declaration: &'static str,
    },
    /// A `CREATE TABLE IF NOT EXISTS` statement for the legacy table `table`.
    CreateTable {
        table: &'static str,
        sql: &'static str,
    },
    /// A `CREATE INDEX IF NOT EXISTS` statement; a name the canonical table already holds
    /// makes it a no-op.
    CreateIndex(&'static str),
    /// A write that holds a value only under a legacy name.
    Row(&'static str),
}

/// What a released build does to one store, and what the next current open must yield.
pub struct StoreReopen {
    pub file: &'static str,
    pub steps: &'static [ReopenStep],
    /// A query returning one text value, and that value after the fold.
    pub merged: &'static [(&'static str, &'static str)],
}

/// Every store whose renamed objects the last released build re-creates.
pub const RELEASED_REOPENS: &[StoreReopen] = &[
    StoreReopen {
        file: "ohlcvs.db",
        steps: &[
            ReopenStep::AddColumnIfMissing {
                table: "ohlcv_pools",
                column: "is_sol_pair",
                declaration: "INTEGER NOT NULL DEFAULT 1",
            },
            ReopenStep::Row(
                "INSERT INTO ohlcv_pools (chain_id, mint, pool_address, dex, is_sol_pair) \
                 VALUES ('solana', 'MINT', 'POOL', 'raydium', 0)",
            ),
        ],
        merged: &[(
            "SELECT CAST(is_native_pair AS TEXT) FROM ohlcv_pools WHERE pool_address = 'POOL'",
            "0",
        )],
    },
    StoreReopen {
        file: "transactions.db",
        steps: &[
            ReopenStep::AddColumnIfMissing {
                table: "processed_transactions",
                column: "fee_sol",
                declaration: "REAL NOT NULL DEFAULT 0",
            },
            ReopenStep::AddColumnIfMissing {
                table: "processed_transactions",
                column: "sol_delta",
                declaration: "REAL",
            },
            ReopenStep::Row(
                "INSERT INTO raw_transactions (signature, wallet_address, timestamp, status, success, fee_raw) \
                 VALUES ('SIG', 'wallet', '2026-01-01T00:00:00Z', 'Finalized', 1, 7000)",
            ),
            ReopenStep::Row(
                "INSERT INTO processed_transactions (signature, wallet_address, transaction_type, type_kind, direction, fee_sol, sol_delta) \
                 VALUES ('SIG', 'wallet', '\"Unknown\"', 'unknown', 'Incoming', 0.000007, 2.5)",
            ),
        ],
        merged: &[(
            "SELECT printf('%g/%g', fee_native, native_delta) FROM processed_transactions WHERE signature = 'SIG'",
            "7e-06/2.5",
        )],
    },
    StoreReopen {
        file: "wallet.db",
        steps: &[
            ReopenStep::AddColumnIfMissing {
                table: "wallet_snapshots",
                column: "total_equity_sol",
                declaration: "REAL",
            },
            ReopenStep::CreateTable {
                table: "sol_flow_cache",
                sql: "CREATE TABLE IF NOT EXISTS sol_flow_cache (\
                      chain_id TEXT NOT NULL DEFAULT 'solana', \
                      wallet_address TEXT NOT NULL DEFAULT '', \
                      signature TEXT NOT NULL, \
                      timestamp TEXT NOT NULL, \
                      sol_delta REAL NOT NULL DEFAULT 0, \
                      created_at TEXT NOT NULL DEFAULT (datetime('now')), \
                      PRIMARY KEY (chain_id, wallet_address, signature))",
            },
            ReopenStep::CreateIndex(
                "CREATE INDEX IF NOT EXISTS idx_flow_cache_chain_wallet_timestamp \
                 ON sol_flow_cache(chain_id, wallet_address, timestamp DESC)",
            ),
            ReopenStep::Row(
                "INSERT INTO wallet_snapshots (wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_sol) \
                 VALUES ('wallet', '2026-01-01T00:00:00+00:00', 2.0, 2000000000, 7.5)",
            ),
            ReopenStep::Row(
                "INSERT INTO sol_flow_cache (wallet_address, signature, timestamp, sol_delta) \
                 VALUES ('wallet', 'SIG', '2026-01-01T00:00:00Z', 2.25)",
            ),
        ],
        merged: &[
            (
                "SELECT printf('%g', total_equity_native) FROM wallet_snapshots WHERE wallet_address = 'wallet'",
                "7.5",
            ),
            (
                "SELECT printf('%g', native_delta) FROM native_flow_cache WHERE signature = 'SIG'",
                "2.25",
            ),
        ],
    },
    StoreReopen {
        file: "copy_trading.db",
        steps: &[ReopenStep::AddColumnIfMissing {
            table: "copy_paper_positions",
            column: "peak_price_sol",
            declaration: "REAL",
        }],
        merged: &[],
    },
];

impl StoreReopen {
    /// Run the released build's steps against `conn`.
    pub fn apply(&self, conn: &Connection) {
        for step in self.steps {
            match step {
                ReopenStep::AddColumnIfMissing {
                    table,
                    column,
                    declaration,
                } => {
                    if !has_column(conn, table, column) {
                        conn.execute_batch(&format!(
                            "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
                        ))
                        .unwrap_or_else(|error| {
                            panic!("{}: add {table}.{column}: {error}", self.file)
                        });
                    }
                }
                ReopenStep::CreateTable { sql, .. }
                | ReopenStep::CreateIndex(sql)
                | ReopenStep::Row(sql) => conn
                    .execute_batch(sql)
                    .unwrap_or_else(|error| panic!("{}: {sql}: {error}", self.file)),
            }
        }
    }

    /// Panic when a legacy object the steps re-create is still present.
    pub fn assert_legacy_objects_gone(&self, conn: &Connection) {
        for step in self.steps {
            match step {
                ReopenStep::AddColumnIfMissing { table, column, .. } => assert!(
                    !has_column(conn, table, column),
                    "{} kept {table}.{column}",
                    self.file
                ),
                ReopenStep::CreateTable { table, .. } => {
                    assert!(!has_table(conn, table), "{} kept table {table}", self.file)
                }
                ReopenStep::CreateIndex(_) | ReopenStep::Row(_) => {}
            }
        }
    }

    /// Panic when a fold produced a value other than the recorded one.
    pub fn assert_merged(&self, conn: &Connection) {
        for (query, expected) in self.merged {
            let value: String = conn
                .query_row(query, [], |row| row.get(0))
                .unwrap_or_else(|error| panic!("{}: {query}: {error}", self.file));
            assert_eq!(value, *expected, "{}: {query}", self.file);
        }
    }
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
        [table, column],
        |row| row.get::<_, i64>(0),
    )
    .expect("inspect table columns")
        > 0
}

fn has_table(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .expect("inspect tables")
        > 0
}
