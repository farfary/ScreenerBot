// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Core PoolsDatabase struct and operations.

use super::super::types::{
    PoolBlacklistPolicy, PoolFailureRecord, PriceResult, PRICE_HISTORY_MAX_ENTRIES,
};
use super::types::DbPriceResult;
use super::writer::run_database_writer;
use crate::logger::{self, LogTag};

use crate::chains::ChainId;
use crate::database;
use crate::errors::{DatabaseError, InternalError};
use crate::pools::Error;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::mpsc;

/// Maximum age for price history entries (7 days)
const MAX_PRICE_HISTORY_AGE_DAYS: i64 = 7;

/// Maximum allowable gap between price updates (1 minute in seconds)
const MAX_PRICE_GAP_SECONDS: i64 = 60;

use super::migrations::migrate_schema;

// =============================================================================
// POOLS DATABASE
// =============================================================================

/// SQLite-based price history storage
#[derive(Debug)]
pub struct PoolsDatabase {
    pub(super) chain_id: ChainId,
    pub(super) db_path: String,
    /// Connection pool, built by `initialize`; `None` until then, which is
    /// exactly the uninitialized-store condition `Error::NotInitialized`
    /// describes.
    pub(super) pool: Option<Pool<SqliteConnectionManager>>,
    pub(super) write_queue: Option<mpsc::UnboundedSender<PriceResult>>,
    // In-memory blacklists (source of truth for runtime checks)
    pub(super) blacklisted_accounts: Arc<RwLock<HashSet<String>>>,
    /// Actively blacklisted pools -> unix time each leaves the blacklist
    pub(super) blacklisted_pools: Arc<RwLock<HashMap<String, i64>>>,
}

impl Clone for PoolsDatabase {
    fn clone(&self) -> Self {
        Self {
            chain_id: self.chain_id,
            db_path: self.db_path.clone(),
            pool: self.pool.clone(),
            write_queue: self.write_queue.clone(),
            blacklisted_accounts: Arc::clone(&self.blacklisted_accounts),
            blacklisted_pools: Arc::clone(&self.blacklisted_pools),
        }
    }
}

impl PoolsDatabase {
    /// Create new pools database instance
    pub fn new(chain_id: ChainId) -> Self {
        Self {
            chain_id,
            db_path: crate::paths::chain_db_path(crate::paths::DbKind::Pools, chain_id)
                .to_string_lossy()
                .to_string(),
            pool: None,
            write_queue: None,
            blacklisted_accounts: Arc::new(RwLock::new(HashSet::new())),
            blacklisted_pools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a clone suitable for async operations
    /// This is the same as clone() but with a more explicit name
    pub fn clone_for_async(&self) -> Self {
        self.clone()
    }

    /// The shared connection pool for `spawn_blocking` bodies — `Pool` is a
    /// cheap handle, so move a clone into the closure and check a connection
    /// out inside it. Fails with the uninitialized-store error until
    /// `initialize` has built the pool. Never hold one checkout while
    /// acquiring another (shared-store rule).
    pub(super) fn shared_pool(&self) -> Result<Pool<SqliteConnectionManager>, Error> {
        self.pool.clone().ok_or_else(|| Error::NotInitialized)
    }

    /// Initialize database and create tables
    pub async fn initialize(&mut self) -> Result<(), Error> {
        // Create the connection pool: every checkout applies the shared
        // SQLite configuration through the manager's init hook.
        let manager = SqliteConnectionManager::file(self.db_path.as_str())
            .with_init(|conn| database::configure_connection(conn, database::POOLS_DB));

        let pool = Pool::builder()
            .max_size(4)
            .idle_timeout(None)
            .max_lifetime(None)
            .build(manager)
            .map_err(|e| DatabaseError::Query {
                operation: "open pools database".to_owned(),
                message: e.to_string(),
            })?;

        // Migrate the schema on one checkout before the writer task or any
        // other reader exists.
        {
            let mut conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "open pools database".to_owned(),
                message: e.to_string(),
            })?;
            migrate_schema(&mut conn)?;
        }

        self.pool = Some(pool.clone());

        // Setup write queue for batched operations
        let (tx, rx) = mpsc::unbounded_channel();
        self.write_queue = Some(tx);

        // Start background writer task
        let chain_id = self.chain_id;
        tokio::spawn(run_database_writer(rx, pool.clone(), chain_id));

        logger::info(
            LogTag::PoolService,
            &format!("Pools database initialized: {}", self.db_path),
        );

        // Load blacklists into memory (priority for runtime checks). Pool rows
        // count only under the current blacklist policy: rows below the failure
        // threshold or older than the TTL stay in the table but do not exclude
        // their pool.
        let pool_policy = PoolBlacklistPolicy::from_config();

        let conn = pool.get().map_err(|e| DatabaseError::Query {
            operation: "load pool blacklist".to_owned(),
            message: e.to_string(),
        })?;

        // Accounts
        let account_keys = match conn
            .prepare("SELECT account_pubkey FROM blacklist_accounts WHERE chain_id = ?")
        {
            Ok(mut stmt) => {
                let rows = stmt.query_map([self.chain_id.as_str()], |row| row.get::<_, String>(0));
                match rows {
                    Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolService,
                            &format!("Failed to load blacklist_accounts into memory: {}", e),
                        );
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                logger::warning(
                    LogTag::PoolService,
                    &format!("Failed to prepare load for blacklist_accounts: {e}"),
                );
                Vec::new()
            }
        };

        // Pools
        let pool_rows = match conn.prepare(
            "SELECT pool_id, error_count, first_failed_at, last_failed_at \
             FROM blacklist_pools WHERE chain_id = ?",
        ) {
            Ok(mut stmt) => {
                let rows = stmt.query_map([self.chain_id.as_str()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        PoolFailureRecord {
                            error_count: row.get::<_, Option<i64>>(1)?.unwrap_or(1),
                            first_failed_at: row.get(2)?,
                            last_failed_at: row.get(3)?,
                        },
                    ))
                });
                match rows {
                    Ok(iter) => iter.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                    Err(e) => {
                        logger::warning(
                            LogTag::PoolService,
                            &format!("Failed to load blacklist_pools into memory: {e}"),
                        );
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                logger::warning(
                    LogTag::PoolService,
                    &format!("Failed to prepare load for blacklist_pools: {e}"),
                );
                Vec::new()
            }
        };

        // Populate memory sets
        {
            let mut accounts = self.blacklisted_accounts.write().unwrap();
            for key in account_keys {
                accounts.insert(key);
            }
        }

        let pool_rows_total = pool_rows.len();
        let now = chrono::Utc::now().timestamp();
        let active_pools = {
            let mut pools = self.blacklisted_pools.write().unwrap();
            for (pool_id, record) in pool_rows {
                if let Some(expires_at) = pool_policy.blacklist_expiry(&record) {
                    if now < expires_at {
                        pools.insert(pool_id, expires_at);
                    }
                }
            }
            pools.len()
        };

        logger::info(
            LogTag::PoolService,
            &format!(
                "Loaded pool blacklist: {} active of {} recorded (threshold {}, ttl {}s)",
                active_pools, pool_rows_total, pool_policy.threshold, pool_policy.ttl_secs
            ),
        );

        Ok(())
    }

