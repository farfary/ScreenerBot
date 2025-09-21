use crate::global::is_debug_ohlcv_enabled;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use rusqlite::{ params, Connection, Row };
use serde::{ Deserialize, Serialize };
use std::fs;
use std::path::Path;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Database file path
const OHLCV_DB_PATH: &str = "data/ohlcvs.db";

/// Maximum age for OHLCV entries (7 days - increased for better analysis)
pub const MAX_OHLCV_AGE_HOURS: i64 = 168;

/// Cache expiration time for 1-minute data (2 minutes)
pub const CACHE_EXPIRY_MINUTES: i64 = 2;

// =============================================================================
// DATABASE STRUCTURES
// =============================================================================

/// OHLCV data point for database storage (SOL-denominated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbOhlcvDataPoint {
    pub id: Option<i64>,
    pub mint: String,
    pub pool_address: String,
    pub timestamp: i64, // Unix timestamp
    pub open: f64, // Open price (SOL-denominated)
    pub high: f64, // High price (SOL-denominated)
    pub low: f64, // Low price (SOL-denominated)
    pub close: f64, // Close price (SOL-denominated)
    pub volume: f64, // Volume (SOL-denominated)
    pub created_at: DateTime<Utc>,
}

impl DbOhlcvDataPoint {
    /// Create new OHLCV data point (SOL-denominated)
    pub fn new(
        mint: &str,
        pool_address: &str,
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64
    ) -> Self {
        Self {
            id: None,
            mint: mint.to_string(),
            pool_address: pool_address.to_string(),
            timestamp,
            open,
            high,
            low,
            close,
            volume,
            created_at: Utc::now(),
        }
    }

    /// Convert to OhlcvDataPoint (from geckoterminal module)
    pub fn to_ohlcv_data_point(&self) -> crate::tokens::geckoterminal::OhlcvDataPoint {
        crate::tokens::geckoterminal::OhlcvDataPoint {
            timestamp: self.timestamp,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }

    /// Create from Row (for rusqlite)
    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let created_at_str: String = row.get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e|
                rusqlite::Error::InvalidColumnType(
                    0,
                    "created_at".to_string(),
                    rusqlite::types::Type::Text
                )
            )?
            .with_timezone(&Utc);

        Ok(Self {
            id: Some(row.get("id")?),
            mint: row.get("mint")?,
            pool_address: row.get("pool_address")?,
            timestamp: row.get("timestamp")?,
            open: row.get("open")?,
            high: row.get("high")?,
            low: row.get("low")?,
            close: row.get("close")?,
            volume: row.get("volume")?,
            created_at,
        })
    }
}

/// OHLCV cache metadata for tracking freshness
#[derive(Debug, Clone)]
pub struct OhlcvCacheMetadata {
    pub mint: String,
    pub pool_address: String,
    pub data_points_count: usize,
    pub last_updated: DateTime<Utc>,
    pub last_timestamp: Option<i64>,
    pub is_expired: bool,
}

// =============================================================================
// OHLCV DATABASE
// =============================================================================

/// SQLite-based OHLCV data storage and caching
#[derive(Debug, Clone)]
pub struct OhlcvDatabase {
    db_path: String,
}

impl Default for OhlcvDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl OhlcvDatabase {
    /// Create new OHLCV database instance
    pub fn new() -> Self {
        Self {
            db_path: OHLCV_DB_PATH.to_string(),
        }
    }

    /// Initialize database and create tables
    pub fn initialize(&self) -> Result<(), String> {
        // Ensure data directory exists
        if let Some(parent) = Path::new(&self.db_path).parent() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database: {}", e)
        )?;

