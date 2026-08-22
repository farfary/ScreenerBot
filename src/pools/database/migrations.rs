//! Pools database schema migrations — legacy-to-chain-scoped upgrade run by
//! `operations::PoolsDatabase::initialize`.

use rusqlite::{Connection, OptionalExtension};

const POOLS_SCHEMA_VERSION: i64 = 1;

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| format!("Failed to inspect {table} schema: {e}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to inspect {table} columns: {e}"))?;
    let has_column = columns.filter_map(Result::ok).any(|name| name == column);
    Ok(has_column)
}

/// Compares row counts between a legacy table and its migrated replacement inside
/// the same transaction. Propagates the count query's own failure instead of
/// treating it as zero rows — a `COUNT(*)` failure (locked table, missing table)
/// must never be silently read as "0 rows, counts match" and let the integrity
/// guard pass vacuously.
fn verify_migrated_row_count(
    tx: &rusqlite::Transaction,
    old: &str,
    new: &str,
) -> Result<(), String> {
    let old_count: i64 = tx
        .query_row(&format!("SELECT COUNT(*) FROM {old}"), [], |row| row.get(0))
        .map_err(|e| format!("Failed to read {old} row count for migration validation: {e}"))?;
    let new_count: i64 = tx
        .query_row(&format!("SELECT COUNT(*) FROM {new}"), [], |row| row.get(0))
        .map_err(|e| format!("Failed to validate migrated {new} row count: {e}"))?;
    if old_count != new_count {
        return Err(format!(
            "Pools migration row-count mismatch for {old}: {old_count} != {new_count}"
        ));
    }
    Ok(())
}

