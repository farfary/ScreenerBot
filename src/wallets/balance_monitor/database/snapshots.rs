//! Balance snapshot database — stores historical balance snapshots for tracking.

use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::logger::{self, LogTag};

use super::super::cache::update_wallet_snapshot_status;
use super::super::types::WalletSnapshot;
use super::WalletDatabase;

impl WalletDatabase {
    /// Save wallet snapshot with token balances
    pub fn save_wallet_snapshot(&self, snapshot: &WalletSnapshot) -> Result<i64, String> {
        let conn = self.get_connection()?;

        // Insert wallet snapshot
        let snapshot_id = conn
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                wallet_address, snapshot_time, sol_balance, sol_balance_lamports, total_equity_sol,
                total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) RETURNING id
            "#,
                params![
                    snapshot.wallet_address,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.sol_balance,
                    snapshot.sol_balance_lamports as i64,
                    snapshot.total_equity_sol,
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

    /// Wallet WORTH (cash + holdings) at or before a specific time.
    ///
    /// This is the baseline every "change today" figure is measured against, so it has
    /// to be the same quantity as the headline. Reading `sol_balance` here while the
    /// headline showed equity meant a wallet holding tokens reported a phantom gain
    /// equal to its entire holdings value, every day. Rows predating the equity column
    /// fall back to their SOL balance.
    /// Uses idx_wallet_snapshots_time index for fast descending time lookup.
    pub fn get_balance_at_time(&self, target_time: DateTime<Utc>) -> Result<Option<f64>, String> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT COALESCE(total_equity_sol, sol_balance)
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

    /// Get the end-of-day wallet WORTH for each calendar day (UTC) within a period.
    /// Picks the last snapshot recorded on each day. Used by the home portfolio calendar.
    /// Returns pairs of (YYYY-MM-DD, total_equity_sol) ordered ascending by day.
    pub fn get_daily_end_balances(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(String, f64)>, String> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT strftime('%Y-%m-%d', snapshot_time) AS day, COALESCE(total_equity_sol, sol_balance)
            FROM wallet_snapshots
            WHERE id IN (
                SELECT MAX(id)
                FROM wallet_snapshots
                WHERE datetime(snapshot_time) >= datetime(?1)
                  AND datetime(snapshot_time) < datetime(?2)
                GROUP BY strftime('%Y-%m-%d', snapshot_time)
            )
            ORDER BY day ASC
            "#,
            )
            .map_err(|e| format!("Failed to prepare daily balances query: {e}"))?;

        let rows = stmt
            .query_map(params![start.to_rfc3339(), end.to_rfc3339()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|e| format!("Failed to execute daily balances query: {e}"))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| format!("Failed to read daily balance row: {e}"))?);
        }
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
}
