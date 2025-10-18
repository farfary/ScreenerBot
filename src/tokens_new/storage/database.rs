// Database connection management and initialization

use crate::tokens_new::storage::schema::{PERFORMANCE_PRAGMAS, SCHEMA_STATEMENTS};
use log::{error, info};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Database wrapper with connection pooling
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    db_path: String,
}

impl Database {
    /// Create or open database at specified path and initialize schema
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let db_path = path.as_ref().to_string_lossy().to_string();

        info!("[TOKENS_NEW] Opening database: {}", db_path);

        let conn =
            Connection::open(&path).map_err(|e| format!("Failed to open database: {}", e))?;

        // Apply performance pragmas
        for pragma in PERFORMANCE_PRAGMAS {
            conn.execute(pragma, [])
                .map_err(|e| format!("Failed to apply pragma '{}': {}", pragma, e))?;
        }

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: db_path.clone(),
        };

        // Initialize schema
        db.initialize_schema()?;

        info!("[TOKENS_NEW] Database initialized: {}", db_path);

        Ok(db)
    }

    /// Initialize database schema (idempotent - safe to call multiple times)
    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        info!("[TOKENS_NEW] Initializing database schema...");

        for (i, statement) in SCHEMA_STATEMENTS.iter().enumerate() {
            conn.execute(statement, [])
                .map_err(|e| format!("Failed to execute schema statement {}: {}", i, e))?;
        }

        info!("[TOKENS_NEW] Schema initialization complete");

        Ok(())
    }

    /// Get database connection for operations
    pub fn get_connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Execute a simple query that doesn't return data
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> SqliteResult<usize> {
        let conn = self.conn.lock().map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Lock failed: {}", e),
            )))
        })?;

        conn.execute(sql, params)
    }

    /// Get database path
    pub fn path(&self) -> &str {
        &self.db_path
    }

    /// Get database size in bytes
    pub fn size(&self) -> Result<u64, String> {
        std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .map_err(|e| format!("Failed to get database size: {}", e))
    }

    /// Get table row counts for diagnostics
    pub fn get_table_stats(&self) -> Result<TableStats, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        let count_tokens: i64 = conn
            .query_row("SELECT COUNT(*) FROM tokens", [], |row| row.get(0))
            .unwrap_or(0);

        let count_dexscreener: i64 = conn
            .query_row("SELECT COUNT(*) FROM data_dexscreener_pools", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let count_geckoterminal: i64 = conn
            .query_row("SELECT COUNT(*) FROM data_geckoterminal_pools", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let count_rugcheck: i64 = conn
            .query_row("SELECT COUNT(*) FROM data_rugcheck_info", [], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        let count_fetch_log: i64 = conn
            .query_row("SELECT COUNT(*) FROM api_fetch_log", [], |row| row.get(0))
            .unwrap_or(0);

        Ok(TableStats {
            tokens: count_tokens as usize,
            dexscreener_pools: count_dexscreener as usize,
            geckoterminal_pools: count_geckoterminal as usize,
            rugcheck_info: count_rugcheck as usize,
            fetch_log: count_fetch_log as usize,
        })
    }

    /// Vacuum database to reclaim space
    pub fn vacuum(&self) -> Result<(), String> {
        info!("[TOKENS_NEW] Vacuuming database...");

        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Failed to lock connection: {}", e))?;

        conn.execute("VACUUM", [])
            .map_err(|e| format!("Failed to vacuum database: {}", e))?;

        info!("[TOKENS_NEW] Vacuum complete");

        Ok(())
    }
}

/// Database table statistics
#[derive(Debug, Clone)]
pub struct TableStats {
    pub tokens: usize,
    pub dexscreener_pools: usize,
    pub geckoterminal_pools: usize,
    pub rugcheck_info: usize,
    pub fetch_log: usize,
}

impl TableStats {
    pub fn total_rows(&self) -> usize {
        self.tokens
            + self.dexscreener_pools
            + self.geckoterminal_pools
            + self.rugcheck_info
            + self.fetch_log
    }
}