        // Check for legacy schema (columns with *_sol suffix) and migrate if needed
        {
            let has_legacy_columns = {
                let mut stmt = conn
                    .prepare("PRAGMA table_info('ohlcv_data')")
                    .map_err(|e| format!("Failed to inspect ohlcv_data schema: {}", e))?;
                let mut has_legacy = false;
                let mut rows = stmt.query([]).map_err(|e| format!("Failed to run PRAGMA: {}", e))?;
                while
                    let Some(row) = rows
                        .next()
                        .map_err(|e| format!("Failed to iterate PRAGMA rows: {}", e))?
                {
                    let col_name: String = row.get(1).unwrap_or_default(); // name column
                    if
                        col_name == "open_sol" ||
                        col_name == "high_sol" ||
                        col_name == "low_sol" ||
                        col_name == "close_sol" ||
                        col_name == "volume_sol"
                    {
                        has_legacy = true;
                        break;
                    }
                }
                has_legacy
            };

            if has_legacy_columns {
                // Perform migration by creating a new table and copying data
                let tx = conn
                    .unchecked_transaction()
                    .map_err(|e| format!("Failed to begin migration transaction: {}", e))?;

                // Create new table with updated column names
                tx
                    .execute(
                        "CREATE TABLE IF NOT EXISTS ohlcv_data_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        mint TEXT NOT NULL,
                        pool_address TEXT NOT NULL,
                        timestamp INTEGER NOT NULL,
                        open REAL NOT NULL,
                        high REAL NOT NULL,
                        low REAL NOT NULL,
                        close REAL NOT NULL,
                        volume REAL NOT NULL,
                        created_at TEXT NOT NULL DEFAULT (datetime('now')),
                        UNIQUE(mint, pool_address, timestamp)
                    )",
                        []
                    )
                    .map_err(|e| format!("Failed to create ohlcv_data_new table: {}", e))?;

                // Copy data from legacy columns
                tx
                    .execute(
                        "INSERT OR REPLACE INTO ohlcv_data_new (mint, pool_address, timestamp, open, high, low, close, volume, created_at)
                     SELECT mint, pool_address, timestamp, open_sol, high_sol, low_sol, close_sol, volume_sol, created_at FROM ohlcv_data",
                        []
                    )
                    .map_err(|e| format!("Failed to migrate legacy OHLCV rows: {}", e))?;

                // Drop old table and rename new one
                tx
                    .execute("DROP TABLE ohlcv_data", [])
                    .map_err(|e| format!("Failed to drop legacy ohlcv_data table: {}", e))?;
                tx
                    .execute("ALTER TABLE ohlcv_data_new RENAME TO ohlcv_data", [])
                    .map_err(|e| format!("Failed to rename migrated table: {}", e))?;

                tx.commit().map_err(|e| format!("Failed to commit migration transaction: {}", e))?;

                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "DB_MIGRATION",
                        "🔧 Migrated ohlcv_data schema from *_sol columns to unified column names"
                    );
                }
            }
        }

        // Create OHLCV data table (SOL-denominated only)
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS ohlcv_data (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pool_address TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                open REAL NOT NULL,
                high REAL NOT NULL,
                low REAL NOT NULL,
                close REAL NOT NULL,
                volume REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(mint, pool_address, timestamp)
            )",
                []
            )
            .map_err(|e| format!("Failed to create ohlcv_data table: {}", e))?;

        // Create indices for faster queries
        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ohlcv_mint_timestamp 
             ON ohlcv_data(mint, timestamp DESC)",
                []
            )
            .map_err(|e| format!("Failed to create mint timestamp index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ohlcv_pool_timestamp 
             ON ohlcv_data(pool_address, timestamp DESC)",
                []
            )
            .map_err(|e| format!("Failed to create pool timestamp index: {}", e))?;

        conn
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_ohlcv_created_at 
             ON ohlcv_data(created_at)",
                []
            )
            .map_err(|e| format!("Failed to create created_at index: {}", e))?;

        // Create metadata table for cache tracking
        conn
            .execute(
                "CREATE TABLE IF NOT EXISTS ohlcv_cache_metadata (
                mint TEXT PRIMARY KEY,
                pool_address TEXT NOT NULL,
                data_points_count INTEGER NOT NULL DEFAULT 0,
                last_updated TEXT NOT NULL,
                last_timestamp INTEGER,
                UNIQUE(mint)
            )",
                []
            )
            .map_err(|e| format!("Failed to create ohlcv_cache_metadata table: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DB_INIT",
                &format!("✅ OHLCV database initialized: {}", self.db_path)
            );
        }

        Ok(())
    }

    /// Store SOL-denominated OHLCV data points for a token
    pub fn store_sol_ohlcv_data(
        &self,
        mint: &str,
        pool_address: &str,
        sol_data_points: &[DbOhlcvDataPoint]
    ) -> Result<(), String> {
        if sol_data_points.is_empty() {
            return Ok(());
        }

        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database for SOL storage: {}", e)
        )?;

        // Begin transaction for atomicity
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to begin SOL transaction: {}", e))?;

        // Insert/update SOL OHLCV data points
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO ohlcv_data 
                      (mint, pool_address, timestamp, open, high, low, close, volume, created_at) 
                      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .map_err(|e| format!("Failed to prepare SOL insert statement: {}", e))?;

            for point in sol_data_points {
                stmt
                    .execute(
                        params![
                            mint,
                            pool_address,
                            point.timestamp,
                            point.open,
                            point.high,
                            point.low,
                            point.close,
                            point.volume,
                            point.created_at.to_rfc3339()
                        ]
                    )
                    .map_err(|e| format!("Failed to insert SOL OHLCV point: {}", e))?;
            }
        }

        // Update metadata
        let last_timestamp = sol_data_points
            .iter()
            .map(|p| p.timestamp)
            .max();
        tx
            .execute(
                "INSERT OR REPLACE INTO ohlcv_cache_metadata 
             (mint, pool_address, data_points_count, last_updated, last_timestamp) 
             VALUES (?, ?, ?, ?, ?)",
                params![
                    mint,
                    pool_address,
                    sol_data_points.len(),
                    Utc::now().to_rfc3339(),
                    last_timestamp
                ]
            )
            .map_err(|e| format!("Failed to update SOL OHLCV metadata: {}", e))?;

        tx.commit().map_err(|e| format!("Failed to commit SOL OHLCV transaction: {}", e))?;

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DB_STORE_SOL",
                &format!(
                    "💾 Stored {} SOL-denominated OHLCV points for {}",
                    sol_data_points.len(),
                    mint
                )
            );
        }

        Ok(())
    }

    /// Get OHLCV data for a token (with limit) - returns SOL-denominated data
    pub fn get_ohlcv_data(
        &self,
        mint: &str,
        limit: Option<u32>
    ) -> Result<Vec<crate::tokens::geckoterminal::OhlcvDataPoint>, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database for reading: {}", e)
        )?;

        let limit = limit.unwrap_or(100).min(1000); // Safety limit

        let mut stmt = conn
            .prepare(
                "SELECT id, mint, pool_address, timestamp, open, high, low, close, volume, created_at 
             FROM ohlcv_data 
             WHERE mint = ? 
             ORDER BY timestamp DESC 
             LIMIT ?"
            )
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let rows = stmt
            .query_map(params![mint, limit], |row| { DbOhlcvDataPoint::from_row(row) })
            .map_err(|e| format!("Failed to query OHLCV data: {}", e))?;

        let mut data_points = Vec::new();
        for row_result in rows {
            match row_result {
                Ok(db_point) => {
                    data_points.push(db_point.to_ohlcv_data_point());
                }
                Err(e) => {
                    log(LogTag::Ohlcv, "WARNING", &format!("Failed to parse OHLCV row: {}", e));
                }
            }
        }

        if is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DB_READ",
                &format!(
                    "📖 Retrieved {} SOL-denominated OHLCV points for {} from database",
                    data_points.len(),
                    mint
                )
            );
        }

        Ok(data_points)
    }

    /// Check if OHLCV data is available and fresh
    pub fn check_data_availability(&self, mint: &str) -> Result<OhlcvCacheMetadata, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database for availability check: {}", e)
        )?;

        // Get metadata
        let mut stmt = conn
            .prepare(
                "SELECT mint, pool_address, data_points_count, last_updated, last_timestamp 
             FROM ohlcv_cache_metadata 
             WHERE mint = ?"
            )
            .map_err(|e| format!("Failed to prepare metadata query: {}", e))?;

        let result = stmt.query_row(params![mint], |row| {
            let last_updated_str: String = row.get("last_updated")?;
            let last_updated = DateTime::parse_from_rfc3339(&last_updated_str)
                .map_err(|_|
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "last_updated".to_string(),
                        rusqlite::types::Type::Text
                    )
                )?
                .with_timezone(&Utc);

            let age_minutes = (Utc::now() - last_updated).num_minutes();
            let is_expired = age_minutes > CACHE_EXPIRY_MINUTES;

            Ok(OhlcvCacheMetadata {
                mint: row.get("mint")?,
                pool_address: row.get("pool_address")?,
                data_points_count: row.get("data_points_count")?,
                last_updated,
                last_timestamp: row.get("last_timestamp")?,
                is_expired,
            })
        });

        match result {
            Ok(metadata) => {
                if is_debug_ohlcv_enabled() {
                    log(
                        LogTag::Ohlcv,
                        "DB_AVAILABILITY",
                        &format!(
                            "📊 OHLCV availability for {}: {} points, fresh: {}",
                            mint,
                            metadata.data_points_count,
                            !metadata.is_expired
                        )
                    );
                }
                Ok(metadata)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // No data available
                Ok(OhlcvCacheMetadata {
                    mint: mint.to_string(),
                    pool_address: String::new(),
                    data_points_count: 0,
                    last_updated: Utc::now() - ChronoDuration::days(1), // Old timestamp
                    last_timestamp: None,
                    is_expired: true,
                })
            }
            Err(e) => Err(format!("Failed to check data availability: {}", e)),
        }
    }

    /// Clean up old OHLCV data
    pub fn cleanup_old_data(&self) -> Result<usize, String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database for cleanup: {}", e)
        )?;

        let cutoff_time = Utc::now() - ChronoDuration::hours(MAX_OHLCV_AGE_HOURS);

        // Delete old OHLCV data
        let deleted_count = conn
            .execute(
                "DELETE FROM ohlcv_data WHERE created_at < ?",
                params![cutoff_time.to_rfc3339()]
            )
            .map_err(|e| format!("Failed to delete old OHLCV data: {}", e))?;

        // Clean up metadata for mints with no data
        conn
            .execute(
                "DELETE FROM ohlcv_cache_metadata 
             WHERE mint NOT IN (SELECT DISTINCT mint FROM ohlcv_data)",
                []
            )
            .map_err(|e| format!("Failed to clean up metadata: {}", e))?;

        if deleted_count > 0 && is_debug_ohlcv_enabled() {
            log(
                LogTag::Ohlcv,
                "DB_CLEANUP",
                &format!("🧹 Cleaned up {} old OHLCV database entries", deleted_count)
            );
        }

        Ok(deleted_count)
    }

    /// Get database statistics
    pub fn get_stats(&self) -> Result<(usize, usize, usize), String> {
        let conn = Connection::open(&self.db_path).map_err(|e|
            format!("Failed to open OHLCV database for stats: {}", e)
        )?;

        // Get total data points
        let total_points: usize = conn
            .query_row("SELECT COUNT(*) FROM ohlcv_data", [], |row| row.get(0))
            .map_err(|e| format!("Failed to get total points count: {}", e))?;

        // Get unique mints count
        let unique_mints: usize = conn
            .query_row("SELECT COUNT(DISTINCT mint) FROM ohlcv_data", [], |row| row.get(0))
            .map_err(|e| format!("Failed to get unique mints count: {}", e))?;

        // Get fresh cache entries (within expiry)
        let fresh_caches: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM ohlcv_cache_metadata 
             WHERE datetime(last_updated) > datetime('now', '-2 minutes')",
                [],
                |row| row.get(0)
            )
            .map_err(|e| format!("Failed to get fresh cache count: {}", e))?;

        Ok((total_points, unique_mints, fresh_caches))
    }
}

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

