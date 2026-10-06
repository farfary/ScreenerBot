// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Balance snapshot database — stores historical balance snapshots for tracking.

use crate::database::WriteTransaction;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

use crate::logger::{self, LogTag};

use crate::errors::DatabaseError;
use crate::wallets::Error;

use super::super::cache::update_wallet_snapshot_status;
use super::super::types::WalletSnapshot;
use super::WalletDatabase;

impl WalletDatabase {
    /// Save wallet snapshot with token balances
    pub fn save_wallet_snapshot(&self, snapshot: &WalletSnapshot) -> Result<i64, Error> {
        let mut conn = self.get_connection()?;
        let tx = conn.write_tx().map_err(DatabaseError::from)?;

        // Insert wallet snapshot
        let snapshot_id = tx
            .query_row(
                r#"
            INSERT INTO wallet_snapshots (
                chain_id, wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_native,
                total_tokens_count, total_nfts_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) RETURNING id
            "#,
                params![
                    self.chain.as_str(), self.subject,
                    snapshot.snapshot_time.to_rfc3339(),
                    snapshot.native_balance,
                    snapshot.native_balance_raw as i64,
                    snapshot.total_equity_native,
                    snapshot.total_tokens_count as i64,
                    snapshot.total_nfts_count as i64
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(DatabaseError::from)?;

        // Insert token balances
        for token_balance in &snapshot.token_balances {
            tx.execute(
                r#"
                INSERT INTO token_balances (
                    snapshot_id, mint, balance, balance_ui, decimals, is_token_2022
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![
                    snapshot_id,
                    token_balance.mint,
                    token_balance.balance,
                    token_balance.balance_ui,
                    token_balance.decimals,
                    token_balance.is_token_2022
                ],
            )
            .map_err(DatabaseError::from)?;
        }

        // Insert NFT balances
        for nft_balance in &snapshot.nft_balances {
            tx.execute(
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
            .map_err(DatabaseError::from)?;
        }

        tx.commit().map_err(DatabaseError::from)?;

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
    /// to be the same quantity as the headline. Reading `native_balance` here while the
    /// headline showed equity meant a wallet holding tokens reported a phantom gain
    /// equal to its entire holdings value, every day. Rows predating the equity column
    /// fall back to their SOL balance.
    /// Uses idx_wallet_snapshots_time index for fast descending time lookup.
    pub fn get_balance_at_time(&self, target_time: DateTime<Utc>) -> Result<Option<f64>, Error> {
        let conn = self.get_connection()?;

        let result = conn
            .query_row(
                r#"
            SELECT COALESCE(total_equity_native, native_balance)
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2 AND datetime(snapshot_time) <= datetime(?3)
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#,
                params![self.chain.as_str(), self.subject, target_time.to_rfc3339()],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;

        Ok(result)
    }

    /// Get the end-of-day wallet WORTH for each calendar day (UTC) within a period.
    /// Picks the last snapshot recorded on each day. Used by the home portfolio calendar.
    /// Returns pairs of (YYYY-MM-DD, total_equity_native) ordered ascending by day.
    pub fn get_daily_end_balances(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<(String, f64)>, Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT strftime('%Y-%m-%d', snapshot_time) AS day, COALESCE(total_equity_native, native_balance)
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2 AND id IN (
                SELECT MAX(id)
                FROM wallet_snapshots
                WHERE chain_id = ?1 AND wallet_address = ?2
                  AND datetime(snapshot_time) >= datetime(?3)
                  AND datetime(snapshot_time) < datetime(?4)
                GROUP BY strftime('%Y-%m-%d', snapshot_time)
            )
            ORDER BY day ASC
            "#,
            )
            .map_err(DatabaseError::from)?;

        let rows = stmt
            .query_map(
                params![
                    self.chain.as_str(),
                    self.subject,
                    start.to_rfc3339(),
                    end.to_rfc3339()
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
            )
            .map_err(DatabaseError::from)?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(DatabaseError::from)?);
        }
        Ok(result)
    }

    /// Get the most recent snapshot timestamp (if any) without loading token data
    pub fn get_latest_snapshot_time(&self) -> Result<Option<DateTime<Utc>>, Error> {
        let conn = self.get_connection()?;

        let snapshot_time_str: Option<String> = conn
            .query_row(
                r#"
            SELECT snapshot_time
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#,
                params![self.chain.as_str(), self.subject],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;

        if let Some(ts_str) = snapshot_time_str {
            let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                .map_err(|_| DatabaseError::Query {
                    operation: "get_latest_snapshot_time".to_owned(),
                    message: format!("invalid snapshot_time stored: {ts_str}"),
                })?
                .with_timezone(&Utc);
            Ok(Some(timestamp))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod amount_tests {
    use super::*;
    use crate::chains::{ChainId, RawAmount};
    use crate::wallets::balance_monitor::database::schema::{
        LEGACY_TOKEN_BALANCES_SCHEMA, SCHEMA_NFT_BALANCES, SCHEMA_TOKEN_BALANCES,
        SCHEMA_WALLET_SNAPSHOTS, WALLET_INDEXES,
    };
    use crate::wallets::balance_monitor::types::{NftBalance, SnapshotTokenBalance};
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;
    use rusqlite::Connection;

    async fn open(path: &std::path::Path) -> WalletDatabase {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            crate::database::configure_connection(conn, crate::database::WALLET_MONITOR_DB)
        });
        let mut db = WalletDatabase {
            pool: Pool::builder()
                .max_size(1)
                .idle_timeout(None)
                .max_lifetime(None)
                .build(manager)
                .unwrap(),
            database_path: path.to_string_lossy().into_owned(),
            schema_version: super::super::schema::WALLET_SCHEMA_VERSION,
            chain: ChainId::Solana,
            subject: "wallet000".to_owned(),
        };
        db.initialize_schema().await.unwrap();
        db
    }

    fn fixture(path: &std::path::Path, token_schema: &str) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(SCHEMA_WALLET_SNAPSHOTS).unwrap();
        conn.execute_batch(token_schema).unwrap();
        conn.execute_batch(SCHEMA_NFT_BALANCES).unwrap();
        for index in WALLET_INDEXES {
            conn.execute_batch(index).unwrap();
        }
        for (n, raw) in [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]
            .into_iter()
            .enumerate()
        {
            let id = n as i64 + 1;
            let bits = i64::from_ne_bytes(raw.to_ne_bytes());
            conn.execute("INSERT INTO wallet_snapshots (id, wallet_address, snapshot_time, native_balance, native_balance_raw, total_tokens_count) VALUES (?1, 'wallet000', '2026-10-05T00:00:00+00:00', 1.5, 1500000000, 1)", [id]).unwrap();
            conn.execute("INSERT INTO token_balances (id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022, created_at) VALUES (?1, ?2, ?3, ?4, 1.25, 6, 1, '2026-10-05 00:00:00')",
                params![id + 10, id, format!("mint{n}"), bits]).unwrap();
        }
        conn.execute("INSERT INTO nft_balances (snapshot_id, mint, account_address) VALUES (1, 'historic_nft', 'account')", []).unwrap();
        conn.execute("INSERT INTO token_balances (id, snapshot_id, mint, balance, balance_ui) VALUES (100, 1, 'sequence_marker', 0, 0)", []).unwrap();
        conn.execute("DELETE FROM token_balances WHERE id = 100", [])
            .unwrap();
    }

    fn snapshot() -> WalletSnapshot {
        WalletSnapshot {
            id: None,
            wallet_address: "wallet000".to_owned(),
            snapshot_time: Utc::now(),
            native_balance: 1.5,
            native_balance_raw: 1_500_000_000,
            total_equity_native: 2.0,
            total_tokens_count: 1,
            total_nfts_count: 1,
            token_balances: vec![SnapshotTokenBalance {
                id: None,
                snapshot_id: None,
                mint: "wide".to_owned(),
                balance: RawAmount::MAX,
                balance_ui: 1.0,
                decimals: 0,
                is_token_2022: false,
            }],
            nft_balances: vec![NftBalance {
                id: None,
                snapshot_id: None,
                mint: "nft".to_owned(),
                account_address: "account".to_owned(),
                name: None,
                symbol: None,
                image_url: None,
                is_token_2022: false,
            }],
        }
    }

    #[tokio::test]
    async fn historical_snapshot_balances_migrate_exactly_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.db");
        fixture(&path, LEGACY_TOKEN_BALANCES_SCHEMA);
        for pass in 0..2 {
            let db = open(&path).await;
            for (n, raw) in [0, i64::MAX as u64, i64::MAX as u64 + 1, u64::MAX]
                .into_iter()
                .enumerate()
            {
                let rows = db.get_token_balances(n as i64 + 1).unwrap();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].balance, RawAmount::from(raw));
                assert_eq!(
                    (
                        rows[0].id,
                        rows[0].balance_ui,
                        rows[0].decimals,
                        rows[0].is_token_2022
                    ),
                    (Some(n as i64 + 11), 1.25, 6, true)
                );
            }
            if pass == 0 {
                let id = db.save_wallet_snapshot(&snapshot()).unwrap();
                let row = db.get_token_balances(id).unwrap().remove(0);
                assert_eq!((row.balance, row.id), (RawAmount::MAX, Some(101)));
            }
            let conn = db.get_connection().unwrap();
            let (kind, value): (String, String) = conn
                .query_row(
                    "SELECT typeof(balance), balance FROM token_balances WHERE mint = 'mint3'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                (kind.as_str(), value.as_str()),
                ("text", "18446744073709551615")
            );
            assert_eq!(
                conn.query_row(
                    "SELECT type FROM pragma_table_info('token_balances') WHERE name = 'balance'",
                    [],
                    |row| row.get::<_, String>(0)
                )
                .unwrap(),
                "TEXT"
            );
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE tbl_name = 'token_balances' AND type = 'index'", [], |row| row.get::<_, i64>(0)).unwrap(), 3);
            let ddl: String = conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name = 'token_balances'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                ddl.split_once('(').unwrap().1.trim(),
                SCHEMA_TOKEN_BALANCES
                    .split_once('(')
                    .unwrap()
                    .1
                    .trim()
                    .trim_end_matches(';')
                    .trim()
            );
            for index in WALLET_INDEXES
                .iter()
                .filter(|sql| sql.contains(" ON token_balances("))
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
                conn.query_row(
                    "SELECT COUNT(*) FROM nft_balances WHERE mint = 'historic_nft'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
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

    #[tokio::test]
    async fn unfamiliar_snapshot_balance_shape_and_value_fail_without_data_loss() {
        for schema in [
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL,\n    extra TEXT,"),
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL CHECK(balance IS NOT NULL),"),
            LEGACY_TOKEN_BALANCES_SCHEMA.replace("    balance INTEGER NOT NULL,", "    balance INTEGER NOT NULL,\n    generated INTEGER GENERATED ALWAYS AS (balance + 1) VIRTUAL,"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("wallet.db");
            fixture(&path, &schema);
            let before = Connection::open(&path).unwrap().query_row("SELECT sql FROM sqlite_master WHERE name = 'token_balances'", [], |row| row.get::<_, String>(0)).unwrap();
            let manager = SqliteConnectionManager::file(&path)
                .with_init(|conn| crate::database::configure_connection(conn, crate::database::WALLET_MONITOR_DB));
            let mut db = WalletDatabase {
                pool: Pool::builder().max_size(1).build(manager).unwrap(),
                database_path: path.to_string_lossy().into_owned(), schema_version: super::super::schema::WALLET_SCHEMA_VERSION,
                chain: ChainId::Solana, subject: "wallet000".to_owned(),
            };
            assert!(db.initialize_schema().await.is_err());
            let conn = Connection::open(&path).unwrap();
            assert_eq!(conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'token_balances'", [], |row| row.get::<_, String>(0)).unwrap(), before);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM token_balances", [], |row| row.get::<_, i64>(0)).unwrap(), 4);
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.db");
        let db = open(&path).await;
        let id = db.save_wallet_snapshot(&snapshot()).unwrap();
        let conn = db.get_connection().unwrap();
        conn.execute(
            "UPDATE token_balances SET balance = '01' WHERE snapshot_id = ?1",
            [id],
        )
        .unwrap();
        drop(conn);
        assert!(db.get_token_balances(id).is_err());
        let conn = db.get_connection().unwrap();
        conn.execute(
            "UPDATE token_balances SET balance = X'31' WHERE snapshot_id = ?1",
            [id],
        )
        .unwrap();
        drop(conn);
        assert!(db.get_token_balances(id).is_err());
    }

    #[tokio::test]
    async fn snapshot_child_failure_rolls_back_parent_and_all_children() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wallet.db");
        let db = open(&path).await;
        let conn = db.get_connection().unwrap();
        conn.execute_batch("CREATE TRIGGER reject_nft BEFORE INSERT ON nft_balances BEGIN SELECT RAISE(ABORT, 'rejected NFT'); END;").unwrap();
        drop(conn);
        assert!(db.save_wallet_snapshot(&snapshot()).is_err());
        let conn = db.get_connection().unwrap();
        for table in ["wallet_snapshots", "token_balances", "nft_balances"] {
            assert_eq!(
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }

    #[tokio::test]
    async fn unfamiliar_snapshot_balance_index_trigger_or_value_preserves_legacy_rows() {
        for extra in [
            "CREATE INDEX extra_balance_index ON token_balances(balance)",
            "CREATE TRIGGER extra_balance_trigger AFTER INSERT ON token_balances BEGIN SELECT 1; END",
            "UPDATE token_balances SET balance = 1.5 WHERE mint = 'mint0'",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("wallet.db");
            fixture(&path, LEGACY_TOKEN_BALANCES_SCHEMA);
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(extra).unwrap();
            let before = conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'token_balances'", [], |row| row.get::<_, String>(0)).unwrap();
            let before_value = conn.query_row("SELECT typeof(balance), quote(balance) FROM token_balances WHERE mint = 'mint0'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap();
            drop(conn);
            let manager = SqliteConnectionManager::file(&path)
                .with_init(|conn| crate::database::configure_connection(conn, crate::database::WALLET_MONITOR_DB));
            let mut db = WalletDatabase {
                pool: Pool::builder().max_size(1).build(manager).unwrap(),
                database_path: path.to_string_lossy().into_owned(), schema_version: super::super::schema::WALLET_SCHEMA_VERSION,
                chain: ChainId::Solana, subject: "wallet000".to_owned(),
            };
            assert!(db.initialize_schema().await.is_err());
            let conn = Connection::open(&path).unwrap();
            assert_eq!(conn.query_row("SELECT sql FROM sqlite_master WHERE name = 'token_balances'", [], |row| row.get::<_, String>(0)).unwrap(), before);
            assert_eq!(conn.query_row("SELECT typeof(balance), quote(balance) FROM token_balances WHERE mint = 'mint0'", [], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).unwrap(), before_value);
            assert_eq!(conn.query_row("SELECT COUNT(*) FROM token_balances", [], |row| row.get::<_, i64>(0)).unwrap(), 4);
        }
    }
}
