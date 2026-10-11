// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! SQLite database operations for wallet balance monitoring.

use chrono::{DateTime, Utc};
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::sync::LazyLock;
use tokio::sync::Mutex;

use crate::chains::ChainId;
use crate::database::schema::{ddl, SchemaContext, StoreSchema};
use crate::database::{self, catalog};
use crate::errors::DatabaseError;
use crate::logger::{self, LogTag};
use crate::wallets::Error;

use super::types::*;

pub(crate) mod column_names;
mod dashboard_metrics;
mod flow_cache;
mod metrics;
mod schema;
mod snapshots;

use schema::{
    DASHBOARD_METRICS_INDEXES, FLOW_CACHE_INDEXES, LEGACY_TOKEN_BALANCES_SCHEMA,
    SCHEMA_NATIVE_FLOW_CACHE, SCHEMA_NFT_BALANCES, SCHEMA_TOKEN_BALANCES,
    SCHEMA_WALLET_DASHBOARD_METRICS, SCHEMA_WALLET_METADATA, SCHEMA_WALLET_SNAPSHOTS,
    WALLET_INDEXES, WALLET_SCHEMA_VERSION,
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
    /// The wallet address every read/write in this database is scoped to.
    /// Resolved ONCE (here and on `rebind_subject`) rather than re-derived on
    /// every query, so a query never has to re-resolve "the configured
    /// wallet" — and never touches signing/key material to do it, since
    /// `configured_address_async()` is key-free once the multi-wallet
    /// database is initialized. See `rebind_subject` for how this stays
    /// current when the main wallet changes.
    subject: String,
}
/// The schema of the wallet-monitor store.
pub(crate) const SCHEMA: StoreSchema = StoreSchema {
    version: WALLET_SCHEMA_VERSION as i64,
    min_reader_version: WALLET_SCHEMA_VERSION as i64,
    baseline: WALLET_SCHEMA_VERSION as i64,
    renames: Some(&column_names::RENAMES),
    step,
    legacy_stamp: None,
};

fn step(tx: &Transaction<'_>, _: &SchemaContext) -> Result<(), DatabaseError> {
    WalletDatabase::create_schema(tx).map_err(|error| match error {
        Error::Database(error) => error,
        other => DatabaseError::Migration {
            store: "wallet.db".to_owned(),
            detail: other.to_string(),
        },
    })
}

impl WalletDatabase {
    /// The wallet-monitor database of `chain`, opened by the storage phase, scoped to
    /// the configured wallet.
    pub async fn new(chain: ChainId) -> Result<Self, Error> {
        let pool = database::open::store(&catalog::WALLET)?;
        let subject = crate::chains::solana::accounts::configured_address_async()
            .await
            .map_err(|e| Error::Dependency {
                dependency: "accounts",
                detail: e.to_string(),
            })?;
        let db = WalletDatabase {
            pool,
            database_path: catalog::WALLET.path(chain).to_string_lossy().to_string(),
            schema_version: WALLET_SCHEMA_VERSION,
            chain,
            subject,
        };
        db.record_current_wallet()?;
        Ok(db)
    }

    /// Rebind the wallet-monitor's subject to whichever wallet is now main,
    /// without decrypting anything (`configured_address_async()` is
    /// key-free once the multi-wallet database is initialized). Called by
    /// `crate::wallets::manager::crud` after any mutation that can change
    /// the main wallet, so every subsequent snapshot/dashboard/flow query
    /// scopes to the new wallet instead of a stale cached address.
    pub async fn rebind_subject(&mut self) -> Result<(), Error> {
        self.subject = crate::chains::solana::accounts::configured_address_async()
            .await
            .map_err(|e| Error::Dependency {
                dependency: "accounts",
                detail: e.to_string(),
            })?;
        Ok(())
    }

    /// Records the subject as the wallet this chain's rows describe.
    fn record_current_wallet(&self) -> Result<(), Error> {
        self.get_connection()?
            .execute(
                "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES (?1, ?2)",
                params![
                    format!("current_wallet:{}", self.chain.as_str()),
                    self.subject
                ],
            )
            .map_err(|e| DatabaseError::classify_sqlite_failure("record current wallet", e))?;
        Ok(())
    }

