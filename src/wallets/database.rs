// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet database operations
//!
//! SQLite storage for the trading wallet and the export-only other wallets,
//! with encrypted private keys.

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Transaction};

use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::secure_storage::{decrypt_private_key, EncryptedData};
use crate::wallets::Error;
use crate::{chains::ChainId, database};

mod schema;
mod wallet_queries;

use schema::{DROP_RETIRED_TABLES, WALLETS_INDEXES, WALLETS_SCHEMA};

/// Wallets database with connection pooling
pub struct WalletsDatabase {
    pool: Pool<SqliteConnectionManager>,
    chain: ChainId,
}

/// The schema of `wallets.db`: the wallets with their encrypted keys and the
/// watched wallets.
///
/// Version 2 drops the per-wallet token balance cache. The oldest reader stays
/// version 1: a version 1 core re-creates that cache empty on open and loses
/// nothing, and the store is the vault, whose readers must not be fenced out over
/// a cache.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: 2,
    min_reader_version: 1,
    baseline: 1,
    renames: None,
    step,
    legacy_stamp: None,
};

fn step(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    WalletsDatabase::create_schema(tx).map_err(|error| match error {
        Error::Database(error) => error,
        other => DatabaseError::Migration {
            store: "wallets.db".to_owned(),
            detail: other.to_string(),
        },
    })
}

/// Whether a copy of the vault still opens its keys: true when it holds no key or at
/// least one key decrypts on this machine. A snapshot that fails it is never restored.
pub(crate) fn snapshot_keys_decrypt(conn: &Connection) -> Result<bool, DatabaseError> {
    let classify = |e| DatabaseError::classify_sqlite_failure("read snapshot wallet keys", e);
    let mut statement = conn
        .prepare("SELECT encrypted_key, nonce FROM wallets")
        .map_err(classify)?;
    let keys = statement
        .query_map([], |row| {
            Ok(EncryptedData {
                ciphertext: row.get(0)?,
                nonce: row.get(1)?,
            })
        })
        .map_err(classify)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(classify)?;
    Ok(keys.is_empty() || keys.iter().any(|key| decrypt_private_key(key).is_ok()))
}

impl WalletsDatabase {
    /// The wallets database of `chain`, opened by the storage phase.
    pub fn new(chain: ChainId) -> Result<Self, Error> {
        let pool = database::open::store(&database::catalog::WALLETS)?;
        Ok(Self { pool, chain })
    }

    /// Snapshot the vault after a key was added. The change stands whatever happens to
    /// the snapshot; a failed one is logged.
    fn snapshot_after_key_change(conn: &Connection) {
        let Some(path) = conn.path().filter(|path| !path.is_empty()) else {
            return;
        };
        if let Err(error) = database::backup::snapshot::take(
            &database::catalog::WALLETS,
            std::path::Path::new(path),
            database::backup::snapshot::SnapshotKind::KeyChange,
        ) {
            logger::warning(
                LogTag::Wallet,
                &format!("Wallet keys changed but the vault snapshot failed: {error}"),
            );
        }
    }