pub(super) fn migrate_schema(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS price_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL,
            price_usd REAL NOT NULL, price_sol REAL NOT NULL, confidence REAL NOT NULL, slot INTEGER NOT NULL,
            timestamp_unix INTEGER NOT NULL, sol_reserves REAL NOT NULL, token_reserves REAL NOT NULL,
            source_pool TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(mint, pool_address, timestamp_unix)
        );
        CREATE TABLE IF NOT EXISTS blacklist_accounts (
            account_pubkey TEXT PRIMARY KEY, reason TEXT NOT NULL, source TEXT, pool_id TEXT, token_mint TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS blacklist_pools (
            pool_id TEXT PRIMARY KEY, reason TEXT NOT NULL, token_mint TEXT, program_id TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("Failed to create legacy pools tables for migration: {e}"))?;

    // `user_version` gates the structural column check below: once it records
    // the current schema version, every later boot skips straight past this
    // block instead of re-running `PRAGMA table_info` on three tables. It is
    // only a fast-path — the structural check (not the version number alone)
    // is still what decides whether a real migration is needed the first time,
    // so a DB whose version was never bumped is migrated correctly regardless.
    let schema_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("Failed to read pools schema version: {e}"))?;

    if schema_version < POOLS_SCHEMA_VERSION {
        let price_history_has_chain = table_has_column(conn, "price_history", "chain_id")?;
        let accounts_has_chain = table_has_column(conn, "blacklist_accounts", "chain_id")?;
        let pools_has_chain = table_has_column(conn, "blacklist_pools", "chain_id")?;

        if !price_history_has_chain || !accounts_has_chain || !pools_has_chain {
            let tx = conn
                .unchecked_transaction()
                .map_err(|e| format!("Failed to start pools schema migration: {e}"))?;

            tx.execute_batch(
                "CREATE TABLE price_history_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id TEXT NOT NULL,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                price_usd REAL NOT NULL,
                price_sol REAL NOT NULL,
                confidence REAL NOT NULL,
                slot INTEGER NOT NULL,
                timestamp_unix INTEGER NOT NULL,
                sol_reserves REAL NOT NULL,
                token_reserves REAL NOT NULL,
                source_pool TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(chain_id, mint, pool_address, timestamp_unix)
            );
            CREATE TABLE blacklist_accounts_new (
                chain_id TEXT NOT NULL,
                account_pubkey TEXT NOT NULL,
                reason TEXT NOT NULL,
                source TEXT,
                pool_id TEXT,
                token_mint TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY(chain_id, account_pubkey)
            );
            CREATE TABLE blacklist_pools_new (
                chain_id TEXT NOT NULL,
                pool_id TEXT NOT NULL,
                reason TEXT NOT NULL,
                token_mint TEXT,
                program_id TEXT,
                error_count INTEGER DEFAULT 1,
                first_failed_at INTEGER NOT NULL,
                last_failed_at INTEGER NOT NULL,
                added_at INTEGER NOT NULL,
                PRIMARY KEY(chain_id, pool_id)
            );",
            )
            .map_err(|e| format!("Failed to create chain-aware pools tables: {e}"))?;

            if price_history_has_chain {
            tx.execute("INSERT INTO price_history_new SELECT * FROM price_history", [])
        } else {
            tx.execute(
                "INSERT INTO price_history_new (id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, source_pool, created_at)
                 SELECT id, 'solana', mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, source_pool, created_at FROM price_history",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate price history: {e}"))?;
            if accounts_has_chain {
            tx.execute("INSERT INTO blacklist_accounts_new SELECT * FROM blacklist_accounts", [])
        } else {
            tx.execute(
                "INSERT INTO blacklist_accounts_new (chain_id, account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at)
                 SELECT 'solana', account_pubkey, reason, source, pool_id, token_mint, error_count, first_failed_at, last_failed_at, added_at FROM blacklist_accounts",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate account blacklist: {e}"))?;
            if pools_has_chain {
            tx.execute("INSERT INTO blacklist_pools_new SELECT * FROM blacklist_pools", [])
        } else {
            tx.execute(
                "INSERT INTO blacklist_pools_new (chain_id, pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at)
                 SELECT 'solana', pool_id, reason, token_mint, program_id, error_count, first_failed_at, last_failed_at, added_at FROM blacklist_pools",
                [],
            )
        }
        .map_err(|e| format!("Failed to migrate pool blacklist: {e}"))?;

            for (old, new) in [
                ("price_history", "price_history_new"),
                ("blacklist_accounts", "blacklist_accounts_new"),
                ("blacklist_pools", "blacklist_pools_new"),
            ] {
                verify_migrated_row_count(&tx, old, new)?;
            }

            tx.execute_batch(
                "DROP TABLE IF EXISTS price_history;
             DROP TABLE IF EXISTS blacklist_accounts;
             DROP TABLE IF EXISTS blacklist_pools;
             ALTER TABLE price_history_new RENAME TO price_history;
             ALTER TABLE blacklist_accounts_new RENAME TO blacklist_accounts;
             ALTER TABLE blacklist_pools_new RENAME TO blacklist_pools;",
            )
            .map_err(|e| format!("Failed to replace legacy pools tables: {e}"))?;
            tx.commit()
                .map_err(|e| format!("Failed to commit pools schema migration: {e}"))?;
        }
    }

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_price_history_chain_mint_timestamp ON price_history(chain_id, mint, timestamp_unix DESC);
         CREATE INDEX IF NOT EXISTS idx_price_history_chain_pool_timestamp ON price_history(chain_id, pool_address, timestamp_unix DESC);
         CREATE INDEX IF NOT EXISTS idx_price_history_created_at ON price_history(created_at);
         CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_pool ON blacklist_accounts(chain_id, pool_id);
         CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_token ON blacklist_accounts(chain_id, token_mint);
         CREATE INDEX IF NOT EXISTS idx_blacklist_pools_chain_token ON blacklist_pools(chain_id, token_mint);",
    )
    .map_err(|e| format!("Failed to create chain-aware pools indexes: {e}"))?;
    let foreign_key_violation: Option<String> = conn
        .query_row("PRAGMA foreign_key_check", [], |row| row.get(0))
        .optional()
        .map_err(|e| format!("Failed to validate pools foreign keys: {e}"))?;
    if let Some(table) = foreign_key_violation {
        return Err(format!(
            "Pools migration foreign-key validation failed for {table}"
        ));
    }
    conn.pragma_update(None, "user_version", POOLS_SCHEMA_VERSION)
        .map_err(|e| format!("Failed to record pools schema version: {e}"))?;
    Ok(())
}

