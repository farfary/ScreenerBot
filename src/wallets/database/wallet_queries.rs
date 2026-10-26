// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet query operations
//!
//! Wallet record inserts and reads in SQLite.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::super::types::{Wallet, WalletRole, WalletType};
use super::WalletsDatabase;
use crate::errors::DatabaseError;
use crate::wallets::Error;

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
    ) -> Result<i64, Error> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO wallets (chain_id, name, address, encrypted_key, nonce, role, wallet_type, created_at, notes)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                self.chain.as_str(), name,
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
                DatabaseError::Query {
                    operation: "insert_wallet".to_owned(),
                    message: format!("wallet with address {address} already exists"),
                }
                .into()
            } else {
                Error::Database(DatabaseError::from(e))
            }
        })?;
        let id = conn.last_insert_rowid();
        Self::snapshot_after_key_change(&conn);

        Ok(id)
    }

    /// Get a wallet by ID
    pub fn get_wallet(&self, id: i64) -> Result<Option<Wallet>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 AND id = ?2
            "#,
            params![self.chain.as_str(), id],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Get the main wallet
    pub fn get_main_wallet(&self) -> Result<Option<Wallet>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 AND role = 'main' AND is_active = 1
            "#,
            params![self.chain.as_str()],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Get encrypted key data for a wallet
    pub fn get_wallet_encrypted_key(&self, id: i64) -> Result<Option<(String, String)>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT encrypted_key, nonce FROM wallets WHERE chain_id = ?1 AND id = ?2 AND is_active = 1",
            params![self.chain.as_str(), id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Get encrypted key data for main wallet
    pub fn get_main_wallet_encrypted_key(&self) -> Result<Option<(String, String)>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT encrypted_key, nonce FROM wallets WHERE chain_id = ?1 AND role = 'main' AND is_active = 1",
            params![self.chain.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// List all wallets
    pub fn list_wallets(&self, include_inactive: bool) -> Result<Vec<Wallet>, Error> {
        let conn = self.conn()?;

        let sql = if include_inactive {
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 ORDER BY role = 'main' DESC, created_at DESC
            "#
        } else {
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 AND is_active = 1 ORDER BY role = 'main' DESC, created_at DESC
            "#
        };

        let mut stmt = conn.prepare(sql).map_err(DatabaseError::from)?;

        let wallets = stmt
            .query_map(params![self.chain.as_str()], |row| Self::row_to_wallet(row))
            .map_err(DatabaseError::from)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;

        Ok(wallets)
    }

    /// List active wallets (main + secondary)
    pub fn list_active_wallets(&self) -> Result<Vec<Wallet>, Error> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 AND is_active = 1 AND role != 'archive'
            ORDER BY role = 'main' DESC, created_at DESC
            "#,
            )
            .map_err(DatabaseError::from)?;

        let wallets = stmt
            .query_map(params![self.chain.as_str()], |row| Self::row_to_wallet(row))
            .map_err(DatabaseError::from)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;

        Ok(wallets)
    }

    /// Get wallet count statistics
    pub fn get_wallet_counts(&self) -> Result<(u32, u32), Error> {
        let conn = self.conn()?;

        let total: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE chain_id = ?1",
                params![self.chain.as_str()],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)?;

        let active: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallets WHERE chain_id = ?1 AND is_active = 1",
                params![self.chain.as_str()],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)?;

        Ok((total, active))
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
