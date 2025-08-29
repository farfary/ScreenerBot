/// Wallet Balance Monitoring Module
///
/// This module provides wallet balance monitoring with historical snapshots stored in SQLite database.
/// It monitors both SOL balance and token balances for the configured wallet address.
///
/// Features:
/// - Background service that checks wallet balance every minute
/// - Delayed RPC calls to avoid overwhelming the global RPC client
/// - Historical snapshots stored in data/wallet.db
/// - Tracks both SOL and token balances
/// - Integration with existing RPC infrastructure

use std::path::Path;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use std::collections::HashSet;
use tokio::sync::{ Notify, Mutex };
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use once_cell::sync::Lazy;
use rusqlite::{ Connection, OptionalExtension, params, Result as SqliteResult };
use r2d2::{ Pool, PooledConnection };
use r2d2_sqlite::SqliteConnectionManager;

use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_wallet_enabled };
use crate::rpc::{ get_rpc_client, TokenAccountInfo };
use crate::utils::get_wallet_address;
use crate::positions::{ get_open_positions, attempt_position_recovery_from_transactions };

// Database schema version
const WALLET_SCHEMA_VERSION: u32 = 1;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

const SCHEMA_WALLET_SNAPSHOTS: &str =
    r#"
CREATE TABLE IF NOT EXISTS wallet_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address TEXT NOT NULL,
    snapshot_time TEXT NOT NULL,
    sol_balance REAL NOT NULL,
    sol_balance_lamports INTEGER NOT NULL,
    total_tokens_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

const SCHEMA_TOKEN_BALANCES: &str =
    r#"
CREATE TABLE IF NOT EXISTS token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

const SCHEMA_WALLET_METADATA: &str =
    r#"
CREATE TABLE IF NOT EXISTS wallet_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Performance indexes
const WALLET_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_address ON wallet_snapshots(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_time ON wallet_snapshots(snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_id ON token_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON token_balances(mint);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_mint ON token_balances(snapshot_id, mint);",
];

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// Wallet balance snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub id: Option<i64>,
    pub wallet_address: String,
    pub snapshot_time: DateTime<Utc>,
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub token_balances: Vec<TokenBalance>,
}

/// Token balance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub id: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub mint: String,
    pub balance: u64, // Raw token amount
    pub balance_ui: f64, // UI amount (adjusted for decimals)
    pub decimals: Option<u8>,
    pub is_token_2022: bool,
}

/// Wallet monitoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMonitorStats {
    pub total_snapshots: u64,
    pub latest_snapshot_time: Option<DateTime<Utc>>,
    pub wallet_address: String,
    pub current_sol_balance: Option<f64>,
    pub current_tokens_count: Option<u32>,
    pub database_size_bytes: u64,
    pub schema_version: u32,
}

