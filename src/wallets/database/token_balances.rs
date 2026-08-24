//! Token balance database operations — store and query on-chain token balances.

use chrono::{DateTime, Utc};
use rusqlite::params;
use std::collections::HashMap;

use super::super::types::TokenBalance;
use super::WalletsDatabase;
use crate::errors::DatabaseError;
use crate::wallets::Error;

impl WalletsDatabase {
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
                balance as i64,
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
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        // Use a transaction for atomicity
        conn.execute("BEGIN IMMEDIATE", [])
            .map_err(DatabaseError::from)?;

        // Clear existing balances for this wallet
        if let Err(e) = conn.execute(
            "DELETE FROM wallet_token_balances WHERE wallet_id = ?1",
            params![wallet_id],
        ) {
            let _ = conn.execute("ROLLBACK", []);
            return Err(DatabaseError::from(e).into());
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
                return Err(DatabaseError::from(e).into());
            }
        }

        conn.execute("COMMIT", []).map_err(DatabaseError::from)?;

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