#[cfg(test)]
pub(super) fn legacy_connection() -> Connection {
    let conn = Connection::open_in_memory().expect("open test database");
    conn.execute_batch(
        "CREATE TABLE price_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT, mint TEXT NOT NULL, pool_address TEXT NOT NULL,
            price_usd REAL NOT NULL, price_sol REAL NOT NULL, confidence REAL NOT NULL, slot INTEGER NOT NULL,
            timestamp_unix INTEGER NOT NULL, sol_reserves REAL NOT NULL, token_reserves REAL NOT NULL,
            source_pool TEXT, created_at TEXT NOT NULL, UNIQUE(mint, pool_address, timestamp_unix)
        );
        CREATE TABLE blacklist_accounts (
            account_pubkey TEXT PRIMARY KEY, reason TEXT NOT NULL, source TEXT, pool_id TEXT, token_mint TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );
        CREATE TABLE blacklist_pools (
            pool_id TEXT PRIMARY KEY, reason TEXT NOT NULL, token_mint TEXT, program_id TEXT,
            error_count INTEGER DEFAULT 1, first_failed_at INTEGER NOT NULL, last_failed_at INTEGER NOT NULL, added_at INTEGER NOT NULL
        );
        INSERT INTO price_history VALUES (1, 'mint', 'pool', 1.0, 2.0, 1.0, 7, 10, 3.0, 4.0, NULL, '2026-01-01T00:00:00Z');
        INSERT INTO blacklist_accounts VALUES ('account', 'reason', NULL, 'pool', 'mint', 1, 1, 1, 1);
        INSERT INTO blacklist_pools VALUES ('pool', 'reason', 'mint', NULL, 1, 1, 1, 1);",
    )
    .expect("seed legacy schema");
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;

    #[test]
    fn legacy_schema_migrates_to_solana_without_losing_rows_and_is_idempotent() {
        let mut conn = legacy_connection();
        migrate_schema(&mut conn).expect("migrate legacy pools database");
        migrate_schema(&mut conn).expect("repeat pools migration");

        for table in ["price_history", "blacklist_accounts", "blacklist_pools"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count migrated rows");
            assert_eq!(count, 1, "{table} row count");
            let chain: String = conn
                .query_row(
                    &format!("SELECT chain_id FROM {table} LIMIT 1"),
                    [],
                    |row| row.get(0),
                )
                .expect("read migrated chain");
            assert_eq!(chain, ChainId::Solana.as_str());
        }
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            POOLS_SCHEMA_VERSION
        );
        assert!(conn
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap()
            .is_none());
    }

    #[test]
    fn verify_migrated_row_count_propagates_the_count_querys_own_failure() {
        let mut conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch("CREATE TABLE new_table (id INTEGER PRIMARY KEY);")
            .expect("create replacement table");
        let tx = conn.unchecked_transaction().expect("start transaction");

        // "old_table" was never created, so the COUNT(*) query itself fails.
        // Before the fix, `.unwrap_or(0)` treated that failure as "0 rows",
        // which would have matched the empty new_table and passed vacuously.
        let err = verify_migrated_row_count(&tx, "old_table", "new_table")
            .expect_err("a failed row-count query must propagate, not read as zero");
        assert!(err.contains("old_table"));
    }

    #[test]
    fn verify_migrated_row_count_rejects_a_genuine_mismatch() {
        let mut conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch(
            "CREATE TABLE old_table (id INTEGER PRIMARY KEY);
             CREATE TABLE new_table (id INTEGER PRIMARY KEY);
             INSERT INTO old_table VALUES (1), (2);
             INSERT INTO new_table VALUES (1);",
        )
        .expect("seed mismatched tables");
        let tx = conn.unchecked_transaction().expect("start transaction");

        let err = verify_migrated_row_count(&tx, "old_table", "new_table")
            .expect_err("a real row-count mismatch must be rejected");
        assert!(err.contains("mismatch"));
    }
}
