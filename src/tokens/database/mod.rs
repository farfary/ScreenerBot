//! SQLite persistence layer for token metadata, market data, and blacklists.
mod assembly;
mod async_api;
mod authority;
mod blacklist;
mod helpers;
mod market;
/// Unified database operations for tokens system
///
/// Split into focused submodules:
/// - metadata: Token CRUD operations
/// - market: DexScreener and GeckoTerminal market data
/// - security: Rugcheck security data
/// - pool_data: Token pool snapshots
/// - rejections: Rejection tracking, history, and stats
/// - blacklist: Token blacklist operations
/// - priority: Priority management
/// - tracking: Update tracking and diagnostics
/// - assembly: Complex token assembly queries
/// - async_api: Async convenience wrappers
mod metadata;
mod pool_data;
mod priority;
mod rejections;
mod security;
mod tracking;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::{Arc, Mutex};

use crate::tokens::types::{TokenError, TokenResult};

// Global database instance for easy access
static GLOBAL_DB: Mutex<Option<Arc<TokenDatabase>>> = Mutex::new(None);

/// Initialize global database (called by service)
pub fn init_global_database(db: Arc<TokenDatabase>) -> Result<(), String> {
    let mut guard = GLOBAL_DB
        .lock()
        .map_err(|e| format!("Lock poisoned: {e}"))?;
    *guard = Some(db);
    Ok(())
}

/// Get global database instance
pub fn get_global_database() -> Option<Arc<TokenDatabase>> {
    GLOBAL_DB.lock().ok()?.clone()
}

/// Clear global database (called on service restart)
pub fn clear_global_database() {
    if let Ok(mut guard) = GLOBAL_DB.lock() {
        *guard = None;
    }
}

/// Token database.
///
/// A real r2d2 pool, matching every other database in the bot. It used to be a single
/// `Mutex<Connection>` shared by the whole process, which made the token database a global
/// serialization point: the filtering snapshot's full-corpus batch load held that one
/// connection for seconds, and EVERY unrelated token read queued behind it — the header's
/// counts, the wallet's balances, the positions panel. Measured against the owner's
/// database, a bare `SELECT COUNT(*)` issued during a snapshot build took 8.9s, tracking
/// the build's duration exactly rather than the ~20ms the query itself costs.
///
/// The database is in WAL mode (see `database::configure_connection`), so readers run
/// concurrently with each other and with a writer; `busy_timeout` covers writer overlap.
pub struct TokenDatabase {
    pool: Pool<SqliteConnectionManager>,
}

/// Token-level blacklist entry with metadata for diagnostics and UI
#[derive(Debug, Clone)]
pub struct TokenBlacklistRecord {
    pub mint: String,
    pub reason: String,
    pub source: String,
    pub added_at: i64,
}

impl TokenDatabase {
    /// Create new database instance
    pub fn new(path: &str) -> TokenResult<Self> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            crate::database::configure_connection(conn, crate::database::TOKENS_DB)
        });

        let pool = Pool::builder()
            .max_size(8)
            .idle_timeout(None)
            .max_lifetime(None)
            .build(manager)
            .map_err(|e| TokenError::Database(format!("Failed to create database pool: {e}")))?;

        let db = Self { pool };

        // Initialize schema
        let conn = db.conn()?;
        crate::tokens::schema::initialize_schema(&conn).map_err(TokenError::Database)?;
        drop(conn);

        Ok(db)
    }

    /// Check out a connection from the pool.
    pub(super) fn conn(&self) -> TokenResult<PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| TokenError::Database(format!("Failed to get connection: {e}")))
    }
}

// Re-export all public items from submodules
pub use async_api::*;
pub use blacklist::*;
