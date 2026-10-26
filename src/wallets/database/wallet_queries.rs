// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet query operations
//!
//! Wallet record inserts and reads in SQLite.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use super::super::types::{Wallet, WalletRole, WalletType, TRADING_WALLET_NAME};
use super::WalletsDatabase;
use crate::database::WriteTransaction;
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

    /// Get a wallet by address
    pub fn get_wallet_by_address(&self, address: &str) -> Result<Option<Wallet>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            r#"
            SELECT id, name, address, role, wallet_type, created_at, last_used_at, notes, is_active
            FROM wallets WHERE chain_id = ?1 AND address = ?2
            "#,
            params![self.chain.as_str(), address],
            |row| Self::row_to_wallet(row),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Make the wallet `new_address` the active main wallet, named
    /// [`TRADING_WALLET_NAME`], and the main wallet `previous_address` a secondary that
    /// stays active, so its key stays exportable; it keeps its name, which labels it as
    /// the previous trading wallet. One IMMEDIATE transaction that commits only when
    /// exactly one active main wallet remains and it is `new_address`; the vault is
    /// snapshotted after the change.
    pub fn promote_to_main(&self, previous_address: &str, new_address: &str) -> Result<(), Error> {
        let mut conn = self.conn()?;
        let chain = self.chain.as_str();
        {
            let tx = conn.write_tx().map_err(DatabaseError::from)?;
            tx.execute(
                "UPDATE wallets SET role = ?1 WHERE chain_id = ?2 AND address = ?3 AND role = ?4",
                params![
                    WalletRole::Secondary.to_string(),
                    chain,
                    previous_address,
                    WalletRole::Main.to_string()
                ],
            )
            .map_err(DatabaseError::from)?;
            let promoted = tx
                .execute(
                    "UPDATE wallets SET role = ?1, is_active = 1, name = ?2 WHERE chain_id = ?3 AND address = ?4",
                    params![
                        WalletRole::Main.to_string(),
                        TRADING_WALLET_NAME,
                        chain,
                        new_address
                    ],
                )
                .map_err(DatabaseError::from)?;
            let mains: Vec<String> = {
                let mut statement = tx
                    .prepare(
                        "SELECT address FROM wallets WHERE chain_id = ?1 AND role = ?2 AND is_active = 1",
                    )
                    .map_err(DatabaseError::from)?;
                let rows = statement
                    .query_map(params![chain, WalletRole::Main.to_string()], |row| {
                        row.get(0)
                    })
                    .map_err(DatabaseError::from)?
                    .collect::<std::result::Result<Vec<String>, _>>()
                    .map_err(DatabaseError::from)?;
                rows
            };
            if promoted != 1 || mains.len() != 1 || mains[0] != new_address {
                // Dropping the transaction rolls it back.
                return Err(DatabaseError::Query {
                    operation: "promote_to_main".to_owned(),
                    message: format!(
                        "expected {new_address} as the one active main wallet, found {mains:?}"
                    ),
                }
                .into());
            }
            tx.commit().map_err(DatabaseError::from)?;
        }
        Self::snapshot_after_key_change(&conn);
        Ok(())
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

    /// The encrypted key of the wallet `address`, active or not.
    pub fn get_wallet_key_by_address(
        &self,
        address: &str,
    ) -> Result<Option<(String, String)>, Error> {
        let conn = self.conn()?;

        conn.query_row(
            "SELECT encrypted_key, nonce FROM wallets WHERE chain_id = ?1 AND address = ?2",
            params![self.chain.as_str(), address],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(DatabaseError::from)
        .map_err(Error::from)
    }

    /// Replace the encrypted key of the wallet `address` with `encrypted_key`/`nonce`,
    /// which must decrypt to the same address. The vault is snapshotted before the change,
    /// so the replaced ciphertext stays recoverable, and after it.
    pub fn replace_wallet_key(
        &self,
        address: &str,
        encrypted_key: &str,
        nonce: &str,
    ) -> Result<(), Error> {
        let conn = self.conn()?;
        Self::snapshot_after_key_change(&conn);
        let updated = conn
            .execute(
                "UPDATE wallets SET encrypted_key = ?1, nonce = ?2 WHERE chain_id = ?3 AND address = ?4",
                params![encrypted_key, nonce, self.chain.as_str(), address],
            )
            .map_err(DatabaseError::from)?;
        if updated != 1 {
            return Err(Error::WalletNotFound {
                address: address.to_owned(),
            });
        }
        Self::snapshot_after_key_change(&conn);
        Ok(())
    }

    /// The addresses of every active main wallet; a consistent vault holds at most one.
    pub fn active_main_addresses(&self) -> Result<Vec<String>, Error> {
        let conn = self.conn()?;
        let mut statement = conn
            .prepare(
                "SELECT address FROM wallets WHERE chain_id = ?1 AND role = ?2 AND is_active = 1",
            )
            .map_err(DatabaseError::from)?;
        let addresses = statement
            .query_map(
                params![self.chain.as_str(), WalletRole::Main.to_string()],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(DatabaseError::from)?;
        Ok(addresses)
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
