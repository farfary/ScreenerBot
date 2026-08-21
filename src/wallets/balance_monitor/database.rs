//! SQLite database operations for wallet balance monitoring

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Result as SqliteResult};
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::logger::{self, LogTag};
use crate::{chains::ChainId, database};

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

/// Flips true once `GLOBAL_WALLET_DB` holds an initialized database. A plain
/// `AtomicBool` (rather than locking `GLOBAL_WALLET_DB` itself) so callers —
/// notably the webserver snapshot collector — can check readiness without
/// contending with the monitoring loop for the same mutex, and without the
/// database handle itself ever leaving this module.
static WALLET_DB_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) fn mark_wallet_db_ready() {
    WALLET_DB_READY.store(true, std::sync::atomic::Ordering::Release);
}

/// True once the wallet-monitor database has finished initializing. Before
/// this, "no snapshot yet" is the expected pre-initialization state, not an
/// error — see `crate::webserver::snapshot::collectors::collect_wallet_snapshot`.
pub fn is_wallet_database_ready() -> bool {
    WALLET_DB_READY.load(std::sync::atomic::Ordering::Acquire)
}

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
    chain: ChainId,
}
impl WalletDatabase {
    /// Create new WalletDatabase with connection pooling
    pub async fn new(chain: ChainId) -> Result<Self, String> {
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
            chain,
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

        // Schema v4. Nullable on purpose: rows written before worth was tracked have
        // no honest equity to backfill (we would have to value yesterday's holdings at
        // today's prices), so they read back as their SOL balance via COALESCE.
        conn.execute(
            "ALTER TABLE wallet_snapshots ADD COLUMN total_equity_sol REAL",
            [],
        )
        .ok(); // Ignore error if column already exists

        self.migrate_chain_identity(&conn)?;

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
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES (?1, ?2)",
            params![
                format!("current_wallet:{}", self.chain.as_str()),
                wallet_address
            ],
        )
        .map_err(|e| format!("Failed to set current_wallet in metadata: {e}"))?;

        logger::debug(
            LogTag::Wallet,
            "Wallet database schema initialized with all tables and indexes",
        );