    /// Creates and migrates every table. The unit-neutral renames have already run,
    /// so every step reads the canonical shape; foreign keys are off, so the table
    /// rebuilds never cascade into child rows.
    fn create_schema(tx: &Transaction<'_>) -> Result<(), Error> {
        database::schema::renames::rename(tx, &column_names::RENAMES)?;

        ddl::run(tx, "create wallet_snapshots", SCHEMA_WALLET_SNAPSHOTS)?;
        ddl::run(tx, "create token_balances", SCHEMA_TOKEN_BALANCES)?;
        Self::migrate_token_balances(tx)?;
        ddl::run(tx, "create nft_balances", SCHEMA_NFT_BALANCES)?;
        ddl::run(tx, "create wallet_metadata", SCHEMA_WALLET_METADATA)?;
        ddl::run(tx, "create native_flow_cache", SCHEMA_NATIVE_FLOW_CACHE)?;
        ddl::run(
            tx,
            "create wallet_dashboard_metrics",
            SCHEMA_WALLET_DASHBOARD_METRICS,
        )?;

        ddl::add_column(
            tx,
            "wallet_snapshots",
            "total_nfts_count",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        // Schema v4. Nullable on purpose: rows written before worth was tracked have
        // no honest equity to backfill (we would have to value yesterday's holdings at
        // today's prices), so they read back as their SOL balance via COALESCE.
        ddl::add_column(tx, "wallet_snapshots", "total_equity_native", "REAL")?;

        Self::migrate_chain_identity(tx)?;

        for index_sql in WALLET_INDEXES
            .iter()
            .chain(FLOW_CACHE_INDEXES)
            .chain(DASHBOARD_METRICS_INDEXES)
        {
            ddl::run(tx, "create wallet-monitor index", index_sql)?;
        }

        tx.execute(
            "INSERT OR REPLACE INTO wallet_metadata (key, value) VALUES ('schema_version', ?1)",
            params![WALLET_SCHEMA_VERSION.to_string()],
        )
        .map_err(|e| DatabaseError::classify_sqlite_failure("stamp wallet-monitor schema", e))?;
        Ok(())
    }

    fn migrate_token_balances(conn: &Connection) -> Result<(), Error> {
        let stored: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'token_balances'",
                [],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)?;
        fn body(ddl: &str) -> Option<&str> {
            ddl.split_once('(')
                .map(|(_, rest)| rest.trim().trim_end_matches(';').trim())
        }
        let legacy = body(&stored) == body(LEGACY_TOKEN_BALANCES_SCHEMA);
        if !legacy && body(&stored) != body(SCHEMA_TOKEN_BALANCES) {
            return Err(Error::Migration {
                step: "inspect snapshot token balances".to_owned(),
                detail: "unrecognized table definition".to_owned(),
            });
        }
        if !legacy {
            return Ok(());
        }

        let objects = conn.prepare("SELECT type, name, sql FROM sqlite_master WHERE tbl_name = 'token_balances' AND type IN ('index', 'trigger') AND sql IS NOT NULL")
            .and_then(|mut stmt| stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>())
            .map_err(DatabaseError::from)?;
        let expected: Vec<String> = WALLET_INDEXES
            .iter()
            .filter(|sql| sql.contains(" ON token_balances("))
            .map(|sql| {
                sql.replace(" IF NOT EXISTS", "")
                    .trim_end_matches(';')
                    .to_owned()
            })
            .collect();
        if objects.len() != expected.len()
            || objects
                .iter()
                .any(|(kind, _, sql)| kind != "index" || !expected.contains(sql))
        {
            return Err(Error::Migration {
                step: "inspect snapshot token balance indexes".to_owned(),
                detail: "unrecognized index or trigger".to_owned(),
            });
        }
        let indexes: Vec<(String, String)> = conn
            .prepare("SELECT name, origin FROM pragma_index_list('token_balances')")
            .and_then(|mut stmt| {
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .map_err(DatabaseError::from)?;
        if indexes.len() != expected.len()
            || indexes.iter().any(|(name, origin)| {
                origin != "c" || !objects.iter().any(|(_, known_name, _)| known_name == name)
            })
        {
            return Err(Error::Migration {
                step: "inspect snapshot token balance keys".to_owned(),
                detail: "unrecognized index".to_owned(),
            });
        }

        let previous_sequence: Option<i64> = conn
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name = 'token_balances'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(DatabaseError::from)?;
        conn.execute_batch(
            &SCHEMA_TOKEN_BALANCES
                .replace("CREATE TABLE IF NOT EXISTS", "CREATE TABLE")
                .replace("token_balances", "token_balances__raw_amount"),
        )
        .map_err(DatabaseError::from)?;
        {
            let mut stmt = conn.prepare("SELECT id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022, created_at FROM token_balances")
                .map_err(DatabaseError::from)?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                })
                .map_err(DatabaseError::from)?;
            for row in rows {
                let (id, snapshot_id, mint, bits, balance_ui, decimals, is_token_2022, created_at) =
                    row.map_err(DatabaseError::from)?;
                let amount = crate::chains::RawAmount::from(u64::from_ne_bytes(bits.to_ne_bytes()));
                conn.execute("INSERT INTO token_balances__raw_amount (id, snapshot_id, mint, balance, balance_ui, decimals, is_token_2022, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![id, snapshot_id, mint, amount, balance_ui, decimals, is_token_2022, created_at])
                    .map_err(DatabaseError::from)?;
            }
        }
        conn.execute_batch("DROP TABLE token_balances; ALTER TABLE token_balances__raw_amount RENAME TO token_balances;")
            .map_err(DatabaseError::from)?;
        if let Some(sequence) = previous_sequence {
            conn.execute(
                "UPDATE sqlite_sequence SET seq = MAX(seq, ?1) WHERE name = 'token_balances'",
                [sequence],
            )
            .map_err(DatabaseError::from)?;
        }
        for index in WALLET_INDEXES
            .iter()
            .filter(|sql| sql.contains(" ON token_balances("))
        {
            conn.execute(index, []).map_err(DatabaseError::from)?;
        }
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(DatabaseError::from)?;
        if violations != 0 {
            return Err(Error::Migration {
                step: "verify snapshot token balance foreign keys".to_owned(),
                detail: format!("{violations} violations"),
            });
        }
        Ok(())
    }

    /// Rebuild the pre-chain-identity wallet-monitor tables to the
    /// `(chain_id, wallet_address, ...)` shape.
    ///
    /// Runs inside the open's transaction with `foreign_keys` off, after the
    /// unit-neutral renames, so the rebuild reads and
    /// writes the canonical column and table names.
    fn migrate_chain_identity(conn: &Connection) -> Result<(), Error> {
        if ddl::has_column(conn, "wallet_snapshots", "chain_id")? {
            return Ok(());
        }

        fn row_count(conn: &Connection, table: &str) -> Result<i64, Error> {
            conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| {
                DatabaseError::classify_sqlite_failure(&format!("count wallet-monitor {table}"), e)
                    .into()
            })
        }
        let expected = [
            ("wallet_snapshots", row_count(conn, "wallet_snapshots")?),
            ("native_flow_cache", row_count(conn, "native_flow_cache")?),
            (
                "wallet_dashboard_metrics",
                row_count(conn, "wallet_dashboard_metrics")?,
            ),
        ];

        conn.execute_batch("CREATE TABLE wallet_snapshots__chain_v1 (id INTEGER PRIMARY KEY AUTOINCREMENT, chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL, snapshot_time TEXT NOT NULL, native_balance REAL NOT NULL, native_balance_raw INTEGER NOT NULL, total_equity_native REAL, total_tokens_count INTEGER NOT NULL DEFAULT 0, total_nfts_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now'))); INSERT INTO wallet_snapshots__chain_v1 (id, chain_id, wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_native, total_tokens_count, total_nfts_count, created_at) SELECT id, 'solana', wallet_address, snapshot_time, native_balance, native_balance_raw, total_equity_native, total_tokens_count, total_nfts_count, created_at FROM wallet_snapshots; DROP TABLE wallet_snapshots; ALTER TABLE wallet_snapshots__chain_v1 RENAME TO wallet_snapshots; CREATE TABLE native_flow_cache__chain_v1 (chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL DEFAULT '', signature TEXT NOT NULL, timestamp TEXT NOT NULL, native_delta REAL NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY(chain_id, wallet_address, signature)); INSERT INTO native_flow_cache__chain_v1 (chain_id, wallet_address, signature, timestamp, native_delta, created_at) SELECT 'solana', COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), ''), signature, timestamp, native_delta, created_at FROM native_flow_cache; DROP TABLE native_flow_cache; ALTER TABLE native_flow_cache__chain_v1 RENAME TO native_flow_cache; CREATE TABLE wallet_dashboard_metrics__chain_v1 (chain_id TEXT NOT NULL DEFAULT 'solana', wallet_address TEXT NOT NULL DEFAULT '', window_key TEXT NOT NULL, window_hours INTEGER NOT NULL, snapshot_limit INTEGER NOT NULL, token_limit INTEGER NOT NULL, payload_blob BLOB NOT NULL, payload_format TEXT NOT NULL DEFAULT 'json-gzip', computed_at TEXT NOT NULL, valid_until TEXT NOT NULL, computation_duration_ms INTEGER, snapshot_count INTEGER NOT NULL DEFAULT 0, flow_cache_rows INTEGER NOT NULL DEFAULT 0, last_processed_timestamp TEXT, last_processed_signature TEXT, window_start TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')), PRIMARY KEY(chain_id, wallet_address, window_key)); INSERT INTO wallet_dashboard_metrics__chain_v1 (chain_id, wallet_address, window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, last_processed_timestamp, last_processed_signature, window_start, created_at, updated_at) SELECT 'solana', COALESCE((SELECT value FROM wallet_metadata WHERE key = 'current_wallet'), ''), window_key, window_hours, snapshot_limit, token_limit, payload_blob, payload_format, computed_at, valid_until, computation_duration_ms, snapshot_count, flow_cache_rows, last_processed_timestamp, last_processed_signature, window_start, created_at, updated_at FROM wallet_dashboard_metrics; DROP TABLE wallet_dashboard_metrics; ALTER TABLE wallet_dashboard_metrics__chain_v1 RENAME TO wallet_dashboard_metrics;")
            .map_err(|e| DatabaseError::classify_sqlite_failure("rebuild wallet-monitor tables by chain", e))?;

        for (table, count) in expected {
            let actual = row_count(conn, table)?;
            if actual != count {
                return Err(Error::Migration {
                    step: format!("row count check ({table})"),
                    detail: format!("expected {count}, found {actual}"),
                });
            }
        }
        let fk_errors: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .map_err(|e| {
                DatabaseError::classify_sqlite_failure("verify wallet-monitor foreign keys", e)
            })?;
        if fk_errors != 0 {
            return Err(Error::Migration {
                step: "foreign key check".to_owned(),
                detail: format!("found {fk_errors} errors"),
            });
        }

        Ok(())
    }

