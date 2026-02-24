//! Wallet query operations
//!
//! CRUD operations for wallet records in SQLite.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::WalletsDatabase;
use super::super::types::{Wallet, WalletRole, WalletType};

impl WalletsDatabase {
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
                format!("Wallet with address {address} already exists")
            } else {
                format!("Failed to insert wallet: {e}")
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
        .map_err(|e| format!("Failed to get wallet: {e}"))
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
        .map_err(|e| format!("Failed to get wallet by address: {e}"))
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
        .map_err(|e| format!("Failed to get main wallet: {e}"))
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
        .map_err(|e| format!("Failed to get encrypted key: {e}"))
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
        .map_err(|e| format!("Failed to get main wallet encrypted key: {e}"))
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
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let wallets = stmt
            .query_map([], |row| Self::row_to_wallet(row))
            .map_err(|e| format!("Failed to query wallets: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect wallets: {e}"))?;

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
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let wallets = stmt
            .query_map([], |row| Self::row_to_wallet(row))
            .map_err(|e| format!("Failed to query wallets: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect wallets: {e}"))?;

        Ok(wallets)
    }

    /// Set a wallet as main (unsets previous main)
    pub fn set_main_wallet(&self, id: i64) -> Result<(), String> {
        let conn = self.conn()?;

        // Start transaction
        conn.execute("BEGIN IMMEDIATE", [])
            .map_err(|e| format!("Failed to begin transaction: {e}"))?;

        // Unset current main wallet
        if let Err(e) = conn.execute(
            "UPDATE wallets SET role = 'secondary' WHERE role = 'main'",
            [],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(format!("Failed to unset main wallet: {e}"));
        }

        // Set new main wallet
        let updated = conn
            .execute(
                "UPDATE wallets SET role = 'main' WHERE id = ?1 AND is_active = 1",
                params![id],
            )
            .map_err(|e| {
                let _ = conn.execute("ROLLBACK", []);
                format!("Failed to set main wallet: {e}")
            })?;

        if updated == 0 {
            let _ = conn.execute("ROLLBACK", []);
            return Err("Wallet not found or inactive".to_owned());
        }

        conn.execute("COMMIT", [])
            .map_err(|e| format!("Failed to commit transaction: {e}"))?;

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
                .map_err(|e| format!("Failed to begin transaction: {e}"))?;

            // Unset current main wallet
            if let Err(e) = conn.execute(
                "UPDATE wallets SET role = 'secondary' WHERE role = 'main'",
                [],
            ) {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Failed to unset main: {e}"));
            }

            // Apply the update
            let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            if let Err(e) = conn.execute(&sql, params.as_slice()) {
                let _ = conn.execute("ROLLBACK", []);
                return Err(format!("Failed to update wallet: {e}"));
            }

            conn.execute("COMMIT", [])
                .map_err(|e| format!("Failed to commit transaction: {e}"))?;
        } else {
            let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
            conn.execute(&sql, params.as_slice())
                .map_err(|e| format!("Failed to update wallet: {e}"))?;
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
            .map_err(|e| format!("Failed to check wallet: {e}"))?
            .unwrap_or_default();

        if is_main {
            return Err(
                "Cannot archive the main wallet. Set another wallet as main first.".to_owned(),
            );
        }

        conn.execute(
            "UPDATE wallets SET is_active = 0, role = 'archive' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Failed to archive wallet: {e}"))?;

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
            .map_err(|e| format!("Failed to check wallet: {e}"))?
            .unwrap_or((false, false));

        if !exists {
            return Err("Wallet not found".to_owned());
        }

        if !is_archived {
            return Err("Wallet is not archived".to_owned());
        }

        conn.execute(
            "UPDATE wallets SET is_active = 1, role = 'secondary' WHERE id = ?1",
            params![id],
        )
        .map_err(|e| format!("Failed to restore wallet: {e}"))?;

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
            .map_err(|e| format!("Failed to check wallet: {e}"))?
            .unwrap_or_default();

        if is_main {
            return Err(
                "Cannot delete the main wallet. Set another wallet as main first.".to_owned(),
            );
        }

        conn.execute("DELETE FROM wallets WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete wallet: {e}"))?;

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
        .map_err(|e| format!("Failed to update last used: {e}"))?;

        Ok(())
    }

    /// Get wallet count statistics
    pub fn get_wallet_counts(&self) -> Result<(u32, u32), String> {
        let conn = self.conn()?;

        let total: u32 = conn
            .query_row("SELECT COUNT(*) FROM wallets", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count wallets: {e}"))?;

        let active: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE is_active = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count active wallets: {e}"))?;

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
            .map_err(|e| format!("Failed to check wallet existence: {e}"))?;

        Ok(count > 0)
    }

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
}
