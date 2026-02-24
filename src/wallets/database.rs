//! Wallet database operations
//!
//! SQLite storage for multi-wallet management with encrypted private keys.

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::database;
use crate::paths::get_data_directory;

mod schema;
mod token_balances;
mod wallet_queries;

use schema::{TOKEN_BALANCES_SCHEMA, WALLETS_INDEXES, WALLETS_SCHEMA};

/// Wallets database with connection pooling
pub struct WalletsDatabase {
    pool: Pool<SqliteConnectionManager>,
}

impl WalletsDatabase {
    /// Create or open the wallets database
    pub fn new() -> Result<Self, String> {
        let db_path = get_data_directory().join("wallets.db");

        // Ensure data directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {e}"))?;
        }

        let manager = SqliteConnectionManager::file(&db_path)
            .with_init(|c| database::configure_connection(c, database::WALLETS_DB));
        let pool = Pool::builder()
            .max_size(3)
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(|e| format!("Failed to create wallets connection pool: {e}"))?;

        let db = Self { pool };
        db.initialize()?;

        Ok(db)
    }

    /// Get a connection from the pool
    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get connection: {e}"))
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<(), String> {
        let conn = self.conn()?;

        // Create tables
        conn.execute(WALLETS_SCHEMA, [])
            .map_err(|e| format!("Failed to create wallets table: {e}"))?;

        conn.execute(TOKEN_BALANCES_SCHEMA, [])
            .map_err(|e| format!("Failed to create token_balances table: {e}"))?;

        // Create indexes
        for index_sql in WALLETS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create index: {e}"))?;
        }

        Ok(())
    }
}