    /// Get database connection from pool
    fn get_connection(&self) -> Result<PooledConnection<SqliteConnectionManager>, Error> {
        self.pool.get().map_err(|e| DatabaseError::from(e).into())
    }

    /// Get recent wallet snapshots.
    ///
    /// Token/NFT balances are NOT loaded (one query per snapshot would make the home
    /// dashboard's 30-snapshot history 30x more expensive). Callers that need holdings
    /// must use `get_wallet_worth()` (live) or `get_latest_snapshot_with_balances()`.
    pub fn get_recent_snapshots(&self, limit: usize) -> Result<Vec<WalletSnapshot>, Error> {
        let conn = self.get_connection()?;

        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, wallet_address, snapshot_time, native_balance, native_balance_raw,
                   COALESCE(total_equity_native, native_balance), total_tokens_count, COALESCE(total_nfts_count, 0)
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2
            ORDER BY snapshot_time DESC
            LIMIT ?3
            "#
            )
            .map_err(DatabaseError::from)?;

        let snapshot_iter = stmt
            .query_map(params![self.chain.as_str(), self.subject, limit], |row| {
                Self::map_snapshot_row(row)
            })
            .map_err(DatabaseError::from)?;

        let mut snapshots = Vec::new();
        for snapshot_result in snapshot_iter {
            snapshots.push(snapshot_result.map_err(DatabaseError::from)?);
        }

