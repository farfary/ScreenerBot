//! SQLite database operations for wallet balance monitoring

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::database;
use crate::logger::{self, LogTag};

use super::cache::{update_wallet_snapshot_status, CachedDashboardMetrics};
use super::types::*;

// Database schema version
const WALLET_SCHEMA_VERSION: u32 = 3;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_WALLET_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address TEXT NOT NULL,
    snapshot_time TEXT NOT NULL,
    sol_balance REAL NOT NULL,
    sol_balance_lamports INTEGER NOT NULL,
    total_tokens_count INTEGER NOT NULL DEFAULT 0,
    total_nfts_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_TOKEN_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 0,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_NFT_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS nft_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    account_address TEXT NOT NULL,
    name TEXT,
    symbol TEXT,
    image_url TEXT,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_WALLET_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Cache table for pre-aggregated SOL flows (one row per processed transaction)
const SCHEMA_SOL_FLOW_CACHE: &str = r#"
CREATE TABLE IF NOT EXISTS sol_flow_cache (
    signature TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    sol_delta REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_WALLET_DASHBOARD_METRICS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_dashboard_metrics (
    window_key TEXT PRIMARY KEY,
    window_hours INTEGER NOT NULL,
    snapshot_limit INTEGER NOT NULL,
    token_limit INTEGER NOT NULL,
    payload_blob BLOB NOT NULL,
    payload_format TEXT NOT NULL DEFAULT 'json-gzip',
    computed_at TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    computation_duration_ms INTEGER,
    snapshot_count INTEGER NOT NULL DEFAULT 0,
    flow_cache_rows INTEGER NOT NULL DEFAULT 0,
    last_processed_timestamp TEXT,
    last_processed_signature TEXT,
    window_start TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Indexes for fast range aggregation on cache
const FLOW_CACHE_INDEXES: &[&str] =
    &["CREATE INDEX IF NOT EXISTS idx_flow_cache_timestamp ON sol_flow_cache(timestamp DESC);"];

const DASHBOARD_METRICS_INDEXES: &[&str] = &["CREATE INDEX IF NOT EXISTS idx_dashboard_metrics_valid_until ON wallet_dashboard_metrics(valid_until DESC);"];

// Performance indexes
const WALLET_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_address ON wallet_snapshots(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_time ON wallet_snapshots(snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_id ON token_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON token_balances(mint);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_mint ON token_balances(snapshot_id, mint);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_snapshot_id ON nft_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_mint ON nft_balances(mint);",
];

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

pub(super) static GLOBAL_WALLET_DB: LazyLock<Mutex<Option<WalletDatabase>>> =
    LazyLock::new(|| Mutex::new(None));

static WALLET_METRICS_OPERATIONS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_ERRORS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_SNAPSHOTS_TAKEN: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
static WALLET_METRICS_FLOW_SYNCS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

pub(super) fn increment_operations() {
    WALLET_METRICS_OPERATIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(super) fn increment_errors() {
    WALLET_METRICS_ERRORS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(super) fn increment_snapshots() {
    WALLET_METRICS_SNAPSHOTS_TAKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(super) fn increment_flow_syncs() {
    WALLET_METRICS_FLOW_SYNCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub fn get_wallet_service_metrics() -> (u64, u64, u64, u64) {
    (
        WALLET_METRICS_OPERATIONS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_ERRORS.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_SNAPSHOTS_TAKEN.load(std::sync::atomic::Ordering::Relaxed),
        WALLET_METRICS_FLOW_SYNCS.load(std::sync::atomic::Ordering::Relaxed),
    )
}

// =============================================================================
// WALLET DATABASE
// =============================================================================

/// Database manager for wallet balance monitoring
pub struct WalletDatabase {
    pool: Pool<SqliteConnectionManager>,
    database_path: String,
    schema_version: u32,
}
impl WalletDatabase {
    /// Create new WalletDatabase with connection pooling
    pub async fn new() -> Result<Self, String> {
        let database_path = crate::paths::get_wallet_db_path();
        let database_path_str = database_path.to_string_lossy().to_string();

        logger::debug(
            LogTag::Wallet,
            &format!("Initializing wallet database at: {database_path_str}"),
        );

        // Configure connection manager with centralized PRAGMAs
        let manager = SqliteConnectionManager::file(&database_path)
            .with_init(|c| database::configure_connection(c, database::WALLET_MONITOR_DB));

        // Create connection pool
        let pool = Pool::builder()
            .max_size(3)
            .min_idle(Some(1))
            .idle_timeout(None) // SQLite: keep connections alive (WAL stability)
            .max_lifetime(None) // SQLite: no connection recycling
            .build(manager)
            .map_err(|e| format!("Failed to create wallet connection pool: {e}"))?;

        let mut db = WalletDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: WALLET_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema().await?;

        logger::debug(LogTag::Wallet, "Wallet database initialized successfully");
        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Create all tables
        conn.execute(SCHEMA_WALLET_SNAPSHOTS, [])
            .map_err(|e| format!("Failed to create wallet_snapshots table: {e}"))?;

        conn.execute(SCHEMA_TOKEN_BALANCES, [])
            .map_err(|e| format!("Failed to create token_balances table: {e}"))?;

        conn.execute(SCHEMA_NFT_BALANCES, [])
            .map_err(|e| format!("Failed to create nft_balances table: {e}"))?;

        conn.execute(SCHEMA_WALLET_METADATA, [])
            .map_err(|e| format!("Failed to create wallet_metadata table: {e}"))?;

        // Flow cache tables
        conn.execute(SCHEMA_SOL_FLOW_CACHE, [])
            .map_err(|e| format!("Failed to create sol_flow_cache table: {e}"))?;

        conn.execute(SCHEMA_WALLET_DASHBOARD_METRICS, [])
            .map_err(|e| format!("Failed to create wallet_dashboard_metrics table: {e}"))?;

        // Migrate existing schema if needed (add missing columns)
        conn.execute(
            "ALTER TABLE wallet_snapshots ADD COLUMN total_nfts_count INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .ok(); // Ignore error if column already exists

        // Create all indexes
        for index_sql in WALLET_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create wallet index: {e}"))?;
        }
        for index_sql in FLOW_CACHE_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create flow cache index: {e}"))?;
        }

        for index_sql in DASHBOARD_METRICS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create dashboard metrics index: {e}"))?;
        }

        // Set schema version
        conn.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('schema_version', ?1)",
            params![self.schema_version.to_string()],
        )
        .map_err(|e| format!("Failed to set wallet schema version: {e}"))?;

        // Store current wallet address in metadata
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {e}"))?;
        conn.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('current_wallet', ?1)",
            params![wallet_address],
        )
        .map_err(|e| format!("Failed to set current_wallet in metadata: {e}"))?;

        logger::debug(
            LogTag::Wallet,
            "Wallet database schema initialized with all tables and indexes",
        );

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get wallet database connection: {e}"))
    }

    /// Aggregate pre-cached SOL flows for a given time window
    pub fn aggregate_cached_flows_sync(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>,
    ) -> Result<(f64, f64, usize), String> {
        let conn = self.get_connection()?;
        let mut query = String::from(
            "SELECT \
                COALESCE(SUM(CASE WHEN sol_delta > 0 THEN sol_delta ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN sol_delta < 0 THEN -sol_delta ELSE 0 END), 0), \
                COUNT(signature) \
             FROM sol_flow_cache \
             WHERE timestamp >= ?1",
        );

        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(from.to_rfc3339())];
        if let Some(to_ts) = to {
            query.push_str(&format!(" AND timestamp <= ?{}", params_vec.len() + 1));
            params_vec.push(Box::new(to_ts.to_rfc3339()));
        }
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&query)
            .map_err(|e| format!("Failed to prepare cached flow aggregation query: {e}"))?;
        let (inflow, outflow, count) = stmt
            .query_row(params_refs.as_slice(), |row| {
                let inflow = row.get::<_, Option<f64>>(0)?.unwrap_or_default();
                let outflow = row.get::<_, Option<f64>>(1)?.unwrap_or_default();
                let count = row.get::<_, i64>(2)?.max(0) as usize;
                Ok((inflow, outflow, count))
            })
            .map_err(|e| format!("Failed to aggregate cached SOL flows: {e}"))?;
        Ok((inflow, outflow, count))
    }

    /// Upsert a batch of flow rows into cache
    pub fn upsert_flow_rows_sync(
        &self,
        rows: &[(String, DateTime<Utc>, f64)],
    ) -> Result<usize, String> {
        if rows.is_empty() {
            return Ok(0);
        }
        let mut conn = self.get_connection()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start flow cache transaction: {e}"))?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO sol_flow_cache(signature, timestamp, sol_delta) VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| format!("Failed to prepare flow cache upsert: {e}"))?;
            for (sig, ts, delta) in rows.iter() {
                stmt.execute(params![sig, ts.to_rfc3339(), *delta])
                    .map_err(|e| format!("Failed to upsert flow row: {e}"))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit flow cache upserts: {e}"))?;
        Ok(rows.len())
    }

    /// Get the max timestamp present in the flow cache
    pub fn get_flow_cache_max_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MAX(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare max timestamp query: {e}"))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query max timestamp: {e}"))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached max timestamp: {e}"))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get the minimum timestamp present in the flow cache (earliest record)
    pub fn get_flow_cache_min_ts_sync(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare("SELECT MIN(timestamp) FROM sol_flow_cache")
            .map_err(|e| format!("Failed to prepare min timestamp query: {e}"))?;
        let ts: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .optional()
            .map_err(|e| format!("Failed to query min timestamp: {e}"))?
            .flatten();
        if let Some(ts) = ts {
            let parsed = DateTime::parse_from_rfc3339(&ts)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Failed to parse cached min timestamp: {e}"))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    /// Get flow cache stats (row count and latest timestamp)
    pub fn get_flow_cache_stats_sync(&self) -> Result<WalletFlowCacheStats, String> {
        let conn = self.get_connection()?;
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM sol_flow_cache", [], |row| row.get(0))
            .unwrap_or(0);
        let max_ts = self.get_flow_cache_max_ts_sync()?.map(|dt| dt.to_rfc3339());
        Ok(WalletFlowCacheStats {
            rows: rows.max(0) as u64,
            max_timestamp: max_ts,
        })
    }

    pub fn get_dashboard_metrics(
        &self,
        window_key: &str,
    ) -> Result<Option<CachedDashboardMetrics>, String> {
        let conn = self.get_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, \
                    computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, \
                    last_processed_timestamp, last_processed_signature, window_start \
                 FROM wallet_dashboard_metrics WHERE window_key = ?1",
            )
            .map_err(|e| format!("Failed to prepare dashboard metrics query: {e}"))?;

        let result = stmt
            .query_row(params![window_key], |row| {
                let computed_at_str: String = row.get(6)?;
                let valid_until_str: String = row.get(7)?;
                let computed_at = DateTime::parse_from_rfc3339(&computed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            6,
                            "computed_at".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?;
                let valid_until = DateTime::parse_from_rfc3339(&valid_until_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            7,
                            "valid_until".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?;

                let last_processed_ts: Option<String> = row.get(11).ok();
                let last_processed_timestamp = last_processed_ts
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                let window_start_ts: Option<String> = row.get(13).ok();
                let window_start = window_start_ts
                    .as_deref()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                    .map(|dt| dt.with_timezone(&Utc));

                Ok(CachedDashboardMetrics {
                    window_key: row.get(0)?,
                    window_hours: row.get::<_, i64>(1)?,
                    snapshot_limit: row.get::<_, i64>(2)? as usize,
                    token_limit: row.get::<_, i64>(3)? as usize,
                    payload: row.get(4)?,
                    payload_format: row.get(5)?,
                    computed_at,
                    valid_until,
                    computation_duration_ms: row.get(8).ok(),
                    snapshot_count: row.get::<_, i64>(9)? as usize,
                    flow_cache_rows: row.get::<_, i64>(10)? as usize,
                    last_processed_timestamp,
                    last_processed_signature: row.get(12).ok(),
                    window_start,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to fetch dashboard metrics: {e}"))?;

        Ok(result)
    }

    pub fn upsert_dashboard_metrics(&self, metrics: &CachedDashboardMetrics) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "INSERT OR REPLACE INTO wallet_dashboard_metrics (
                window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format,
                computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows,
                last_processed_timestamp, last_processed_signature, window_start, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))",
            params![
                metrics.window_key,
                metrics.window_hours,
                metrics.snapshot_limit as i64,
                metrics.token_limit as i64,
                metrics.payload,
                metrics.payload_format,
                metrics.computed_at.to_rfc3339(),
                metrics.valid_until.to_rfc3339(),
                metrics.computation_duration_ms,
                metrics.snapshot_count as i64,
                metrics.flow_cache_rows as i64,
                metrics
                    .last_processed_timestamp
                    .as_ref()
                    .map(|ts| ts.to_rfc3339()),
                metrics.last_processed_signature,
                metrics.window_start.as_ref().map(|ts| ts.to_rfc3339()),
            ],
        )
        .map_err(|e| format!("Failed to upsert dashboard metrics: {e}"))?;
        Ok(())
    }

    pub fn invalidate_dashboard_metrics(&self, window_key: &str) -> Result<(), String> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM wallet_dashboard_metrics WHERE window_key = ?1",
            params![window_key],
        )
        .map_err(|e| format!("Failed to invalidate dashboard metrics: {e}"))?;
        Ok(())
    }

    pub fn cleanup_expired_metrics(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;
        let deleted = conn
            .execute(
                "DELETE FROM wallet_dashboard_metrics WHERE valid_until < datetime('now')",
                [],
            )
            .map_err(|e| format!("Failed to cleanup dashboard metrics: {e}"))?;
        Ok(deleted.max(0) as u64)
    }

    /// Save wallet snapshot with token balances (synchronous version)
    pub fn save_wallet_snapshot_sync(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64,
                    snapshot.total_nfts_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {e}"))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {e}"))?;
        }

        // Insert NFT balances
        for nft_balance in &snapshot.nft_balances {
            conn.execute(
                r#"
                INSERT INTO nft_balances (
                    snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    snapshot_id,
                    nft_balance.mint,
                    nft_balance.account_address,
                    nft_balance.name,
                    nft_balance.symbol,
                    nft_balance.image_url,
                    nft_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert NFT balance: {e}"))?;
        }

        logger::debug(
            LogTag::Wallet,
            &format!(
                "Saved wallet snapshot ID {} with {} tokens, {} NFTs for {}",
                snapshot_id,
                snapshot.token_balances.len(),
                snapshot.nft_balances.len(),
                &snapshot.wallet_address[..8]
            ),
        );

        update_wallet_snapshot_status(snapshot.snapshot_time);

        Ok(snapshot_id)
    }

    /// Save wallet snapshot with token balances (async version)
    pub async fn save_wallet_snapshot(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64,
                    snapshot.total_nfts_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {e}"))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance as i64,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert token balance: {e}"))?;
        }

        // Insert NFT balances
        for nft_balance in &snapshot.nft_balances {
            conn.execute(
                r#"
                INSERT INTO nft_balances (
                    snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    snapshot_id,
                    nft_balance.mint,
                    nft_balance.account_address,
                    nft_balance.name,
                    nft_balance.symbol,
                    nft_balance.image_url,
                    nft_balance.is_token_2022
                ],
            )
            .map_err(|e| format!("Failed to insert NFT balance: {e}"))?;
        }

        logger::debug(
            LogTag::Wallet,
            &format!(
                "Saved wallet snapshot ID {} with {} tokens, {} NFTs for {}",
                snapshot_id,
                snapshot.token_balances.len(),
                snapshot.nft_balances.len(),
                &snapshot.wallet_address[..8]
            ),
        );

        update_wallet_snapshot_status(snapshot.snapshot_time);

        Ok(snapshot_id)
    }

    /// Get SOL balance at or before a specific time (optimized for single value)
    /// Uses idx_wallet_snapshots_time index for fast descending time lookup
    pub fn get_balance_at_time_sync(
        &self,
        target_time: DateTime<Utc>,
    ) -> Result<Option<f64>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT sol_balance 
            FROM wallet_snapshots 
            WHERE datetime(snapshot_time) <= datetime(?1)
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                params![target_time.to_rfc3339()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query balance at time: {e}"))?;

        Ok(result)
    }

    /// Get the most recent snapshot timestamp (if any) without loading token data
    pub fn get_latest_snapshot_time(&self) -> Result<Option<DateTime<Utc>>, String> {
        let conn = self.get_connection()?;

        let snapshot_time_str: Option<String> = conn
            .query_row(
                r#"
            SELECT snapshot_time
            FROM wallet_snapshots
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#,
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to fetch latest wallet snapshot time: {e}"))?;

        if let Some(ts_str) = snapshot_time_str {
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|_| format!("Invalid snapshot_time stored: {ts_str}"))?
                .with_timezone(&Utc);
            Ok(Some(timestamp))
        } else {
            Ok(None)
        }
    }

    /// Get recent wallet snapshots (synchronous version)
    pub fn get_recent_snapshots_sync(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {e}"))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    total_nfts_count: row.get::<_, i64>(6)? as u32,
                    token_balances: Vec::new(), // Loaded separately if needed
                    nft_balances: Vec::new(),   // Loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {e}"))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {e}"))?);
        }

        Ok(snapshots)
    }

    /// Get recent wallet snapshots (async version)
    pub async fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {e}"))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_| {
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_owned(),
                            rusqlite::types::Type::Text,
                        )
                    })?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    total_nfts_count: row.get::<_, i64>(6)? as u32,
                    token_balances: Vec::new(), // Loaded separately if needed
                    nft_balances: Vec::new(),   // Loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {e}"))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {e}"))?);
        }

        Ok(snapshots)
    }

    /// Get wallet monitoring statistics (synchronous version)
    pub fn get_monitor_stats_sync(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {e}"))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {e}"))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {e}"))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_owned(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get wallet monitoring statistics (async version)
    pub async fn get_monitor_stats(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count snapshots: {e}"))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {e}"))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| format!("Failed to parse latest snapshot time: {e}"))?
                    .with_timezone(&Utc);
                (addr, Some(time), Some(balance), Some(count as u32))
            } else {
                ("Unknown".to_owned(), None, None, None)
            };

        // Get database file size
        let database_size = std::fs::metadata(&self.database_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(WalletMonitorStats {
            total_snapshots: total_snapshots as u64,
            latest_snapshot_time,
            wallet_address,
            current_sol_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get token balances for a specific snapshot (synchronous version)
    pub fn get_token_balances_sync(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, COALESCE(decimals, 0), is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {e}"))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get::<_, i64>(5)? as u8,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {e}"))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {e}"))?,
            );
        }

        Ok(balances)
    }

    /// Get NFT balances for a specific snapshot (synchronous version)
    pub fn get_nft_balances_sync(&self, snapshot_id: i64) -> Result<Vec<NftBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, account_address, name, symbol, image_url, is_token_2022
            FROM nft_balances 
            WHERE snapshot_id = ?1
            ORDER BY name ASC
            "#,
            )
            .map_err(|e| format!("Failed to prepare nft balances query: {e}"))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(NftBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    account_address: row.get(3)?,
                    name: row.get(4)?,
                    symbol: row.get(5)?,
                    image_url: row.get(6)?,
                    is_token_2022: row.get(7)?,
                })
            })
            .map_err(|e| format!("Failed to execute nft balances query: {e}"))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse nft balance row: {e}"))?,
            );
        }

        Ok(balances)
    }

    /// Get token balances for a specific snapshot (async version)
    pub async fn get_token_balances(&self, snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, snapshot_id, mint, balance, balance_ui, COALESCE(decimals, 0), is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {e}"))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get::<_, i64>(5)? as u8,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {e}"))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {e}"))?,
            );
        }

        Ok(balances)
    }

    /// Cleanup old snapshots (keep last 1000) - synchronous version
    pub fn cleanup_old_snapshots_sync(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {e}"))?;

        if deleted_count > 0 {
            logger::info(
                LogTag::Wallet,
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }

    /// Cleanup old snapshots (keep last 1000) - async version
    pub async fn cleanup_old_snapshots(&self) -> Result<u64, String> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots 
            WHERE id NOT IN (
                SELECT id FROM wallet_snapshots 
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                [],
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {e}"))?;

        if deleted_count > 0 {
            logger::info(
                LogTag::Wallet,
                &format!("Cleaned up {} old wallet snapshots", deleted_count),
            );
        }

        Ok(deleted_count as u64)
    }
}
