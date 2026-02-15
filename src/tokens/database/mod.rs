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
mod market;
mod security;
mod pool_data;
mod rejections;
mod blacklist;
mod priority;
mod tracking;
mod assembly;
mod async_api;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::tokens::types::{TokenError, TokenResult};

// Global database instance for easy access
static GLOBAL_DB: Mutex<Option<Arc<TokenDatabase>>> = Mutex::new(None);

/// Initialize global database (called by service)
pub fn init_global_database(db: Arc<TokenDatabase>) -> Result<(), String> {
    let mut guard = GLOBAL_DB
        .lock()
        .map_err(|e| format!("Lock poisoned: {}", e))?;
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

/// Token database with connection pool
pub struct TokenDatabase {
    pub(super) conn: Arc<Mutex<Connection>>,
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
        let conn = Connection::open(path)
            .map_err(|e| TokenError::Database(format!("Failed to open database: {}", e)))?;

        // Initialize schema
        crate::tokens::schema::initialize_schema(&conn).map_err(|e| TokenError::Database(e))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Get connection for external schema operations
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }
}

// Re-export all public items from submodules
pub use async_api::*;
pub use blacklist::*;