// =============================================================================
// WALLET DATABASE MANAGER
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
        // Database should be at data/wallet.db
        let data_dir = std::path::PathBuf::from("data");

        // Ensure data directory exists
        if !data_dir.exists() {
            std::fs
                ::create_dir_all(&data_dir)
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let database_path = data_dir.join("wallet.db");
        let database_path_str = database_path.to_string_lossy().to_string();

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "INIT",
                &format!("Initializing wallet database at: {}", database_path_str)
            );
        }

        // Configure connection manager
        let manager = SqliteConnectionManager::file(&database_path);

        // Create connection pool
        let pool = Pool::builder()
            .max_size(3) // Small pool for wallet monitoring
            .min_idle(Some(1))
            .build(manager)
            .map_err(|e| format!("Failed to create wallet connection pool: {}", e))?;

        let mut db = WalletDatabase {
            pool,
            database_path: database_path_str.clone(),
            schema_version: WALLET_SCHEMA_VERSION,
        };

        // Initialize database schema
        db.initialize_schema().await?;

        log(LogTag::Wallet, "READY", "Wallet database initialized successfully");
        Ok(db)
    }

    /// Initialize database schema with all tables and indexes
    async fn initialize_schema(&mut self) -> Result<(), String> {
        let conn = self.get_connection()?;

        // Configure database settings
        conn
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set WAL mode: {}", e))?;
        conn
            .pragma_update(None, "foreign_keys", true)
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        conn
            .pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| format!("Failed to set synchronous mode: {}", e))?;

        // Create all tables
        conn
            .execute(SCHEMA_WALLET_SNAPSHOTS, [])
            .map_err(|e| format!("Failed to create wallet_snapshots table: {}", e))?;

        conn
            .execute(SCHEMA_TOKEN_BALANCES, [])
            .map_err(|e| format!("Failed to create token_balances table: {}", e))?;

        conn
            .execute(SCHEMA_WALLET_METADATA, [])
            .map_err(|e| format!("Failed to create wallet_metadata table: {}", e))?;

        // Create all indexes
        for index_sql in WALLET_INDEXES {
            conn
                .execute(index_sql, [])
                .map_err(|e| format!("Failed to create wallet index: {}", e))?;
        }

        // Set schema version
        conn
            .execute(
                "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('schema_version', ?1)",
                params![self.schema_version.to_string()]
            )
            .map_err(|e| format!("Failed to set wallet schema version: {}", e))?;

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SCHEMA",
                "Wallet database schema initialized with all tables and indexes"
            );
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool.get().map_err(|e| format!("Failed to get wallet database connection: {}", e))
    }

    /// Save wallet snapshot with token balances (synchronous version)
    pub fn save_wallet_snapshot_sync(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            ) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64
                ],
                |row| row.get::<_, i64>(0)
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn
                .execute(
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
                    ]
                )
                .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SAVE",
                &format!(
                    "Saved wallet snapshot ID {} with {} tokens for {}",
                    snapshot_id,
                    snapshot.token_balances.len(),
                    &snapshot.wallet_address[..8]
                )
            );
        }

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
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            ) VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_tokens_count as i64
                ],
                |row| row.get::<_, i64>(0)
            )
            .map_err(|e| format!("Failed to insert wallet snapshot: {}", e))?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            conn
                .execute(
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
                    ]
                )
                .map_err(|e| format!("Failed to insert token balance: {}", e))?;
        }

        if is_debug_wallet_enabled() {
            log(
                LogTag::Wallet,
                "SAVE",
                &format!(
                    "Saved wallet snapshot ID {} with {} tokens for {}",
                    snapshot_id,
                    snapshot.token_balances.len(),
                    &snapshot.wallet_address[..8]
                )
            );
        }

        Ok(snapshot_id)
    }

    /// Get recent wallet snapshots (synchronous version)
    pub fn get_recent_snapshots_sync(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    token_balances: Vec::new(), // Will be loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots.push(
                snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?
            );
        }

        Ok(snapshots)
    }

    /// Get recent wallet snapshots (async version)
    pub async fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_tokens_count
            FROM wallet_snapshots 
            ORDER BY snapshot_time DESC 
            LIMIT ?1
            "#
            )
            .map_err(|e| format!("Failed to prepare snapshots query: {}", e))?;

        let snapshot_iter = stmt
            .query_map(params![limit], |row| {
                let snapshot_time_str: String = row.get(2)?;
                let snapshot_time = DateTime::parse_from_rfc3339(&snapshot_time_str)
                    .map_err(|_|
                        rusqlite::Error::InvalidColumnType(
                            2,
                            "Invalid snapshot_time".to_string(),
                            rusqlite::types::Type::Text
                        )
                    )?
                    .with_timezone(&Utc);

                Ok(WalletSnapshot {
                    id: Some(row.get(0)?),
                    wallet_address: row.get(1)?,
                    snapshot_time,
                    sol_balance: row.get(3)?,
                    sol_balance_lamports: row.get::<_, i64>(4)? as u64,
                    total_tokens_count: row.get::<_, i64>(5)? as u32,
                    token_balances: Vec::new(), // Will be loaded separately if needed
                })
            })
            .map_err(|e| format!("Failed to execute snapshots query: {}", e))?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots.push(
                snapshot_result.map_err(|e| format!("Failed to parse snapshot row: {}", e))?
            );
        }

        Ok(snapshots)
    }

    /// Get wallet monitoring statistics (synchronous version)
    pub fn get_monitor_stats_sync(&self) -> Result<WalletMonitorStats, String> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

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
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) = if
            let Some((addr, time_str, balance, count)) = latest_info
        {
            let time = DateTime::parse_from_rfc3339(&time_str)
                .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                .with_timezone(&Utc);
            (addr, Some(time), Some(balance), Some(count as u32))
        } else {
            ("Unknown".to_string(), None, None, None)
        };

        // Get database file size
        let database_size = std::fs
            ::metadata(&self.database_path)
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
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count snapshots: {}", e))?;

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
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            )
            .optional()
            .map_err(|e| format!("Failed to get latest snapshot: {}", e))?;

        let (wallet_address, latest_snapshot_time, current_sol_balance, current_tokens_count) = if
            let Some((addr, time_str, balance, count)) = latest_info
        {
            let time = DateTime::parse_from_rfc3339(&time_str)
                .map_err(|e| format!("Failed to parse latest snapshot time: {}", e))?
                .with_timezone(&Utc);
            (addr, Some(time), Some(balance), Some(count as u32))
        } else {
            ("Unknown".to_string(), None, None, None)
        };

        // Get database file size
        let database_size = std::fs
            ::metadata(&self.database_path)
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
            SELECT id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get(5)?,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?
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
            SELECT id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
            FROM token_balances 
            WHERE snapshot_id = ?1
            ORDER BY balance_ui DESC
            "#
            )
            .map_err(|e| format!("Failed to prepare token balances query: {}", e))?;

        let balances_iter = stmt
            .query_map(params![snapshot_id], |row| {
                Ok(TokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get::<_, i64>(3)? as u64,
                    balance_ui: row.get(4)?,
                    decimals: row.get(5)?,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(|e| format!("Failed to execute token balances query: {}", e))?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(
                balance_result.map_err(|e| format!("Failed to parse token balance row: {}", e))?
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
                []
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Wallet,
                "CLEANUP",
                &format!("Cleaned up {} old wallet snapshots", deleted_count)
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
                []
            )
            .map_err(|e| format!("Failed to cleanup old snapshots: {}", e))?;

        if deleted_count > 0 {
            log(
                LogTag::Wallet,
                "CLEANUP",
                &format!("Cleaned up {} old wallet snapshots", deleted_count)
            );
        }

        Ok(deleted_count as u64)
    }
}

// =============================================================================
// GLOBAL WALLET DATABASE INSTANCE
// =============================================================================

/// Global wallet database instance
static GLOBAL_WALLET_DB: Lazy<Arc<Mutex<Option<WalletDatabase>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// Initialize the global wallet database
pub async fn initialize_wallet_database() -> Result<(), String> {
    let mut db_lock = GLOBAL_WALLET_DB.lock().await;
    if db_lock.is_some() {
        return Ok(()); // Already initialized
    }

    let db = WalletDatabase::new().await?;
    *db_lock = Some(db);

    log(LogTag::Wallet, "INIT", "Global wallet database initialized successfully");
    Ok(())
}

// Helper functions removed to avoid lifetime issues - using direct database access instead

// =============================================================================
// WALLET MONITORING SERVICE
// =============================================================================

/// Collect current wallet balance and token balances
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    // Get wallet address
    let wallet_address = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;

    let rpc_client = get_rpc_client();
    let snapshot_time = Utc::now();

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "COLLECT",
            &format!("Collecting wallet snapshot for {}", &wallet_address[..8])
        );
    }

    // Add small delay to avoid overwhelming RPC client
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get SOL balance
    let sol_balance = rpc_client
        .get_sol_balance(&wallet_address).await
        .map_err(|e| format!("Failed to get SOL balance: {}", e))?;

    let sol_balance_lamports = (sol_balance * 1_000_000_000.0) as u64;

    // Add another small delay before token accounts fetch
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Get all token accounts
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_address).await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    // Convert to TokenBalance format
    let mut token_balances = Vec::new();
    for account_info in &token_accounts {
        // Skip accounts with zero balance
        if account_info.balance == 0 {
            continue;
        }

        let balance_ui = if
            let Some(decimals) = crate::tokens::decimals::get_cached_decimals(&account_info.mint)
        {
            (account_info.balance as f64) / (10_f64).powi(decimals as i32)
        } else {
            account_info.balance as f64 // Fallback without decimals
        };

        token_balances.push(TokenBalance {
            id: None,
            snapshot_id: None,
            mint: account_info.mint.clone(),
            balance: account_info.balance,
            balance_ui,
            decimals: crate::tokens::decimals::get_cached_decimals(&account_info.mint),
            is_token_2022: account_info.is_token_2022,
        });
    }

    let total_tokens_count = token_balances.len() as u32;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "SNAPSHOT",
            &format!("Collected snapshot: SOL {:.6}, {} tokens", sol_balance, total_tokens_count)
        );
    }

    // POSITION RECOVERY LOGIC: Check for orphaned positions with zero wallet balance
    // This handles cases where tokens were sold but position wasn't properly closed
    let wallet_token_mints: HashSet<String> = token_balances
        .iter()
        .map(|tb| tb.mint.clone())
        .collect();

    let open_positions = get_open_positions().await;
    let mut recovery_count = 0;

    for position in open_positions {
        // Check if position token is NOT in wallet (zero balance)
        if !wallet_token_mints.contains(&position.mint) {
            if is_debug_wallet_enabled() {
                log(
                    LogTag::Wallet,
                    "POSITION_RECOVERY_ATTEMPT",
                    &format!(
                        "Detected orphaned position: {} ({}) - not in wallet, attempting recovery",
                        position.symbol,
                        &position.mint[..8]
                    )
                );
            }

            // Attempt to recover the position by finding the sell transaction
            match
                attempt_position_recovery_from_transactions(&position.mint, &position.symbol).await
            {
                Ok(recovery_signature) => {
                    recovery_count += 1;
                    log(
                        LogTag::Wallet,
                        "POSITION_RECOVERY_SUCCESS",
                        &format!(
                            "Successfully recovered orphaned position {} with transaction {}",
                            position.symbol,
                            &recovery_signature[..8]
                        )
                    );
                }
                Err(e) => {
                    if is_debug_wallet_enabled() {
                        log(
                            LogTag::Wallet,
                            "POSITION_RECOVERY_FAILED",
                            &format!("Failed to recover position {}: {}", position.symbol, e)
                        );
                    }
                }
            }
        }
    }

    if recovery_count > 0 {
        log(
            LogTag::Wallet,
            "POSITION_RECOVERY_SUMMARY",
            &format!("Recovered {} orphaned positions during wallet snapshot", recovery_count)
        );
    }

    Ok(WalletSnapshot {
        id: None,
        wallet_address,
        snapshot_time,
        sol_balance,
        sol_balance_lamports,
        total_tokens_count,
        token_balances,
    })
}