        Ok(snapshots)
    }

    /// The newest snapshot WITH its token and NFT balances loaded.
    ///
    /// Used once at startup to hydrate the live worth cache, so the header and hero
    /// show a real figure before the first collection tick rather than zeroes.
    pub fn get_latest_snapshot_with_balances(&self) -> Result<Option<WalletSnapshot>, Error> {
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
            native_balance: row.get(3)?,
            native_balance_raw: row.get::<_, i64>(4)? as u64,
            total_equity_native: row.get(5)?,
            total_tokens_count: row.get::<_, i64>(6)? as u32,
            total_nfts_count: row.get::<_, i64>(7)? as u32,
            token_balances: Vec::new(), // Loaded separately if needed
            nft_balances: Vec::new(),   // Loaded separately if needed
        })
    }

    /// Get wallet monitoring statistics
    pub fn get_monitor_stats(&self) -> Result<WalletMonitorStats, Error> {
        let conn = self.get_connection()?;

        let total_snapshots: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM wallet_snapshots WHERE chain_id = ?1 AND wallet_address = ?2",
                params![self.chain.as_str(), self.subject],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)?;

        // Get latest snapshot info
        let latest_info: Option<(String, String, f64, i64)> = conn
            .query_row(
                r#"
            SELECT wallet_address, snapshot_time, native_balance, total_tokens_count
            FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2
            ORDER BY snapshot_time DESC
            LIMIT 1
            "#,
                params![self.chain.as_str(), self.subject],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(DatabaseError::from)?;

        let (wallet_address, latest_snapshot_time, current_native_balance, current_tokens_count) =
            if let Some((addr, time_str, balance, count)) = latest_info {
                let time = DateTime::parse_from_rfc3339(&time_str)
                    .map_err(|e| Error::Migration {
                        step: "parse latest snapshot time".to_owned(),
                        detail: e.to_string(),
                    })?
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
            current_native_balance,
            current_tokens_count,
            database_size_bytes: database_size,
            schema_version: self.schema_version,
        })
    }

    /// Get token balances for a specific snapshot
    pub fn get_token_balances(&self, snapshot_id: i64) -> Result<Vec<SnapshotTokenBalance>, Error> {
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
            .map_err(DatabaseError::from)?;

        let balances_iter = stmt
            .query_map(params![snapshot_id, self.chain.as_str()], |row| {
                Ok(SnapshotTokenBalance {
                    id: Some(row.get(0)?),
                    snapshot_id: Some(row.get(1)?),
                    mint: row.get(2)?,
                    balance: row.get(3)?,
                    balance_ui: row.get(4)?,
                    decimals: row.get::<_, i64>(5)? as u8,
                    is_token_2022: row.get(6)?,
                })
            })
            .map_err(DatabaseError::from)?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(balance_result.map_err(DatabaseError::from)?);
        }

        Ok(balances)
    }

    /// Get NFT balances for a specific snapshot
    pub fn get_nft_balances(&self, snapshot_id: i64) -> Result<Vec<NftBalance>, Error> {
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
            .map_err(DatabaseError::from)?;

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
            .map_err(DatabaseError::from)?;

        let mut balances = Vec::new();
        for balance_result in balances_iter {
            balances.push(balance_result.map_err(DatabaseError::from)?);
        }

        Ok(balances)
    }

    /// Cleanup old snapshots (keep last 1000)
    pub fn cleanup_old_snapshots(&self) -> Result<u64, Error> {
        let conn = self.get_connection()?;

        let deleted_count = conn
            .execute(
                r#"
            DELETE FROM wallet_snapshots
            WHERE chain_id = ?1 AND wallet_address = ?2 AND id NOT IN (
                SELECT id FROM wallet_snapshots
                WHERE chain_id = ?1 AND wallet_address = ?2
                ORDER BY snapshot_time DESC
                LIMIT 1000
            )
            "#,
                params![self.chain.as_str(), self.subject],
            )
            .map_err(DatabaseError::from)?;

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

    fn upgrade(conn: &mut Connection) -> Result<(), DatabaseError> {
        crate::database::open::upgrade_connection(conn, &catalog::WALLET, vec![ChainId::Solana])
    }

    #[test]
    fn legacy_wallet_monitor_tables_migrate_to_chain_scoped_schema_losslessly_and_idempotently() {
        let mut conn = legacy_connection();

        upgrade(&mut conn).expect("migrate legacy wallet-monitor database");
        upgrade(&mut conn).expect("repeat wallet-monitor chain migration");

        for (table, expected) in [
            ("wallet_snapshots", 1),
            ("native_flow_cache", 1),
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

        // native_flow_cache and wallet_dashboard_metrics had no wallet_address column
        // pre-migration — it is backfilled from wallet_metadata's current_wallet.
        let flow_wallet: String = conn
            .query_row(
                "SELECT wallet_address FROM native_flow_cache LIMIT 1",
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

    /// Opens a current wallet-monitor store holding `metadata`, re-creates the legacy
    /// flow table an older build writes with one row, reopens it and returns the
    /// wallet the merged row is attributed to.
    fn merged_flow_wallet(metadata: &[(&str, &str)]) -> String {
        let mut conn = Connection::open_in_memory().expect("open test database");
        upgrade(&mut conn).expect("create wallet-monitor schema");
        for (key, value) in metadata {
            conn.execute(
                "INSERT INTO wallet_metadata (key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .expect("record wallet metadata");
        }
        conn.execute_batch(
            "CREATE TABLE sol_flow_cache (
                signature TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                sol_delta REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            INSERT INTO sol_flow_cache (signature, timestamp, sol_delta)
            VALUES ('sig-legacy', '2026-01-01T00:00:00Z', 0.25);",
        )
        .expect("re-create the legacy flow table");

        upgrade(&mut conn).expect("merge the legacy flow table");

        conn.query_row(
            "SELECT wallet_address FROM native_flow_cache WHERE signature = 'sig-legacy'",
            [],
            |row| row.get(0),
        )
        .expect("read merged flow row")
    }

    #[test]
    fn merged_legacy_flow_rows_take_the_chain_scoped_wallet_key_first() {
        let wallet = merged_flow_wallet(&[
            ("current_wallet:solana", "SCOPED_WALLET"),
            ("current_wallet", "UNSCOPED_WALLET"),
        ]);
        assert_eq!(wallet, "SCOPED_WALLET");
    }

    #[test]
    fn merged_legacy_flow_rows_fall_back_to_the_unscoped_wallet_key() {
        let wallet = merged_flow_wallet(&[("current_wallet", "UNSCOPED_WALLET")]);
        assert_eq!(wallet, "UNSCOPED_WALLET");
    }

    /// A full disk during the chain-identity rebuild is a failure of the store's
    /// surroundings: the open keeps the disk-full class, so recovery never rebuilds the
    /// store over it.
    #[test]
    fn disk_full_during_chain_identity_rebuild_is_an_environment_failure() {
        let mut conn = legacy_connection();
        let pages: i64 = conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .expect("read page count");
        conn.execute_batch(&format!("PRAGMA max_page_count = {pages}"))
            .expect("cap the database at its current size");

        let error = upgrade(&mut conn).expect_err("a capped database cannot be rebuilt");

        assert!(
            matches!(error, DatabaseError::DiskFull { .. }),
            "open must keep the disk-full class, got {error:?}"
        );
        assert!(error.is_environment_failure());
    }

    /// A migration that fails partway (here: the rebuild's very first `CREATE
    /// TABLE` collides with a pre-existing `wallet_snapshots__chain_v1`, a
    /// deterministic and reproducible way to fail before any row is copied)
    /// must leave the connection exactly as it found it: autocommit restored and
    /// the original legacy data untouched, because every step of the open runs in
    /// one transaction.
    #[test]
    fn failed_wallet_monitor_chain_migration_leaves_connection_clean_and_data_intact() {
        let mut conn = legacy_connection();
        conn.execute_batch("CREATE TABLE wallet_snapshots__chain_v1 (poison INTEGER);")
            .expect("seed a colliding table to force the migration to fail");

        let result = upgrade(&mut conn);
        assert!(result.is_err(), "colliding table must fail the migration");

        assert!(
            conn.is_autocommit(),
            "a failed migration must not leave the connection mid-transaction"
        );

        // Original legacy data must be exactly as it was -- the migration never
        // reached a copy or a commit.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wallet_snapshots", [], |row| {
                row.get(0)
            })
            .expect("count legacy wallet_snapshots rows");
        assert_eq!(count, 1, "legacy wallet_snapshots row must survive intact");
        let has_chain_id: bool = conn
            .prepare("PRAGMA table_info(wallet_snapshots)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .any(|column| column == "chain_id");
        assert!(
            !has_chain_id,
            "legacy wallet_snapshots must still be pre-migration shape"
        );
        let current_wallet: String = conn
            .query_row(
                "SELECT value FROM wallet_metadata WHERE key = 'current_wallet'",
                [],
                |row| row.get(0),
            )
            .expect("read current_wallet metadata");
        assert_eq!(current_wallet, "MAIN_WALLET_ADDR");

        // Now remove the collision and confirm the migration is still usable
        // (retryable) and idempotent from a clean connection state.
        conn.execute_batch("DROP TABLE wallet_snapshots__chain_v1;")
            .expect("remove colliding table");
        upgrade(&mut conn).expect("migration succeeds once the collision is gone");
        upgrade(&mut conn).expect("second migration call remains a no-op");
        assert!(conn.is_autocommit());
    }
}