        Ok(())
    }

    fn migrate_chain_identity(&self, conn: &Connection) -> Result<(), String> {
        let has_chain: bool = conn
            .prepare("PRAGMA table_info(wallet_snapshots)")
            .map_err(|e| format!("Failed to inspect wallet snapshot schema: {e}"))?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| format!("Failed to inspect wallet snapshot schema: {e}"))?
            .filter_map(Result::ok)
            .any(|column| column == "chain_id");
        if has_chain {
            return Ok(());
        }
        let row_count = |table: &str| -> Result<i64, String> {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count {table} before wallet chain migration: {e}"))
        };
        let expected = [
            ("wallet_snapshots", row_count("wallet_snapshots")?),
            ("sol_flow_cache", row_count("sol_flow_cache")?),
            (
                "wallet_dashboard_metrics",
                row_count("wallet_dashboard_metrics")?,
            ),
        ];
        conn.execute_batch("PRAGMA foreign_keys = OFF; BEGIN IMMEDIATE; CREATE TABLE wallet_snapshots__chain_v1 (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL, snapshot_time TEXT NOT NULL, sol_balance REAL NOT NULL, sol_balance_lamports INTEGER NOT NULL, total_equity_sol REAL, total_tokens_count INTEGER NOT NULL DEFAULT 0, total_nfts_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))); INSERT INTO wallet_snapshots__chain_v1 (id, chain_id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_equity_sol, total_tokens_count, total_nfts_count, created_at) SELECT id, 'solana', wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_equity_sol, total_tokens_count, total_nfts_count, created_at FROM wallet_snapshots; DROP TABLE wallet_snapshots; ALTER TABLE wallet_snapshots__chain_v1 RENAME TO wallet_snapshots; CREATE TABLE sol_flow_cache__chain_v1 (chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL DEFAULT '', signature TEXT NOT NULL, timestamp TEXT NOT NULL, sol_delta REAL NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY(chain_id, wallet_address, signature)); INSERT INTO sol_flow_cache__chain_v1 (chain_id, wallet_address, signature, timestamp, sol_delta, created_at) SELECT 'solana', COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), ''), signature, timestamp, sol_delta, created_at FROM sol_flow_cache; DROP TABLE sol_flow_cache; ALTER TABLE sol_flow_cache__chain_v1 RENAME TO sol_flow_cache; CREATE TABLE wallet_dashboard_metrics__chain_v1 (chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL DEFAULT '', window_key TEXT NOT NULL, window_hours INTEGER NOT NULL, snapshot_limit INTEGER NOT NULL, token_limit INTEGER NOT NULL, payload_blob BLOB NOT NULL, payload_format TEXT NOT NULL DEFAULT 'json-gzip', computed_at TEXT NOT NULL, valid_until TEXT NOT NULL, computation_duration_ms INTEGER, snapshot_count INTEGER NOT NULL DEFAULT 0, flow_cache_rows INTEGER NOT NULL DEFAULT 0, last_processed_timestamp TEXT, last_processed_signature TEXT, window_start TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY(chain_id, wallet_address, window_key)); INSERT INTO wallet_dashboard_metrics__chain_v1 (chain_id, wallet_address, window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, last_processed_timestamp, last_processed_signature, window_start, created_at, updated_at) SELECT 'solana', COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), ''), window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, last_processed_timestamp, last_processed_signature, window_start, created_at, updated_at FROM wallet_dashboard_metrics; DROP TABLE wallet_dashboard_metrics; ALTER TABLE wallet_dashboard_metrics__chain_v1 RENAME TO wallet_dashboard_metrics; COMMIT; PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("Failed to migrate wallet chain identity: {e}"))?;
        for (table, count) in expected {
            let actual = row_count(table)?;
            if actual != count {
                return Err(format!("Wallet monitor chain migration changed {table} row count: expected {count}, found {actual}"));
            }
        }
        let fk_errors: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to verify wallet chain migration: {e}"))?;
        if fk_errors != 0 {
            return Err(format!(
                "Wallet monitor chain migration foreign key check found {fk_errors} errors"
            ));
        }
        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get wallet database connection: {e}"))
    }

    /// Get recent wallet snapshots.
    ///
    /// Token/NFT balances are NOT loaded (one query per snapshot would make the home
    /// dashboard's 30-snapshot history 30x more expensive). Callers that need holdings
    /// must use `get_wallet_worth()` (live) or `get_latest_snapshot_with_balances()`.
    pub fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;
        let wallet_address = crate::utils::get_wallet_address()
            .map_err(|e| format!("Failed to get wallet address: {e}"))?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports,
                   COALESCE(total_equity_sol, sol_balance), total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2
            ORDER BY snapshot_time DESC
            LIMIT ?3
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {e}"))?;

        let snapshot_iter = stmt
            .query_map(params![self.chain.as_str(), wallet_address, limit], |row| {
                Self::map_snapshot_row(row)
            })
            .map_err(|e| format!("Failed to execute snapshots query: {e}"))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots
                .push(snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {e}"))?);
        }

        Ok(snapshots)
    }

    /// The newest snapshot WITH its token and NFT balances loaded.
    ///
    /// Used once at startup to hydrate the live worth cache, so the header and hero
    /// show a real figure before the first collection tick rather than zeroes.
    pub fn get_latest_snapshot_with_balances(&self) -> Result<Option<WalletSnapshot>, String> {
        let mut snapshot = match self.get_recent_snapshots(1)?.into_iter().next() {
            Some(snapshot) => snapshot,
            None => return Ok(None),
        };

        if let Some(id) = snapshot.id {
            snapshot.token_balances = self.get_token_balances(id)?;
            snapshot.nft_balances = self.get_nft_balances(id)?;
        }

        Ok(Some(snapshot))
    }

    fn map_snapshot_row(row: &rusqlite::Row) -> rusqlite::Result<WalletSnapshot> {
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
            total_equity_sol: row.get(5)?,
            total_tokens_count: row.get::<_, i64>(6)? as u32,
            total_nfts_count: row.get::<_, i64>(7)? as u32,
            token_balances: Vec::new(), // Loaded separately if needed
            nft_balances: Vec::new(),   // Loaded separately if needed
        })
    }

    /// Get wallet monitoring statistics
    pub fn get_monitor_stats(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallet_snapshots WHERE chain_id = ?1",
                params![self.chain.as_str()],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count snapshots: {e}"))?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, sol_balance, total_tokens_count
            FROM wallet_snapshots 
            WHERE chain_id = ?1
            ORDER BY snapshot_time DESC 
            LIMIT 1
            "#,
                params![self.chain.as_str()],
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
              AND EXISTS (SELECT 1 FROM wallet_snapshots s WHERE s.id = token_balances.snapshot_id AND s.chain_id = ?2)
            ORDER BY balance_ui DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare token balances query: {e}"))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id, self.chain.as_str()], |row| {
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
              AND EXISTS (SELECT 1 FROM wallet_snapshots s WHERE s.id = nft_balances.snapshot_id AND s.chain_id = ?2)
            ORDER BY name ASC
            "#,
            )
            .map_err(|e| format!("Failed to prepare nft balances query: {e}"))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id, self.chain.as_str()], |row| {
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
            WHERE chain_id = ?1 AND id NOT IN (
                SELECT id FROM wallet_snapshots 
                WHERE chain_id = ?1
                ORDER BY snapshot_time DESC 
                LIMIT 1000
            )
            "#,
                params![self.chain.as_str()],
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

#[cfg(test)]
mod migration_tests {
    use super::*;

    fn legacy_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open test database");
        conn.execute_batch(
            "CREATE TABLE wallet_snapshots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                wallet_address TEXT NOT NULL,
                snapshot_time TEXT NOT NULL,
                sol_balance REAL NOT NULL,
                sol_balance_lamports INTEGER NOT NULL,
                total_equity_sol REAL,
                total_tokens_count INTEGER NOT NULL DEFAULT 0,
                total_nfts_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE sol_flow_cache (
                signature TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                sol_delta REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE wallet_dashboard_metrics (
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
            CREATE TABLE wallet_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            INSERT INTO wallet_metadata (key, value) VALUES ('current_wallet', 'MAIN_WALLET_ADDR');
            INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count, total_nfts_count)
            VALUES (1, 'MAIN_WALLET_ADDR', '2026-01-01T00:00:00Z', 1.5, 1500000000, 3, 0);
            INSERT INTO sol_flow_cache (signature, timestamp, sol_delta) VALUES ('sig1', '2026-01-01T00:00:00Z', 0.5);
            INSERT INTO wallet_dashboard_metrics (window_key, window_hours, snapshot_limit, token_limit, payload_blob, computed_at, valid_until)
            VALUES ('24h', 24, 30, 50, X'00', '2026-01-01T00:00:00Z', '2026-01-01T01:00:00Z');",
        )
        .expect("seed legacy wallet-monitor schema");
        conn
    }

    fn unmigrated_db() -> WalletDatabase {
        WalletDatabase {
            pool: Pool::builder()
                .max_size(1)
                .build(SqliteConnectionManager::memory())
                .expect("build unused pool for migration helper"),
            database_path: ":memory:".to_owned(),
            schema_version: WALLET_SCHEMA_VERSION,
            chain: ChainId::Solana,
        }
    }

    #[test]
    fn legacy_wallet_monitor_tables_migrate_to_chain_scoped_schema_losslessly_and_idempotently() {
        let conn = legacy_connection();
        let db = unmigrated_db();

        db.migrate_chain_identity(&conn)
            .expect("migrate legacy wallet-monitor database");
        db.migrate_chain_identity(&conn)
            .expect("repeat wallet-monitor chain migration");

        for (table, expected) in [
            ("wallet_snapshots", 1),
            ("sol_flow_cache", 1),
            ("wallet_dashboard_metrics", 1),
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count migrated rows");
            assert_eq!(count, expected, "{table} row count must survive migration");
            let chain: String = conn
                .query_row(
                    &format!("SELECT chain_id FROM {table} LIMIT 1"),
                    [],
                    |row| row.get(0),
                )
                .expect("read migrated chain_id");
            assert_eq!(chain, "solana", "{table} rows assigned to solana");
        }

        // sol_flow_cache and wallet_dashboard_metrics had no wallet_address column
        // pre-migration — it is backfilled from wallet_metadata's current_wallet.
        let flow_wallet: String = conn
            .query_row(
                "SELECT wallet_address FROM sol_flow_cache LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("read backfilled flow-cache wallet address");
        assert_eq!(flow_wallet, "MAIN_WALLET_ADDR");

        assert!(conn
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()
            .unwrap()
            .is_none());
    }
}
