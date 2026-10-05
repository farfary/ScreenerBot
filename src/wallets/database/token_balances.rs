// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token balance database operations — store and query on-chain token balances.

use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashMap;

use super::super::types::TokenBalance;
use super::WalletsDatabase;
use crate::chains::RawAmount;
use crate::database::WriteTransaction;
use crate::errors::DatabaseError;
use crate::wallets::Error;

impl WalletsDatabase {
    /// Upsert a single token balance
    pub fn upsert_token_balance(
        &self,
        wallet_id: i64,
        mint: &str,
        balance: RawAmount,
        ui_amount: f64,
        decimals: u8,
        symbol: Option<&str>,
        name: Option<&str>,
        is_token_2022: bool,
    ) -> Result<(), Error> {
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
                balance,
                ui_amount,
                decimals as i32,
                symbol,
                name,
                is_token_2022 as i32,
                now,
            ],
        )
        .map_err(DatabaseError::from)?;

        Ok(())
    }

    /// Get all token balances for a wallet
    pub fn get_token_balances(&self, wallet_id: i64) -> Result<Vec<TokenBalance>, Error> {
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
            .map_err(DatabaseError::from)?;

        let balances = stmt
            .query_map(params![wallet_id], |row| Self::row_to_token_balance(row))
            .map_err(DatabaseError::from)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(DatabaseError::from)?;

        Ok(balances)
    }

    /// Get all token balances for all wallets
    pub fn get_all_token_balances(&self) -> Result<HashMap<i64, Vec<TokenBalance>>, Error> {
        let conn = self.conn()?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at
                FROM wallet_token_balances
                ORDER BY wallet_id, ui_amount DESC
                "#,
            )
            .map_err(DatabaseError::from)?;

        let mut balances_map: HashMap<i64, Vec<TokenBalance>> = HashMap::new();

        let rows = stmt
            .query_map([], |row| Self::row_to_token_balance(row))
            .map_err(DatabaseError::from)?;

        for row in rows {
            let balance = row.map_err(DatabaseError::from)?;
            balances_map
                .entry(balance.wallet_id)
                .or_default()
                .push(balance);
        }

        Ok(balances_map)
    }

    /// Clear all token balances for a wallet
    pub fn clear_token_balances(&self, wallet_id: i64) -> Result<u64, Error> {
        let conn = self.conn()?;

        let deleted = conn
            .execute(
                "DELETE FROM wallet_token_balances WHERE wallet_id = ?1",
                params![wallet_id],
            )
            .map_err(DatabaseError::from)?;

        Ok(deleted as u64)
    }

    /// Bulk update token balances for a wallet (replaces all existing)
    pub fn update_balances_bulk(
        &self,
        wallet_id: i64,
        balances: &[TokenBalance],
    ) -> Result<(), Error> {
        let mut conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let tx = conn.write_tx().map_err(DatabaseError::from)?;

        // Clear existing balances for this wallet
        tx.execute(
            "DELETE FROM wallet_token_balances WHERE wallet_id = ?1",
            params![wallet_id],
        )
        .map_err(DatabaseError::from)?;

        // Insert new balances
        for balance in balances {
            tx.execute(
                r#"
                INSERT INTO wallet_token_balances 
                    (wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                "#,
                params![
                    wallet_id,
                    &balance.mint,
                    balance.balance,
                    balance.ui_amount,
                    balance.decimals as i32,
                    &balance.symbol,
                    &balance.name,
                    balance.is_token_2022 as i32,
                    &now,
                ],
            ).map_err(DatabaseError::from)?;
        }

        tx.commit().map_err(DatabaseError::from)?;

        Ok(())
    }

    /// Convert a database row to TokenBalance struct
    fn row_to_token_balance(row: &rusqlite::Row) -> rusqlite::Result<TokenBalance> {
        let updated_str: String = row.get(8)?;

        Ok(TokenBalance {
            wallet_id: row.get(0)?,
            mint: row.get(1)?,
            balance: row.get(2)?,
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

#[cfg(test)]
mod amount_tests {
    use super::*;
    use crate::chains::ChainId;
    use crate::wallets::database::schema::{
        LEGACY_TOKEN_BALANCES_SCHEMA, TOKEN_BALANCES_SCHEMA, WALLETS_INDEXES, WALLETS_SCHEMA,
    };
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::Connection;

    fn open(path: &std::path::Path) -> WalletsDatabase {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            crate::database::configure_connection(conn, crate::database::WALLETS_DB)
        });
        let db = WalletsDatabase {
            pool: Pool::builder()
                .max_size(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .build(manager)
                .unwrap(),
            chain: ChainId::Solana,
        };
        db.initialize().unwrap();
        db
    }

    fn fixture(path: &std::path::Path, legacy_schema: &str) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(WALLETS_SCHEMA).unwrap();
        conn.execute_batch(legacy_schema).unwrap();
        conn.execute("INSERT INTO wallets (id, name, address, encrypted_key, nonce) VALUES (1, 'wallet', 'address', 'ciphertext', 'nonce')", []).unwrap();
        for index in WALLETS_INDEXES {
            conn.execute_batch(index).unwrap();
        }
        for (n, raw) in [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]
            .into_iter()
            .enumerate()
        {
            let bits = i64::from_ne_bytes(raw.to_ne_bytes());
            conn.execute("INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, symbol, name, is_token_2022, updated_at) VALUES (1, ?1, ?2, 1.25, 6, 'SYM', NULL, 1, '2026-10-05T00:00:00+00:00')",
                params![format!("mint{n}"), bits]).unwrap();
        }
    }

    #[test]
    fn historical_wallet_balances_migrate_exactly_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallets.db");
        fixture(&path, LEGACY_TOKEN_BALANCES_SCHEMA);
        for _ in 0..2 {
            let db = open(&path);
            let balances = db.get_token_balances(1).unwrap();
            for (n, raw) in [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]
                .into_iter()
                .enumerate()
            {
                let row = balances
                    .iter()
                    .find(|row| row.mint == format!("mint{n}"))
                    .unwrap();
                assert_eq!(row.balance, RawAmount::from(raw));
                assert_eq!(
                    (
                        row.ui_amount,
                        row.decimals,
                        row.symbol.as_deref(),
                        row.name.as_deref(),
                        row.is_token_2022
                    ),
                    (1.25, 6, Some("SYM"), None, true)
                );
            }
            db.upsert_token_balance(1, "wide", RawAmount::MAX, 1.0, 0, None, None, false)
                .unwrap();
            assert_eq!(
                db.get_token_balances(1)
                    .unwrap()
                    .iter()
                    .find(|row| row.mint == "wide")
                    .unwrap()
                    .balance,
                RawAmount::MAX
            );
            let conn = db.conn().unwrap();
            let (kind, value): (String, String) = conn.query_row("SELECT typeof(balance), balance FROM wallet_token_balances WHERE mint = 'mint3'", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
            assert_eq!(
                (kind.as_str(), value.as_str()),
                ("text", "18446744073709551615")
            );
            assert_eq!(conn.query_row("SELECT type FROM pragma_table_info('wallet_token_balances') WHERE name = 'balance'", [], |row| row.get::<_, String>(0)).unwrap(), "TEXT");
            let objects: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE tbl_name = 'wallet_token_balances' AND type = 'index'", [], |row| row.get(0)).unwrap();
            assert_eq!(objects, 3);
            let ddl: String = conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name = 'wallet_token_balances'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                ddl.split_once('(').unwrap().1.trim(),
                TOKEN_BALANCES_SCHEMA
                    .split_once('(')
                    .unwrap()
                    .1
                    .trim()
                    .trim_end_matches(';')
                    .trim()
            );
            for index in WALLETS_INDEXES
                .iter()
                .filter(|sql| sql.contains(" ON wallet_token_balances("))
            {
                let name = index.split_whitespace().nth(5).unwrap();
                let stored: String = conn
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name = ?1",
                        [name],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(
                    stored,
                    index.replace(" IF NOT EXISTS", "").trim_end_matches(';')
                );
            }
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM wallets WHERE id = 1", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
                1
            );
            assert_eq!(
                conn.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
                    .unwrap(),
                "ok"
            );
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
                0
            );
        }
    }

    #[test]
    fn unfamiliar_wallet_balance_shape_and_value_fail_without_data_loss() {
        for schema in [
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL,\n    extra TEXT,"),
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL CHECK(balance IS NOT NULL),"),
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL,\n    generated INTEGER GENERATED ALWAYS AS (balance + 1) VIRTUAL,"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("wallets.db");
            fixture(&path, &schema);
            let before = Connection::open(&path).unwrap().query_row("SELECT sql FROM sqlite_master WHERE name = 'wallet_token_balances'", [], |row| row.get::<_, String>(0)).unwrap();
            let manager = SqliteConnectionManager::file(&path)
                .with_init(|conn| crate::database::configure_connection(conn, crate::database::WALLETS_DB));
            let db = WalletsDatabase { pool: Pool::builder().max_size(1).build(manager).unwrap(), chain: ChainId::Solana };
            assert!(db.initialize().is_err());
            let conn = Connection::open(&path).unwrap();
            assert_eq!(conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'wallet_token_balances'", [], |row| row.get::<_, String>(0)).unwrap(), before);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM wallet_token_balances", [], |row| row.get::<_, i64>(0)).unwrap(), 4);
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallets.db");
        let db = open(&path);
        db.upsert_token_balance(1, "unused", RawAmount::ZERO, 0.0, 0, None, None, false)
            .unwrap_err();
        let conn = db.conn().unwrap();
        conn.execute("INSERT INTO wallets (id, name, address, encrypted_key, nonce) VALUES (1, 'wallet', 'address', 'ciphertext', 'nonce')", []).unwrap();
        conn.execute("INSERT INTO wallet_token_balances (wallet_id, mint, balance, ui_amount, decimals, updated_at) VALUES (1, 'bad', '01', 1, 0, '2026-10-05T00:00:00+00:00')", []).unwrap();
        drop(conn);
        assert!(db.get_token_balances(1).is_err());
        let conn = db.conn().unwrap();
        conn.execute(
            "UPDATE wallet_token_balances SET balance = X'31' WHERE mint = 'bad'",
            [],
        )
        .unwrap();
        drop(conn);
        assert!(db.get_token_balances(1).is_err());
    }

    #[test]
    fn bulk_balance_replace_rolls_back_on_child_failure() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallets.db");
        fixture(&path, LEGACY_TOKEN_BALANCES_SCHEMA);
        let db = open(&path);
        let mut balances = db.get_token_balances(1).unwrap();
        balances.truncate(1);
        balances.push(balances[0].clone());
        assert!(db.update_balances_bulk(1, &balances).is_err());
        assert_eq!(db.get_token_balances(1).unwrap().len(), 4);
    }

    #[test]
    fn unfamiliar_wallet_balance_index_trigger_or_value_preserves_legacy_rows() {
        for extra in [
            "CREATE INDEX extra_balance_index ON wallet_token_balances(balance)",
            "CREATE TRIGGER extra_balance_trigger AFTER INSERT ON wallet_token_balances BEGIN SELECT 1; END",
            "UPDATE wallet_token_balances SET balance = 1.5 WHERE mint = 'mint0'",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("wallets.db");
            fixture(&path, LEGACY_TOKEN_BALANCES_SCHEMA);
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(extra).unwrap();
            let before = conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'wallet_token_balances'", [], |row| row.get::<_, String>(0)).unwrap();
            let before_value = conn.query_row("SELECT typeof(balance), quote(balance) FROM wallet_token_balances WHERE mint = 'mint0'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap();
            drop(conn);
            let manager = SqliteConnectionManager::file(&path)
                .with_init(|conn| crate::database::configure_connection(conn, crate::database::WALLETS_DB));
            let db = WalletsDatabase { pool: Pool::builder().max_size(1).build(manager).unwrap(), chain: ChainId::Solana };
            assert!(db.initialize().is_err());
            let conn = Connection::open(&path).unwrap();
            assert_eq!(conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'wallet_token_balances'", [], |row| row.get::<_, String>(0)).unwrap(), before);
            assert_eq!(conn.query_row("SELECT typeof(balance), quote(balance) FROM wallet_token_balances WHERE mint = 'mint0'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap(), before_value);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM wallet_token_balances", [], |row| row.get::<_, i64>(0)).unwrap(), 4);
        }
    }
}