    /// Queue a price result for async storage (non-blocking)
    pub async fn queue_price_for_storage(&self, price: PriceResult) -> Result<(), Error> {
        if let Some(ref tx) = self.write_queue {
            tx.send(price).map_err(|e| Error::QueueUnavailable {
                detail: format!("failed to queue price for storage: {e}"),
            })?;
            Ok(())
        } else {
            Err(Error::QueueUnavailable {
                detail: "write queue not initialized".to_owned(),
            })
        }
    }

    /// Load recent price history for cache initialization
    pub async fn load_recent_price_history(
        &self,
        mint: &str,
        limit: usize,
    ) -> Result<Vec<PriceResult>, Error> {
        let mint_str = mint.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ?
         ORDER BY timestamp_unix DESC
         LIMIT ?",
                )
                .map_err(|e| DatabaseError::Query { operation: "prepare query".to_owned(), message: e.to_string() })?;

            let rows = stmt
                .query_map(params![chain_id, mint_str, limit], |row| DbPriceResult::from_row(row))
                .map_err(|e| DatabaseError::Query { operation: "query price history".to_owned(), message: e.to_string() })?;

            let mut results = Vec::new();
            for row in rows {
                let db_price = row.map_err(|e| DatabaseError::Query { operation: "read row".to_owned(), message: e.to_string() })?;
                results.push(db_price.to_price_result());
            }

            // Reverse so oldest comes first
            results.reverse();

