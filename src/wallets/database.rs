//! Wallet database operations
//!
//! SQLite storage for multi-wallet management with encrypted private keys.

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;

use super::types::{TokenBalance, Wallet, WalletRole, WalletType};
use crate::paths::get_data_directory;

// =============================================================================
// DATABASE SCHEMA
// =============================================================================

const WALLETS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS wallets (
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
"#;

const TOKEN_BALANCES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_token_balances (
    wallet_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    ui_amount REAL NOT NULL,
    decimals INTEGER NOT NULL,
    symbol TEXT,
    name TEXT,
    is_token_2022 INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (wallet_id, mint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
);
"#;

const WALLETS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallets_address ON wallets(address);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_role ON wallets(role);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_active ON wallets(is_active, role);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_wallet ON wallet_token_balances(wallet_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON wallet_token_balances(mint);",
];

// =============================================================================
// DATABASE STRUCT
// =============================================================================

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
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }

        let manager = SqliteConnectionManager::file(&db_path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .map_err(|e| format!("Failed to create wallets connection pool: {}", e))?;

        let db = Self { pool };
        db.initialize()?;

        Ok(db)
    }

    /// Get a connection from the pool
    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>, String> {
        self.pool
            .get()
            .map_err(|e| format!("Failed to get connection: {}", e))
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<(), String> {
        let conn = self.conn()?;

        // Enable WAL mode for better concurrency
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 30000;
             PRAGMA cache_size = 5000;
             PRAGMA temp_store = memory;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(|e| format!("Failed to set pragmas: {}", e))?;

        // Create tables
        conn.execute(WALLETS_SCHEMA, [])
            .map_err(|e| format!("Failed to create wallets table: {}", e))?;

        conn.execute(TOKEN_BALANCES_SCHEMA, [])
            .map_err(|e| format!("Failed to create token_balances table: {}", e))?;

        // Create indexes
        for index_sql in WALLETS_INDEXES {
            conn.execute(index_sql, [])
                .map_err(|e| format!("Failed to create index: {}", e))?;
        }

        Ok(())
    }

    // =========================================================================
    // CRUD OPERATIONS
    // =========================================================================

    /// Insert a new wallet
    pub fn insert_wallet(
        &self,
        name: &str,
        address: &str,
        encrypted_key: &str,
        nonce: &str,
        role: WalletRole,
        wallet_type: WalletType,
        notes: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO wallets (name, address, encrypted_key, nonce, role, wallet_type, created_at, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                name,
                address,
                encrypted_key,
                nonce,
                role.to_string(),
                wallet_type.to_string(),
                now,
                notes,
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint failed") {
                format!("Wallet with address {} already exists", address)
            } else {
                format!("Failed to insert wallet: {}", e)
            }
        })?;

        Ok(conn.last_insert_rowid())
    }

    /// Get a wallet by ID
    pub fn get_wallet(&self, id: i64) -> Result<Option<Wallet>, String> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE id = ?1
            "#,
            params![id],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(|e| format!("Failed to get wallet: {}", e))
    }

    /// Get a wallet by address
    pub fn get_wallet_by_address(&self, address: &str) -> Result<Option<Wallet>, String> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE address = ?1
            "#,
            params![address],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(|e| format!("Failed to get wallet by address: {}", e))
    }

    /// Get the main wallet
    pub fn get_main_wallet(&self) -> Result<Option<Wallet>, String> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE role = 'main' AND is_active = 1
            "#,
            [],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(|e| format!("Failed to get main wallet: {}", e))
    }

    /// Get encrypted key data for a wallet
    pub fn get_wallet_encrypted_key(&self, id: i64) -> Result<Option<(String, String)>, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT encrypted_key, nonce FROM wallets WHERE id = ?1 AND is_active = 1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("Failed to get encrypted key: {}", e))
    }

    /// Get encrypted key data for main wallet
    pub fn get_main_wallet_encrypted_key(&self) -> Result<Option<(String, String)>, String> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT encrypted_key, nonce FROM wallets WHERE role = 'main' AND is_active = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("Failed to get main wallet encrypted key: {}", e))
    }

    /// List all wallets
    pub fn list_wallets(&self, include_inactive: bool) -> Result<Vec<Wallet>, String> {
        let conn = self.conn()?;

        let sql = if include_inactive {
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets ORDER BY role = 'main' DESC, created_at DESC
            "#
        } else {
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE is_active = 1 ORDER BY role = 'main' DESC, created_at DESC
            "#
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let wallets = stmt
            .query_map([], |row| Self::row_to_wallet(row))
            .map_err(|e| format!("Failed to query wallets: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect wallets: {}", e))?;

        Ok(wallets)
    }

    /// List active wallets (main + secondary)
    pub fn list_active_wallets(&self) -> Result<Vec<Wallet>, String> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE is_active = 1 AND role != 'archive'
            ORDER BY role = 'main' DESC, created_at DESC
            "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let wallets = stmt
            .query_map([], |row| Self::row_to_wallet(row))
            .map_err(|e| format!("Failed to query wallets: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect wallets: {}", e))?;

        Ok(wallets)
    }

    /// Set a wallet as main (unsets previous main)
    pub fn set_main_wallet(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;

        // Start transaction
        conn.execute("BEGIN IMMEDIATE", [])
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // Unset current main wallet
        if let Err(e) = conn.execute(
            "UPDATE wallets SET role = 'secondary' WHERE role = 'main'",
            [],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!("Failed to unset main wallet: {}", e));
        }

        // Set new main wallet
        let updated = conn
            .execute(
                "UPDATE wallets SET role = 'main' WHERE id = ?1 AND is_active = 1",
                params![id],
            )
            .map_err(|e| {
                let _ = conn.execute("ROLLBACK", []);
                format!("Failed to set main wallet: {}", e)
            })?;

        if updated == 0 {
            let _ = conn.execute("ROLLBACK", []);
            return Err("Wallet not found or inactive".to_string());
        }

        conn.execute("COMMIT", [])
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Update wallet metadata
    pub fn update_wallet(
        &self,
        id: i64,
        name: Option<&str>,
        notes: Option<&str>,
        role: Option<WalletRole>,
    ) -> Result<(), String> {
        let conn = self.conn()?;

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(n) = name {
            updates.push("name = ?");
            values.push(Box::new(n.to_string()));
        }
        if let Some(n) = notes {
            updates.push("notes = ?");
            values.push(Box::new(n.to_string()));
        }

        let changing_to_main = matches!(role, Some(WalletRole::Main));
        if let Some(r) = role {
            updates.push("role = ?");
            values.push(Box::new(r.to_string()));
        }

        if updates.is_empty() {
            return Ok(());
        }

        values.push(Box::new(id));
        let sql = format!("UPDATE wallets SET {} WHERE id = ?", updates.join(", "));

        // If changing role to main, wrap in transaction like set_main_wallet does
        if changing_to_main {
            conn.execute("BEGIN IMMEDIATE", [])
                .map_err(|e| format!("Failed to begin transaction: {}", e))?;

            // Unset current main wallet
            if let Err(e) = conn.execute(
                "UPDATE wallets SET role = 'secondary' WHERE role = 'main'",
                [],
            ) {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Failed to unset main: {}", e));
            }

            // Apply the update
            let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            if let Err(e) = conn.execute(&sql, params.as_slice()) {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Failed to update wallet: {}", e));
            }

            conn.execute("COMMIT", [])
                .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        } else {
            let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            conn.execute(&sql, params.as_slice())
                .map_err(|e| format!("Failed to update wallet: {}", e))?;
        }

        Ok(())
    }

    /// Soft delete (archive) a wallet
    pub fn archive_wallet(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;

        // Check if it's the main wallet
        let is_main: bool = conn
            .query_row(
                "SELECT role = 'main' FROM wallets WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to check wallet: {}", e))?
            .unwrap_or(false);

        if is_main {
            return Err(
                "Cannot archive the main wallet. Set another wallet as main first.".to_string(),
            );
        }

        conn.execute(
            "UPDATE wallets SET is_active = 0, role = 'archive' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Failed to archive wallet: {}", e))?;

        Ok(())
    }

    /// Restore an archived wallet (unarchive)
    pub fn restore_wallet(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;

        // Check if wallet exists and is archived
        let (exists, is_archived): (bool, bool) = conn
            .query_row(
                "SELECT 1, is_active = 0 FROM wallets WHERE id = ?1",
                params![id],
                |row| Ok((row.get::<_, i32>(0)? == 1, row.get::<_, bool>(1)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to check wallet: {}", e))?
            .unwrap_or((false, false));

        if !exists {
            return Err("Wallet not found".to_string());
        }

        if !is_archived {
            return Err("Wallet is not archived".to_string());
        }

        conn.execute(
            "UPDATE wallets SET is_active = 1, role = 'secondary' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Failed to restore wallet: {}", e))?;

        Ok(())
    }

    /// Permanently delete a wallet
    pub fn delete_wallet(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;

        // Check if it's the main wallet
        let is_main: bool = conn
            .query_row(
                "SELECT role = 'main' FROM wallets WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to check wallet: {}", e))?
            .unwrap_or(false);

        if is_main {
            return Err(
                "Cannot delete the main wallet. Set another wallet as main first.".to_string(),
            );
        }

        conn.execute("DELETE FROM wallets WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete wallet: {}", e))?;

        Ok(())
    }

    /// Update last used timestamp
    pub fn update_last_used(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE wallets SET last_used_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| format!("Failed to update last used: {}", e))?;

        Ok(())
    }

    /// Get wallet count statistics
    pub fn get_wallet_counts(&self) -> Result<(u32, u32), String> {
        let conn = self.conn()?;

        let total: u32 = conn
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count wallets: {}", e))?;

        let active: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE is_active = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count active wallets: {}", e))?;

        Ok((total, active))
    }

    /// Check if a wallet with this address exists
    pub fn wallet_exists(&self, address: &str) -> Result<bool, String> {
        let conn = self.conn()?;

        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE address = ?1",
                params![address],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check wallet existence: {}", e))?;

        Ok(count > 0)
    }

    // =========================================================================
    // HELPERS
    // =========================================================================

    /// Convert a database row to Wallet struct
    fn row_to_wallet(row: &rusqlite::Row) -> rusqlite::Result<Wallet> {
        let role_str: String = row.get(3)?;
        let type_str: String = row.get(4)?;
        let created_str: String = row.get(5)?;
        let last_used_str: Option<String> = row.get(6)?;

        Ok(Wallet {
            id: row.get(0)?,
            name: row.get(1)?,
            address: row.get(2)?,
            role: role_str.parse().unwrap_or(WalletRole::Secondary),
            wallet_type: type_str.parse().unwrap_or(WalletType::Generated),
            created_at: DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_used_at: last_used_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            }),
            notes: row.get(7)?,
            is_active: row.get::<_, i32>(8)? != 0,
        })
    }

    // =========================================================================
    // TOKEN BALANCE OPERATIONS
    // =========================================================================

    /// Upsert a single token balance
    pub fn upsert_token_balance(
        &self,
        wallet_id: i64,
        mint: &str,
        balance: u64,
        ui_amount: f64,
        decimals: u8,
        symbol: Option<&str>,
        name: Option<&str>,
        is_token_2022: bool,
    ) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO wallet_token_balances 
                (wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT (wallet_id, mint) DO UPDATE SET
                balance = excluded.balance,
                ui_amount = excluded.ui_amount,
                decimals = excluded.decimals,
                symbol = COALESCE(excluded.symbol, wallet_token_balances.symbol),
                name = COALESCE(excluded.name, wallet_token_balances.name),
                is_token_2022 = excluded.is_token_2022,
                updated_at = excluded.updated_at
            "#,
            params![
                wallet_id,
                mint,
                balance as i64,
                ui_amount,
                decimals as i32,
                symbol,
                name,
                is_token_2022 as i32,
                now,
            ],
        )
        .map_err(|e| format!("Failed to upsert token balance: {}", e))?;

        Ok(())
    }

    /// Get all token balances for a wallet
    pub fn get_token_balances(&self, wallet_id: i64) -> Result<Vec<TokenBalance>, String> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at
                FROM wallet_token_balances
                WHERE wallet_id = ?1
                ORDER BY ui_amount DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let balances = stmt
            .query_map(params![wallet_id], |row| Self::row_to_token_balance(row))
            .map_err(|e| format!("Failed to query token balances: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect token balances: {}", e))?;

        Ok(balances)
    }

    /// Get all token balances for all wallets
    pub fn get_all_token_balances(&self) -> Result<HashMap<i64, Vec<TokenBalance>>, String> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at
                FROM wallet_token_balances
                ORDER BY wallet_id, ui_amount DESC
                "#,
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let mut balances_map: HashMap<i64, Vec<TokenBalance>> = HashMap::new();

        let rows = stmt
            .query_map([], |row| Self::row_to_token_balance(row))
            .map_err(|e| format!("Failed to query all token balances: {}", e))?;

        for row in rows {
            let balance = row.map_err(|e| format!("Failed to parse token balance: {}", e))?;
            balances_map
                .entry(balance.wallet_id)
                .or_default()
                .push(balance);
        }

        Ok(balances_map)
    }

    /// Clear all token balances for a wallet
    pub fn clear_token_balances(&self, wallet_id: i64) -> Result<u64, String> {
        let conn = self.conn()?;

        let deleted = conn
            .execute(
                "DELETE FROM wallet_token_balances WHERE wallet_id = ?1",
                params![wallet_id],
            )
            .map_err(|e| format!("Failed to clear token balances: {}", e))?;

        Ok(deleted as u64)
    }

    /// Bulk update token balances for a wallet (replaces all existing)
    pub fn update_balances_bulk(
        &self,
        wallet_id: i64,
        balances: &[TokenBalance],
    ) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // Use a transaction for atomicity
        conn.execute("BEGIN IMMEDIATE", [])
            .map_err(|e| format!("Failed to begin transaction: {}", e))?;

        // Clear existing balances for this wallet
        if let Err(e) = conn.execute(
            "DELETE FROM wallet_token_balances WHERE wallet_id = ?1",
            params![wallet_id],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!("Failed to clear existing balances: {}", e));
        }

        // Insert new balances
        for balance in balances {
            if let Err(e) = conn.execute(
                r#"
                INSERT INTO wallet_token_balances 
                    (wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    wallet_id,
                    &balance.mint,
                    balance.balance as i64,
                    balance.ui_amount,
                    balance.decimals as i32,
                    &balance.symbol,
                    &balance.name,
                    balance.is_token_2022 as i32,
                    &now,
                ],
            ) {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Failed to insert token balance: {}", e));
            }
        }

        conn.execute("COMMIT", [])
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        Ok(())
    }

    /// Convert a database row to TokenBalance struct
    fn row_to_token_balance(row: &rusqlite::Row) -> rusqlite::Result<TokenBalance> {
        let updated_str: String = row.get(8)?;

        Ok(TokenBalance {
            wallet_id: row.get(0)?,
            mint: row.get(1)?,
            balance: row.get::<_, i64>(2)? as u64,
            ui_amount: row.get(3)?,
            decimals: row.get::<_, i32>(4)? as u8,
            symbol: row.get(5)?,
            name: row.get(6)?,
            is_token_2022: row.get::<_, i32>(7)? != 0,
            updated_at: DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }
}