use std::sync::LazyLock;
use std::sync::RwLock as StdRwLock;

/// Global OHLCV database instance
static GLOBAL_OHLCV_DB: LazyLock<StdRwLock<Option<OhlcvDatabase>>> = LazyLock::new(||
    StdRwLock::new(None)
);

/// Initialize global OHLCV database
pub fn init_ohlcv_database() -> Result<(), String> {
    let mut db_guard = GLOBAL_OHLCV_DB.write().map_err(|e|
        format!("Failed to acquire database write lock: {}", e)
    )?;

    if db_guard.is_some() {
        // Already initialized
        return Ok(());
    }

    let db = OhlcvDatabase::new();
    db.initialize()?;

    *db_guard = Some(db);
    log(LogTag::Ohlcv, "DB_INIT", "✅ Global OHLCV database initialized");
    Ok(())
}

/// Get OHLCV database instance
pub fn get_ohlcv_database() -> Result<OhlcvDatabase, String> {
    let db_guard = GLOBAL_OHLCV_DB.read().map_err(|e|
        format!("Failed to acquire database read lock: {}", e)
    )?;

    match db_guard.as_ref() {
        Some(db) => Ok(db.clone()),
        None =>
            Err("OHLCV database not initialized - call init_ohlcv_database() first".to_string()),
    }
}