    /// Get a connection from the pool
    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        self.pool.get().map_err(|e| DatabaseError::from(e).into())
    }

    /// Creates and migrates every table of the store, the watched wallets' included.
    fn create_schema(tx: &Transaction<'_>) -> Result<(), Error> {
        // An older core re-creates the retired cache whenever it opens the store, so
        // every open drops it rather than only the upgrade to version 2.
        ddl::run_batch(tx, "drop retired wallet tables", DROP_RETIRED_TABLES)?;
        ddl::run(tx, "create wallets table", WALLETS_SCHEMA)?;
        Self::migrate_chain_identity(tx)?;
        for index_sql in WALLETS_INDEXES {
            ddl::run(tx, "create wallets index", index_sql)?;
        }
        crate::wallets::watch::database::create_schema(tx)
    }

    /// Rebuilds the pre-chain-identity `wallets` table onto `UNIQUE(chain_id, address)`.
    /// Runs inside the open's transaction with foreign keys off.
    fn migrate_chain_identity(tx: &Transaction<'_>) -> Result<(), Error> {
        if ddl::has_column(tx, "wallets", "chain_id")? {
            return Ok(());
        }
        tx.execute(
            "CREATE TABLE wallets__chain_v1 (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL DEFAULT 'solana', name TEXT NOT NULL, address TEXT NOT NULL, encrypted_key TEXT NOT NULL, nonce TEXT NOT NULL, role TEXT NOT NULL DEFAULT 'secondary', wallet_type TEXT NOT NULL DEFAULT 'generated', created_at TEXT NOT NULL DEFAULT (datetime('now')), last_used_at TEXT, notes TEXT, is_active INTEGER NOT NULL DEFAULT 1, UNIQUE(chain_id, address))",
            [],
        ).map_err(|e| Error::Migration { step: "create chain-aware table".to_owned(), detail: e.to_string() })?;
        let before: i64 = tx
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .map_err(|e| Error::Migration {
                step: "count wallets".to_owned(),
                detail: e.to_string(),
            })?;
        tx.execute("INSERT INTO wallets__chain_v1 (id, chain_id, name, address, encrypted_key, nonce, role, wallet_type, created_at, last_used_at, notes, is_active) SELECT id, 'solana', name, address, encrypted_key, nonce, role, wallet_type, created_at, last_used_at, notes, is_active FROM wallets", [])
            .map_err(|e| Error::Migration { step: "copy wallets".to_owned(), detail: e.to_string() })?;
        let after: i64 = tx
            .query_row("SELECT COUNT(*) FROM wallets__chain_v1", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::Migration {
                step: "count migrated wallets".to_owned(),
                detail: e.to_string(),
            })?;
        if before != after {
            return Err(Error::Migration {
                step: "row count check".to_owned(),
                detail: format!("row count mismatch: {before} != {after}"),
            });
        }
        tx.execute("DROP TABLE wallets", [])
            .map_err(|e| Error::Migration {
                step: "drop legacy table".to_owned(),
                detail: e.to_string(),
            })?;
        tx.execute("ALTER TABLE wallets__chain_v1 RENAME TO wallets", [])
            .map_err(|e| Error::Migration {
                step: "rename migrated table".to_owned(),
                detail: e.to_string(),
            })?;
        let fk_errors: i64 = tx
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| Error::Migration {
                step: "verify foreign keys".to_owned(),
                detail: e.to_string(),
            })?;
        if fk_errors != 0 {
            return Err(Error::Migration {
                step: "foreign key check".to_owned(),
                detail: format!("found {fk_errors} errors"),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    fn legacy_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch(
            "CREATE TABLE wallets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                address TEXT NOT NULL UNIQUE,
                encrypted_key TEXT NOT NULL,
                nonce TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'secondary',
                wallet_type TEXT NOT NULL DEFAULT 'generated',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at TEXT,
                notes TEXT,
                is_active INTEGER NOT NULL DEFAULT 1
            );
            INSERT INTO wallets (id, name, address, encrypted_key, nonce, role, wallet_type, is_active)
            VALUES (1, 'main', 'AAAA', 'enc', 'nonce', 'primary', 'imported', 1),
                   (2, 'sub', 'BBBB', 'enc2', 'nonce2', 'secondary', 'generated', 1);",
        )
        .expect("seed legacy wallets schema");
        conn
    }

    #[test]
    fn legacy_wallets_migrate_to_chain_scoped_schema_losslessly_and_idempotently() {
        let mut conn = legacy_connection();
        let upgrade = |conn: &mut Connection| {
            database::open::upgrade_connection(
                conn,
                &database::catalog::WALLETS,
                vec![ChainId::Solana],
            )
        };

        upgrade(&mut conn).expect("migrate legacy wallets database");
        // Idempotent: a second pass over the already-migrated schema is a no-op.
        upgrade(&mut conn).expect("repeat wallets chain migration");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .expect("count migrated wallets");
        assert_eq!(count, 2, "row count must survive migration");

        let addresses: Vec<(String, String)> = conn
            .prepare("SELECT chain_id, address FROM wallets ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            addresses,
            vec![
                ("solana".to_owned(), "AAAA".to_owned()),
                ("solana".to_owned(), "BBBB".to_owned()),
            ],
            "every legacy row is assigned chain_id = solana, address preserved"
        );

        // Chain-aware uniqueness: same address under a different (hypothetical)
        // chain is not a conflict — the UNIQUE index is (chain_id, address).
        conn.execute(
            "INSERT INTO wallets (chain_id, name, address, encrypted_key, nonce) VALUES ('other-chain', 'x', 'AAAA', 'enc', 'nonce')",
            [],
        )
        .expect("chain-scoped unique index allows the same address under a different chain");

        assert!(conn
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap()
            .is_none());
    }

    /// The retired token balance cache, with its indexes and rows, leaves the store
    /// on upgrade and on every later open, and every wallet row stays.
    #[test]
    fn the_retired_token_balance_cache_is_dropped_and_wallets_stay() {
        let mut conn = legacy_connection();
        conn.execute_batch(
            "CREATE TABLE wallet_token_balances (
                wallet_id INTEGER NOT NULL,
                mint TEXT NOT NULL,
                balance INTEGER NOT NULL,
                ui_amount REAL NOT NULL,
                decimals INTEGER NOT NULL,
                symbol TEXT,
                name TEXT,
                is_token_2022 INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (wallet_id, mint),
                FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
            );
            CREATE INDEX idx_token_balances_wallet ON wallet_token_balances(wallet_id);
            CREATE INDEX idx_token_balances_mint ON wallet_token_balances(mint);
            INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, updated_at)
            VALUES (1, 'mint', 5, 0.000000005, 9, '2025-01-02');",
        )
        .expect("seed the retired cache");
        let upgrade = |conn: &mut Connection| {
            database::open::upgrade_connection(
                conn,
                &database::catalog::WALLETS,
                vec![ChainId::Solana],
            )
        };

        upgrade(&mut conn).expect("upgrade the wallets store");
        // A core without the drop re-creates the cache; the next open drops it again.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS wallet_token_balances (wallet_id INTEGER NOT NULL, mint TEXT NOT NULL, PRIMARY KEY (wallet_id, mint));",
        )
        .expect("re-create the cache as an older core does");
        upgrade(&mut conn).expect("reopen the wallets store");

        let retired: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name IN ('wallet_token_balances', 'idx_token_balances_wallet', 'idx_token_balances_mint')",
                [],
                |row| row.get(0),
            )
            .expect("look for the retired objects");
        assert_eq!(retired, 0, "the cache and its indexes are gone");
        let wallets: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .expect("count wallets");
        assert_eq!(wallets, 2, "every wallet row stays");
        let ledger: (i64, i64) = conn
            .query_row(
                "SELECT version, min_reader_version FROM store_schema WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read the store ledger");
        assert_eq!(ledger, (2, 1));
    }

    /// Every key added to the vault leaves a snapshot to restore the vault from.
    #[test]
    fn every_key_change_snapshots_the_vault() {
        use crate::database::backup::snapshot::{self, SnapshotKind};
        use crate::wallets::types::{WalletRole, WalletType};

        let directory = tempfile::tempdir().expect("temp dir");
        let path = directory.path().join("wallets.db");
        let db = WalletsDatabase {
            pool: database::open::pool_at(
                &database::catalog::WALLETS,
                &path,
                vec![ChainId::Solana],
            )
            .expect("open vault"),
            chain: ChainId::Solana,
        };
        let insert = |address: &str| {
            db.insert_wallet(
                address,
                address,
                "ciphertext",
                "nonce",
                WalletRole::Secondary,
                WalletType::Generated,
                None,
            )
            .expect("insert wallet")
        };
        insert("AAAA");
        insert("BBBB");

        let snapshots = snapshot::list(&path);
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots
            .iter()
            .all(|snapshot| snapshot.kind == SnapshotKind::KeyChange));
        let newest = Connection::open(&snapshots[0].path).expect("open newest snapshot");
        let addresses: i64 = newest
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .expect("count wallets");
        assert_eq!(
            addresses, 2,
            "the newest snapshot holds the vault after the second key"
        );
    }
}
