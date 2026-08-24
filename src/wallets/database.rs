//! Wallet database operations
//!
//! SQLite storage for multi-wallet management with encrypted private keys.

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::errors::{DatabaseError, IoError};
use crate::paths::get_wallets_db_path;
use crate::wallets::Error;
use crate::{chains::ChainId, database};

mod schema;
mod token_balances;
mod wallet_queries;

use crate::database::WriteTransaction;
use schema::{TOKEN_BALANCES_SCHEMA, WALLETS_INDEXES, WALLETS_SCHEMA};

/// Wallets database with connection pooling
pub struct WalletsDatabase {
    pool: Pool<SqliteConnectionManager>,
    chain: ChainId,
}

impl WalletsDatabase {
    /// Create or open the wallets database
    pub fn new(chain: ChainId) -> Result<Self, Error> {
        let db_path = get_wallets_db_path();

        // Ensure data directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(IoError::from)?;
        }

        let manager = SqliteConnectionManager::file(&db_path)
            .with_init(|c| database::configure_connection(c, database::WALLETS_DB));
        let pool = Pool::builder()
            .max_size(3)
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(DatabaseError::from)?;

        let db = Self { pool, chain };
        db.initialize()?;

        Ok(db)
    }

    /// Get a connection from the pool
    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        self.pool.get().map_err(|e| DatabaseError::from(e).into())
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<(), Error> {
        let mut conn = self.conn()?;

        // Create tables
        conn.execute(WALLETS_SCHEMA, [])
            .map_err(DatabaseError::from)?;

        self.migrate_chain_identity(&mut conn)?;

        conn.execute(TOKEN_BALANCES_SCHEMA, [])
            .map_err(DatabaseError::from)?;

        // Create indexes
        for index_sql in WALLETS_INDEXES {
            conn.execute(index_sql, []).map_err(DatabaseError::from)?;
        }

        Ok(())
    }

    fn migrate_chain_identity(&self, conn: &mut rusqlite::Connection) -> Result<(), Error> {
        let has_chain: bool = conn
            .prepare("PRAGMA table_info(wallets)")
            .map_err(|e| Error::SchemaInspect {
                table: "wallets",
                detail: e.to_string(),
            })?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| Error::SchemaInspect {
                table: "wallets",
                detail: e.to_string(),
            })?
            .filter_map(std::result::Result::ok)
            .any(|column| column == "chain_id");
        if has_chain {
            return Ok(());
        }
        conn.execute("PRAGMA foreign_keys = OFF", [])
            .map_err(DatabaseError::from)?;
        let result = (|| -> Result<(), Error> {
            let tx = conn.write_tx().map_err(|e| Error::Migration {
                step: "begin".to_owned(),
                detail: e.to_string(),
            })?;
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
            tx.commit().map_err(|e| Error::Migration {
                step: "commit".to_owned(),
                detail: e.to_string(),
            })
        })();
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(DatabaseError::from)?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, OptionalExtension};

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
        let db = WalletsDatabase {
            pool: Pool::builder()
                .max_size(1)
                .build(SqliteConnectionManager::memory())
                .expect("build unused pool for migration helper"),
            chain: ChainId::Solana,
        };

        db.migrate_chain_identity(&mut conn)
            .expect("migrate legacy wallets database");
        // Idempotent: a second pass over the already-migrated schema is a no-op.
        db.migrate_chain_identity(&mut conn)
            .expect("repeat wallets chain migration");

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
}