            Ok::<_, Error>(results)
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Get price history with optional filtering
    pub async fn get_price_history(
        &self,
        mint: &str,
        limit: Option<usize>,
        since_timestamp: Option<i64>,
    ) -> Result<Vec<PriceResult>, Error> {
        let mint_str = mint.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();
        let limit = limit.unwrap_or(PRICE_HISTORY_MAX_ENTRIES);

        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            let mut results = Vec::new();

            if let Some(ts) = since_timestamp {
                let query = format!(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ? AND timestamp_unix >= ?
         ORDER BY timestamp_unix DESC
         LIMIT {limit}"
                );

                let mut stmt = conn
                    .prepare(&query)
                    .map_err(|e| DatabaseError::Query { operation: "prepare query".to_owned(), message: e.to_string() })?;

                let rows = stmt
                    .query_map(params![chain_id, mint_str, ts], |row| DbPriceResult::from_row(row))
                    .map_err(|e| DatabaseError::Query { operation: "query price history".to_owned(), message: e.to_string() })?;

                for row in rows {
                    let db_price = row.map_err(|e| DatabaseError::Query { operation: "read row".to_owned(), message: e.to_string() })?;
                    results.push(db_price.to_price_result());
                }
            } else {
                let query = format!(
                    "SELECT id, chain_id, mint, pool_address, price_usd, price_sol, confidence, slot,
               timestamp_unix, sol_reserves, token_reserves, source_pool, created_at
         FROM price_history
         WHERE chain_id = ? AND mint = ?
         ORDER BY timestamp_unix DESC
         LIMIT {limit}"
                );

                let mut stmt = conn
                    .prepare(&query)
                    .map_err(|e| DatabaseError::Query { operation: "prepare query".to_owned(), message: e.to_string() })?;

                let rows = stmt
                    .query_map(params![chain_id, mint_str], |row| DbPriceResult::from_row(row))
                    .map_err(|e| DatabaseError::Query { operation: "query price history".to_owned(), message: e.to_string() })?;

                for row in rows {
                    let db_price = row.map_err(|e| DatabaseError::Query { operation: "read row".to_owned(), message: e.to_string() })?;
                    results.push(db_price.to_price_result());
                }
            }

            // Reverse so oldest comes first
            results.reverse();

            Ok::<_, Error>(results)
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Cleanup old database entries beyond retention period
    pub async fn cleanup_old_entries(&self) -> Result<usize, Error> {
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            // Calculate cutoff date
            let cutoff_date =
                chrono::Utc::now() - chrono::Duration::days(MAX_PRICE_HISTORY_AGE_DAYS);
            let cutoff_str = cutoff_date.to_rfc3339();

            let deleted = conn
                .execute(
                    "DELETE FROM price_history WHERE chain_id = ? AND created_at < ?",
                    params![chain_id, cutoff_str],
                )
                .map_err(|e| DatabaseError::Query {
                    operation: "cleanup old entries".to_owned(),
                    message: e.to_string(),
                })?;

            Ok::<_, Error>(deleted)
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Cleanup gapped data for a specific token
    /// Removes price history entries older than the first significant gap
    pub async fn cleanup_gapped_data_for_token(&self, mint: &str) -> Result<usize, Error> {
        // Find the cutoff point (first gap > 1 minute)
        let cutoff_timestamp = match self.find_first_price_gap(mint).await? {
            Some(ts) => ts,
            None => return Ok(0), // No gaps found
        };

        // Delete everything older than the cutoff
        let mint_str = mint.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            let deleted = conn
                .execute(
                    "DELETE FROM price_history WHERE chain_id = ? AND mint = ? AND timestamp_unix <= ?",
                    params![chain_id, mint_str, cutoff_timestamp],
                )
                .map_err(|e| DatabaseError::Query { operation: "delete gapped data".to_owned(), message: e.to_string() })?;

            Ok::<_, Error>(deleted)
        })
        .await
        .map_err(InternalError::from)?
    }

