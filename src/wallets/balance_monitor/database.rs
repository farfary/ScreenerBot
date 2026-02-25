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

use super::types::*;

mod dashboard_metrics;
mod flow_cache;
mod metrics;
mod schema;
mod snapshots;

use schema::{
    DASHBOARD_METRICS_INDEXES, FLOW_CACHE_INDEXES, SCHEMA_NFT_BALANCES, SCHEMA_SOL_FLOW_CACHE,
    SCHEMA_TOKEN_BALANCES, SCHEMA_WALLET_DASHBOARD_METRICS, SCHEMA_WALLET_METADATA,
    SCHEMA_WALLET_SNAPSHOTS, WALLET_INDEXES, WALLET_SCHEMA_VERSION,
};

// =============================================================================
// GLOBAL DATABASE INSTANCE
// =============================================================================

pub(super) static GLOBAL_WALLET_DB: LazyLock<Mutex<Option<WalletDatabase>>> =
    LazyLock::new(|| Mutex::new(None));

pub use metrics::get_wallet_service_metrics;
pub(super) use metrics::{
    increment_errors, increment_flow_syncs, increment_operations, increment_snapshots,
};

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

    /// Get recent wallet snapshots
    pub fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
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

    /// Get wallet monitoring statistics
    pub fn get_monitor_stats(&self) -> Result<WalletMonitorStats, String> {
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
            .unwrap_or_default();

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

    /// Get token balances for a specific snapshot
    pub fn get_token_balances(
        &self,
        snapshot_id: i64,
    ) -> Result<Vec<SnapshotTokenBalance>, String> {
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
                Ok(SnapshotTokenBalance {
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

    /// Get NFT balances for a specific snapshot
    pub fn get_nft_balances(&self, snapshot_id: i64) -> Result<Vec<NftBalance>, String> {
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
            balances
                .push(balance_result.map_err(|e| format!("Failed to parse nft balance row: {e}"))?);
        }

        Ok(balances)
    }

    /// Cleanup old snapshots (keep last 1000)
    pub fn cleanup_old_snapshots(&self) -> Result<u64, String> {
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
                &format!("Cleaned up {deleted_count} old wallet snapshots"),
            );
        }

        Ok(deleted_count as u64)
    }
}