/// Background service for wallet monitoring
pub async fn start_wallet_monitoring_service(shutdown: Arc<Notify>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        log(LogTag::Wallet, "START", "Wallet monitoring service started");

        // Initialize database
        if let Err(e) = initialize_wallet_database().await {
            log(LogTag::Wallet, "ERROR", &format!("Failed to initialize wallet database: {}", e));
            return;
        }

        let mut interval = tokio::time::interval(Duration::from_secs(60)); // 1 minute
        let mut cleanup_counter = 0;

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Wallet, "SHUTDOWN", "Wallet monitoring service shutting down");
                    break;
                }
                _ = interval.tick() => {
                    // Collect wallet snapshot
                    match collect_wallet_snapshot().await {
                        Ok(snapshot) => {
                            // Save to database
                            let db_guard = GLOBAL_WALLET_DB.lock().await;
                            match db_guard.as_ref() {
                                Some(db) => {
                                    match db.save_wallet_snapshot_sync(&snapshot) {
                                        Ok(snapshot_id) => {
                                            if is_debug_wallet_enabled() {
                                                log(
                                                    LogTag::Wallet,
                                                    "SAVED",
                                                    &format!(
                                                        "Saved snapshot ID {} - SOL: {:.6}, Tokens: {}",
                                                        snapshot_id,
                                                        snapshot.sol_balance,
                                                        snapshot.total_tokens_count
                                                    )
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log(LogTag::Wallet, "ERROR", &format!("Failed to save wallet snapshot: {}", e));
                                        }
                                    }
                                }
                                None => {
                                    log(LogTag::Wallet, "ERROR", "Wallet database not initialized");
                                }
                            }
                        }
                        Err(e) => {
                            log(LogTag::Wallet, "ERROR", &format!("Failed to collect wallet snapshot: {}", e));
                        }
                    }

                    // Cleanup old snapshots every 60 intervals (1 hour)
                    cleanup_counter += 1;
                    if cleanup_counter >= 60 {
                        cleanup_counter = 0;
                        
                        let db_guard = GLOBAL_WALLET_DB.lock().await;
                        match db_guard.as_ref() {
                            Some(db) => {
                                if let Err(e) = db.cleanup_old_snapshots_sync() {
                                    log(LogTag::Wallet, "WARN", &format!("Failed to cleanup old snapshots: {}", e));
                                }
                            }
                            None => {
                                log(LogTag::Wallet, "WARN", "Wallet database not initialized for cleanup");
                            }
                        }
                    }
                }
            }
        }

        log(LogTag::Wallet, "STOPPED", "Wallet monitoring service stopped");
    })
}

// =============================================================================
// PUBLIC API FUNCTIONS
// =============================================================================

/// Get recent wallet snapshots
pub async fn get_recent_wallet_snapshots(limit: usize) -> Result<Vec<WalletSnapshot>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => {
            // Use the synchronous version to avoid lifetime issues
            db.get_recent_snapshots_sync(limit)
        }
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get wallet monitoring statistics
pub async fn get_wallet_monitor_stats() -> Result<WalletMonitorStats, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_monitor_stats_sync(),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get token balances for a snapshot
pub async fn get_snapshot_token_balances(snapshot_id: i64) -> Result<Vec<TokenBalance>, String> {
    let db_guard = GLOBAL_WALLET_DB.lock().await;
    match db_guard.as_ref() {
        Some(db) => db.get_token_balances_sync(snapshot_id),
        None => Err("Wallet database not initialized".to_string()),
    }
}

/// Get current wallet status (latest snapshot data)
pub async fn get_current_wallet_status() -> Result<Option<WalletSnapshot>, String> {
    let snapshots = get_recent_wallet_snapshots(1).await?;
    Ok(snapshots.into_iter().next())
}