    /// Find the first significant gap in price data for a token
    /// Returns the timestamp of the older entry at the gap point
    async fn find_first_price_gap(&self, mint: &str) -> Result<Option<i64>, Error> {
        let mint_str = mint.to_string();
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();

        let timestamps = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            let mut stmt = conn
                .prepare(
                    "SELECT timestamp_unix 
           FROM price_history 
           WHERE chain_id = ? AND mint = ?
           ORDER BY timestamp_unix DESC",
                )
                .map_err(|e| DatabaseError::Query {
                    operation: "prepare gap query".to_owned(),
                    message: e.to_string(),
                })?;

            let rows = stmt
                .query_map(params![chain_id, mint_str], |row| row.get::<_, i64>(0))
                .map_err(|e| DatabaseError::Query {
                    operation: "query timestamps".to_owned(),
                    message: e.to_string(),
                })?;

            let mut timestamps = Vec::new();
            for row in rows {
                timestamps.push(row.map_err(|e| DatabaseError::Query {
                    operation: "read timestamp".to_owned(),
                    message: e.to_string(),
                })?);
            }

            Ok::<_, Error>(timestamps)
        })
        .await
        .map_err(InternalError::from)??;

        if timestamps.len() < 2 {
            return Ok(None); // Not enough data to find a gap
        }

        // Work backwards to find the first gap > 1 minute
        for i in 1..timestamps.len() {
            let current_time = timestamps[i - 1]; // Newer timestamp
            let prev_time = timestamps[i]; // Older timestamp

            let gap = current_time - prev_time;

            if gap > (MAX_PRICE_GAP_SECONDS as i64) {
                // Found a gap - return the older timestamp as cutoff point
                return Ok(Some(prev_time));
            }
        }

        Ok(None) // No significant gaps found
    }

    /// Cleanup gapped data for all tokens
    pub async fn cleanup_all_gapped_data(&self) -> Result<usize, Error> {
        let pool = self.shared_pool()?;
        let chain_id = self.chain_id.as_str().to_owned();

        // Get all unique tokens in the database
        let tokens = tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(|e| DatabaseError::Query {
                operation: "get pooled connection".to_owned(),
                message: e.to_string(),
            })?;

            let mut stmt = conn
                .prepare("SELECT DISTINCT mint FROM price_history WHERE chain_id = ?")
                .map_err(|e| DatabaseError::Query {
                    operation: "prepare token list query".to_owned(),
                    message: e.to_string(),
                })?;

            let rows = stmt
                .query_map([chain_id], |row| Ok(row.get::<_, String>("mint")?))
                .map_err(|e| DatabaseError::Query {
                    operation: "execute token list query".to_owned(),
                    message: e.to_string(),
                })?;

            let mut tokens = Vec::new();
            for row in rows {
                tokens.push(row.map_err(|e| DatabaseError::Query {
                    operation: "parse token mint".to_owned(),
                    message: e.to_string(),
                })?);
            }

            Ok::<_, Error>(tokens)
        })
        .await
        .map_err(InternalError::from)??;

        // Clean up gapped data for each token
        let mut total_deleted = 0;
        for token in tokens {
            match self.cleanup_gapped_data_for_token(&token).await {
                Ok(deleted) => {
                    total_deleted += deleted;
                }
                Err(e) => {
                    logger::error(
                        LogTag::PoolCache,
                        &format!("Failed to cleanup gapped data for token {token}: {e}"),
                    );
                }
            }
        }

        if total_deleted > 0 {
            logger::debug(
                LogTag::PoolService,
                &format!(
                    "Removed {} total gapped price entries across all tokens",
                    total_deleted
                ),
            );
        }

        Ok(total_deleted)
    }
}

/// Test-only: a pooled temp-file store pre-migrated from the legacy
/// single-chain schema fixture (the same seed `migrations::legacy_connection`
/// builds). Pooled checkouts do not share a `:memory:` database, so tests
/// that read back what earlier awaits wrote need a real file.
#[cfg(test)]
pub(super) fn pooled_legacy_database(label: &str) -> PoolsDatabase {
    let path = std::env::temp_dir().join(format!(
        "screenerbot-pools-{label}-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let manager = SqliteConnectionManager::file(&path)
        .with_init(|conn| database::configure_connection(conn, database::POOLS_DB));
    let pool = Pool::builder()
        .max_size(4)
        .idle_timeout(None)
        .max_lifetime(None)
        .build(manager)
        .expect("build pooled test database");
    {
        let mut conn = pool.get().expect("checkout test connection");
        super::migrations::seed_legacy_schema(&conn);
        migrate_schema(&mut conn).expect("migrate test database");
    }
    PoolsDatabase {
        chain_id: ChainId::Solana,
        db_path: path.to_string_lossy().to_string(),
        pool: Some(pool),
        write_queue: None,
        blacklisted_accounts: Arc::new(RwLock::new(HashSet::new())),
        blacklisted_pools: Arc::new(RwLock::new(HashMap::new())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn solana_repository_ignores_raw_rows_from_another_chain() {
        let db = pooled_legacy_database("foreign-chain");
        {
            let conn = db
                .shared_pool()
                .expect("test pool")
                .get()
                .expect("checkout test connection");
            conn.execute(
                "INSERT INTO price_history (chain_id, mint, pool_address, price_usd, price_sol, confidence, slot, timestamp_unix, sol_reserves, token_reserves, created_at)
                 VALUES ('ethereum', 'mint', 'pool', 1.0, 9.0, 1.0, 8, 20, 3.0, 4.0, '2026-01-01T00:00:20Z')",
                [],
            )
            .expect("insert conceptual foreign-chain row");
            conn.execute(
                "INSERT INTO blacklist_pools (chain_id, pool_id, reason, token_mint, error_count, first_failed_at, last_failed_at, added_at)
                 VALUES ('ethereum', 'pool', 'foreign', 'mint', 1, 1, 1, 1)",
                [],
            )
            .expect("insert conceptual foreign-chain blacklist");
        }
        let history = db
            .get_price_history("mint", None, None)
            .await
            .expect("read solana history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].price_sol, 2.0);
        let pools = db
            .list_blacklisted_pools(
                None,
                PoolBlacklistPolicy {
                    threshold: 1,
                    ttl_secs: i64::MAX,
                },
            )
            .await
            .expect("read solana blacklist");
        assert_eq!(pools.len(), 1);
        assert!(pools
            .iter()
            .all(|record| record.chain_id == ChainId::Solana));
    }
}
